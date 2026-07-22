**PackageRow** — a dense, scannable search-result row (package · attr, version, description + platform/flag chips, date, actions). Built on Chip + VersionBadge. Render several inside a headed list container.

```jsx
<PackageRow pkg={{name:'python27', attr:'python27', version:'2.7.18',
  description:'High-level dynamically-typed language', license:'Python-2.0',
  hash:'e4a45f9', first:'Jan 14, 2022', platforms:['x86_64·linux'],
  insecure:true}} onCopy={copyFlake} onHistory={openDrawer} />
```

Tone auto-derives: `insecure` → danger, `legacy` → warn, else brand. Pair with `PackageCard` behind a `SegmentedToggle`.
