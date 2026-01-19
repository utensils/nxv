# Indexer Refactor Tracking

Branch: `refactor/indexer`
Owner: Codex (GPT-5)

## Goals

- Re-architect the indexer around deterministic file-to-attribute mapping with minimal CLI surface.
- Preserve Nix C API evaluation while improving incremental coverage and speed.
- Ensure every attribute/version is captured without relying on fragile attrpos mapping.

## Proposed Refactor (High-Level)

1. **Indexer Pipeline**
   - Stage 1: DB-derived attribute catalog (attribute -> source_path, complete attr list).
   - Stage 2: Static all-packages parsing (blob-cache keyed) to map file -> attrs.
   - Stage 3: Commit planner (diff -> attr targets) with deterministic fallbacks.
   - Stage 4: Extraction executor (Nix C API, batched by system).
   - Stage 5: Mapping feedback loop (update file->attrs with extracted source_path).

2. **State Model**
   - Persist a stable attribute catalog in the DB; refresh at startup.
   - Use DB state to avoid re-evaluating global attr lists unless rebuilding.

3. **CLI Simplification**
   - Keep `--full`, `--since`, `--until`, `--systems`, and `--memory`.
   - Move advanced knobs to config/env (hidden).

4. **Correctness Guarantees**
   - Always index all attributes for the initial baseline (full rebuild).
   - For incremental runs: diff + static map + DB catalog ensures no missed versions.
   - If a file change cannot be mapped, fall back to attrs with missing source_path (not full scan).

## Implementation Log

- 2025-02-XX: Added DB queries for attribute catalog and source_path coverage.
- 2025-02-XX: Hybrid mapping now merges DB-derived mappings to avoid costly Nix fallback.
- 2025-02-XX: Incremental path targeting now uses DB missing-source fallback to prevent misses.
- 2025-02-XX: Advanced indexer knobs moved to JSON config/env; CLI trimmed.
- 2025-02-XX: Full rebuild now deletes existing DB/WAL/SHM + bloom before indexing.
- 2025-02-XX: Target selection now refreshes file-to-attr map when unknown paths appear; falls back to DB missing-source attrs or full all-packages list.
- 2025-02-XX: Extraction batching now bisects failed batches down to single attrs to minimize skipped packages.
- 2025-02-XX: Removed dead extractor helpers (commit-local extract, nix_list helpers) and trimmed unused JSON helpers.
- 2025-02-XX: On batch failure, extractor retries without store-path extraction before bisecting.
- 2025-02-XX: Added skip metrics tracking and end-of-run summary (failed batches + sample skipped attrs).
- 2025-02-XX: Updated regression fixture version regexes for current nixpkgs versions.
- 2025-02-XX: Ran full index from 2017-01-01 to 2019-01-01 (x86_64-linux). Run exceeded 30 minutes but processed multiple baseline batches successfully; continue run to completion.
- 2025-02-XX: Ran parallel ranges 2017+2018 with NXV_INDEXER_CONFIG; process ran >2 hours and was still progressing when the command timed out.
- 2025-02-XX: Re-ran 2017-2018 full index with parallel ranges (2017,2018); run progressed to ~5% of 2017 range before timing out (command timeout, not indexer failure).

## Open Questions

- Confirm preferred default for parallel ranges (kept via config only).

## Next Steps

- Continue extracting static map only when `all-packages.nix` changes (already cached).
- Evaluate removing parallel range CLI or migrating to config-only.
- Add performance benchmarks for mapping + extraction stages.

## Config Example

```json
{
  "checkpoint_interval": 50,
  "workers": 1,
  "gc_interval": 10,
  "max_range_workers": 4,
  "max_commits": null,
  "full_extraction_interval": 0,
  "full_extraction_parallelism": 1,
  "parallel_ranges": null
}
```
