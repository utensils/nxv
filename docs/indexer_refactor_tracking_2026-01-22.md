# Indexer Refactor Tracking (2026-01-22)

## Summary
- Added memory-aware parent batching and safer defaults to reduce OOM/restarts.
- Switched to always use batched extraction with worker pools (small lists still fall back internally).
- Added auto worker-count logic (base = systems; scale to 2x only when memory allows) and new tests.
- Improved memory error detection to avoid skipping due to OOM-style failures.

## Code Changes
- Parent batch sizing now scales from per-worker memory (divisors: 128 for metadata, 192 for store paths) with a conservative initial batch size.
- Worker pool batch sizing logs include per-worker caps and starting batch size.
- Memory errors detected from Nix eval messages now propagate as memory errors.
- Auto worker-count logic:
  - Base: one worker per system (default).
  - Scale up to 2x systems only when per-worker memory after scaling is >= 8 GiB.
  - If base per-worker memory drops below 2 GiB, workers are reduced.
- Batched extraction is now used whenever a worker pool is present; small lists shortcut inside the pool.

## 15-Minute Debug Runs (2020-2021)

### Run A (default workers, 32 GiB)
- Command: `target/release/nxv index --nixpkgs-path nixpkgs --since 2020-01-01 --until 2021-01-01 --systems x86_64-linux,aarch64-linux,x86_64-darwin,aarch64-darwin --max-memory 32GiB --full`
- Log: `tmp/indexer-runs/run-2020-2021-15m-1.log.2026-01-22`
- Elapsed: ~898s
- Max progress: 450 / 21072 commits (~30.1 commits/min)
- Worker restarts: 49
- Batch reductions: 42
- Worker deaths: 0
- Store-path failures: 357 (eval-failed, not memory-related)

### Run B (workers=8, 32 GiB)
- Config: `tmp/indexer-runs/indexer-config-8w.json` with `{ "workers": 8 }`
- Log: `tmp/indexer-runs/run-2020-2021-15m-8w.log.2026-01-22`
- Elapsed: ~894s
- Max progress: 50 / 21072 commits (~3.4 commits/min)
- Worker restarts: 289
- Batch reductions: 266
- Worker deaths: 2
- Outcome: 8 workers at 4 GiB each thrashes; 4 workers is faster and more stable at 32 GiB.

## Next Steps
- Run 1–2h soak with 4 workers @ 32 GiB to confirm long-run stability.
- If stable, run 12h index up to 2026 as requested.

## Verification
- `cargo fmt`
- `cargo clippy --features indexer -- -D warnings`
- `cargo test --features indexer` (unit: 598 passed, 11 ignored; integration: 67 passed, 4 ignored)
