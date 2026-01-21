# Indexer Refactor Tracking (2026-01)

## Goals
- Eliminate memory-related skips while staying within configured memory limits.
- Support range reprocessing with --full + --since/--until without deleting the DB.
- Improve observability for skipped attrs and batch failures.

## Changes
- Added worker watchdog kill tracking to classify memory-limit kills as recoverable.
- Added single-worker fallback pool to retry memory-constrained extractions using full budget.
- Added memory-aware error classification to avoid skipping attrs when memory limits are hit.
- Updated --full behavior to only reprocess the requested range when --since/--until is provided.
- Added tests for range labels, single-worker config, and memory error classification.
- Updated website docs for indexer CLI and range reprocessing behavior.

## Tests Run
- cargo fmt
- cargo clippy --features indexer -- -D warnings
- cargo test
- cargo test --features indexer
- cargo build --release --features indexer

## Notes
- Release binary: target/release/nxv
- Skipped attrs are now treated as non-memory failures; memory-limit failures retry with a single-worker pool or abort if still failing.
