# Building Indexes

This guide covers building your own nxv index from a local nixpkgs checkout.
This is an advanced topic for users who want to:

- Create indexes for custom nixpkgs forks
- Build indexes with different date ranges
- Self-host their own nxv infrastructure
- Contribute to the official index

::: tip Most Users If you just want to use nxv, run `nxv update` to download the
pre-built index. Building your own index takes 24+ hours and significant
resources. :::

## Prerequisites

### Software Requirements

- **nxv with indexer feature** - The indexer is feature-gated to keep the main
  binary small
- **Nix** - With flakes enabled (for evaluation)
- **Git** - For cloning and traversing nixpkgs history

### Hardware Requirements

| Resource | Minimum    | Recommended         |
| -------- | ---------- | ------------------- |
| RAM      | 8 GB       | 32 GB               |
| Disk     | 50 GB free | 100 GB free         |
| CPU      | 4 cores    | 8+ cores            |
| Time     | 24 hours   | 12 hours (parallel) |

### Getting the Indexer

```bash
# Using Nix flakes (recommended)
nix run github:utensils/nxv#nxv-indexer -- --help

# From source
cargo build --release --features indexer
./target/release/nxv index --help
```

### Cloning nixpkgs

```bash
# Full clone (~3 GB)
git clone https://github.com/NixOS/nixpkgs.git

# Or shallow clone for faster download (limits --since range)
git clone --depth 10000 https://github.com/NixOS/nixpkgs.git
```

## Indexing Workflow

### Full Index (First Time)

A full index processes all nixpkgs commits since 2017-01-01:

```bash
nxv index --nixpkgs-path ./nixpkgs
```

This takes 24-48 hours depending on hardware. Progress is checkpointed every 100
commits, so you can safely interrupt with Ctrl+C and resume later.

### Resuming Interrupted Indexing

Just run the same command again:

```bash
# Picks up from the last checkpoint automatically
nxv index --nixpkgs-path ./nixpkgs
```

To force a fresh start (ignoring checkpoints):

```bash
nxv index --nixpkgs-path ./nixpkgs --full
```

### Indexing a Specific Date Range

To index only recent commits:

```bash
# Only 2024 onwards
nxv index --nixpkgs-path ./nixpkgs --since 2024-01-01

# Specific range
nxv index --nixpkgs-path ./nixpkgs --since 2023-01-01 --until 2024-01-01
```

## Backfilling Metadata

After indexing, some metadata may be missing (source paths, homepages,
vulnerability info). Use `backfill` to update:

### HEAD Mode (Default)

Extracts from the current nixpkgs checkout. Fast but may miss renamed/removed
packages:

```bash
nxv backfill --nixpkgs-path ./nixpkgs
```

### Historical Mode

Traverses git history to find each package's original commit. Slower but
complete:

```bash
nxv backfill --nixpkgs-path ./nixpkgs --history
```

### Selective Backfill

Update only specific fields:

```bash
# Only source paths
nxv backfill --nixpkgs-path ./nixpkgs --fields source-path

# Multiple fields
nxv backfill --nixpkgs-path ./nixpkgs --fields source-path,homepage
```

## Publishing

Generate compressed artifacts for distribution:

```bash
# Basic publish
nxv publish --output ./publish

# With signing (recommended)
nxv keygen  # Creates nxv.key and nxv.pub
nxv publish --output ./publish --sign --secret-key ./nxv.key
```

### Generated Artifacts

| File            | Size   | Description                           |
| --------------- | ------ | ------------------------------------- |
| `index.db.zst`  | ~28 MB | Zstd-compressed SQLite database       |
| `bloom.bin`     | ~96 KB | Bloom filter for fast lookups         |
| `manifest.json` | ~1 KB  | Metadata with checksums and signature |

### Hosting

Upload artifacts to any HTTP server. Update your manifest's `url_prefix`:

```bash
nxv publish --output ./publish \
  --url-prefix "https://example.com/nxv" \
  --sign --secret-key ./nxv.key
```

Users can then configure their nxv to use your index:

```bash
export NXV_MANIFEST_URL="https://example.com/nxv/manifest.json"
export NXV_PUBLIC_KEY="RWTxxxxxxxx..."
nxv update
```

## Architecture Deep Dive

### Version Extraction Fallback Chain

