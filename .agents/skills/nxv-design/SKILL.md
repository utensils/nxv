---
name: nxv-design
description: Use this skill to generate well-branded interfaces and assets for nxv (Nix Version Index) — either for production or throwaway prototypes/mocks. Contains essential design guidelines, colors, type, fonts, assets, and UI kit components for prototyping the "Blueprint" dark developer-tool look.
user-invocable: true
---

Read the `README.md` file within this skill, and explore the other available files.

If creating visual artifacts (slides, mocks, throwaway prototypes, etc.), copy assets out and create static HTML files for the user to view. If working on production code, you can copy assets and read the rules here to become an expert in designing with this brand.

If the user invokes this skill without any other guidance, ask them what they want to build or design, ask some questions, and act as an expert designer who outputs HTML artifacts _or_ production code, depending on the need.

## Quick orientation

- **Look:** "Blueprint" — dark-only, deep cool-navy canvas with a faint 48px engineering grid (masked to fade behind content), the colourful Nix snowflake as the hero mark, JetBrains Mono as the signature voice, one Nix-blue accent.
- **Tokens:** link `styles.css` (it `@import`s everything in `tokens/`). Reference semantic aliases — `--surface-page`, `--surface-glass`, `--text`, `--accent`, `--border`, `--ring-focus`, `--grid-image` — not raw hexes. All colour is oklch.
- **Fonts:** `fonts/inter-var.woff2` (UI/body) + `fonts/jetbrains-mono-var.woff2` (data/code/headings). Real binaries, no substitutes.
- **Assets:** `assets/nix-snowflake.svg` (hero mark + favicon), `assets/nxv-logo-dark.svg` / `nxv-logo-light.svg` (bracket monogram).
- **Icons:** Lucide-style 2px-stroke line icons (`stroke="currentColor"`, no fill). No emoji, no icon font.
- **Voice:** lowercase terminal-native labels, second-person imperative prose, dry wit, concrete numbers and real package names. Mono for anything machine-shaped. Eyebrows read like `// code comments`.

## Components

Compiled to `_ds_bundle.js` under `window.NxvDesignSystem_0fa7ce`. Core: Button, Chip, Kbd, StatusPill, Panel, Metric, SegmentedToggle, Callout, Toast. Product: SearchPrompt, Terminal, VersionBadge, PackageRow, PackageCard, VersionTimeline, ActivityBars. Each component has a `.prompt.md` with a usage example. `ui_kits/search-app/` is a full interactive reference; `Landing.html` is the marketing page.
