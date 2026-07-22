# nxv Design System — "Blueprint"

A fresh, dark-only design system for **nxv — Nix Version Index**, a blazingly
fast CLI (and web app + API) for finding any version of any Nix package. This
system replaces nxv's original terminal/CRT web frontend with a calmer,
confident, brand-forward look built on the same developer-tool soul.

> **Direction:** the "Blueprint" aesthetic — a deep cool-navy canvas with a
> faint engineering grid, the colourful Nix snowflake as the hero mark,
> JetBrains Mono as the signature voice, and Nix blue as the single accent.
> Chosen by the user from four explored landing directions.

## Sources

Everything here is grounded in the real product. Explore these to build even
better nxv work:

- **GitHub — `utensils/nxv`:** <https://github.com/utensils/nxv>
  - `frontend/` — the original search web app (vanilla JS + Tailwind, the CRT
    look we're replacing). Source of the component inventory, data shapes, and
    interaction model.
  - `website/` — the VitePress documentation site (there is **no** Astro code;
    the docs are VitePress with a CSS theme override). Source of the docs
    components (callouts, feature cards) and copy/tone.
  - `README.md`, `website/guide/*`, `website/api/*`, `src/skill/SKILL.md` —
    product copy, CLI reference, API shapes, tone.
- **Live product:** <https://nxv.urandom.io> (web UI) · docs at
  <https://utensils.io/nxv/>
- Built for the nix community by **@jamesbrink** / utensils.io.

Fonts (Inter, JetBrains Mono) and logos (bracket monogram, Nix snowflake) are
the **real** binaries/SVGs copied from the repo — no substitutions were needed.

---

## Content Fundamentals

How nxv writes. Match this voice in any nxv copy.

- **Slogan (canonical).** **“Find any version of any Nix package, instantly.”**
  Use this exact string as the primary tagline on every surface (page
  `<title>`s, docs lead, landing). The landing renders the punchy stylized
  form — *Any version. Any package. Instantly.* — as its hero display, with the
  full slogan beneath it.
- **Eyebrow (canonical).** `// nix version index` — lowercase, mono,
  comment-style. The one section kicker that identifies the product.
- **Lockup (canonical).** The Nix snowflake + the `nxv` wordmark (JetBrains
  Mono 700), in that order, ~26px. Used in every header.
- **Lowercase, terminal-native.** UI labels, nav, chips, and commands are
  lowercase mono: `search`, `stats`, `api`, `sort: date`, `include insecure`.
  Prose headings use sentence case, never Title Case.
- **Direct and a little dry-witty.** The product has a sense of humour but never
  at the cost of clarity. Real examples from the repo: *"Because sometimes you
  need Python 2.7 for that legacy project nobody wants to touch."* · *"Or Ruby
  2.6 because the Gemfile hasn't been updated since the Obama administration."*
  · *"Stop spelunking through GitHub commits and praying to the Nix gods."*
- **Second person, imperative.** Talk to *you*; lead with verbs — "Find the
  exact commit", "Download the index once", "Query locally".
- **Concrete over abstract.** Always ground claims in real numbers and real
  package names: `python27`, `nodejs 18`, `nixpkgs/e4a45f9#python27`, "~220MB
  index", "9+ years", "p50 34ms". Prefer a `nix shell` one-liner to a paragraph.
- **Mono for anything machine-shaped.** Commands, attribute paths, versions,
  hashes, dates, counts — all monospace, always. Slashed zero on figures.
- **Eyebrows are comment-style.** Section kickers read like code comments:
  `// nix version index`, `// why nxv`, `// how it works`.
- **No emoji in the redesign.** The old VitePress home used emoji feature icons
  (⚡📦🔒🛡️); Blueprint replaces them with line icons. Keep copy emoji-free.
- **Punctuation flourishes:** the middot `·` as a separator, the box pipe `│`
  as a divider, `→` for ranges/flow, `›` as an attribute sigil, `$` / `nxv:~$`
  as prompt sigils, `↵ / ⌘K` for shortcuts.

---

## Visual Foundations

- **Theme — dark only.** No light mode (by design). One canvas: deep cool-navy
  ink (`--ink-850`, hue ~264). Never ship a light variant.
- **Colour.** A cool-navy **ink** surface scale (page → panel → border), a
  bright **fog** text scale, and a single **Nix-blue** accent (`--nix-400/500/600`,
  hue ~258). Semantic status only where it means something: green =
  operational/ok, amber = pre-flakes/warn, red = insecure/danger. Max one
  accent hue plus status. All colours are authored in **oklch** for even tonal
  steps — extend harmonically, don't invent hexes.
- **Type.** Two families: **Inter** (UI, body, section headings) and
  **JetBrains Mono** (the brand voice — display headlines, eyebrows, labels,
  all data/code). Hero display is mono 700 at `--tracking-display` (-0.045em);
  section headings are Inter 700 at -0.03em; body is Inter 15–19px.
- **Backgrounds.** The signature is the **blueprint grid** (`--grid-image`, a
  48px hairline grid) laid over the ink canvas and **masked with a radial
  gradient** so it fades out behind content — never a flat full-strength grid.
  A single soft radial **accent glow** sits behind the hero. No photography, no
  illustration except the Nix snowflake. No busy gradients.
- **The snowflake.** The colourful Nix snowflake is the one illustrative mark —
  shown large in the hero with a blue drop-glow (`--glow-snowflake`), gently
  floating (7s ease-in-out). It also serves as the favicon.
- **Borders & surfaces.** Hairline borders everywhere (`--border`, 1px). Panels
  are translucent "glass" (`--surface-glass`) so the grid shows through; code
  wells and the terminal use the deepest ink (`--ink-900`). Cards and metrics
  carry a subtle 2px left **accent rail** at 50% opacity.
- **Corner radii.** Soft-but-technical: chips/kbd 7px, buttons/inputs 9px,
  cards/metrics 13px, panels 16px. Pills and status dots only are fully round.
- **Elevation.** Depth is quiet — thin hairlines plus one downward shadow
  (`--shadow-md` on hover, `--shadow-lg` on the terminal). The primary button
  gets a blue **glow** (`--glow-accent`) rather than a hard shadow.
- **Motion.** Restrained. 120–220ms `--ease` cubic transitions; hover lifts
  elements `translateY(-1px/-2px)`; the snowflake floats; the search caret
  blinks (1.1s steps). No bounces, no parallax.
- **Hover / press.** Hover: border shifts to `--accent-hover`, text brightens to
  `--text-heading`, surface fills to `--surface-hover`, slight lift. Primary
  buttons brighten to `--accent-hover`. Press/active is a colour change, not a
  shrink.
- **Focus.** A 3px Nix-blue ring (`--ring-focus`) on inputs and interactive
  elements. The search shell lifts its border to the accent and gains the ring
  on focus-within.
- **Transparency & blur.** Sticky header uses `--blur-nav` (12px) over a
  translucent ink. Modal/drawer backdrops use `--overlay` + `--blur-overlay`
  (3px + slight saturate) — dimmed toward the page ink, never pitch black.
- **Layout.** Centered `--container` (1160–1240px) with 40px gutters and calm
  `--section-y` (72px) rhythm. Density lives *inside* data components (rows,
  chips, tables), not in the page spacing.

---

## Iconography

- **UI icons: Lucide-style line icons.** The product draws inline SVGs with
  `stroke="currentColor"`, `stroke-width="2"`, no fill (Feather/Lucide family):
  magnifier, clock, copy, play, shield, lightning, box, etc. Reuse these inline
  or pull **[Lucide](https://lucide.dev)** from CDN — it's the exact match for
  stroke weight and style. Size 12–21px in the UI; icons sit in 40px rounded
  accent-wash tiles in feature cards.
- **The one illustrative mark: the Nix snowflake** (`assets/nix-snowflake.svg`),
  a filled multi-blue gradient hexagon. Used for the favicon and the hero mark.
- **Brand monogram:** `assets/nxv-logo-dark.svg` / `nxv-logo-light.svg` — the
  `[nxv]` bracket monogram (JetBrains Mono wordmark cradled by two brackets).
  Dark = `#86a4e5` for dark backgrounds; light = `#3a55a3` for light.
- **No icon font, no emoji, no unicode-glyph icons** in the redesign. Status is
  shown with a glowing coloured dot (`StatusPill`), not an emoji. Decorative
  ASCII/box-drawing from the old UI is dropped (the `│ · → ›` punctuation stays
  as inline text separators, not as iconography).
- Never hand-draw or approximate the Nix logo — the copied SVG is the only
  source of truth for it.

---

## Component index

React primitives live under `components/`. Import from the compiled bundle:
`const { Button } = window.NxvDesignSystem_0fa7ce`. Each has a `.d.ts` (props),
a `.prompt.md` (usage), and a specimen in its directory's `@dsCard` HTML.

**Core** (`components/core/`) — generic primitives:

- **Button** — mono action button; `primary` / `default` / `ghost`, three sizes, optional `$` prompt sigil.
- **Chip** — compact mono token for filters, tags, platforms, flags; tones default/active/ok/warn/danger.
- **Kbd** — keyboard-key cap for shortcut hints.
- **StatusPill** — bordered pill with a glowing status dot (API health).
- **Panel** — base surface container (glass over grid, optional accent rail).
- **Metric** — big mono figure + label in a railed panel.
- **SegmentedToggle** — mono segmented control (results rows/cards).
- **Callout** — docs admonition (tip / info / warn / danger).
- **Toast** — transient copy/confirmation notice.
- **CommandPalette** — the ⌘K jump-to overlay (search input + item list).
- **Pagination** — results pager (range readout + prev/next buttons).

**Product** (`components/product/`) — nxv-domain widgets:

- **SearchPrompt** — the signature terminal-style search bar (`nxv:~$ search …`).
- **Terminal** — window-chrome card wrapping syntax-coloured mono output.
- **VersionBadge** — version tag whose tone signals status.
- **PackageRow** — dense scannable search-result row.
- **PackageCard** — card/grid presentation of a search result.
- **VersionTimeline** — version lifespan bars over a time axis (history drawer).
- **ActivityBars** — request-activity sparkline (stats panel).

_Composed, not primitives:_ feature cards are built from `Panel` + an icon
inside the UI kits rather than shipped separately — the source defines them as
compositions. (The command palette and pagination, previously composed, are now
first-class components: `CommandPalette` and `Pagination`.)

---

## Index / manifest

- **`styles.css`** — root entry point (consumers link this). `@import` manifest only.
- **`tokens/`** — `fonts.css`, `colors.css`, `typography.css`, `spacing.css`,
  `radius.css`, `effects.css` (123 CSS custom properties: base scales + semantic aliases).
- **`fonts/`** — `inter-var.woff2`, `jetbrains-mono-var.woff2`.
- **`assets/`** — `nix-snowflake.svg`, `nxv-logo-dark.svg`, `nxv-logo-light.svg`.
- **`components/core/`**, **`components/product/`** — the 16 primitives above,
  each with `.jsx` + `.d.ts` + `.prompt.md`, plus one `@dsCard` specimen per dir.
- **`guidelines/`** — foundation specimen cards (Colors, Type, Spacing, Brand).
- **`ui_kits/search-app/`** — interactive recreation of the nxv search web app
  (see its `README.md`).
- **`ui_kits/docs-site/`** — interactive recreation of the nxv documentation
  site (VitePress-style: top nav, sidebar, article, on-this-page rail; see its
  `README.md`).
- **`Landing.html`** (root) — the marketing landing page (the Blueprint direction).
- **`SKILL.md`** — Agent-Skills-standard entry for using this system in Claude Code.
- **`thumbnail.html`** — project tile for the design-system homepage.

The Design System tab renders every `@dsCard`-tagged card, grouped: **Colors,
Type, Spacing, Brand** (foundations), **Components** (primitives), **Search
App** and **Docs Site** (the kits).

---

## Caveats & next steps

- Both product surfaces are covered: the **search web app** and the
  **documentation site**. All three artifacts (plus `Landing.html`) share the
  canonical brand — snowflake+`nxv` lockup, `// nix version index` eyebrow, and
  the slogan *“Find any version of any Nix package, instantly.”*
- The UI kits use **fake data** mirroring the real `/api/v1` shapes and the
  real docs content; wire to the live API / MD source when productionising.
- The command palette and pagination are shipped as `CommandPalette` /
  `Pagination` components; feature cards remain **composed inside the kits**
  (`Panel` + an icon) rather than shipped as a standalone primitive.
- The UI-kit apps are **inlined into their `index.html`** (not sibling `.jsx`)
  so they stay out of the shared `_ds_bundle.js` — the bundle carries only the
  16 components + tokens.
