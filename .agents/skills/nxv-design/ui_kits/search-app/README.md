# Search App — UI kit

A high-fidelity, interactive recreation of the **nxv search web interface** in
the Blueprint design language. This is the fresh redesign that replaces the
original terminal/CRT frontend (`frontend/` in `utensils/nxv`).

Open `index.html`. It composes design-system components only — no primitive is
re-implemented here.

## What's interactive

- **Search** — type in the terminal-style `SearchPrompt`; the fake dataset
  filters live (name / attr / description).
- **Filters** — `exact`, `sort`, and `include insecure` chips cycle and
  re-filter/sort results.
- **View toggle** — `SegmentedToggle` switches results between dense
  `PackageRow`s and a `PackageCard` grid.
- **History drawer** — click any row/card to open the right-side drawer with a
  `VersionTimeline` and the full version list.
- **Command palette** — `⌘K` (or the header button) opens a jump-to palette;
  `esc` closes any overlay.
- **Copy toast** — the copy actions raise a `Toast` with the `nix shell` command.

## Files

- `index.html` — the whole kit: page shell + inlined data + app composition
  (Header, Hero, Results, StatsRow, Drawer, CommandPalette, Pagination).
  Inlined (not a sibling `.jsx`) so the app stays out of the shared component
  bundle.

## Components used

`SearchPrompt`, `PackageRow`, `PackageCard`, `VersionBadge`, `VersionTimeline`,
`ActivityBars`, `SegmentedToggle`, `Chip`, `Button`, `Kbd`, `StatusPill`,
`Metric`, `Panel`, `Toast`, `CommandPalette`, `Pagination`.

The marketing landing page (`Landing.html` at the project root) is the other
surface built on the same tokens.
