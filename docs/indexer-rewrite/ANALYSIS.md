# nxv Indexer: Root-Cause Analysis & Rewrite Strategy

> **Status: complete** — the rewrite this analysis drove shipped in nxv 0.3.0
> (index schema v4, released 2026-06-11); issues #5, #21, and #23 are fixed
> and closed. Historical artifact; "current" below means the pre-rewrite
> git-walking indexer.

> Synthesis of six parallel investigations (current architecture, OOM-branch
> post-mortems, recursive-indexing branch, external landscape research, and
> production DB forensics), 2026-06-09. This document is the source of truth
> driving the indexer rewrite.

**Production index state at time of analysis** (schema v3, last indexed
2026-05-29 at `76394d1a`): 202,232 rows, 28,120 distinct attribute paths,
only 7,934 attrs "open" at HEAD. Real nixpkgs: 144,241 attributes
(nixos-unstable `packages.json`, including nested sets).

---

## 1. Root Causes, Ranked by Confidence, per Symptom

### 1a. Missing versions (thunderbird 143-145; also firefox 143-145, vscode 1.104-1.106, nodejs 22.20/23.x/24.0-24.11, linux 6.12.47-6.12.62)

**RC-1 (CONFIRMED - DB forensics + git history): total indexing blind window
2025-09-10 -> 2026-01-07.** Zero rows have a first_commit_date or
last_commit_date between 2025-09-15 and 2025-12-31 (monthly birth histogram:
2025-09 = 512, then 0, 0, 0, then 2026-01 = 7,872). The dominant cause is the
**merge-commit empty-diff bug**: until fix `474bcd2` (PR #18, landed
2026-01-06), `git diff-tree` returned an *empty changed-file list for merge
commits* - and nixpkgs mainline is almost entirely merge commits - so runs
during this window extracted essentially nothing. The day after the fix
(2026-01-07, commit `865f635c`), 1,440 packages were "reborn": thunderbird
146.0.1, firefox 146.0.1, nodejs 24.12.0, vscode 1.107.1, linux 6.12.63 all
share that exact first_commit. NixHub confirms the missing versions shipped
upstream in the window (thunderbird 143.0 on 2025-09-18, 143.0.1 on
2025-10-07, 144.0.1 on 2025-11-15, 145.0 on 2025-12-18).

**RC-2 (CONFIRMED - code + upstream verification): change-driven targeting is
structurally blind to thunderbird-style version files, even in healthy runs.**
Extraction is triggered only when a changed file maps to an attr via a
position map built from `builtins.unsafeGetAttrPos` over *top-level* attrs
(`src/index/mod.rs:1197-1251`), plus path heuristics
(`src/index/mod.rs:~966-1008`). `pkgs/applications/networking/mailreaders/thunderbird/packages.nix`
contains no top-level-positioned attr (thunderbird is bound in
all-packages.nix); the filename heuristic derives the attr "packages", which
fails validation. **A thunderbird version-bump commit therefore targets
nothing**; the new version is only picked up when an unrelated commit happens
to touch all-packages.nix. This blind-spot class covers wrapper packages,
shared version files, and `inherit`-ed versions generally.

**RC-3 (CONFIRMED - code + DB): the gap is permanent under the current
design.** Checkpoints (`last_indexed_commit`) advanced past the window;
incremental runs walk only `{last}..HEAD`. The CI workflow
(`publish-index.yml`) uses shallow clones with a `--depth 1000` fallback, so
the Oct-Dec 2025 commits were never re-walked. There is no reconciliation
pass; nothing can self-heal the hole.

**RC-4 (CONFIRMED - independent tree check): phantom range extension
fabricated "142.0 alive until Jan 2026".** At every checkpoint and at run
end, *all* open ranges are stamped with the latest processed commit without
re-evaluation (`src/index/mod.rs:1124-1172`) - "no evaluation" is treated as
"unchanged". At the recorded last_commit `4be29992...` the nixpkgs tree
actually contained thunderbird **146.0.1** (verified via GitHub contents
API), i.e. the DB asserts a (commit, version) pair that is provably false.
2,302 rows share that one stamp; other run boundaries show the same
clustering (11,799 rows at `22e95fb3`, 8,041 at `76394d1a`).

