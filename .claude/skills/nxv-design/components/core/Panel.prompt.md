**Panel** — the base surface container; everything with a border and background is built on it (feature cards, stat panels, install blocks, modals). Translucent "glass" by default so the blueprint grid shows through.

```jsx
<Panel rail><h3>Blazingly fast</h3><p>Bloom filter + FTS5.</p></Panel>
<Panel glass={false} pad="lg" radius="2xl">…</Panel>
```

Props: `glass` (translucent vs solid), `rail` (left accent bar), `pad` (none/sm/md/lg), `radius` (lg/xl/2xl).
