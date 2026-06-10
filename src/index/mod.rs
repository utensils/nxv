//! Index building from nixpkgs channel-release snapshots.
//!
//! The indexer ingests *observations*: "(attribute, version) was present in
//! channel release R at commit C". Sources, in order of preference:
//!
//! - `packages.json.br` per release from releases.nixos.org (2020-03-27 →
//!   today; no Nix evaluation at all),
//! - `nix-env -qaP --json --meta` over each release's `nixexprs.tar.xz`
//!   (2016-09-28 → 2020-03-27),
//! - an optional `--head-eval` pass over the GitHub tarball of master HEAD
//!   for channel-stuck periods.
//!
//! See docs/indexer-rewrite/DESIGN.md for the full specification and
//! docs/indexer-rewrite/ANALYSIS.md for why the previous git-walking,
//! file-change-driven indexer was replaced.

pub mod eval;
pub mod monitor;
pub mod publisher;
pub mod releases;
pub mod snapshot;

use crate::bloom::PackageBloomFilter;
use crate::db::Database;
use crate::error::{NxvError, Result};

/// Entry point for `nxv index`.
///
/// Coordinator under construction: the snapshot pipeline modules (releases,
/// snapshot, monitor, eval) land in subsequent commits per DESIGN.md.
pub fn run_index(_cli: &crate::cli::Cli, _args: &crate::cli::IndexArgs) -> Result<()> {
    Err(NxvError::Config(
        "the snapshot indexer coordinator is not wired up yet (rewrite in progress)".to_string(),
    ))
}

/// Build a bloom filter over every distinct attribute path in the database.
///
/// Dotted (nested) attribute paths are inserted verbatim — the query layer
/// checks exact attribute paths against the filter.
// TODO(indexer-v2): drop the allow once the coordinator is wired up.
#[allow(dead_code)]
pub fn build_bloom_filter(db: &Database) -> Result<PackageBloomFilter> {
    use crate::db::queries;

    let attrs = queries::get_all_unique_attrs(db.connection())?;

    // 1% false-positive rate
    let mut filter = PackageBloomFilter::new(attrs.len().max(1000), 0.01);
    for attr in &attrs {
        filter.insert(attr);
    }

    Ok(filter)
}

/// Build and save a bloom filter for the index.
// TODO(indexer-v2): drop the allow once the coordinator is wired up.
#[allow(dead_code)]
pub fn save_bloom_filter<P: AsRef<std::path::Path>>(db: &Database, bloom_path: P) -> Result<()> {
    let filter = build_bloom_filter(db)?;
    let bloom_path = bloom_path.as_ref();

    if let Some(parent) = bloom_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    filter.save(bloom_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::PackageVersion;
    use chrono::TimeZone;
    use chrono::Utc;
    use tempfile::tempdir;

    fn test_package(attr: &str, version: &str) -> PackageVersion {
        PackageVersion {
            id: 0,
            name: attr.rsplit('.').next().unwrap_or(attr).to_string(),
            version: version.to_string(),
            first_commit_hash: "a".repeat(40),
            first_commit_date: Utc.timestamp_opt(1_600_000_000, 0).unwrap(),
            last_commit_hash: "b".repeat(40),
            last_commit_date: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            attribute_path: attr.to_string(),
            description: None,
            license: None,
            homepage: None,
            maintainers: None,
            platforms: None,
            source_path: None,
            known_vulnerabilities: None,
        }
    }

    #[test]
    fn test_build_bloom_filter_includes_dotted_attrs() {
        let dir = tempdir().unwrap();
        let mut db = Database::open(dir.path().join("test.db")).unwrap();
        db.upsert_observations(&[
            test_package("firefox", "100.0"),
            test_package("python313Packages.requests", "2.32.3"),
        ])
        .unwrap();

        let filter = build_bloom_filter(&db).unwrap();
        assert!(filter.contains("firefox"));
        assert!(filter.contains("python313Packages.requests"));
        assert!(!filter.contains("definitely-not-a-package-xyz"));
    }

    #[test]
    fn test_save_bloom_filter_roundtrip() {
        let dir = tempdir().unwrap();
        let mut db = Database::open(dir.path().join("test.db")).unwrap();
        db.upsert_observations(&[test_package("hello", "2.12")])
            .unwrap();

        let bloom_path = dir.path().join("sub").join("bloom.bin");
        save_bloom_filter(&db, &bloom_path).unwrap();

        let loaded = PackageBloomFilter::load(&bloom_path).unwrap();
        assert!(loaded.contains("hello"));
    }
}