**Recurrence:** the same gap-then-mass-rebirth signature appears in 2017-08,
2021-08, and 2022-05..07 - this is the 4th and largest occurrence of a
long-standing failure mode, not a one-off.

### 1b. "Phantom" commit hashes unknown to GitHub

**RC-1 (CONFIRMED by four independent investigators): there are no phantom
hashes. The premise of issue #21's 422 is wrong.**
`gh api repos/NixOS/nixpkgs/commits/4be299927040299d510838cef346b4e4b8539442`
resolves: "python3Packages.sqlcipher3: 0.5.4 -> 0.6.0 (#475904)", committed
2026-01-01T17:23:33Z - exactly matching the DB's stored date - and
`compare/4be2999...master` shows `behind_by=0` (reachable from master). The
**422 came from querying the 7-character prefix**, which GitHub's commits
endpoint rejects as ambiguous in a repo of nixpkgs' size. DB forensics
sampled **37/37 distinct hashes across 2017-2026** (including every
mass-event hash): all resolve upstream with exact committer-date matches.

> One investigator (architecture review) initially attributed the hash to a
> local-clone merge/rebase commit poisoning the index ("first-parent walk
> from ambient HEAD"). That *vulnerability is real in the code* - nothing
> pins the walk to a verified upstream ref or validates ancestry against
> origin/master - but the empirical claim is **refuted**: every sampled hash
> exists upstream. Verdict: real latent vulnerability, never (detectably)
> exercised. Fix it anyway in the rewrite.

**RC-2 (CONFIRMED): the actual corruption is semantic, not cryptographic.**
The hashes are valid; the *(commit, version) claims attached to them are
false* (see 1a/RC-4). `last_commit_hash` means "last commit processed while
the package was assumed present," not "last commit where this version was
observed" - and the stamp commit is arbitrary (the 4be2999 stamp on
thunderbird is a *sqlcipher3* bump).

**Action items:** always pass full 40-char SHAs (>=12 minimum) to the GitHub
API; correct issue #21's narrative; make both range endpoints
observation-backed in the new schema.

### 1c. Missing top-level attrs (`nh` absent; `nh-unwrapped` present)

Three compounding confirmed causes plus one open hypothesis:

**RC-1 (CONFIRMED): pre-2026 history erased by the by-name parsing bug +
discovery collapse.** Pre-#18 code derived attr "package" from
`pkgs/by-name/XX/name/package.nix` paths, and by-name attrs have *null*
`unsafeGetAttrPos` positions, so they failed validation and were never
targeted - by-name packages were systematically invisible for the entire
historical walk. DB forensics quantifies the result: first-seen attrs per
year collapsed to **626 (2024) and 316 (2025)** vs ~2,700/yr prior; `nh`
(added Feb 2024), `uv`, `ghostty`, `zed-editor`, `niri`, and
`nixfmt-rfc-style` all have zero pre-2026 rows.

**RC-2 (CONFIRMED): the nh/nh-unwrapped split (upstream `d3cc6458`,
2025-11-23) landed inside the merge-blind window** and the relevant commits
sit before the post-fix resume point, so they were never reprocessed.

**RC-3 (CONFIRMED): post-split, `nh` is unreachable by design.** The `nh`
wrapper (`symlinkJoin` with `inherit (unwrapped) version`) never has its own
`package.nix` change on version bumps - verified upstream:
`pkgs/by-name/nh/nh/package.nix` has had **zero commits since 2026-01-07**,
while every bump touches `nh-unwrapped/package.nix`, so targeting yields only
`nh-unwrapped`. Under file-change-driven indexing, `nh` can never reappear
until someone edits the wrapper file. This generalizes to every
wrapper/alias/inherited-version attr.

