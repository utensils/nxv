//! Eval-based ingestion: the pre-packages.json era and `--head-eval`.
//!
//! Both paths use one verified recipe — `nix-env -f <tree> -qaP --json
//! --meta --argstr system x86_64-linux` with all NIXPKGS_ALLOW_* set —
//! which is exactly the mechanism Hydra's channel scripts later froze into
//! `packages.json`. Current Nix evaluates every era back to 2016 with it
//! (verified 2018/2020 samples: ~15–25 s, 0.5–1.4 GB RSS), and it
//! auto-recurses `recurseForDerivations` sets, so nested attrs are covered.
//!
//! Memory policy: the evaluator is a subprocess; process exit is the
//! reclamation mechanism. No watchdogs, no rlimits, no in-process
//! evaluator — the lessons of three OOM-killed branches (ANALYSIS.md §2).
//!
//! `nix-env` (part of every Nix installation) is the only external tool
//! these paths use; the snapshot path needs none.

use crate::error::{NxvError, Result};
use crate::index::releases::S3Client;
use crate::index::snapshot::{SnapshotEntry, parse_nix_env_json};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Eval system: pinned for reproducibility AND because pre-2020 nixpkgs
/// fails to evaluate natively on aarch64-darwin hosts (verified).
const EVAL_SYSTEM: &str = "x86_64-linux";

/// Hard ceilings for subprocesses: a wedged tar/nix-env would otherwise
/// stall a worker forever (and a CI job until GitHub's 6h kill).
const TAR_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const NIX_ENV_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// Run a command with a hard timeout, streaming stdout/stderr to temp files
/// (a pipe would deadlock the child once its buffer fills — nix-env output
/// reaches tens of MB). Kills the child on expiry.
fn run_with_timeout(cmd: &mut Command, timeout: Duration, label: &str) -> Result<Vec<u8>> {
    let stdout_file = tempfile::NamedTempFile::new()?;
    let stderr_file = tempfile::NamedTempFile::new()?;

    let mut child = cmd
        .stdout(Stdio::from(stdout_file.reopen()?))
        .stderr(Stdio::from(stderr_file.reopen()?))
        .spawn()
        .map_err(|e| NxvError::NixEval(format!("failed to run {label}: {e}")))?;

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait()? {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(NxvError::NixEval(format!(
                    "{label} exceeded its {}s timeout and was killed",
                    timeout.as_secs()
                )));
            }
            None => std::thread::sleep(Duration::from_millis(250)),
        }
    };

    if !status.success() {
        let stderr = std::fs::read_to_string(stderr_file.path()).unwrap_or_default();
        return Err(NxvError::NixEval(format!(
            "{label} failed: {}",
            stderr.lines().rev().take(5).collect::<Vec<_>>().join(" | ")
        )));
    }

    Ok(std::fs::read(stdout_file.path())?)
}

/// Download `<prefix><release>/nixexprs.tar.xz` into `dir` and return the
/// extracted source tree root.
pub fn fetch_nixexprs(
    s3: &S3Client,
    prefix: &str,
    release_name: &str,
    dir: &Path,
) -> Result<PathBuf> {
    let url = format!(
        "{}/{}{}/nixexprs.tar.xz",
        s3.base_url(),
        prefix,
        release_name
    );
    let tarball = dir.join("nixexprs.tar.xz");
    s3.download_to_file(&url, &tarball)?;
    extract_tarball(&tarball, dir)?;
    find_single_root_dir(dir)
}

/// Extract a tarball with the system `tar` (handles .tar.xz and .tar.gz).
fn extract_tarball(tarball: &Path, dest: &Path) -> Result<()> {
    run_with_timeout(
        Command::new("tar")
            .arg("-xf")
            .arg(tarball)
            .arg("-C")
            .arg(dest),
        TAR_TIMEOUT,
        "tar extraction",
    )?;
    Ok(())
}

/// Find the single directory entry inside an extraction dir (release
/// tarballs contain exactly one root directory).
fn find_single_root_dir(dir: &Path) -> Result<PathBuf> {
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            dirs.push(entry.path());
        }
    }
    match dirs.len() {
        1 => Ok(dirs.remove(0)),
        n => Err(NxvError::NixEval(format!(
            "expected exactly one root directory after extraction, found {n} in {}",
            dir.display()
        ))),
    }
}

