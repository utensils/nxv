# Docs Site — UI kit

A high-fidelity recreation of the **nxv documentation site** (originally
VitePress) in the Blueprint design language — the second product surface after
the search web app.

Open `index.html`. It composes design-system components (`Callout`, `Kbd`,
`StatusPill`, `Button`, `Terminal`) plus docs-kit-local layout pieces
(`CodeBlock`, `Table`, sidebar, on-this-page rail).

## What's interactive

- **Sidebar + top nav** switch between four real doc pages (Getting Started,
  Installation, CLI Reference, HTTP API) with active states.
- **Prev / next** footer buttons walk the page order.
- **On-this-page** rail mirrors each page's headings.

Content is lifted from the real `utensils/nxv` docs (`website/guide/*`,
`website/api/*`).

## Files

- `index.html` — the whole kit: page shell + inlined app (nav model, page
  content, TopBar, Sidebar, Toc). Inlined (not a sibling `.jsx`) so it stays out
  of the shared component bundle.

## Branding

Carries the common nxv brand: the Nix-snowflake + `nxv` lockup, the
`// nix version index` eyebrow style, and the shared slogan —
**"Find any version of any Nix package, instantly."** — matching `Landing.html`
and the search-app kit.
