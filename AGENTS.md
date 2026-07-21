# AGENTS.md

This file provides guidance to AI coding agents (Claude Code, Codex, etc.) when working with code in this repository. `CLAUDE.md` is a symlink to this file — edit here.

## Project Overview

nxv (Nix Versions) is a Rust CLI tool for quickly finding specific versions of Nix packages across nixpkgs history. It uses a pre-built SQLite index with bloom filters for fast lookups. Also provides an HTTP API server with web frontend.

## Development Environment

This project uses Nix flakes with crane for reproducible builds:

```bash
nix develop          # Enter devshell with Rust toolchain
direnv allow         # Or use direnv for automatic shell activation
```

## Build Commands

```bash
cargo build                      # Debug build
cargo build --release            # Release build
cargo build --features indexer   # Build with indexer feature
cargo run -- <args>              # Run with arguments
cargo test                       # Run default test suite (unit + integration)
cargo test --features indexer    # Include indexer tests (some #[ignore]d ones need `nix` + network)
cargo test <test_name>           # Run a single test by name
cargo test db::                  # Run tests in a specific module
cargo clippy --features indexer -- -D warnings   # Lint (matches CI; CI also runs the featureless variant)
cargo fmt                        # Format code
cargo bench                      # Run benchmarks (bloom, search, indexer)
nix flake check                  # Run all Nix checks (build, clippy, fmt, tests)
```

The devshell provides shortcuts for all of these (`build`, `clippy`, `fmt-check`, `run-tests`, `coverage`, ...). `ci-local` runs the exact CI sequence: fmt-check, clippy, test — all with `--features indexer`.

## Nix Flake Outputs

```bash
nix build                        # Build nxv (user binary)
nix build .#nxv-indexer          # Build with indexer feature
nix build .#nxv-static           # Static musl build (Linux only; native arch)
nix build .#nxv-static-aarch64   # Static musl build (aarch64, cross-compiled on x86_64 Linux)
nix build .#nxv-docker           # Docker image (Linux only, use --system x86_64-linux on macOS)
nix run                          # Run nxv directly
nix run .#nxv-indexer            # Run with indexer feature
```

## Architecture

### Data Flow

1. **Index Download** (`remote/`): User runs `nxv update` → downloads compressed SQLite DB + bloom filter from remote manifest
2. **Search** (`db/queries.rs`): Queries go through bloom filter first (fast negative lookup), then SQLite with FTS5
3. **Output** (`output/`): Results formatted as table (default), JSON, or plain text

### Backend Abstraction (`backend.rs`, `client.rs`)

The CLI transparently runs against either a local index or a remote `nxv serve` instance. `main.rs` branches on `NXV_API_URL` — if set, `ApiClient` (HTTP) is used; otherwise the local SQLite/bloom backend is used. Both implement the same backend trait, so command handlers are backend-agnostic. `search.rs` holds the search logic shared by the CLI commands and the API server handlers.

### Self-Update (`self_update.rs`)

There is no standalone `self-update` subcommand — `nxv update` refreshes the index first, then checks GitHub for a newer nxv release. On local installs (install.sh, manual download) the binary is replaced atomically after SHA-256 verification against the release's SHA256SUMS.txt; on managed installs (Nix, cargo, Homebrew — detected from the executable path) it only prints the matching upgrade command. If the index refresh fails with an incompatible-index (schema too new) error, the self-update check still runs before exiting so users can recover. Skip with `--no-self-update` or `NXV_NO_SELF_UPDATE`. Note: `NXV_VERSION` is read only by `install.sh` to pin the installer download — the binary's self-update always targets the latest release.

### API Server (`server/`)

The `nxv serve` command runs an HTTP API server with:

- REST API at `/api/v1/*` (search, package info, version history, stats)
- Web frontend at `/` served from `frontend/` (static HTML/CSS/JS, embedded at build time)
- OpenAPI documentation at `/docs` (generated in `server/openapi.rs`)
- Configurable CORS support
- In-memory runtime metrics (`server/metrics.rs`): rolling latency window, per-minute activity buckets over the last 30 minutes, uptime — all lost on restart by design

### Shell Completions (`completions.rs`)

`nxv completions <shell>` emits clap-generated completions; for bash/zsh/fish it appends custom functions that call the hidden `nxv complete-package` subcommand for dynamic package-name completion against the local index.

### Indexer (`src/index/`, feature-gated)

Note the directory is `src/index/` even though the Cargo feature is named `indexer`. The indexer ingests **channel-release snapshots** from releases.nixos.org — it does NOT walk git history or read a nixpkgs checkout (see `docs/indexer-rewrite/DESIGN.md` for the full specification and `ANALYSIS.md` for why the git-walking design was replaced):

