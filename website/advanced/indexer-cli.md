# Indexer CLI Reference

Complete documentation for nxv indexer commands. These commands are only
available when nxv is built with the `indexer` feature.

::: warning Feature-Gated These commands require `nxv-indexer` or building with
`--features indexer`. Most users should use `nxv update` to download the
pre-built index instead. :::

## index

Build the package index from a local nixpkgs repository.

```bash
nxv index --nixpkgs-path <PATH> [OPTIONS]
```

::: tip Verbose Logging Use global `-v` for debug logging (`nxv -v index ...`).
The `-v` flag must come before the subcommand. :::

### Options

| Flag                        | Default    | Description                                       |
| --------------------------- | ---------- | ------------------------------------------------- |
| `--nixpkgs-path <PATH>`     | (required) | Path to local nixpkgs clone                       |
| `--full`                    | `false`    | Force full rebuild, ignoring checkpoints          |
| `--checkpoint-interval <N>` | `100`      | Commits between checkpoint saves                  |
| `--systems <SYSTEMS>`       | all 4      | Comma-separated systems to evaluate               |
| `--since <DATE>`            | -          | Only process commits after this date (YYYY-MM-DD) |
| `--until <DATE>`            | -          | Only process commits before this date             |
| `--max-commits <N>`         | -          | Limit total commits processed                     |

### Default Systems

When `--systems` is not specified, these 4 systems are evaluated:

- `x86_64-linux`
- `aarch64-linux`
- `x86_64-darwin`
- `aarch64-darwin`

### Examples

```bash
# Full index from scratch (24+ hours)
nxv index --nixpkgs-path ./nixpkgs

# Resume interrupted indexing (auto-detects checkpoint)
nxv index --nixpkgs-path ./nixpkgs

# Index only 2024 commits
nxv index --nixpkgs-path ./nixpkgs --since 2024-01-01

# Linux-only indexing
nxv index --nixpkgs-path ./nixpkgs \
  --systems x86_64-linux,aarch64-linux

# Force fresh start
nxv index --nixpkgs-path ./nixpkgs --full
```

---

## backfill

Update existing database records with missing metadata (source paths, homepages,
vulnerability info).

```bash
nxv backfill --nixpkgs-path <PATH> [OPTIONS]
```

### Options

| Flag                    | Default    | Description                                 |
| ----------------------- | ---------- | ------------------------------------------- |
| `--nixpkgs-path <PATH>` | (required) | Path to local nixpkgs clone                 |
| `--fields <FIELDS>`     | all        | Comma-separated fields to backfill          |
| `--limit <N>`           | -          | Process only first N packages               |
| `--dry-run`             | `false`    | Show what would be updated without changes  |
| `--history`             | `false`    | Use historical mode (slower, comprehensive) |

### Fields

Available fields for `--fields`:

- `source-path` - Path to package source (e.g., `pkgs/tools/foo/default.nix`)
- `homepage` - Package homepage URL
- `known-vulnerabilities` - CVE list for vulnerable packages

### Modes

**HEAD mode** (default):

- Extracts from current nixpkgs checkout
- Fast (~30-60 minutes for full database)
- May miss renamed/removed packages

**Historical mode** (`--history`):

- Traverses git to each package's original commit
- Slow (~24+ hours for full database)
- Complete coverage including removed packages

### Examples

```bash
# Fast backfill from current checkout
nxv backfill --nixpkgs-path ./nixpkgs

# Preview what would be updated
nxv backfill --nixpkgs-path ./nixpkgs --dry-run

# Comprehensive historical backfill
nxv backfill --nixpkgs-path ./nixpkgs --history

# Only fill source paths
nxv backfill --nixpkgs-path ./nixpkgs --fields source-path

```

---

## reset

Reset the nixpkgs repository to a known state. Useful after interrupted indexing
or to prepare for a fresh run.

```bash
nxv reset --nixpkgs-path <PATH> [OPTIONS]
```