Not all packages expose versions the same way. The indexer tries multiple
sources:

| Priority | Source                           | Example                   |
| -------- | -------------------------------- | ------------------------- |
| 1        | `pkg.version`                    | Most packages             |
| 2        | `pkg.unwrapped.version`          | Wrapper packages (neovim) |
| 3        | `pkg.passthru.unwrapped.version` | Passthru metadata         |
| 4        | Parse from `pkg.name`            | `"hello-2.12"` → `"2.12"` |

### all-packages.nix Optimization

The file `pkgs/top-level/all-packages.nix` changes frequently but usually
affects only a few packages. Instead of extracting all ~18,000 packages on every
commit:

1. Parse the git diff for changed lines
2. Extract affected attribute names (assignment patterns, inherit statements)
3. Evaluate only those specific packages
4. Average: ~7 packages per commit vs 18,000

This optimization provides 100x+ speedup for incremental indexing.

### Checkpointing and Ctrl+C Safety

Progress is saved every 100 commits (configurable via `--checkpoint-interval`):

- **Checkpoint data**: Last indexed commit hash, date, statistics
- **Atomic writes**: Database commits are transactional
- **Signal handling**: Ctrl+C triggers graceful shutdown with checkpoint save
- **Resume**: Next run reads checkpoint and continues from last position

## Database Schema

The index uses SQLite with this schema (version 3):

```sql
CREATE TABLE package_versions (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,            -- Short package name, e.g. "python3"
    version TEXT NOT NULL,         -- e.g., "3.11.4"
    first_commit_hash TEXT NOT NULL,  -- Earliest commit with this version
    first_commit_date INTEGER NOT NULL,  -- Unix timestamp (seconds)
    last_commit_hash TEXT NOT NULL,      -- Latest commit with this version
    last_commit_date INTEGER NOT NULL,   -- Unix timestamp (seconds)
    attribute_path TEXT NOT NULL,  -- e.g., "python311"
    description TEXT,
    license TEXT,                  -- JSON array
    homepage TEXT,
    maintainers TEXT,              -- JSON array
    platforms TEXT,                -- JSON array
    source_path TEXT,              -- e.g., "pkgs/tools/foo/default.nix"
    known_vulnerabilities TEXT,    -- JSON array of CVEs
    UNIQUE(attribute_path, version, first_commit_hash)
);

-- Full-text search index (auto-synced via triggers)
CREATE VIRTUAL TABLE package_versions_fts USING fts5(description);

-- Metadata
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT
);
```

### Key Design: Merged Version Ranges

Rows are merged so that a given `(attribute_path, version)` pair collapses into
a single range per indexing run. When the same version appears in multiple
commits:

- `first_commit_*` tracks the earliest appearance
- `last_commit_*` tracks the latest appearance

The `UNIQUE(attribute_path, version, first_commit_hash)` constraint tolerates
re-runs that discover an earlier `first_commit_hash`; `nxv dedupe` can collapse
any residual duplicates left behind by older indexer versions (pre-0.1.5).

### Insecure Packages

`is_insecure` is not stored as a column — it's derived at query time from
`known_vulnerabilities` (a non-empty JSON array means the package is flagged
insecure). The HTTP API's version-history endpoint surfaces this as a boolean.

## Troubleshooting

### Stuck on a Commit

Some commits may have evaluation issues. Use `--max-commits` to limit processing
and isolate the problematic commit, or use `--since`/`--until` to skip a date
range:

```bash
# Limit to next 100 commits to isolate the issue
nxv index --nixpkgs-path ./nixpkgs --max-commits 100

# Skip problematic date range
nxv index --nixpkgs-path ./nixpkgs --since 2023-06-01
```

### Database Corruption

If the database becomes corrupted after a crash:

```bash
# Reset to fresh state
rm ~/.local/share/nxv/index.db  # Linux
rm ~/Library/Application\ Support/nxv/index.db  # macOS

# Re-run indexing
nxv index --nixpkgs-path ./nixpkgs
```

### nixpkgs Repository Issues

If the nixpkgs clone is in an inconsistent state:

```bash
# Reset to known good state
nxv reset --nixpkgs-path ./nixpkgs --fetch
```

## CLI Reference

For complete command documentation, see the
[Indexer CLI Reference](/advanced/indexer-cli).
