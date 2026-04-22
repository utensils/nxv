#!/usr/bin/env python3
"""Static WCAG 2.1 audit for nxv's frontend.

Parses frontend/index.html (or any path passed on the command line), enforces
a markup rule set tuned to this project, and audits contrast of the oklch
design tokens declared in the `@theme` block.

Exit code: 0 on success, 1 if any error-level findings, 2 on usage errors.
"""

from __future__ import annotations

import math
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

from bs4 import BeautifulSoup, Tag
import wcag_contrast_ratio as wcag


RESET = "\033[0m"
RED = "\033[31m"
YELLOW = "\033[33m"
GREEN = "\033[32m"
DIM = "\033[2m"
BOLD = "\033[1m"


@dataclass
class Finding:
    severity: str  # "error" | "warn" | "info"
    rule: str
    message: str
    where: str = ""
    hint: str = ""


@dataclass
class Report:
    findings: list[Finding] = field(default_factory=list)

    def error(self, rule: str, message: str, where: str = "", hint: str = "") -> None:
        self.findings.append(Finding("error", rule, message, where, hint))

    def warn(self, rule: str, message: str, where: str = "", hint: str = "") -> None:
        self.findings.append(Finding("warn", rule, message, where, hint))

    def info(self, rule: str, message: str, where: str = "", hint: str = "") -> None:
        self.findings.append(Finding("info", rule, message, where, hint))

    def has_errors(self) -> bool:
        return any(f.severity == "error" for f in self.findings)

    def print(self, use_color: bool) -> None:
        def c(code: str, text: str) -> str:
            return f"{code}{text}{RESET}" if use_color else text

        if not self.findings:
            print(c(GREEN, "✓ no a11y findings"))
            return

        by_sev = {"error": [], "warn": [], "info": []}
        for f in self.findings:
            by_sev[f.severity].append(f)

        for sev, color in (("error", RED), ("warn", YELLOW), ("info", DIM)):
            items = by_sev[sev]
            if not items:
                continue
            print(c(BOLD, f"{sev.upper()} ({len(items)})"))
            for f in items:
                head = f"  {c(color, '•')} [{f.rule}] {f.message}"
                if f.where:
                    head += c(DIM, f"  — {f.where}")
                print(head)
                if f.hint:
                    print(c(DIM, f"      → {f.hint}"))
            print()

        counts = ", ".join(
            f"{len(by_sev[s])} {s}" for s in ("error", "warn", "info") if by_sev[s]
        )
        print(c(BOLD, f"summary: {counts}"))


# ---------------------------------------------------------------------------
# oklch → sRGB, then wcag contrast.
# Math per https://bottosson.github.io/posts/oklab/ — standard D65 sRGB pipeline.


def _oklch_to_oklab(L: float, C: float, h_deg: float) -> tuple[float, float, float]:
    h = math.radians(h_deg)
    return (L, C * math.cos(h), C * math.sin(h))


def _oklab_to_linear_srgb(L: float, a: float, b: float) -> tuple[float, float, float]:
    l_ = L + 0.3963377774 * a + 0.2158037573 * b
    m_ = L - 0.1055613458 * a - 0.0638541728 * b
    s_ = L - 0.0894841775 * a - 1.2914855480 * b
    l, m, s = l_ ** 3, m_ ** 3, s_ ** 3
    return (
        +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    )


def _linear_to_srgb(c: float) -> float:
    c = max(0.0, min(1.0, c))
    if c <= 0.0031308:
        return 12.92 * c
    return 1.055 * (c ** (1.0 / 2.4)) - 0.055


def oklch_to_srgb(L: float, C: float, h: float) -> tuple[float, float, float]:
    lab = _oklch_to_oklab(L, C, h)
    lin = _oklab_to_linear_srgb(*lab)
    return tuple(_linear_to_srgb(c) for c in lin)  # type: ignore[return-value]