/// Run the verified nix-env recipe over an extracted nixpkgs tree and parse
/// the resulting package set.
pub fn eval_tree(tree: &Path) -> Result<Vec<SnapshotEntry>> {
    // Distinguish "nix-env missing" up front: the coordinator hard-fails
    // the run on that message instead of failing every release.
    let stdout = run_with_timeout(
        Command::new("nix-env")
            .arg("-f")
            .arg(tree)
            .args([
                "-qaP",
                "--json",
                "--meta",
                "--argstr",
                "system",
                EVAL_SYSTEM,
            ])
            .env("NIXPKGS_ALLOW_UNFREE", "1")
            .env("NIXPKGS_ALLOW_BROKEN", "1")
            .env("NIXPKGS_ALLOW_INSECURE", "1")
            .env("NIXPKGS_ALLOW_UNSUPPORTED_SYSTEM", "1"),
        NIX_ENV_TIMEOUT,
        "nix-env evaluation",
    )
    .map_err(|e| match e {
        NxvError::NixEval(msg) if msg.contains("failed to run") => NxvError::NixEval(format!(
            "{msg} (is nix installed? required for --backfill-evals/--head-eval)"
        )),
        other => other,
    })?;

    parse_nix_env_json(&stdout[..])
}

/// Ingest one nix-env-era release: download, extract, evaluate, parse.
/// The temp dir (tarball + tree, ~300 MB) is dropped before returning.
pub fn ingest_nix_env_release(
    s3: &S3Client,
    prefix: &str,
    release_name: &str,
) -> Result<Vec<SnapshotEntry>> {
    let temp = tempfile::tempdir()?;
    let tree = fetch_nixexprs(s3, prefix, release_name, temp.path())?;
    eval_tree(&tree)
}

/// Master HEAD resolved for `--head-eval`.
#[derive(Debug, Clone)]
pub struct HeadCommit {
    pub sha: String,
    pub committed_at: DateTime<Utc>,
}

/// Resolve nixpkgs master HEAD via the GitHub API. Uses `GITHUB_TOKEN` when
/// present (CI) to avoid anonymous rate limits.
pub fn resolve_master_head(s3: &S3Client) -> Result<HeadCommit> {
    #[derive(serde::Deserialize)]
    struct ApiCommit {
        sha: String,
        commit: ApiCommitInner,
    }
    #[derive(serde::Deserialize)]
    struct ApiCommitInner {
        committer: ApiSignature,
    }
    #[derive(serde::Deserialize)]
    struct ApiSignature {
        date: DateTime<Utc>,
    }

    let mut request = s3
        .http()
        .get("https://api.github.com/repos/NixOS/nixpkgs/commits/master")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json");
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.is_empty()
    {
        request = request.bearer_auth(token);
    }

    let resp = request.send().map_err(NxvError::Network)?;
    if !resp.status().is_success() {
        return Err(NxvError::NetworkMessage(format!(
            "GitHub API HEAD resolution failed: HTTP {}",
            resp.status()
        )));
    }
    let api: ApiCommit = resp.json().map_err(NxvError::Network)?;
    if api.sha.len() != 40 {
        return Err(NxvError::NetworkMessage(format!(
            "GitHub API returned invalid sha {:?}",
            api.sha
        )));
    }
    Ok(HeadCommit {
        sha: api.sha,
        committed_at: api.commit.committer.date,
    })
}

/// Ingest master HEAD directly: download the GitHub tarball of `head.sha`,
/// extract, evaluate with the same recipe. Used when channel observations
/// lag (the channel-stuck scenario in DESIGN.md §2a).
pub fn ingest_master_head(s3: &S3Client, head: &HeadCommit) -> Result<Vec<SnapshotEntry>> {
    let temp = tempfile::tempdir()?;
    let url = format!(
        "https://github.com/NixOS/nixpkgs/archive/{}.tar.gz",
        head.sha
    );
    let tarball = temp.path().join("nixpkgs.tar.gz");
    s3.download_to_file(&url, &tarball)?;
    extract_tarball(&tarball, temp.path())?;
    let tree = find_single_root_dir(temp.path())?;
    eval_tree(&tree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_find_single_root_dir() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("nixpkgs-20.09pre218523.4a3f9aced7f")).unwrap();
        std::fs::write(dir.path().join("nixexprs.tar.xz"), b"ignored file").unwrap();

        let root = find_single_root_dir(dir.path()).unwrap();
        assert!(root.ends_with("nixpkgs-20.09pre218523.4a3f9aced7f"));

        std::fs::create_dir(dir.path().join("second-dir")).unwrap();
        assert!(find_single_root_dir(dir.path()).is_err());
    }

    /// Requires nix + network; exercised by the real backfill instead.
    #[test]
    #[ignore]
    fn test_eval_real_2018_release() {
        let s3 = S3Client::new(crate::index::releases::DEFAULT_BASE_URL).unwrap();
        let entries =
            ingest_nix_env_release(&s3, "nixpkgs/", "nixpkgs-18.03pre114401.017561209e").unwrap();
        assert!(entries.len() > 10_000, "got {} entries", entries.len());
        assert!(entries.iter().any(|e| e.attribute_path == "firefox"));
        // 2018-era nix-env recursion covers python/perl/ocaml sets (~50%
        // dotted attrs) but NOT haskellPackages (no recurseForDerivations
        // there until later; the optional -A haskellPackages pass covers it).
        assert!(
            entries
                .iter()
                .any(|e| e.attribute_path == "python27Packages.requests")
        );
    }
}
