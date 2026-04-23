# CLI Reference

Complete documentation for all nxv commands and options.

## Global Options

| Flag                   | Description                                             |
| ---------------------- | ------------------------------------------------------- |
| `--db-path <DB_PATH>`  | Path to the index database (default: platform data dir) |
| `-v, --verbose...`     | Enable verbose output (`-v` info, `-vv` debug)          |
| `-q, --quiet`          | Suppress all output except errors                       |
| `--no-color`           | Disable colored output (also: `NO_COLOR` env)           |
| `--api-timeout <SECS>` | API request timeout in seconds (default: 30)            |
| `-h, --help`           | Print help                                              |
| `-V, --version`        | Print version                                           |

## Commands

### search

Search for packages by name, version, or description.

```bash
nxv search <PACKAGE> [VERSION]
```

**Options:**

| Flag                      | Description                                   |
| ------------------------- | --------------------------------------------- |
| `-V, --version <VERSION>` | Filter by version (alternative to positional) |
| `-e, --exact`             | Exact name match only                         |
| `--desc`                  | Search in descriptions (full-text)            |
| `--license <LICENSE>`     | Filter by license                             |
| `--show-platforms`        | Show platforms column                         |
| `--sort <ORDER>`          | Sort order: date, version, name               |
| `-r, --reverse`           | Reverse sort order                            |
| `-n, --limit <N>`         | Maximum results (default: 50, 0=unlimited)    |
| `--full`                  | Show all commits (no deduplication)           |
| `--ascii`                 | ASCII table borders                           |
| `-f, --format <FORMAT>`   | Output format: table, json, plain             |

**Examples:**

```bash
# Basic search
nxv search python

# Find specific version
nxv search python 3.11.4

# Search descriptions
nxv search "web server" --desc

# JSON output for scripts
nxv search python --format json

# Sort by date, newest first
nxv search python --sort date
```

### info

Show detailed information about a specific package version.

```bash
nxv info <package> [version] [options]
```

**Options:**

| Flag                      | Description                                  |
| ------------------------- | -------------------------------------------- |
| `-V, --version <VERSION>` | Specific version (alternative to positional) |
| `-f, --format <FORMAT>`   | Output format: table, json, plain            |

**Examples:**

```bash
# Latest version info
nxv info python311

# Specific version
nxv info python311 3.11.4
```

### history

Show version history for a package.

```bash
nxv history <PACKAGE> [VERSION]
```

**Options:**

| Flag                    | Description                       |
| ----------------------- | --------------------------------- |
| `-f, --format <FORMAT>` | Output format: table, json, plain |
| `--full`                | Show full details                 |
| `--ascii`               | ASCII table borders               |

**Examples:**

```bash
# Full history
nxv history python311
```

### update

Download or update the package index.

```bash
nxv update [options]
```

**Options:**

| Flag                 | Description                          |
| -------------------- | ------------------------------------ |
| `-f, --force`        | Force full re-download               |
| `--skip-verify`      | Skip manifest signature verification |
| `--public-key <KEY>` | Custom public key for verification   |

### self-update

Update the nxv **binary** itself to the latest GitHub release. This is
separate from `nxv update`, which updates the package _index_.

```bash
nxv self-update [options]
```

**Options:**

| Flag                  | Description                                               |
| --------------------- | --------------------------------------------------------- |
| `--check`             | Only report whether a newer release exists; no install    |
| `--force`             | Reinstall even if the current version already matches     |
| `--version <TAG>`     | Install a specific release tag (e.g. `v0.2.0`)            |

**Behaviour by install method:**

| Install method | Action                                                      |
| -------------- | ----------------------------------------------------------- |
| Nix            | Prints `nix profile upgrade nxv` / flake-update hint, exits |
| `cargo install`| Prints `cargo install --locked nxv`, exits                  |
| Homebrew       | Prints `brew upgrade nxv`, exits                            |
| Local          | Downloads, verifies SHA-256, atomically swaps the binary    |

Managed-install branches exit with status `2` so scripts can distinguish
them from a successful in-place update. Set `GITHUB_TOKEN` to avoid
unauthenticated rate limits when calling the GitHub API.

**Examples:**

```bash
# Check if a new release is available
nxv self-update --check

# Update to latest
nxv self-update

# Pin to a specific version
nxv self-update --version v0.2.0
```

### serve

Start the HTTP API server.

```bash
nxv serve [options]
```

**Options:**

| Flag                       | Description                       |
| -------------------------- | --------------------------------- |
| `-H, --host <HOST>`        | Host address (default: 127.0.0.1) |
| `-p, --port <PORT>`        | Listen port (default: 8080)       |
| `--cors`                   | Enable CORS for all origins       |
| `--cors-origins <ORIGINS>` | Specific CORS origins             |
| `--rate-limit <N>`         | Rate limiting per IP (req/sec)    |
| `--rate-limit-burst <N>`   | Burst size for rate limiting      |

**Examples:**

```bash
# Default (localhost:8080)
nxv serve

# Public server with CORS
nxv serve --host 0.0.0.0 --port 3000 --cors
```

### stats

Show index statistics and metadata.

```bash
nxv stats
```

### dedupe

Collapse duplicate `(attribute_path, version)` rows in the local index. Repairs
databases bloated by the pre-0.1.5 incremental indexer bug. Keeps one row per
unique pair with the earliest `first_commit_*` and the latest `last_commit_*`
across the duplicates, then `VACUUM`s.

Requires the `indexer` feature (`nxv-indexer` or
`cargo build --features indexer`).

```bash
nxv dedupe [options]
```

**Options:**

| Flag          | Description                                               |
| ------------- | --------------------------------------------------------- |
| `--dry-run`   | Report what would change without modifying the database   |
| `--no-vacuum` | Skip the trailing `VACUUM` (faster, DB file won't shrink) |

### completions

Generate shell completion scripts.

```bash
nxv completions <shell>
```

**Shells:** bash, zsh, fish, powershell, elvish

**Examples:**

```bash
# Bash
nxv completions bash > ~/.local/share/bash-completion/completions/nxv

# Zsh
nxv completions zsh > ~/.zfunc/_nxv

# Fish
nxv completions fish > ~/.config/fish/completions/nxv.fish
```

## Output Formats

### table (default)

Human-readable table with colors:

```
Package          Version   Date         Commit
python311        3.11.4    2023-06-15   abc1234
python311        3.11.3    2023-04-05   def5678
```

### json

Machine-readable JSON:

```json
[
  {
    "attribute_path": "python311",
    "version": "3.11.4",
    "first_commit_date": "2023-06-15T00:00:00Z",
    "first_commit_hash": "abc1234..."
  }
]
```

### plain

Tab-separated values for scripts:

```
python311	3.11.4	2023-06-15	abc1234
python311	3.11.3	2023-04-05	def5678
```
