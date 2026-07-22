**Button** — the primary action control; mono-labelled to match nxv's CLI voice. Use `primary` for the single main CTA per view, `ghost` for secondary actions, `default` otherwise.

```jsx
<Button variant="primary" prompt>try it now</Button>
<Button variant="ghost" iconRight={<Arrow/>}>read the guide</Button>
<Button size="sm" variant="default">copy</Button>
```

Variants: `primary` (solid Nix blue + glow), `default` (raised surface + hairline), `ghost` (transparent + hairline). Sizes: `sm` / `md` / `lg`. Set `prompt` to lead the label with a `$` sigil. Renders as `<a>` when given `href`.
