# nxv Indexer v2: Design Specification

> Driven by [ANALYSIS.md](./ANALYSIS.md). Status: draft for review.
> Hard requirements: (1) nxv stays a single binary — no external tool
> dependencies beyond `nix` itself (and `nix` only for the pre-2021 backfill);
> (2) index builds must surface missing-package anomalies early and loudly.

## 1. Core model

**Observation-based snapshot indexing.** Every row in the index is backed by
real observations: "(attr, version) was present in channel release R at
commit C". No file→attr inference, no range stamping, no "no evaluation =
unchanged". The unit of work is a **channel release**, not a git commit.

Two ingestion eras:

| Era | Source | Mechanism | Nix eval? |
|---|---|---|---|
| ~late 2020 → today | `packages.json.br` per release from releases.nixos.org (S3) | download → stream brotli → stream JSON parse | **No** |
| 2017 → ~late 2020 | `nixexprs.tar.xz` per release from the same S3 dirs | `nix-env -f <tarball> -qaP --json --meta` in recycled subprocesses | Yes (a few hundred runs, one-time) |

Both eras need **no git clone at all**. The `git2` dependency is dropped from
the indexer feature entirely.

Why this is correct where the old design was not:
- Every stored commit is a real channel commit at which the version existed
  (and is Hydra-built, so `nix shell nixpkgs/<commit>#pkg` hits the binary
  cache — a user-facing upgrade over arbitrary master commits).
- Wrapper/inherited-version packages (`nh`) appear in every snapshot —
  unmissable by construction.
