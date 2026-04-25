---
name: nxv
description: Find any version of any Nix package across nixpkgs git history using the nxv CLI or HTTP API. Use when asked which nixpkgs commit shipped a specific package version (e.g. "python 2.7", "nodejs 15", "ruby 2.6"), when looking up package metadata/license/homepage, when generating a `nix shell nixpkgs/<commit>#pkg` invocation for an old version, or when querying the public/private nxv server.
when_to_use: Trigger on requests like "find python 2.7 in nixpkgs", "which commit had nodejs 15.14", "when was foo added/removed", "give me the nix shell command for ruby 2.6", "search nixpkgs for X", or any question about historical Nix package versions.
argument-hint: [subcommand or query]
allowed-tools: Bash, Read, Glob, Grep
---

# nxv — Nix Version Index

`nxv` is a Rust CLI + HTTP API that indexes the entire nixpkgs git history (2017+) into a local SQLite database with a bloom filter for fast lookups. It answers: *"which exact nixpkgs commit shipped version X of package Y?"* and produces the `nix shell nixpkgs/<commit>#pkg` command you need to actually use it.

## Quick Reference

```bash
nxv search python                        # All python packages (most recent per version)
nxv search python 2.7                    # Filter by version (prefix match)
nxv search python --exact                # Exact attribute name only
nxv search "json parser" --desc          # Full-text search package descriptions
nxv info python311                       # Detailed info for current version
nxv info python311 3.11.4                # Detailed info for specific version
nxv history python311                    # Version timeline (first/last seen)
nxv history python311 3.11.4             # When was 3.11.4 available?
nxv stats                                # Index statistics
nxv update                               # Refresh index + check for newer nxv binary
nxv update --no-self-update              # Refresh index only (CI / systemd timers)
nxv serve --host 0.0.0.0 --port 8080     # Start HTTP API + web UI
nxv completions zsh                      # Generate shell completions
```

## How to Use This Skill

Parse `$ARGUMENTS` to determine the action:

- If arguments look like a **subcommand** (`search`, `info`, `history`, `stats`, `update`, `serve`, `completions`, and indexer-only `index`, `backfill`, `dedupe`, `publish`, `keygen`, `reset`), run that subcommand.
- If arguments look like a **package name** (e.g. `python`, `nodejs 15`, `ruby 2.6`), default to `nxv search`.
- If arguments look like a **question** ("when was X added", "which commit has Y"), pick `search` or `history` accordingly.
- If no arguments, run `nxv stats` to give the user a quick health check of their index.

**For agents**: always pass `--format json` (CLI) or hit the HTTP API and pipe to `jq`. The table format is human-only; column widths are terminal-dependent.

## Global Options

Every CLI invocation accepts:

| Flag                   | Description                                                 |
| ---------------------- | ----------------------------------------------------------- |
| `--db-path <PATH>`     | Path to the index database (default: platform data dir)     |
| `-v, --verbose`        | `-v` info, `-vv` debug (SQL queries, HTTP requests)         |
| `-q, --quiet`          | Suppress all output except errors                           |
| `--no-color`           | Disable colored output (also honors `NO_COLOR`)             |
| `--api-timeout <SECS>` | API request timeout when using remote backend (default: 30) |

## Local vs Remote Backend

`nxv` transparently runs against either a local SQLite index or a remote `nxv serve` instance. Set `NXV_API_URL` to switch:

```bash
# Use the public hosted instance — no local index needed
NXV_API_URL=https://nxv.urandom.io nxv search nodejs 15
NXV_API_URL=https://nxv.urandom.io nxv info python311

# Or your own private instance
export NXV_API_URL=http://gpu-host:8080
nxv search rust 1.70
```

If `NXV_API_URL` is unset, the CLI uses the local index at `~/Library/Application Support/nxv/index.db` (macOS) or `~/.local/share/nxv/index.db` (Linux). Run `nxv update` to download the latest published index on first use.

## Search

Find packages and the commits where each version existed:

```bash
nxv search python                                    # Recent per (pkg, version)
nxv search python 3.11                               # Version prefix filter
nxv search python --exact                            # Exact attribute name only
nxv search python --license MIT                      # Filter by license
nxv search python --sort date --reverse              # Oldest first
nxv search "web server" --desc                       # FTS5 description search
nxv search python --show-platforms                   # Add platforms column
nxv search python --full                             # All commits (no dedup)
nxv search python --limit 5                          # Cap at 5 results
nxv search python --format json                      # Machine-readable JSON
nxv search python --format plain                     # TSV for shell scripts
```

