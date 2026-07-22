**Chip** — compact mono token for filters, package tags, platforms, and status flags. Add `onClick` to make it an interactive filter chip.

```jsx
<Chip tone="active">sort: date</Chip>
<Chip tone="danger" icon={<ShieldIcon/>}>insecure</Chip>
<Chip tone="warn">pre-flakes</Chip>
<Chip size="sm">x86_64·linux</Chip>
```

Tones: `default`, `active` (selected, Nix-blue wash), `ok`, `warn` (amber, pre-flakes), `danger` (red, insecure). Sizes `sm` / `md`.
