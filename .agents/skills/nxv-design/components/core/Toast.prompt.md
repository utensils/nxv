**Toast** — the transient confirmation notice (nxv shows one on copy). Mono, sits bottom-right, controlled by `open`.

```jsx
<Toast open={copied}>copied · nix shell nixpkgs/e4a45f9#python27</Toast>
```

Drive `open` from state and auto-hide after ~1.8s. Position it `fixed` at the app root.