- Nested package sets (issue #5) are already enumerated in `packages.json`
  (144k attrs) — no recursive eval cost.
- A failed/missing release stays `pending` in the DB and is retried next
  run — gaps self-heal instead of being checkpointed past.

Known tradeoff: versions that lived shorter than one channel advance
(~½–1 day on nixpkgs-unstable) are not observed. This is the same tradeoff
NixHub makes; an optional later phase can refine with per-file git log. The
old indexer's *theoretical* per-commit granularity produced 4-month holes in
practice.

## 2. Data sources

- **Release enumeration:** S3 ListObjectsV2 on `nix-releases.s3.amazonaws.com`
  (public, no auth), prefixes `nixpkgs/` (nixpkgs-unstable releases,
  `nixpkgs-YY.MMpre<count>.<shortrev>/`). Each release dir contains
  `git-revision` (full 40-char hash), `packages.json.br` (post-2020),
  `nixexprs.tar.xz`. Default channel: `nixpkgs-unstable` (~2 advances/day,
  most packages, longest unbroken S3 history).
- **Cross-check (one-time):** gsc.io channel history for pre-S3-era advance
  dates if S3 listing has holes for 2017–2018.
- Hydra's API is explicitly NOT used (slow/unreliable, verified).

## 3. Database schema (v4)

```sql
CREATE TABLE package_versions (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,                -- pname (fallback: parsed name)
    version TEXT NOT NULL,
    attribute_path TEXT NOT NULL,      -- full dotted path (python313Packages.requests)
    first_commit_hash TEXT NOT NULL,   -- first release commit observed containing it
    first_commit_date INTEGER NOT NULL,
    last_commit_hash TEXT NOT NULL,    -- last release commit observed containing it
    last_commit_date INTEGER NOT NULL,
    description TEXT, license TEXT, homepage TEXT,
    maintainers TEXT, platforms TEXT, source_path TEXT,
    known_vulnerabilities TEXT,
    UNIQUE(attribute_path, version),
    CHECK(first_commit_date <= last_commit_date)
);

CREATE TABLE releases (
    id INTEGER PRIMARY KEY,
    channel TEXT NOT NULL,             -- "nixpkgs-unstable"
    release_name TEXT NOT NULL,        -- "nixpkgs-26.05pre880076.f3007fa61f17"
    commit_hash TEXT NOT NULL,         -- full 40-char from git-revision
    release_date INTEGER NOT NULL,     -- S3 LastModified of the release
    source TEXT NOT NULL,              -- packages_json | nix_env
    status TEXT NOT NULL,              -- pending | ingested | failed | skipped
    attr_count INTEGER,                -- packages observed (monitoring)
    error TEXT,                        -- last failure reason
    ingested_at INTEGER,
    UNIQUE(channel, release_name)
);
```

- `package_versions` keeps its column set → the query/server/output layers
  keep working with minimal changes. The UNIQUE key tightens from
  `(attr, version, first_commit_hash)` to `(attr, version)` (slim model).
- Writes go through an **order-agnostic widen-only upsert** (CASE-WHEN
  bounds extension, salvaged from `feature/reverse-indexing`): correct under
  any ingestion order and parallelism. Metadata updates prefer the *newer*
  observation (overwrite when the incoming snapshot is newer than
  `last_commit_date`, not COALESCE).
- FTS5 table unchanged. Bloom filter gains **full dotted attribute paths**
  (and final path segments, so `requests` still pre-checks true).
- `meta`: `schema_version = 4`; per-channel watermark is derived from
  `releases` (max ingested release), NOT a single fragile
  `last_indexed_commit` hash.

## 4. Pipeline architecture (src/index/)

```
mod.rs        coordinator: plan → ingest (parallel) → monitor (in-order) → publish-ready
releases.rs   S3 listing/pagination, release parsing, git-revision fetch, plan diffing
snapshot.rs   one release → SnapshotData: streaming .br download+decompress+JSON parse
eval.rs       pre-2021 fallback: nix-env -qaP --json --meta subprocess workers (recycled)
monitor.rs    data-quality gates: counts, sentinels, births/deaths, report
```

Flow per run (`nxv index`):
1. **Plan**: list S3 → upsert unknown releases as `pending` → work list =
   all `pending` + `failed` releases (bounded retries), oldest→newest.
2. **Ingest**: N parallel workers (default `min(4, cores)`), each:
   download `packages.json.br` (~10 MB) → stream-decompress (brotli crate,
   pure Rust) → stream-parse (`serde_json` custom `MapAccess` visitor — the
   378 MB JSON is never materialized) → fold to
   `HashMap<attr, {pname, version, meta…}>` (~150–250 MB peak per worker,
   dropped after upsert).
3. **Write**: single writer thread; batched upserts (≤50k rows per
   transaction); workers hand off via bounded channel (backpressure, no
   unbounded accumulators anywhere).
4. **Monitor** (in-order by release, buffered like the salvaged seq-numbered
   BTreeMap): see §5. A release is marked `ingested` only after its rows are
   committed AND its monitors pass; failures mark `failed` + reason and do
   NOT advance anything.
5. **Finish**: rebuild bloom, print/write coverage report.

Interrupt safety: ctrlc flag checked between releases; a killed run leaves
`pending`/`failed` rows that the next run picks up. There is no checkpoint
hash to strand.

The pre-2021 era (`nxv index --backfill-evals`) uses the same plan/monitor
loop but `eval.rs` ingestion: download `nixexprs.tar.xz`, run
`nix-env -f <tar> -qaP --json --meta` (top-level only — nested sets aren't
reasonably evaluable pre-2020), one subprocess per release (process exit =
guaranteed memory reclaim — no watchdogs), concurrency sized by RAM
(default 2), failure → `failed` + retry with `--keep-going` semantics.

## 5. Monitoring (hard requirement: catch missing packages EARLY)

Per-release gates, evaluated in chronological order during the run:
- **Count floor**: `attr_count >= 0.90 × previous ingested release's count`
  (and an absolute floor: 10k pre-2021, 80k post-2021). Violation ⇒ release
  marked `failed`, loud warning; `--strict` (CI) exits non-zero.
- **Sentinels**: configurable list with applicability windows, checked per
  release: `firefox` (always), `thunderbird` (always), `nh` (≥2024-01),
  `python3Packages.requests`-style nested attr (≥2021). Missing sentinel ⇒
  same failure path. Defaults embedded; extensible via flag.
- **Births/deaths accounting**: per release, log
  `+births −deaths Δnet (total)`; deaths >5% of total in one advance ⇒
  warning with a sample of the dead attrs (legitimate mass-renames happen,
  e.g. python3xPackages flips, so advisory unless `--strict`).
- **End-of-run report** (stdout + `--report report.json`): releases ingested/
  failed, total attrs at HEAD vs search.nixos.org floor, top gaps, sentinel
  table, mass-death events. CI publishes this as a job summary.
- **Regression fixtures** (tests): thunderbird 142.0 must not span
  2025-08→2026-01; `nh` present; nested attr present + bloom-resolvable.

## 6. CLI surface

- `nxv index` — snapshot indexing (new default). `--channel`, `--since`,
  `--until`, `--jobs`, `--strict`, `--report <path>`, `--retry-failed`.
  `--nixpkgs-path` is gone (the indexer no longer reads a clone) — the
  argument is accepted-but-warned for one release cycle.
- `nxv index --backfill-evals` — one-time 2017→2020 era (requires `nix`).
- `nxv index-reset` — removed (no checkout to reset). `nxv backfill` —
  removed (packages.json carries position/homepage/etc.).
- `nxv publish`, `nxv stats`, search/serve commands unchanged (stats gains
  release coverage).

## 7. Dependency changes

- **Remove**: `git2` (indexer feature becomes `["ctrlc"]` + pure-Rust deps).
- **Add**: `brotli` (pure-Rust decompressor), `quick-xml` (S3 ListObjects
  XML; tiny, pure Rust). Both compile into the single static binary.
- reqwest (existing) does all HTTP.

## 8. CI (publish-index.yml)

Massively simplified: no nixpkgs clone, no clone-depth logic, no GitHub
compare API. Steps: download existing index → `nxv index --strict --report` →
publish → upload report to job summary. Schedule unchanged (6h). The
mass-extinction/dead-scheduler detectors run as part of `--strict`.

## 9. Performance & memory budget

| Operation | Budget |
|---|---|
| Full rebuild post-2020 (~2k releases, ~20 GB transfer) | hours, parse-bound, 4 workers × ~250 MB + writer ≈ **<1.5 GB RSS** |
| Backfill 2017–2020 (~300–900 nix-env runs) | 1–2 days one-time, 2 workers × ~2.5 GB ≈ **<6 GB RSS** |
| Incremental (CI, ~2 advances per 6h window) | **seconds–minutes**, <500 MB |
| Memory policy | bounded channels + per-release maps dropped after write + subprocess-exit reclamation. No watchdogs, no rlimits, no in-process evaluator. |

## 10. Migration

- Full republish (schema v4) replaces the production index; old DB kept
  locally for diffing. The 2024–2025 strata are unrecoverable incrementally —
  rebuild is the fix.
- Client compatibility: `nxv update` clients download the new artifact; the
  reader code tolerates schema v4 via the existing schema_version gate
  (clients with old binaries get a "please upgrade" path — verify the gate's
  behavior before publish).
- Docs: AGENTS.md indexer section, SKILL.md (CLI changes), website guide,
  issues #5/#21/#23 updated with root causes and fix.

## 11. Explicitly rejected alternatives

- **Per-commit evaluation (any variant)** — killed three branch lineages;
  see ANALYSIS §2. The granularity is illusory: in practice it produced
  months-long holes.
- **nix-eval-jobs integration** — external binary dependency (violates the
  single-binary requirement); reserved as manual fallback documentation only.
- **In-process Nix FFI** — Boehm-GC growth, build complexity (libclang,
  version coupling), proven OOM source.
- **File→attr change mapping** — structurally incomplete (wrappers,
  inherited versions, shared version files); the root cause of issue #23.
