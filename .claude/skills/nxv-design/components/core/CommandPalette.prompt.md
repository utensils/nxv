**CommandPalette** — nxv's ⌘K jump-to overlay: a search input over a list of jump/command items with keyboard hints. Render at the app root and drive `open` from a ⌘K key handler.

```jsx
<CommandPalette open={open} onClose={close} onSubmit={runSearch}
  items={[
    { label: 'python', hint: 'popular package', icon: <SearchIcon/>, onSelect: () => run('python') },
    { label: 'clear filters', hint: 'reset', onSelect: resetFilters },
  ]} />
```

Use `embedded` to render just the panel (no fixed overlay) for specimens or a docked command bar.