**RC-4 (OPEN - investigator conflict):** DB forensics hypothesized the
extractor's nix-side filtering (tryEval/null guards) additionally drops the
wrapper derivation even when targeted. This conflicts with the verified
file-history evidence above (the file simply never changes, so the attr is
never targeted at all); weight of evidence favors RC-3. Cheap to settle with
a live repro - see section 5. Note: it is *not* a short-name filter (166
two-char attrs like `bc`, `jq`, `fd` are indexed).

### 1d. Missing nested package sets (issue #5: haskellPackages, python3Packages, ...)

**RC-1 (CONFIRMED): excluded by design.** The extractor enumerates *top-level
`attrNames` only*; the positions map is top-level only; the bloom filter has
no dotted attribute paths. 28,120 indexed attrs vs 144,241 real attributes
(24,212 top-level + 120,029 nested: rPackages 32,443; haskellPackages
19,448; python313Packages 10,891; emacsPackages 6,685; ...).

**Feasibility is proven, cost is the problem:** the
`experimental/nix-eval-jobs-indexer` branch's recursive `extract.nix` reached
**119,146 unique packages** (2017 -> mid-2023, 6.2M rows, 7.7 GB DB) before
stalling, and the `recursive-package-indexing` branch built a complete
whitelist + `recurseForDerivations` + depth-cap enumeration with an
`AttrPath` API - but per-commit recursive evaluation is computationally
infeasible (minutes + multi-GB per commit x ~10^5 commits). The external
research resolves this: post-2020 channel snapshots
(`packages.json.br`) contain all nested sets pre-evaluated by Hydra (section 3).
If nested paths are indexed, the **bloom filter must learn dotted paths** or
every nested lookup dies at the bloom pre-check (the branch never did this -
spec Phase 6.3 unchecked).

---

## 2. Post-Mortem Lessons from the Abandoned Branches

Five branches in three efforts. None ever had a PR.

### 2.1 `feature/recursive-package-indexing` (last commit 2026-01-04)