**JSON shape per row:**

```json
{
  "attribute_path": "python311",
  "version": "3.11.4",
  "description": "A high-level dynamically-typed programming language",
  "license": "Python-2.0",
  "first_commit_hash": "abc123...",
  "first_commit_date": "2023-06-15T00:00:00Z",
  "last_commit_hash": "def456...",
  "last_commit_date": "2023-12-01T00:00:00Z"
}
```

## Info

Detailed metadata for one package version (description, license, homepage, platforms, source path, known vulnerabilities):

```bash
nxv info python311                       # Latest known version
nxv info python311 3.11.4                # Specific version (positional)
nxv info python311 -V 3.11.4             # Specific version (flag form)
nxv info python311 --format json
```

## History

Version timeline — when each version first appeared and when it was last seen:

```bash
nxv history python311                    # All versions of python311
nxv history python311 3.11.4             # Just one version's window
nxv history python311 --full             # Add commits, license, homepage, etc.
nxv history python311 --format json
```

**JSON shape per row:**

```json
{
  "version": "3.11.4",
  "first_seen": "2023-06-15T00:00:00Z",
  "last_seen": "2023-12-01T00:00:00Z",
  "is_insecure": false
}
```

## Using a Found Version

The whole point. Take a `first_commit_hash` (or `last_commit_hash`) from search/history output and feed it to Nix:

```bash
# Drop into a shell with that exact version
nix shell nixpkgs/e4a45f9#python

# Run it once
nix run nixpkgs/e4a45f9#python

# Add to a flake input
inputs.nixpkgs-python27.url = "github:NixOS/nixpkgs/<commit>";
```

Pick `first_commit_hash` for the canonical "introduced in" commit; pick `last_commit_hash` if you want the most recent commit that still shipped that version.

## Index Management

```bash
nxv update                               # Refresh index + check for newer nxv binary
nxv update --force                       # Force full re-download of the index
nxv update --no-self-update              # Only refresh the index, leave binary alone
nxv update --skip-verify                 # Skip minisign signature check (INSECURE)
nxv update --public-key /path/key.pub    # Use a custom public key (self-hosted index)
nxv update --manifest-url <URL>          # Use a custom manifest (self-hosted index)
nxv stats                                # Index size, commit range, last update
```

`nxv update` always refreshes the package index first. Then it checks GitHub for a newer nxv release:

- **Local install** (install.sh / manual download): downloads the platform binary, verifies SHA-256, atomically swaps the running executable.
- **Nix / cargo / Homebrew**: prints the matching upgrade hint (e.g. `brew upgrade nxv`) and exits successfully.

Set `NXV_NO_SELF_UPDATE=1` for CI/systemd timers that should only refresh the index.

## API Server

```bash
nxv serve                                            # 127.0.0.1:8080 (default)
nxv serve --host 0.0.0.0 --port 3000                 # Public bind
nxv serve --cors                                     # Enable CORS for all origins
nxv serve --cors-origins https://app.example.com     # Restrict CORS
nxv serve --rate-limit 10 --rate-limit-burst 20      # Per-IP rate limit
```

The server bundles:

- **Web UI** at `/` (Tailwind v4 + vanilla JS, embedded at build time)
- **OpenAPI docs** at `/docs` (Scalar UI)
- **REST API** at `/api/v1/*` — see endpoints below
- **Cache headers**: 1h on cacheable package routes; never on `/health`, `/metrics`

### HTTP Endpoints

All paths are under `/api/v1`. Wrap responses always look like `{ "data": ..., "meta": {...} }` for paginated lists, `{ "data": ... }` for single items.

