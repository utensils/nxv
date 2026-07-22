**PackageCard** — the card/grid presentation of a search result. Same `pkg` shape as PackageRow, plus a first/last/rev meta strip and three actions (flake / run / history). Lay out in an auto-fill grid (`minmax(296px,1fr)`).

```jsx
<PackageCard pkg={record} onCopyFlake={copyFlake} onCopyRun={copyRun} onHistory={openDrawer} />
```

Tone auto-derives from `insecure` / `legacy`. Toggle against PackageRow with a `SegmentedToggle`.