_OKLCH_RE = re.compile(
    r"oklch\(\s*([0-9.]+)\s+([0-9.]+)\s+([0-9.]+)(?:\s*/\s*[0-9.]+)?\s*\)"
)


def parse_oklch(value: str) -> tuple[float, float, float] | None:
    m = _OKLCH_RE.search(value)
    if not m:
        return None
    return (float(m.group(1)), float(m.group(2)), float(m.group(3)))


# ---------------------------------------------------------------------------
# Design tokens — extracted from the @theme block inside <style type="text/tailwindcss">.


_THEME_BLOCK_RE = re.compile(r"@theme\s*\{(.*?)\}", re.DOTALL)
_TOKEN_RE = re.compile(r"--color-([a-z0-9-]+)\s*:\s*([^;]+?)\s*;")


def extract_color_tokens(html: str) -> dict[str, tuple[float, float, float]]:
    """Return {name: (L, C, h)} for each `--color-*: oklch(...)` token in @theme."""
    tokens: dict[str, tuple[float, float, float]] = {}
    for theme in _THEME_BLOCK_RE.finditer(html):
        for name, value in _TOKEN_RE.findall(theme.group(1)):
            parsed = parse_oklch(value)
            if parsed:
                tokens[name] = parsed
    return tokens


# Documented foreground/background pairs used across the design.
# (fg_token, bg_token, role) — role controls the WCAG threshold.
# role="text": 4.5:1 required (AA normal text)
# role="large": 3.0:1 required (AA large / bold text)
# role="ui":   3.0:1 required (AA non-text UI component boundary)
CONTRAST_PAIRS: list[tuple[str, str, str]] = [
    # body text on primary surfaces
    ("fog-0", "ink-0", "text"),
    ("fog-1", "ink-0", "text"),
    ("fog-2", "ink-0", "text"),
    ("fog-3", "ink-0", "text"),
    ("fog-4", "ink-0", "text"),
    ("fog-0", "ink-1", "text"),
    ("fog-1", "ink-1", "text"),
    ("fog-2", "ink-1", "text"),
    ("fog-3", "ink-1", "text"),
    ("fog-4", "ink-1", "text"),
    ("fog-2", "ink-2", "text"),
    ("fog-3", "ink-2", "text"),
    ("fog-4", "ink-2", "text"),
    # accent / semantic on ground
    ("nix-300", "ink-0", "text"),
    ("nix-400", "ink-0", "text"),
    ("phos-400", "ink-0", "text"),
    ("amber-glow", "ink-0", "text"),
    ("red-glow", "ink-0", "text"),
    ("green-glow", "ink-0", "text"),
    # primary button: white text on nix-600
    # we synthesize "white" below
]


def audit_contrast(
    tokens: dict[str, tuple[float, float, float]], report: Report
) -> None:
    if not tokens:
        report.warn(
            "theme-tokens",
            "no `--color-*: oklch(...)` tokens found in @theme block",
            hint="skipped contrast audit — verify the inline <style type=\"text/tailwindcss\"> block is present",
        )
        return

    for fg, bg, role in CONTRAST_PAIRS:
        if fg not in tokens or bg not in tokens:
            continue
        fg_rgb = oklch_to_srgb(*tokens[fg])
        bg_rgb = oklch_to_srgb(*tokens[bg])
        ratio = wcag.rgb(fg_rgb, bg_rgb)
        threshold = 4.5 if role == "text" else 3.0
        where = f"--color-{fg} on --color-{bg}"
        if ratio < threshold:
            report.error(
                "contrast",
                f"{ratio:.2f}:1 fails WCAG AA ({role}: needs ≥{threshold}:1)",
                where=where,
                hint="darken the background or lighten the foreground token until ratio ≥ threshold",
            )
        elif ratio < threshold + 1.5:
            report.info(
                "contrast",
                f"{ratio:.2f}:1 passes AA ({role} ≥{threshold}:1) but below AAA",
                where=where,
            )


# ---------------------------------------------------------------------------
# Markup rules.