| Method | Path                                                | Purpose                              |
| ------ | --------------------------------------------------- | ------------------------------------ |
| `GET`  | `/search?q=<name>&limit=&offset=&sort=&exact=`      | Search packages                      |
| `GET`  | `/search/description?q=<text>&limit=&offset=`       | Full-text search descriptions (FTS5) |
| `GET`  | `/packages/{attr}`                                  | All version records for a package    |
| `GET`  | `/packages/{attr}/history`                          | Version timeline (first/last seen)   |
| `GET`  | `/packages/{attr}/versions/{version}`               | All records for one version          |
| `GET`  | `/packages/{attr}/versions/{version}/first`         | First-seen commit                    |
| `GET`  | `/packages/{attr}/versions/{version}/last`          | Last-seen commit                     |
| `GET`  | `/stats`                                            | Index statistics                     |
| `GET`  | `/health`                                           | Liveness probe (uncached)            |
| `GET`  | `/metrics`                                          | Server metrics (uncached)            |

**Search query parameters:**

| Parameter | Type    | Description                          |
| --------- | ------- | ------------------------------------ |
| `q`       | string  | Search query (required)              |
| `version` | string  | Version filter (prefix match)        |
| `exact`   | boolean | Exact attribute name match           |
| `license` | string  | License filter                       |
| `sort`    | string  | `date`, `version`, or `name`         |
| `reverse` | boolean | Reverse sort order                   |
| `limit`   | integer | Max results (default 50)             |
| `offset`  | integer | Results to skip (default 0)          |

**Quick examples against the public instance:**

```bash
curl -s "https://nxv.urandom.io/api/v1/search?q=python&version=3.11&limit=5" | jq
curl -s "https://nxv.urandom.io/api/v1/packages/python311/history" | jq '.data[0:3]'
curl -s "https://nxv.urandom.io/api/v1/packages/nodejs_15/versions/15.14.0/first" | jq
curl -s "https://nxv.urandom.io/api/v1/stats" | jq '.data'
```

## Indexer Commands (feature-gated)

These require nxv built with `--features indexer` (`cargo build --features indexer` or `nix build .#nxv-indexer`) and a local nixpkgs git checkout:

```bash
# Build the index from a local nixpkgs clone
nxv index --nixpkgs-path ./nixpkgs                   # Incremental from last commit
nxv index --nixpkgs-path ./nixpkgs --full            # Full rebuild from scratch
nxv index --nixpkgs-path ./nixpkgs --since 2023-01-01

# Backfill missing metadata (source_path, homepage, known-vulnerabilities)
nxv backfill --nixpkgs-path ./nixpkgs                # Fast: HEAD only
nxv backfill --nixpkgs-path ./nixpkgs --history      # Slow: traverses git for accuracy

# Repair pre-0.1.5 incremental indexer bug
nxv dedupe --dry-run                                 # Preview
nxv dedupe                                           # Run

# Reset nixpkgs checkout to a known state
nxv reset --nixpkgs-path ./nixpkgs --to origin/master --fetch

# Publish distribution-ready compressed artifacts + manifest
nxv publish --output ./publish --url-prefix https://your-server/nxv
nxv publish --output ./publish --url-prefix https://... --sign --secret-key nxv.key

# Generate a minisign keypair for signing manifests
nxv keygen --secret-key ./nxv.key --public-key ./nxv.pub
```

Most users never need these — they consume a pre-built published index via `nxv update`. Only run these when self-hosting an index.

## Key Environment Variables

| Variable             | Purpose                                                                |
| -------------------- | ---------------------------------------------------------------------- |
| `NXV_API_URL`        | Point CLI at a remote `nxv serve` instead of the local DB              |
| `NXV_DB_PATH`        | Override local SQLite path                                             |
| `NXV_API_TIMEOUT`    | HTTP client timeout in seconds (default 30)                            |
| `NXV_MANIFEST_URL`   | Override the manifest URL used by `nxv update`                         |
| `NXV_PUBLIC_KEY`     | Public key for manifest verification (path or raw key)                 |
| `NXV_SECRET_KEY`     | Secret key for `nxv publish --sign` (path or raw content)              |
| `NXV_SKIP_VERIFY`    | Skip minisign signature check (INSECURE — dev/testing only)            |
| `NXV_NO_SELF_UPDATE` | Skip the binary self-update check during `nxv update`                  |
| `NXV_HOST`           | `nxv serve` bind host                                                  |
| `NXV_PORT`           | `nxv serve` listen port                                                |
| `NXV_RATE_LIMIT`     | `nxv serve` per-IP rate limit (req/sec)                                |
| `NXV_FRONTEND_DIR`   | `nxv serve` reads frontend assets from disk on every request (dev mode)|
| `NO_COLOR`           | Disable ANSI colors                                                    |