- `releases.rs`: S3 bucket listing (event-based XML parsing), release-name parsing, git-revision fetch, planning (diff against the `releases` ledger)
- `snapshot.rs`: Streaming brotli + JSON parsing of `packages.json.br` (~380 MB decompressed, never materialized; era-tolerant field handling back to 2020)
- `eval.rs`: `nix-env` over `nixexprs.tar.xz` for the pre-2020 era (`--backfill-evals`), and the `--head-eval` master-tarball fallback for channel-stuck periods (the only paths that need `nix`)
- `monitor.rs`: Data-quality gates that run BEFORE any write — count floors, rolling baselines, sentinel packages (firefox, thunderbird, nh, python*Packages.requests), births/deaths, head-lag; plus the end-of-run report (`--report report.json`)
- `mod.rs`: Coordinator — plan → parallel fetch/parse → in-order gating → aggregated widen-only upserts (atomic with the ledger) → FTS/bloom rebuild + watermarks
- `publisher.rs`: Generates compressed index files and manifest for distribution (min_version defaults to the schema version; refuses to publish v4 ungated)

### Database Schema (`db/mod.rs`, schema v4)

- `package_versions`: One row per `(attribute_path, version)` with **observation-backed** bounds: the pair was seen at `first_commit` and `last_commit` (real, Hydra-built channel commits); interior presence is interpolated, not guaranteed
- `idx_packages_search_nocase`: Covering ASCII-NOCASE index for prefix and prefix+version candidate ranking; bulk ingestion drops and transactionally rebuilds it, and publication repairs or refuses an incomplete index
- `releases`: Per-channel ingestion ledger (pending/ingested/failed/skipped + retry backoff) — replaces the old single-checkpoint model; a gap is just a pending row the next run picks up
- `package_versions_fts`: FTS5 virtual table for description search (triggers are WHEN-guarded; bulk runs drop + rebuild)
- `meta`: Key-value store (last_indexed_commit = newest ingested release commit, schema_version, min_schema_version)

### Key Design Decisions

