**StatusPill** — a bordered mono pill with a glowing status dot. The nxv header uses it for live API health and latency.

```jsx
<StatusPill tone="ok">api operational · p50 34ms</StatusPill>
<StatusPill tone="danger">api unreachable</StatusPill>
```

Tones: `ok` (green), `warn` (amber), `danger` (red), `idle` (muted, no glow).