## Data Paths

The local index lives in platform data directories:

- **macOS**: `~/Library/Application Support/nxv/`
- **Linux**: `~/.local/share/nxv/`

Files:

- `index.db` — SQLite database with package versions
- `bloom.bin` — Bloom filter sibling for fast negative lookups (loaded at search time)

Safe to delete; `nxv update` will rebuild from the published manifest.

## Agent Patterns

**Find the commit that introduced a specific version (CLI):**

```bash
nxv search python 3.11.4 --exact --format json | \
  jq '.[0] | {pkg: .attribute_path, version, commit: .first_commit_hash, date: .first_commit_date}'
```

**Generate the `nix shell` invocation directly (CLI):**

```bash
nxv search nodejs 15 --exact --format json | \
  jq -r '.[0] | "nix shell nixpkgs/\(.first_commit_hash | .[0:7])#\(.attribute_path)"'
```

**Find a package version on the public API:**

```bash
curl -s "https://nxv.urandom.io/api/v1/packages/python27/versions/2.7.18/first" | \
  jq -r '.data | "nix shell nixpkgs/\(.first_commit_hash | .[0:7])#\(.attribute_path)"'
```

**Check whether a version of a package was ever in nixpkgs (HTTP):**

```bash
curl -s "https://nxv.urandom.io/api/v1/packages/ruby/history" | \
  jq '.data[] | select(.version | startswith("2.6"))'
```

**Get the index freshness (when was nixpkgs last walked):**

```bash
curl -s "https://nxv.urandom.io/api/v1/stats" | \
  jq '.data | {commit: .last_indexed_commit, date: .last_indexed_date, packages: .unique_names}'
```

## Key Invariants for Agents

- **Search is dedup-by-default**: by default, only the most recent record per `(attribute_path, version)` pair is returned. Pass `--full` (CLI) or use the `/packages/{attr}` HTTP endpoint to see every commit.
- **Version filters are prefix matches**: `nxv search python 3.11` matches `3.11.0`, `3.11.4`, `3.11.10`, etc. Use `--exact` for whole-attribute matches, not for whole-version matches.
- **Bloom filter gives instant negatives**: a search for a nonsense package name returns in <1 ms because the bloom filter rejects it before SQLite is touched. False positives are possible but rare.
- **Coverage starts in 2017**: nixpkgs commits before 2017 are not indexed. Anything older needs raw git spelunking.
- **Self-hosted indexes need a public key**: pass `--public-key` or set `NXV_PUBLIC_KEY` when consuming a manifest you signed yourself, otherwise `nxv update` rejects the signature.
- **`--format json` shape is stable**: safe to pipe to `jq`. Breaking shape changes would be a semver bump.
- **`/api/v1` responses always wrap in `{data, meta}` (or `{data}` for single items)**: do `jq '.data'` first.

## Practical Tips

- **Just want a python 2.7 shell?** `nxv search python 2.7 --exact --format json | jq -r '.[0].first_commit_hash'`, then `nix shell nixpkgs/<hash>#python27`.
- **Use `--exact`** when the package name is unambiguous; otherwise `python` returns dozens of variants (`python27`, `python311`, `python311Packages.numpy`, etc.).
- **Use `--desc`** for fuzzy intent ("a package that does X") instead of exact name searches.
- **Set `NXV_API_URL=https://nxv.urandom.io`** to skip the ~100MB index download entirely if you only need occasional lookups.
- **Update weekly**: the public index is republished on a weekly schedule (`publish-index.yml`).
- **For CI**: pin to `NXV_NO_SELF_UPDATE=1 nxv update` so the runner refreshes the index but never tries to swap its own binary.

## Updating This Skill

This skill is maintained in the nxv repository on GitHub. To pull the latest version:

```bash
# Source repository
https://github.com/utensils/nxv

# Skill file location within the repo
.claude/skills/nxv/SKILL.md

# Fetch the latest skill directly
curl -sL https://raw.githubusercontent.com/utensils/nxv/main/.claude/skills/nxv/SKILL.md \
  -o ~/.claude/skills/nxv/SKILL.md
```

When copying this skill to other workspaces or agents, always pull from `main` to get the latest subcommands, flags, and JSON shapes.