**What it built:** hybrid nested-set enumeration (static whitelist of ~30
scopes + `*Packages`/`linuxPackages_*` name patterns + dynamic
`recurseForDerivations`, depth cap 2); `AttrPath(Vec<String>)` +
`builtins.getAttrByPath` for dotted paths; a well-tested worktree-per-worker
module (`worktree_pool.rs`, 692 lines, flock-based orphan cleanup, never
touches the user's checkout); seq-numbered BTreeMap out-of-order result
buffering with in-order checkpoints.

**Why it died:** a cost bomb - to avoid stale-map races, *every worker
rebuilt the full file->attr map for every commit* via a deepSeq of
`meta.position` across all ~130k packages: minutes of eval and multiple GB
RSS per commit per worker x ~10^5 first-parent commits = weeks-to-months of
pure evaluation. Never profiled before abandonment (every benchmark checkbox
in its own spec unchecked). Its two production-critical discoveries (the
merge-commit empty-diff bug; by-name misparsing) were cherry-picked to main
as PR #18, relieving the pressure; main then advanced 45 commits, making the
4,295-line rebase uneconomical.

**Correctness hazards found in its pipeline (avoid in rewrite):** per-system
extraction errors swallowed (`Err(_) => continue`) while targets stay
populated -> transient eval failure marks live packages as *removed*; worker
panic -> empty result -> checkpoint advances past the commit -> silent
permanent loss with no retry.

### 2.2 `experimental/nix-eval-jobs-indexer` -> `feature/reverse-indexing` (tips 2026-01-07)

**Misnamed:** never integrated nix-eval-jobs. It embedded the **Nix C API
in-process** (`nix-bindings` 2.28 FFI, hand-rolled safe wrapper in
`nix_ffi.rs`), then bolted on a nix-eval-jobs-rs-*inspired* subprocess worker
pool (`src/index/worker/`, ~2,000 lines, JSON-over-pipes IPC, memory-capped
auto-restart) precisely because the in-process evaluator is single-threaded
and leaks.

**What worked:** nested-set coverage 28k -> 119k packages; store-isolation
fixes earned the hard way (`auto-optimise-store = true` corrupts the store
during long indexing runs; `dummy://` store breaks evaluation entirely - "0
packages extracted"; settled on temp `local?root=` eval store + periodic GC +
disk guards); infrastructure-file exclusion from position maps (91s -> 300ms
per infra commit). `feature/reverse-indexing` added the **slim schema**
(one row per (attr, version), order-agnostic CASE-WHEN upsert: 28 MB vs
1.26 GB compressed, avg 23x cold-query speedup), `--sample-interval`, and
newest-first `--reverse` traversal (lazamar-style).

**What failed:** the parent's persistent in-process evaluator **grew to
45 GB+ before OOM kill** (Boehm GC never returns memory); deep recursion blew
default stacks (fixed with dedicated 64 MB-stack threads); rapid thread
churn caused macOS `thread_suspend` aborts; 4 x 6 GB workers OOM'd machines
(cut to 2 GB). Latent bug: `--reverse` without `--slim` silently writes
**inverted ranges** (first_commit_date > last_commit_date, no normalization).
Abandoned without a note; full rebuild had only reached 2023-05; schema v7
vs production v3 meant a disruptive republish; build burden (build.rs,
pkg-config, libclang, Nix C lib version coupling) was heavy.

### 2.3 `feat/native-nix-indexer` -> `refactor/indexer` (one lineage; through 2026-01-22)

`refactor/indexer`'s merge-base **is** `feat/native-nix-indexer`'s HEAD - one
continuous effort (98 + 23 commits), continuing the FFI architecture above
with a hybrid static planner and full parallelism, per its
`INDEXER_REDESIGN_SPEC.md` (which mandated "NO sampling - every commit" and
"NO shell-based nix eval").

**The OOM mechanism, precisely (all CONFIRMED):**

1. **Primary: persistent in-process Boehm-GC evaluator that only grows.**
   Each worker holds one global, never-freed `NixEvaluator` (the Nix C API
   has no reset/shutdown); each batch re-imports nixpkgs into the *same*
   EvalState; ~1.5 GiB baseline ("leaving only 500 MB for packages" in a
   2 GiB worker) and the heap never shrinks. Death by design within a few
   batches.
2. **Multiplication:** year-ranges x systems x pool workers, each carrying a
   1.5-2 GiB+ evaluator (4 ranges x 4 systems = 16 evaluators) - aggregate
   exceeded the machine; commits literally titled "evenly distribute memory
   budget" and "stagger startup to prevent saturation".
3. **Parent-side accumulation:** unbounded `pending_upserts`
   (50k-flush cap added only in the branch's final week) and an in-RAM blob
   cache holding every parsed all-packages.nix map for the whole run.

**The losing war:** ~1,100 lines of memory enforcement (setrlimit RLIMIT_AS -
fragile on Boehm-GC processes due to VA reservations; /proc PSI pressure
monitoring; a 250 ms RSS watchdog SIGTERM/SIGKILLing workers; batch size
500 -> 100 -> adaptive; bisection retry pools). Empirical results from its own
tracking docs: **49-289 worker restarts per 15-minute run; throughput
collapsed to 3-30 commits/min against ~21,000 commits/year**; 8 workers were
*slower* than 4 at the same budget. OOM kills also caused attrs to be
**silently skipped** - a data-quality bug, not just instability - until
error-classification commits near the end. The branch died mid-firefight;
the "1-2h soak" next-step was never checked off. The no-sampling constraint
and the memory budget were mutually incompatible on one machine.

### 2.4 Worth salvaging (consensus across investigators)

| Artifact | Source | Why |
|---|---|---|
| Slim schema + order-agnostic CASE-WHEN upsert | `feature/reverse-indexing` (`b022244`, branch `db/mod.rs:642+`); also `f4addef` on the FFI lineage | One row per (attr, version), widen-only bounds, safe in any traversal order; 23x query speedup, 28 MB artifact; the correct write primitive for parallel/resumable indexing |
| Worker subprocess pool skeleton | `experimental/nix-eval-jobs-indexer` `src/index/worker/` | Industry-standard isolation, debugged through real OOM/IPC-desync/fork-safety failures - reuse the *mechanism*, change the lifecycle *policy* |
| `worktree_pool.rs` | `recursive-package-indexing` | Worker-owned detached worktrees, flock orphan cleanup, 7 unit tests, never mutates the user's clone |
| Recursive `extract.nix` + `AttrPath`/`getAttrByPath` | both eval branches | Proven 28k -> 119k coverage for the residual eval path |
| Static rnix planner + blob-SHA cache | `refactor/indexer` (`static_analysis.rs`, `blob_cache.rs`) | Pure Rust, ~500k commits -> ~30k unique blobs; useful for the optional phase-2 git refinement |
| Batch bisection + OOM-aware error classification | FFI lineage (`96dc475`, `fe44912`) | OOM must surface as retriable, never as silently skipped attrs |
| Store isolation (temp `local?root=` eval store, GC, disk guards) | `4c20868`, `1b0f5c7` | Mandatory for any long eval run; `dummy://` and `auto-optimise-store` are proven traps |
| `meta.position` vs `unsafeGetAttrPos` insight | recursive branch spec section 2.4 | position = implementation file; unsafeGetAttrPos = assignment site; essential for any mapping layer |

### 2.5 Must avoid (consensus)

1. Persistent in-process Nix evaluator without a recycle policy; if FFI ever
   returns, workers must self-recycle between batches - never a parent-side
   SIGKILL watchdog as the primary memory policy.
2. Multiplicative parallelism of full evaluators (memory, not CPU, is the
   scarce resource; size concurrency as RAM / ~2.5 GiB).
3. RLIMIT_AS on Boehm-GC processes.
4. "Evaluate at every commit" as an absolute constraint - it is what killed
   every branch.
5. File-change -> attr inference as the *sole* re-evaluation trigger (proven
   multi-month blind spots; wrapper packages structurally invisible).
6. "No evaluation = unchanged" / closing-stamping all open ranges at
   checkpoints and run end (fabricates false (commit, version) pairs).
7. Exact-hash resume seeding (`last_commit_hash == last_indexed_commit`) -
   any mismatch strands the entire open set (observed four times in
   production: 11,799 / 4,208 / 2,559 / 2,302-row mass extinctions; 6,606
   attrs stranded since 2025-06-12 incl. still-alive grafx2, meslo-lg).
8. Treating eval failure as "package disappeared"; panics/errors must block
   the checkpoint, not advance past it.
9. Unbounded parent-side accumulators (upsert buffers, blob caches).
10. Abbreviated SHAs against the GitHub API (the 422 that spawned a false
    corruption theory).
11. Silent full-index fallback over a shallow clone against an existing DB
    (`src/index/mod.rs:621-686` + publish-index.yml depth-1000 fallback).

---

## 3. External Landscape

**Every serious nixpkgs version-history tool avoids per-commit evaluation.**

| Tool | Strategy | Granularity | Notes |
|---|---|---|---|
| **NixHub** (Jetify) | Parses Hydra's per-channel-eval metadata JSON; prefix-tree -> SQLite | Channel/Hydra evals | 143 thunderbird versions incl. the 143/144.0.1/145 nxv misses; 26 nh versions; reference commits are *shared channel commits* across unrelated packages. Even NixHub misses sub-advance versions (thunderbird 144.0) |
| **search.nixos.org** (flake-info) | Downloads the Hydra-built `packages.json.br` channel artifact; zero self-evaluation | Single snapshot, no history | The authoritative answer to "how do they enumerate nested sets": they don't - Hydra already did |
| **lazamar/nix-package-versions** | Evaluates one revision per channel per **5-week** interval | Coarse sampling | README documents non-exhaustiveness + missing data from failed evals - the cautionary tale |
| **nix-eval-jobs** (NixOS org) | Parallel evaluator, streamed JSON, workers restarted over `--max-memory-size` (4 GiB soft default) | n/a (a tool, not an index) | Right tool for residual eval work. nixpkgs' own CI shards at chunkSize 5000 and recommends >=16 GB RAM; full modern eval = tens of CPU-minutes per commit |

**Available historical data sources (verified live):**

- **`channels.nixos.org/<channel>/packages.json.br`** - today's
  nixos-unstable artifact: 9.8 MB brotli / 378 MB raw JSON, **144,241
  attributes including all nested sets**, with full meta (description,
  homepage, license, mainProgram, platforms, **position** - which retires
  most of `backfill.rs`). Contains both `nh` (4.3.2) and `nh-unwrapped`.
- **releases.nixos.org S3 bucket** (`nix-releases`, publicly listable) -
  every channel release ever; each dir has `git-revision`;
  `packages.json.br` present from the **nixos-21.05pre era (~late 2020)**
  onward (verified present 21.05pre/21.11pre/26.05pre; absent
  18.03-20.09pre). This is the ground-truth list of channel-advance commits.
- **gsc.io nix-channel-monitor** (`channels.nix.gsc.io`) -
  nixpkgs-unstable: **2,288 advances since 2017-06-03** (~250/yr), per-line
  `commit-hash date [advance-date]`. Polite polling requested (>=15 min);
  mirror once. The nixos-unstable file was reset (starts 2023-05) - use S3
  for that channel.
- **Hydra evals API** - timed out repeatedly (>90 s) during testing; do not
  build on it.

Channel commits are Hydra-tested and binary-cache-backed - exactly what makes
`nix shell nixpkgs/<commit>#pkg` work without compiling from source, which is
the user-facing contract nxv outputs.

---

## 4. Rewrite Strategy

### 4.1 Core model: channel-advance snapshots, not master-commit walking

Adopt the snapshot-diff (lazamar/NixHub) model with the best available data
source per era. Ground truth = "this (attr, version) pair was **observed** in
snapshot S at channel commit C". Ranges are derived facts between observed
snapshots; *every stored commit is one at which the version actually
existed.* This eliminates, by construction: phantom range extension (1a/RC-4,
1b/RC-2), file-mapping blind spots (1a/RC-2, 1c/RC-3), nested-set exclusion
(1d), checkpoint stranding, and the entire file->attr heuristic stack.

**Commit selection:**
- Enumerate every historical release from the releases.nixos.org S3 listing
  (prefixes `nixpkgs/` and `nixos/unstable/`), each providing `git-revision`,
  release name (commit-count + short rev), and date. Cross-check/augment
  2017-2020 with a one-time mirror of gsc.io history files.
- Store channel-release identity (release name, channel, date) alongside
  commits so results are explainable ("on nixpkgs-unstable between releases
  X and Y") and incremental updates are a listing diff / `git-revision` poll.

**Extraction mechanism, two eras:**
- **>= late 2020 (the bulk):** download `packages.json.br`, stream-decompress,
  stream-parse (never materialize 378 MB JSON as a DOM), fold directly into
  (attr -> name/version/meta) and diff against the previous snapshot. **Zero
  Nix evaluation.** ~1.5-2k snapshots, ~20 GB total transfer.
- **2017 -> late 2020 backfill:** evaluate only at channel-advance commits
  (a few hundred): prefer `nix-env -f nixexprs.tar.xz -qaP --json --meta`
  (the exact mechanism that later became packages.json; the tarball is in
  the same S3 dirs), falling back to `nix-eval-jobs --force-recurse
  pkgs/top-level/release.nix` at a checkout. Run inside the salvaged worker
  pool: subprocess isolation, temp `local?root=` eval store, GC between
  jobs, batch bisection, OOM-classified retries - and **self-recycling**
  workers, no parent watchdog.

**Schema:** the slim model - one row per (attribute_path, version) with
widen-only CASE-WHEN upserts (order-agnostic, so parallel and reverse
processing are safe); columns for first/last *observed* release + commit +
date; `CHECK (first_commit_date <= last_commit_date)`; metadata (description,
license, homepage, mainProgram, platforms, position/source_path) harvested
from the snapshot's meta block. Bloom filter includes full dotted attribute
paths. Publish slim as the default download (~tens of MB), full as opt-in.

**What this fixes per symptom:** thunderbird 143/143.0.1/144.0.1/145 are all
present at channel granularity (NixHub-verified); `nh` and all 120k nested
attrs appear in every modern snapshot; every recorded hash is a real,
Hydra-cached channel commit at which the version verifiably existed.

### 4.2 Memory bounds and parallelism

- **Downloads/parsing:** 4-8 parallel snapshot pipelines; each bounded by
  streaming parse state (~hundreds of MB ceiling, not multi-GB); single
  writer thread; flush upserts at a fixed cap (~50k rows) - every
  accumulator bounded from day one.
- **Backfill evals:** concurrency = available RAM / ~2.5 GiB per live
  evaluator (the branches' empirical floor), independent of CPU count;
  nix-eval-jobs soft memory limit ~4 GiB; failures retry with backoff and
  **block the checkpoint** - never recorded as disappearance, never skipped
  silently.
- No RLIMIT_AS, no PSI/watchdog subsystem, no in-process evaluator in any
  long-lived process.

### 4.3 Expected wall-clock for a full build

| Phase | Work | Estimate |
|---|---|---|
| Snapshot enumeration | S3 listing + gsc.io mirror | minutes |
| 2020 -> today | ~1.5-2k x 10 MB downloads (~20 GB) + stream-parse + diff; parse-bound, embarrassingly parallel | **hours (~half a day)** on a workstation |
| 2017 -> 2020 backfill | ~300-900 evals at advance commits, ~5-15 min each, 2-4 memory-capped workers | **~1-2 days, one-time** |
| **Total full rebuild** | | **~1-3 days**, dominated by the one-time pre-2020 backfill; the post-2020 portion is re-runnable in hours |
| Incremental update | poll `git-revision` per channel (~2 advances/day), 1 download + parse each | **seconds-minutes per run** |

Contrast: per-commit evaluation at the branches' observed 30 commits/min over
~21k commits/year ~= 11.7 h/year/range *while OOM-thrashing*; the
recursive branch's design was weeks-to-months. The no-sampling constraint is
the thing to delete, not optimize.

### 4.4 Optional phase 2: sub-advance refinement

For packages needing finer-than-advance granularity (e.g. thunderbird 144.0,
which even NixHub misses): between two adjacent advances, run
`git log --first-parent -- <meta.position path>` in the local clone and parse
version literals from diffs (rnix; versions are almost always string
literals), with Nix eval only for ambiguous cases. The salvaged static
planner/blob cache fits here. Strictly additive; the core index never
depends on it.

### 4.5 Migration & operational hardening

- **Full rebuild required.** The 2024-2025 strata (discovery collapse,
  stranded ranges, blackout windows) are unrecoverable incrementally.
- Keep the old DB for diffing; ship the new index as a schema-bumped full
  republish (slim default).
- Pin any residual git walking to fetched `origin/master`, validate ancestry
  before recording (closes the latent poisoning vulnerability from 1b).
- Data-quality monitors in publish CI: alert on >500 ranges closed by one
  commit (mass-extinction detector), zero births over a multi-week window
  (dead-scheduler detector - note GitHub's 60-day cron auto-disable), open
  attrs at HEAD vs search.nixos.org package-count floor, random full-SHA
  resolution spot-checks, `first <= last` assertion.
- Regression tests from this incident: thunderbird 142.0 must *not* span
  Aug 2025 -> Jan 2026; thunderbird 143.0 present; `nh` present; a nested attr
  (`python313Packages.requests`) present and bloom-resolvable.
- Fix docs drift: CLAUDE.md says publish-index is weekly; the cron is every
  6 hours. Update issue #21 (premise corrected) and #23 (root cause) with
  this analysis.

---

## 5. Open Questions to Verify Against the Local nixpkgs Clone (once downloaded)

1. **Hash-ancestry sweep (close out 1b definitively):** batch-verify all
   ~30k distinct first/last_commit_hashes in the production DB are ancestors
   of `origin/master` (`git merge-base --is-ancestor`). Expectation: 100%
   pass, formally retiring the local-poisoning theory.
2. **The 2025-09-10 boundary:** why do births stop exactly at `ccf4fe5b`?
   Check the merge vs non-merge commit ratio around Sep 2025 (did direct
   pushes to master effectively cease, making the merge-blind bug total?)
   and correlate with publish-index run history (was the cron also dead -
   GitHub 60-day auto-disable?). Determines whether monitoring alone would
   have caught it.
3. **`nh` extractor repro (settle 1c/RC-4):** ~~run the current extraction
   expression against modern nixpkgs for `["nh", "nh-unwrapped"]` directly.~~
   **SETTLED (2026-06-09, live repro against the local clone at `f3007fa6`):**
   `EXTRACT_NIX` with `attrNames = ["nh" "nh-unwrapped" "firefox"]` returns
   all three (`nh 4.2.0`, `nh-unwrapped 4.2.0`, `firefox 146.0.1`). RC-3
   (never targeted) is the complete explanation; the nix-side-filter
   hypothesis is dead.
4. **Channel-commit reachability:** verify the S3/gsc.io advance commits
   exist in the clone and how they relate to first-parent master (channel
   advances may point at commits not on the *first-parent* chain). Affects
   the phase-2 refinement's `git log` anchoring, not the core design.
5. **2017-2020 evaluability:** sample 5-10 advance commits per year
   2017-2020 and test `nix-env -f nixexprs.tar.xz -qaP --json --meta` under
   current Nix (fall back to a pinned older Nix if needed). Determines
   backfill tooling and whether the 2017-2020 estimate holds.
6. **Sub-advance loss quantification:** for a basket of fast-moving packages
   (thunderbird, firefox, linux, nodejs), count versions whose master
   lifetime was shorter than one channel advance, by git-logging their
   `meta.position` paths between adjacent advances. Decides whether phase 2
   ships at launch or later.
7. **Snapshot diff semantics for removals/renames:** confirm how
   attr removals, renames, and alias churn appear across adjacent
   packages.json snapshots (e.g. the nh split in Nov 2025) so range-close
   semantics are specified before implementation.
8. **packages.json field stability over time:** spot-check 2021/2022/2024
   artifacts for schema drift (field names, position format, license shape)
   to size the parser's tolerance.

---

## Appendix: Evidence Conflicts and Resolutions

| Conflict | Positions | Resolution |
|---|---|---|
| 4be2999 "phantom hash" | Architecture review: local-clone commit poisoning (likely). DB forensics + 3 others: hash exists upstream; 422 = 7-char prefix ambiguity (confirmed, 37/37 samples valid, exact date matches) | **Refuted as exercised bug; retained as latent code vulnerability** (no upstream-ancestry validation). Fix in rewrite; verify formally via open question #1 |
| `nh` wrapper drop mechanism | DB forensics: extractor nix-side filter drops the wrapper; "version bumps since Feb 2026 touched its file". Architecture + recursive-branch reviews: wrapper file has zero commits since 2026-01-07 (GitHub-verified); inherits version, never targeted | **File-never-changes explanation favored** (verified upstream, twice, independently); extractor-filter hypothesis demoted to open question #3 |
| Thunderbird gap cause | "Sampling" (issue framing): refuted - no sampling flags ever merged to main. Merge-blind window + blackout + shallow-clone resume: confirmed by code, git history, and DB histograms independently | Consolidated as 1a RC-1..4 |
| Branch count/lineage | Six branch names across reports | Three efforts: recursive-package-indexing (standalone); experimental/nix-eval-jobs-indexer -> feature/reverse-indexing; feat/native-nix-indexer -> refactor/indexer (merge-base verified; spec cites precursor experiment/smart-indexer) |
