**Metric** — a big mono figure with a small uppercase label, in a railed Panel. Lay four across a grid for the nxv index-stats row.

```jsx
<div style={{display:'grid',gridTemplateColumns:'repeat(4,1fr)',gap:14}}>
  <Metric value="186,417" label="packages" />
  <Metric value="4.2M" label="versions" />
  <Metric value="98,317" label="commits" />
  <Metric value="9+ yrs" label="of history" />
</div>
```