- **Observations, not inference**: every stored commit is one at which the version verifiably existed; no file→attr change mapping, no "no evaluation = unchanged" (the root causes of issues #21/#23)
- **Nested package sets included**: packages.json already enumerates all ~144k attrs (python3xxPackages.*, haskellPackages.*, ...) — issue #5 is covered with zero evaluation cost
- **Currency**: nixos-unstable-small ingestion keeps the index hours behind master; `--head-eval` covers channel stalls; head lag over 72h marks the run unhealthy (a fatal error under `--strict`; the publish workflow instead publishes first and alerts after)
- **Bloom filter**: Serialized to separate file, loaded at search time for instant "not found" responses; contains full dotted attribute paths
- **Feature gates**: Indexer code (ctrlc, brotli, quick-xml) only compiled with `--features indexer` to keep user binary small

## Dependency Management

**Always use current versions.** Before adding dependencies:

```bash
cargo search <crate>           # Find latest version
```

## Environment Variables

Most are also exposed as CLI flags (see `src/cli.rs`); env vars are useful for tests and deployment.

- `NXV_API_URL` — point the CLI at a remote `nxv serve` instead of the local DB
- `NXV_API_TIMEOUT` — HTTP client timeout in seconds (default 30)
- `NXV_DB_PATH` — override local SQLite path
- `NXV_MANIFEST_URL` — override the manifest URL for `nxv update`
- `NXV_SKIP_VERIFY` — skip minisign verification of the manifest
- `NXV_PUBLIC_KEY` — override the embedded minisign public key
- `NXV_NO_SELF_UPDATE` — make `nxv update` only refresh the index, skipping the binary check
- `NXV_VERSION` — pin the version `install.sh` downloads (not read by the binary itself)
- `NXV_HOST`, `NXV_PORT`, `NXV_RATE_LIMIT`, `NXV_RATE_LIMIT_BURST` — `nxv serve` bind/host/rate-limit
- `NXV_MAX_DB_CONNECTIONS`, `NXV_DB_TIMEOUT_SECS` — `nxv serve` DB concurrency cap (default 32) and per-operation timeout (default 30s)
- `NXV_LOG_FORMAT` — set to `json` for structured `nxv serve` logs (combine with `RUST_LOG`)
- `NXV_SECRET_KEY` — minisign secret key (path or contents) for `nxv publish` signing
- `NXV_RELEASES_URL` — override the releases.nixos.org S3 endpoint for `nxv index` (tests, mirrors)
- `NXV_FRONTEND_DIR` — serve `index.html`/`app.js`/`favicon.svg` from this directory on every request instead of the embedded copy, and disable the 24h `Cache-Control` for those routes. Used by the devshell `dev` command for live frontend reload (edit → browser refresh, no rebuild; `dev` also runs cargo-watch so `src/` changes rebuild and restart the server). Unset in production.
- `NXV_GIT_REV` — set by the Nix flake build to embed the git rev in `--version`

## Data Paths

The database and bloom filter are stored in platform-specific data directories:

- **macOS**: `~/Library/Application Support/nxv/`
- **Linux**: `~/.local/share/nxv/`

Files:

- `index.db` - SQLite database with package versions
- `bloom.bin` - Bloom filter for fast negative lookups

## Testing

- Unit tests are in each module's `mod tests` section
- Integration tests in `tests/integration.rs` use `assert_cmd` to test CLI behavior
- Tests create temporary databases using `tempfile`
- Some indexer tests require `nix` to be installed (marked `#[ignore]`)

## Accessibility (WCAG)

Frontend a11y is audited by two devshell commands:

- `a11y` — static, fully offline. Runs `html5validator` against `frontend/*.html`
  (CSS validation is skipped — vnu.jar predates Tailwind v4 oklch/`@theme`/
  `@layer`/`color-mix`) and then `scripts/a11y_check.py frontend/index.html`.
  The Python script enforces landmarks, form labels, alt/role on images and
  SVGs, heading hierarchy, skip-link presence, dialog semantics, and converts
  every oklch token in the `@theme` block to sRGB to flag fg/bg pairs below
  WCAG 2.1 AA (4.5:1 text, 3:1 large/UI). Wired into `nix flake check` as
  `nxv-a11y`.
- `a11y-live` — dynamic, opt-in. Runs `pa11y-ci` via `npx` against the local
  `nxv serve` (start it first with `dev`). Config at `frontend/.pa11yci.json`
  covers the home page (desktop + mobile), a search-populated state, and the
  command palette open state. Not wired into `nix flake check` — needs a
  running server and pulls from npm on first run.

## Docs Site (VitePress)

The VitePress site under `website/` is managed by four devshell commands
(they `cd website` and delegate to `bun`, which is provided in the shell):

- `docs-dev` — start the local dev server (`bun run dev`). Default port is
  5173; the site is served under `/nxv/` to match the GitHub Pages base.
- `docs-build` — build the static site into `website/.vitepress/dist`.
- `docs-preview` — serve the already-built site for a final look.
- `docs-fmt` — run Prettier over `website/`.

## NixOS Module

A NixOS module is provided for running nxv as a systemd service:

```nix
{
  imports = [ inputs.nxv.nixosModules.default ];
  services.nxv = {
    enable = true;
    port = 8080;
    # indexPath = "/path/to/index.db";  # Optional custom path
  };
}
```

## Agent Skill

The canonical skill template lives at `src/skill/SKILL.md` — it is embedded
in the binary and installed by `nxv skill install` for every major AI coding
agent (Claude Code, Codex, Pi, OpenClaw, Copilot, Cursor, Gemini, Amp,
Goose; see `src/skill/mod.rs` for the path table). The checked-in copies at
`.claude/skills/nxv/SKILL.md` and `.agents/skills/nxv/SKILL.md` are
**generated** — never edit them directly. After editing the template,
regenerate both with `cargo run -- skill install claude agents --dir .`
(the `test_checked_in_skill_copies_match_template` integration test enforces
this). When changing CLI flags, adding/removing subcommands, or altering
JSON / API response shapes, also update the template so the skill stays
accurate. The user-facing guide lives at `website/guide/skill.md`.

## Releasing

Releases are automated with **release-plz** (`release-plz.toml` +
`.github/workflows/release-plz.yml`). On every push to main it maintains a
`chore: release vX.Y.Z` PR (version bump + CHANGELOG from conventional
commits; `feat:` bumps the 0.x minor via `features_always_increment_minor`).
**Merging that PR ships the release**: release-plz pushes the `vX.Y.Z` tag,
which triggers `release.yml` and `flakehub-publish-tagged.yml`. Both
release-plz jobs author with a GitHub App token (secrets
`RELEASE_PLZ_APP_ID` / `RELEASE_PLZ_APP_PRIVATE_KEY`) — the default
`GITHUB_TOKEN` would neither trigger CI on the release PR nor fire the
tag-push workflows. Use `/release` to review and merge the release PR with
confirmation; never merge it without explicit user approval.

## CI/CD & Index Publishing

### GitHub Actions Workflows

- `ci.yml`: Runs on PRs and main - tests (cargo + nix), clippy, fmt, builds Docker latest on main
- `release-plz.yml`: On pushes to main - maintains the release PR; tags `vX.Y.Z` when it merges (see Releasing)
- `release.yml`: Triggered by `v*` tags - builds static binaries, publishes to crates.io, pushes versioned Docker images
- `publish-index.yml`: Every 6 hours or manual - ingests new channel-release snapshots into the index (`--head-eval --report`, deliberately not `--strict`) and republishes to `index-latest` only when something was ingested (or `force_publish`); monitor anomalies turn the run red AFTER publishing instead of blocking it
- `pages.yml`: Deploys the VitePress docs site (`website/`) to GitHub Pages on pushes to main
- `flakehub-publish-tagged.yml`: Publishes tagged releases to FlakeHub

### Publishing the Index

The default manifest URL is `https://github.com/utensils/nxv/releases/download/index-latest/manifest.json`.

To publish manually with signing:

```bash
nxv publish --url-prefix "https://github.com/utensils/nxv/releases/download/index-latest" --secret-key keys/nxv.key
```

Or trigger the workflow:

```bash
gh workflow run publish-index.yml
```

### Required Secrets

- `CACHIX_AUTH_TOKEN`: Nix binary cache
- `CARGO_REGISTRY_TOKEN`: crates.io publishing
- `NXV_SIGNING_KEY`: Manifest signing (minisign secret key)
