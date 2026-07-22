**Pagination** — the results pager: a mono "page x / y · showing a–b of n" readout with prev/next ghost buttons. Sits beneath a results list.

```jsx
<Pagination page={page} pageSize={50} total={1047}
  onPrev={() => setPage(p => p-1)} onNext={() => setPage(p => p+1)} />
```

Derives total pages and the showing-range from `page`/`pageSize`/`total`; pass `hasMore` to override the next-button state for cursor-style APIs.