def _selector(tag: Tag) -> str:
    tag_name = tag.name or "?"
    id_ = tag.get("id")
    cls = tag.get("class") or []
    bits = [tag_name]
    if id_:
        bits.append(f"#{id_}")
    if cls:
        bits.append("." + ".".join(cls[:2]))
    return "".join(bits)


def _accessible_text(tag: Tag) -> str:
    if tag.get("aria-label"):
        return str(tag["aria-label"]).strip()
    if tag.get("aria-labelledby"):
        return f"[labelledby:{tag['aria-labelledby']}]"
    text = tag.get_text(strip=True)
    if text:
        return text
    # titled child svg/img counts as an accessible name
    title = tag.find("title")
    if title and title.get_text(strip=True):
        return title.get_text(strip=True)
    for child in tag.find_all(["svg", "img"]):
        if child.get("aria-label") or child.get("title"):
            return str(child.get("aria-label") or child.get("title"))
    return ""


def check_document(soup: BeautifulSoup, report: Report) -> None:
    html_tag = soup.find("html")
    if not isinstance(html_tag, Tag) or not html_tag.get("lang"):
        report.error(
            "html-lang",
            "<html> element has no lang attribute",
            hint='add lang="en" (or appropriate) so assistive tech announces the language',
        )

    title = soup.find("title")
    if not title or not title.get_text(strip=True):
        report.error("title", "missing or empty <title>")

    body = soup.find("body")
    if not isinstance(body, Tag):
        report.error("body", "no <body> element found")


def check_landmarks(soup: BeautifulSoup, report: Report) -> None:
    main_tags = soup.find_all("main")
    if len(main_tags) == 0:
        report.error(
            "landmark-main",
            "no <main> landmark",
            hint="wrap the primary page content in <main> so screen-reader users can jump to it",
        )
    elif len(main_tags) > 1:
        report.warn("landmark-main", f"{len(main_tags)} <main> elements — there should be exactly one")

    for lm in ("header", "nav", "footer"):
        if not soup.find(lm):
            report.warn(
                "landmark",
                f"no <{lm}> landmark found",
                hint=f"use <{lm}> to group page chrome; helps landmark navigation",
            )


def check_skip_link(soup: BeautifulSoup, report: Report) -> None:
    body = soup.find("body")
    if not isinstance(body, Tag):
        return
    # First focusable descendant should be a same-page link targeting #main-ish.
    first_focusable = body.find(lambda t: isinstance(t, Tag) and t.name in ("a", "button"))
    if not isinstance(first_focusable, Tag):
        return
    is_skip = (
        first_focusable.name == "a"
        and str(first_focusable.get("href", "")).startswith("#")
        and "skip" in (first_focusable.get_text(strip=True) or "").lower()
    )
    if not is_skip:
        report.error(
            "skip-link",
            "no skip-to-content link as first focusable element in <body>",
            hint='add e.g. <a href="#main" class="sr-only focus:not-sr-only">Skip to main content</a> and ensure <main id="main">',
        )


def check_form_controls(soup: BeautifulSoup, report: Report) -> None:
    for ctrl in soup.find_all(["input", "textarea", "select"]):
        if ctrl.get("type") in ("hidden", "submit", "button", "reset", "image"):
            continue
        id_ = ctrl.get("id")
        has_label = False
        if id_:
            has_label = bool(soup.find("label", attrs={"for": id_}))
        has_label = has_label or bool(ctrl.get("aria-label") or ctrl.get("aria-labelledby"))
        # title as a last-resort accessible name
        has_label = has_label or bool(ctrl.get("title"))
        # wrapped <label>input</label> pattern
        if not has_label:
            parent = ctrl.find_parent("label")
            has_label = parent is not None
        if not has_label:
            report.error(
                "form-label",
                "form control has no associated label",
                where=_selector(ctrl),
                hint='attach a <label for="id">, aria-label, or aria-labelledby — placeholder text is not a label',
            )