### Options

| Flag                    | Default         | Description                        |
| ----------------------- | --------------- | ---------------------------------- |
| `--nixpkgs-path <PATH>` | (required)      | Path to nixpkgs repository         |
| `--to <REF>`            | `origin/master` | Git ref to reset to                |
| `--fetch`               | `false`         | Fetch from origin before resetting |

### Examples

```bash
# Reset to master (default)
nxv reset --nixpkgs-path ./nixpkgs

# Fetch latest and reset
nxv reset --nixpkgs-path ./nixpkgs --fetch

# Reset to specific branch
nxv reset --nixpkgs-path ./nixpkgs --to origin/nixos-24.05

# Reset to specific commit
nxv reset --nixpkgs-path ./nixpkgs --to abc123def
```

---

## publish

Generate publishable index artifacts: compressed database, bloom filter, and
signed manifest.

```bash
nxv publish [OPTIONS]
```

### Options

| Flag                  | Default     | Description                             |
| --------------------- | ----------- | --------------------------------------- |
| `--output <DIR>`      | `./publish` | Output directory for artifacts          |
| `--url-prefix <URL>`  | -           | Base URL for manifest download links    |
| `--sign`              | `false`     | Sign manifest with minisign             |
| `--secret-key <PATH>` | -           | Path to minisign secret key             |
| `--min-version <N>`   | -           | Minimum schema version required to read |

### Generated Files

| File            | Size   | Description                                    |
| --------------- | ------ | ---------------------------------------------- |
| `index.db.zst`  | ~28 MB | Zstd-compressed SQLite database                |
| `bloom.bin`     | ~96 KB | Bloom filter for fast negative lookups         |
| `manifest.json` | ~1 KB  | Metadata with checksums and optional signature |

### Examples

```bash
# Basic publish (no signing)
nxv publish --output ./publish

# Publish with signing
nxv publish --output ./publish \
  --sign --secret-key ./keys/nxv.key

# Full publish for GitHub releases
nxv publish --output ./publish \
  --sign --secret-key ./keys/nxv.key \
  --url-prefix "https://github.com/utensils/nxv/releases/download/index-latest"
```

### Manifest Format

```json
{
  "version": 1,
  "schema_version": 4,
  "database": {
    "url": "https://example.com/index.db.zst",
    "sha256": "abc123...",
    "size": 29000000
  },
  "bloom_filter": {
    "url": "https://example.com/bloom.bin",
    "sha256": "def456...",
    "size": 98304
  },
  "signature": "untrusted comment: ...\nRWTxxxxxxxxxx..."
}
```

---

## keygen

Generate a minisign keypair for signing index manifests.

```bash
nxv keygen [OPTIONS]
```

### Options

| Flag                  | Default             | Description                   |
| --------------------- | ------------------- | ----------------------------- |
| `--secret-key <PATH>` | `./nxv.key`         | Secret key output path        |
| `--public-key <PATH>` | `./nxv.pub`         | Public key output path        |
| `--comment <TEXT>`    | `"nxv signing key"` | Comment embedded in key files |
| `--force`             | `false`             | Overwrite existing key files  |

### Generated Files

| File      | Description                             |
| --------- | --------------------------------------- |
| `nxv.key` | Secret key (keep private!)              |
| `nxv.pub` | Public key (distribute with your index) |

### Examples

```bash
# Generate keypair in current directory
nxv keygen

# Custom output paths
nxv keygen --secret-key ./keys/nxv.key --public-key ./keys/nxv.pub

# With custom comment
nxv keygen --comment "My nxv index signing key"

# Overwrite existing keys
nxv keygen --force
```

### Security Notes

- Keep the secret key (`nxv.key`) private
- Distribute the public key with your published index
- Users verify signatures using `NXV_PUBLIC_KEY` environment variable
- Signature verification can be skipped with `NXV_SKIP_VERIFY=1`