def check_images(soup: BeautifulSoup, report: Report) -> None:
    for img in soup.find_all("img"):
        if img.get("alt") is None:
            report.error(
                "img-alt",
                "<img> missing alt attribute",
                where=_selector(img),
                hint='add alt="" for decorative images, or descriptive alt for meaningful ones',
            )


def check_svgs(soup: BeautifulSoup, report: Report) -> None:
    for svg in soup.find_all("svg"):
        if svg.get("aria-hidden") == "true":
            continue
        if svg.get("role") == "img" and (svg.get("aria-label") or svg.find("title")):
            continue
        # svg embedded inside a button/link with its own accessible name is OK
        parent = svg.find_parent(lambda t: isinstance(t, Tag) and t.name in ("a", "button"))
        if parent is not None and _accessible_text(parent).replace(
            _accessible_text(svg), ""
        ).strip():
            continue
        report.warn(
            "svg-a11y",
            "decorative <svg> should be hidden, or meaningful <svg> needs role+label",
            where=_selector(svg),
            hint='add aria-hidden="true" if decorative, or role="img" + <title> / aria-label if informative',
        )


def check_headings(soup: BeautifulSoup, report: Report) -> None:
    headings = soup.find_all(re.compile(r"^h[1-6]$"))
    levels = [int(h.name[1]) for h in headings]
    if not levels:
        report.warn("headings", "no headings found")
        return
    if 1 not in levels:
        report.warn("headings", "no <h1> on the page")
    prev = 0
    for i, lvl in enumerate(levels):
        if prev and lvl > prev + 1:
            report.warn(
                "heading-skip",
                f"heading jumps from h{prev} to h{lvl} — skipping levels",
                where=_selector(headings[i]),
                hint="don't skip levels — a user navigating by heading expects a continuous outline",
            )
        prev = lvl


def check_interactive_text(soup: BeautifulSoup, report: Report) -> None:
    for el in soup.find_all(["button", "a"]):
        if el.name == "a" and not el.get("href"):
            continue  # anchor without href isn't focusable
        if el.get("aria-hidden") == "true":
            continue
        if not _accessible_text(el):
            report.error(
                "accessible-name",
                f"<{el.name}> has no accessible name",
                where=_selector(el),
                hint="add text content, aria-label, or a titled <svg> child",
            )


def check_dialog_roles(soup: BeautifulSoup, report: Report) -> None:
    for el in soup.find_all(attrs={"role": "dialog"}):
        if el.get("aria-modal") != "true":
            report.warn(
                "dialog-modal",
                'role="dialog" missing aria-modal="true"',
                where=_selector(el),
            )
        if not (el.get("aria-label") or el.get("aria-labelledby")):
            report.error(
                "dialog-label",
                'role="dialog" needs aria-label or aria-labelledby',
                where=_selector(el),
            )


# Callable ordering for the markup pass.
MARKUP_CHECKS = [
    check_document,
    check_landmarks,
    check_skip_link,
    check_form_controls,
    check_images,
    check_svgs,
    check_headings,
    check_interactive_text,
    check_dialog_roles,
]


# ---------------------------------------------------------------------------
# Entry point.


def run(paths: Iterable[Path]) -> int:
    any_errors = False
    for path in paths:
        html = path.read_text(encoding="utf-8")
        soup = BeautifulSoup(html, "html.parser")
        report = Report()

        print(f"{BOLD}a11y check · {path}{RESET}")
        for check in MARKUP_CHECKS:
            check(soup, report)
        audit_contrast(extract_color_tokens(html), report)
        report.print(use_color=sys.stdout.isatty())
        if report.has_errors():
            any_errors = True
        print()

    return 1 if any_errors else 0


def main(argv: list[str]) -> int:
    args = argv[1:]
    if not args:
        args = ["frontend/index.html"]
    paths = [Path(a) for a in args]
    missing = [p for p in paths if not p.exists()]
    if missing:
        for p in missing:
            print(f"error: {p} does not exist", file=sys.stderr)
        return 2
    return run(paths)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
