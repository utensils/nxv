//! Shared search logic for CLI and API.
//!
//! This module provides common search functionality that can be reused
//! by both the CLI commands and the API server.

use crate::db::queries::{self, PackageVersion};
use crate::error::Result;
use clap::ValueEnum;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Sort order for search results.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum, utoipa::ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Sort by date (newest first).
    #[default]
    Date,
    /// Sort by version (semver-aware).
    Version,
    /// Sort by name (alphabetical).
    Name,
}

/// Common search options shared between CLI and API.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Package name or attribute path to search for.
    pub query: String,
    /// Filter by version (prefix match).
    pub version: Option<String>,
    /// Perform exact name match only.
    pub exact: bool,
    /// Search in package descriptions (FTS).
    pub desc: bool,
    /// Filter by license (case-insensitive contains).
    pub license: Option<String>,
    /// Sort order for results.
    pub sort: SortOrder,
    /// Reverse the sort order.
    pub reverse: bool,
    /// Show all commits (skip deduplication).
    pub full: bool,
    /// Maximum number of results (0 for unlimited).
    pub limit: usize,
    /// Offset for pagination.
    pub offset: usize,
}

impl Default for SearchOptions {
    /// Creates the default set of search options used by the CLI and API.
    ///
    /// Defaults:
    /// - `query`: empty string
    /// - `version`: `None`
    /// - `exact`: `false`
    /// - `desc`: `false`
    /// - `license`: `None`
    /// - `sort`: `SortOrder::Date`
    /// - `reverse`: `false`
    /// - `full`: `false`
    /// - `limit`: `50`
    /// - `offset`: `0`
    ///
    /// # Examples
    ///
    /// ```
    /// let opts = crate::search::SearchOptions::default();
    /// assert_eq!(opts.query, "");
    /// assert!(opts.version.is_none());
    /// assert!(!opts.exact && !opts.desc && !opts.reverse && !opts.full);
    /// assert!(opts.license.is_none());
    /// assert_eq!(opts.sort, crate::search::SortOrder::Date);
    /// assert_eq!(opts.limit, 50);
    /// assert_eq!(opts.offset, 0);
    /// ```
    fn default() -> Self {
        Self {
            query: String::new(),
            version: None,
            exact: false,
            desc: false,
            license: None,
            sort: SortOrder::Date,
            reverse: false,
            full: false,
            limit: 50,
            offset: 0,
        }
    }
}

/// Result of a search operation with pagination metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The matching packages.
    pub data: Vec<PackageVersion>,
    /// Total count before pagination.
    pub total: usize,
    /// Whether there are more results available.
    pub has_more: bool,
    /// The actual limit applied by the server (may be less than requested due to server caps).
    /// Only set when using a remote API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_limit: Option<usize>,
}

/// Performs a package search using the provided options and returns paginated results.
///
/// Applies filters, sorting, deduplication (unless disabled), and pagination according to `opts`.
///
/// # Returns
///
/// `SearchResult` containing the matching `PackageVersion` entries, the total number of matches
/// before pagination, and a `has_more` flag indicating if additional results exist.
///
/// # Examples
///
/// ```no_run
/// use rusqlite::Connection;
/// let conn = Connection::open_in_memory().unwrap();
/// let opts = SearchOptions::default();
/// let res = execute_search(&conn, &opts).unwrap();
/// // `res.data` holds the matching package versions, `res.total` is the total matches.
/// assert!(res.total >= 0);
/// ```
pub fn execute_search(conn: &Connection, opts: &SearchOptions) -> Result<SearchResult> {
    // Step 1: Query database
    let results = if opts.desc {
        // FTS search on description
        queries::search_by_description(conn, &opts.query)?
    } else if let Some(ref version) = opts.version {
        // Search by name and version
        queries::search_by_name_version(conn, &opts.query, version)?
    } else if opts.exact {
        // Exact match on attribute_path. Avoid the old prefix query + Rust
        // filter path, which materialized thousands of sibling attrs on v4 DBs.
        queries::search_by_attr_exact(conn, &opts.query)?
    } else {
        // Prefix search on attribute_path
        queries::search_by_attr(conn, &opts.query)?
    };

    // Step 2: Apply license filter
    let results = filter_by_license(results, opts.license.as_deref());

    // Step 3: Sort results
    let mut results = results;
    sort_results(&mut results, opts.sort, opts.reverse);

    // Step 4: Deduplicate (unless full mode)
    let results = if opts.full {
        results
    } else {
        deduplicate(results)
    };

    // Step 5: Apply pagination
    let (data, total) = paginate(results, opts.limit, opts.offset);
    let has_more = opts.limit > 0 && total > opts.offset + data.len();

    Ok(SearchResult {
        data,
        total,
        has_more,
        applied_limit: None, // Local searches don't cap limits
    })
}

/// Filter package versions by license using a case-insensitive substring match.
///
/// If `license` is `None`, the input `results` are returned unchanged. When `license` is
/// provided, only entries whose `license` field is present and contains the given string
/// (case-insensitive) are retained.
///
/// # Examples
///
/// ```
/// let empty: Vec<PackageVersion> = Vec::new();
/// let filtered = filter_by_license(empty, Some("mit"));
/// assert!(filtered.is_empty());
/// ```
pub fn filter_by_license(
    results: Vec<PackageVersion>,
    license: Option<&str>,
) -> Vec<PackageVersion> {
    match license {
        Some(license) => {
            let license_lower = license.to_lowercase();
            results
                .into_iter()
                .filter(|p| {
                    p.license
                        .as_ref()
                        .is_some_and(|l| l.to_lowercase().contains(&license_lower))
                })
                .collect()
        }
        None => results,
    }
}

/// Sort results based on sort order.
///
/// For `Version` sort, uses semver-aware comparison with fallback to string comparison.
pub fn sort_results(results: &mut [PackageVersion], order: SortOrder, reverse: bool) {
    match order {
        SortOrder::Date => {
            results.sort_by_key(|r| std::cmp::Reverse(r.last_commit_date));
        }
        SortOrder::Version => {
            results.sort_by(|a, b| {
                // Semver-aware version comparison
                match (
                    semver::Version::parse(&a.version),
                    semver::Version::parse(&b.version),
                ) {
                    (Ok(va), Ok(vb)) => va.cmp(&vb),
                    (Ok(_), Err(_)) => std::cmp::Ordering::Less, // Valid semver sorts before invalid
                    (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                    (Err(_), Err(_)) => a.version.cmp(&b.version), // Fall back to string comparison
                }
            });
        }
        SortOrder::Name => {
            results.sort_by_key(|r| r.name.clone());
        }
    }

    if reverse {
        results.reverse();
    }
}

/// Remove duplicate package versions identified by (attribute_path, version),
/// preserving the most recent occurrence according to the input order.
///
/// Duplicates are determined by the tuple `(attribute_path, version)`. The first
/// occurrence of each unique tuple in `results` is kept; subsequent duplicates
/// are discarded.
///
/// # Examples
///
/// ```
/// use crate::search::deduplicate;
/// use crate::db::PackageVersion;
///
/// let a1 = PackageVersion { attribute_path: "pkg::a".into(), version: "1.0".into(), ..Default::default() };
/// let a2 = PackageVersion { attribute_path: "pkg::a".into(), version: "1.0".into(), ..Default::default() };
/// let a3 = PackageVersion { attribute_path: "pkg::a".into(), version: "2.0".into(), ..Default::default() };
///
/// let out = deduplicate(vec![a1, a2, a3]);
/// assert_eq!(out.len(), 2);
/// ```
pub fn deduplicate(results: Vec<PackageVersion>) -> Vec<PackageVersion> {
    let mut seen: HashSet<(String, String)> = HashSet::new();
    results
        .into_iter()
        .filter(|p| seen.insert((p.attribute_path.clone(), p.version.clone())))
        .collect()
}

/// Paginate a list of `PackageVersion` by applying `offset` and an optional `limit`.
///
/// If `limit` is zero, all items after `offset` are returned. The second element of the
/// returned tuple is the total number of items before pagination.
///
/// # Examples
///
/// ```
/// let (page, total) = paginate(Vec::<_>::new(), 10, 0);
/// assert!(page.is_empty());
/// assert_eq!(total, 0);
/// ```
pub fn paginate(
    results: Vec<PackageVersion>,
    limit: usize,
    offset: usize,
) -> (Vec<PackageVersion>, usize) {
    let total = results.len();

    let data: Vec<_> = if limit > 0 {
        results.into_iter().skip(offset).take(limit).collect()
    } else {
        results.into_iter().skip(offset).collect()
    };

    (data, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    /// Create a PackageVersion test instance with the given name, version, attribute path, and a last-commit date offset in days.
    ///
    /// The `date_offset` is subtracted from the current UTC time to set `last_commit_date` (and `first_commit_date` is set 10 days earlier).
    ///
    /// # Examples
    ///
    /// ```
    /// let pkg = make_package("foo", "1.0.0", "foo.attr", 5);
    /// assert_eq!(pkg.name, "foo");
    /// assert_eq!(pkg.version, "1.0.0");
    /// assert_eq!(pkg.attribute_path, "foo.attr");
    /// ```
    fn make_package(name: &str, version: &str, attr: &str, date_offset: i64) -> PackageVersion {
        let now = Utc::now();
        PackageVersion {
            id: 1,
            name: name.to_string(),
            version: version.to_string(),
            first_commit_hash: "abc1234567890".to_string(),
            first_commit_date: now - chrono::Duration::days(date_offset + 10),
            last_commit_hash: "def1234567890".to_string(),
            last_commit_date: now - chrono::Duration::days(date_offset),
            attribute_path: attr.to_string(),
            description: Some(format!("{} package", name)),
            license: Some("MIT".to_string()),
            homepage: None,
            maintainers: None,
            platforms: None,
            source_path: None,
            known_vulnerabilities: None,
        }
    }

    #[test]
    fn test_filter_by_license() {
        let packages = vec![
            {
                let mut p = make_package("foo", "1.0", "foo", 0);
                p.license = Some("MIT".to_string());
                p
            },
            {
                let mut p = make_package("bar", "1.0", "bar", 1);
                p.license = Some("GPL-3.0".to_string());
                p
            },
            {
                let mut p = make_package("baz", "1.0", "baz", 2);
                p.license = None;
                p
            },
        ];

        let filtered = filter_by_license(packages.clone(), Some("mit"));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "foo");

        let filtered = filter_by_license(packages.clone(), Some("GPL"));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "bar");

        let filtered = filter_by_license(packages, None);
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_sort_by_date() {
        let mut packages = vec![
            make_package("a", "1.0", "a", 10),
            make_package("b", "1.0", "b", 5),
            make_package("c", "1.0", "c", 15),
        ];

        sort_results(&mut packages, SortOrder::Date, false);
        assert_eq!(packages[0].name, "b"); // Most recent (5 days ago)
        assert_eq!(packages[1].name, "a");
        assert_eq!(packages[2].name, "c"); // Oldest (15 days ago)
    }

    #[test]
    fn test_sort_by_version() {
        let mut packages = vec![
            make_package("a", "1.10.0", "a", 0),
            make_package("a", "1.2.0", "a", 1),
            make_package("a", "1.9.0", "a", 2),
        ];

        sort_results(&mut packages, SortOrder::Version, false);
        assert_eq!(packages[0].version, "1.2.0");
        assert_eq!(packages[1].version, "1.9.0");
        assert_eq!(packages[2].version, "1.10.0");
    }

    #[test]
    fn test_sort_by_name() {
        let mut packages = vec![
            make_package("zsh", "1.0", "zsh", 0),
            make_package("bash", "1.0", "bash", 1),
            make_package("fish", "1.0", "fish", 2),
        ];

        sort_results(&mut packages, SortOrder::Name, false);
        assert_eq!(packages[0].name, "bash");
        assert_eq!(packages[1].name, "fish");
        assert_eq!(packages[2].name, "zsh");
    }

    #[test]
    fn test_sort_reverse() {
        let mut packages = vec![
            make_package("a", "1.0", "a", 0),
            make_package("b", "1.0", "b", 1),
            make_package("c", "1.0", "c", 2),
        ];

        sort_results(&mut packages, SortOrder::Name, true);
        assert_eq!(packages[0].name, "c");
        assert_eq!(packages[1].name, "b");
        assert_eq!(packages[2].name, "a");
    }

    #[test]
    fn test_deduplicate() {
        let packages = vec![
            make_package("python", "3.11.0", "python", 0),
            make_package("python", "3.11.0", "python", 5), // Duplicate
            make_package("python", "3.12.0", "python", 1),
        ];

        let deduped = deduplicate(packages);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_paginate() {
        let packages: Vec<_> = (0..10)
            .map(|i| make_package(&format!("pkg{}", i), "1.0", &format!("pkg{}", i), i))
            .collect();

        // Test limit
        let (data, total) = paginate(packages.clone(), 5, 0);
        assert_eq!(data.len(), 5);
        assert_eq!(total, 10);

        // Test offset
        let (data, total) = paginate(packages.clone(), 5, 5);
        assert_eq!(data.len(), 5);
        assert_eq!(total, 10);
        assert_eq!(data[0].name, "pkg5");

        // Test offset + limit exceeding total
        let (data, total) = paginate(packages.clone(), 5, 8);
        assert_eq!(data.len(), 2);
        assert_eq!(total, 10);

        // Test unlimited (limit = 0)
        let (data, total) = paginate(packages, 0, 0);
        assert_eq!(data.len(), 10);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_search_options_default() {
        let opts = SearchOptions::default();
        assert_eq!(opts.limit, 50);
        assert_eq!(opts.offset, 0);
        assert!(!opts.exact);
        assert!(!opts.desc);
        assert!(!opts.full);
        assert!(!opts.reverse);
        assert_eq!(opts.sort, SortOrder::Date);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        /// Generate an arbitrary PackageVersion for testing.
        fn arb_package_version()(
            name in "[a-z][a-z0-9_-]{0,30}",
            version in "[0-9]{1,3}\\.[0-9]{1,3}(\\.[0-9]{1,3})?",
            attr in "[a-z][a-z0-9_-]{0,30}",
            license in prop_oneof![
                Just(Some("MIT".to_string())),
                Just(Some("GPL-3.0".to_string())),
                Just(Some("Apache-2.0".to_string())),
                Just(None),
            ],
        ) -> PackageVersion {
            PackageVersion {
                id: 1,
                name,
                version,
                first_commit_hash: "abc123".to_string(),
                first_commit_date: chrono::Utc::now(),
                last_commit_hash: "def456".to_string(),
                last_commit_date: chrono::Utc::now(),
                attribute_path: attr,
                description: Some("test package".to_string()),
                license,
                homepage: None,
                maintainers: None,
                platforms: None,
                source_path: None,
                known_vulnerabilities: None,
            }
        }
    }

    proptest! {
        /// Sorting should never panic regardless of input.
        #[test]
        fn sort_never_panics(
            packages in prop::collection::vec(arb_package_version(), 0..100),
            order in prop_oneof![
                Just(SortOrder::Date),
                Just(SortOrder::Version),
                Just(SortOrder::Name),
            ],
            reverse in any::<bool>(),
        ) {
            let mut pkgs = packages;
            sort_results(&mut pkgs, order, reverse);
            // If we get here without panicking, the test passes
        }

        /// Deduplication should never increase the number of results.
        #[test]
        fn deduplicate_never_increases(
            packages in prop::collection::vec(arb_package_version(), 0..100),
        ) {
            let original_len = packages.len();
            let deduped = deduplicate(packages);
            prop_assert!(deduped.len() <= original_len);
        }

        /// Pagination should never return more items than requested.
        #[test]
        fn paginate_respects_limit(
            packages in prop::collection::vec(arb_package_version(), 0..100),
            limit in 0usize..50,
            offset in 0usize..50,
        ) {
            let (data, total) = paginate(packages.clone(), limit, offset);
            prop_assert_eq!(total, packages.len());
            if limit > 0 {
                prop_assert!(data.len() <= limit);
            }
        }

        /// License filter should only return packages with matching license.
        #[test]
        fn license_filter_correctness(
            packages in prop::collection::vec(arb_package_version(), 0..50),
            filter in "[A-Za-z]{1,10}",
        ) {
            let filtered = filter_by_license(packages, Some(&filter));
            let filter_lower = filter.to_lowercase();
            for pkg in filtered {
                let license = pkg.license.unwrap();
                prop_assert!(license.to_lowercase().contains(&filter_lower));
            }
        }

        /// Version sorting should be stable (same input = same output order).
        #[test]
        fn sort_is_deterministic(
            packages in prop::collection::vec(arb_package_version(), 1..20),
            order in prop_oneof![
                Just(SortOrder::Date),
                Just(SortOrder::Version),
                Just(SortOrder::Name),
            ],
        ) {
            let mut pkgs1 = packages.clone();
            let mut pkgs2 = packages;
            sort_results(&mut pkgs1, order, false);
            sort_results(&mut pkgs2, order, false);

            for (p1, p2) in pkgs1.iter().zip(pkgs2.iter()) {
                prop_assert_eq!(&p1.name, &p2.name);
                prop_assert_eq!(&p1.version, &p2.version);
            }
        }
    }
}

/// Prove the covering candidate index and the scan fallback yield identical
/// observable output through the full `execute_search` pipeline — license
/// filtering, sorting, dedup/full, `--limit 0`, and offset pagination across
/// tie boundaries. Each test builds two databases with identical rows: one
/// carrying `idx_packages_search_nocase`, one with it dropped so its candidate
/// queries take the scan path. A green assertion therefore means the changed
/// indexed path matches the fallback observably, not that both ran the fallback.
#[cfg(test)]
mod indexed_parity_tests {
    use super::*;
    use crate::db::Database;
    use tempfile::{TempDir, tempdir};

    // Rows are hand-tuned so higher-level modes are non-trivial. The production
    // schema enforces UNIQUE(attribute_path, version) and
    // CHECK(first_commit_date <= last_commit_date), so every row is a distinct
    // pair with first_date <= last_date:
    //  * `parity.alpha/bravo/charlie` share depth and `last_commit_date`,
    //    forcing sort ties that only the SQL `id ASC` tie-break resolves
    //    deterministically; `charlie` is GPL so a `license` filter drops it;
    //  * `vpar.*` exercise the version-filtered path (`vpar.four` is excluded by
    //    a `1` version prefix);
    //  * `lw%hit` / `lw_hit` / `lwXhit` verify literal `%` and `_` handling.
    const SEED: &str = r#"
        INSERT INTO package_versions
            (name, version, first_commit_hash, first_commit_date,
             last_commit_hash, last_commit_date, attribute_path, description, license)
        VALUES
            ('aaa', '1.0.0', 'h1',  100, 'l1', 900, 'parity.alpha',   'd', 'MIT'),
            ('bbb', '1.0.0', 'h2',  100, 'l2', 900, 'parity.bravo',   'd', 'MIT'),
            ('ccc', '1.0.0', 'h3',  100, 'l3', 900, 'parity.charlie', 'd', 'GPL-3.0'),
            ('ddd', '2.0.0', 'h4',  200, 'l4', 800, 'parity.delta',   'd', 'MIT'),
            ('eee', '2.0.0', 'h5',  200, 'l5', 700, 'parity.echo',    'd', 'MIT'),
            ('vaa', '1.1.0', 'h7',  300, 'l7', 600, 'vpar.one',       'd', 'MIT'),
            ('vbb', '1.1.0', 'h8',  300, 'l8', 600, 'vpar.two',       'd', 'MIT'),
            ('vcc', '1.2.0', 'h9',  300, 'l9', 650, 'vpar.three',     'd', 'MIT'),
            ('vdd', '2.0.0', 'h10', 300, 'la', 660, 'vpar.four',      'd', 'MIT'),
            ('lw1', '1.0.0', 'h11', 400, 'lb', 500, 'lw%hit',         'd', 'MIT'),
            ('lw2', '1.0.0', 'h12', 400, 'lc', 500, 'lw_hit',         'd', 'MIT'),
            ('lw3', '1.0.0', 'h13', 400, 'ld', 500, 'lwXhit',         'd', 'MIT');
    "#;

    fn index_present(db: &Database) -> bool {
        db.connection()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_packages_search_nocase'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0
    }

    /// `(dir, indexed, fallback)` seeded identically; `fallback` has the covering
    /// index dropped so its candidate queries take the scan path.
    fn parity_dbs() -> (TempDir, Database, Database) {
        let dir = tempdir().unwrap();
        let indexed = Database::open(dir.path().join("indexed.db")).unwrap();
        indexed.connection().execute_batch(SEED).unwrap();
        let fallback = Database::open(dir.path().join("fallback.db")).unwrap();
        fallback.connection().execute_batch(SEED).unwrap();
        fallback
            .connection()
            .execute("DROP INDEX idx_packages_search_nocase", [])
            .unwrap();

        // Guard: the two databases genuinely differ in the covering index, so a
        // green parity assertion can never mean "both fell back to the scan".
        assert!(
            index_present(&indexed),
            "indexed db must carry the covering index"
        );
        assert!(
            !index_present(&fallback),
            "fallback db must not carry the covering index"
        );
        (dir, indexed, fallback)
    }

    fn ids(result: &SearchResult) -> Vec<i64> {
        result.data.iter().map(|p| p.id).collect()
    }

    fn base(query: &str) -> SearchOptions {
        SearchOptions {
            query: query.to_string(),
            limit: 50,
            ..Default::default()
        }
    }

    /// `execute_search` must agree on rows, order, total, and `has_more` across
    /// both databases for the given options.
    fn assert_parity(indexed: &Database, fallback: &Database, opts: &SearchOptions) {
        let a = execute_search(indexed.connection(), opts).unwrap();
        let b = execute_search(fallback.connection(), opts).unwrap();
        assert_eq!(ids(&a), ids(&b), "row id order diverged for {opts:?}");
        assert_eq!(a.total, b.total, "total diverged for {opts:?}");
        assert_eq!(a.has_more, b.has_more, "has_more diverged for {opts:?}");
    }

    #[test]
    fn indexed_matches_fallback_across_filter_sort_full_and_limit() {
        let (_dir, indexed, fallback) = parity_dbs();

        let mut matrix: Vec<SearchOptions> = vec![
            base("parity"),
            SearchOptions {
                license: Some("mit".to_string()),
                ..base("parity")
            },
            SearchOptions {
                full: true,
                ..base("parity")
            },
            SearchOptions {
                limit: 0,
                ..base("parity")
            },
        ];
        for sort in [SortOrder::Date, SortOrder::Version, SortOrder::Name] {
            for reverse in [false, true] {
                matrix.push(SearchOptions {
                    sort,
                    reverse,
                    ..base("parity")
                });
            }
        }

        for opts in &matrix {
            assert_parity(&indexed, &fallback, opts);
        }

        // Sanity: the license filter actually changes the set, so the parity
        // above is meaningful rather than vacuous. (Production's
        // UNIQUE(attribute_path, version) means dedup and `--full` return the
        // same rows on real data, so only their cross-path agreement is
        // asserted, not a count difference.)
        let all = execute_search(indexed.connection(), &base("parity")).unwrap();
        assert_eq!(all.total, 5, "five distinct parity rows expected");
        let mit = execute_search(
            indexed.connection(),
            &SearchOptions {
                license: Some("mit".to_string()),
                ..base("parity")
            },
        )
        .unwrap();
        assert!(
            mit.total < all.total,
            "expected license filter to drop the GPL row"
        );
    }

    #[test]
    fn indexed_offset_pagination_is_deterministic_across_ties() {
        let (_dir, indexed, fallback) = parity_dbs();

        // Walk fixed-size pages and confirm both databases return the same page
        // at every offset — including across the `parity.*` tie boundary.
        for offset in [0, 2, 4, 6] {
            assert_parity(
                &indexed,
                &fallback,
                &SearchOptions {
                    limit: 2,
                    offset,
                    ..base("parity")
                },
            );
        }

        // Concatenating pages must reconstruct the unpaginated order exactly, so
        // no tie row is dropped or repeated across a page boundary.
        let full_ids = ids(&execute_search(
            indexed.connection(),
            &SearchOptions {
                limit: 0,
                ..base("parity")
            },
        )
        .unwrap());
        let mut paged: Vec<i64> = Vec::new();
        let mut offset = 0;
        loop {
            let page = execute_search(
                indexed.connection(),
                &SearchOptions {
                    limit: 2,
                    offset,
                    ..base("parity")
                },
            )
            .unwrap();
            if page.data.is_empty() {
                break;
            }
            paged.extend(ids(&page));
            offset += 2;
            assert!(offset < 100, "pagination walk failed to terminate");
        }
        assert_eq!(
            paged, full_ids,
            "paged walk diverged from unpaginated order"
        );
    }

    #[test]
    fn indexed_matches_fallback_on_version_filtered_path() {
        let (_dir, indexed, fallback) = parity_dbs();

        let versioned = || SearchOptions {
            version: Some("1".to_string()),
            ..base("vpar")
        };

        // The `1` version prefix excludes `vpar.four` (2.0.0) on both paths.
        let res = execute_search(indexed.connection(), &versioned()).unwrap();
        assert_eq!(res.total, 3, "version prefix should keep only the 1.x rows");
        assert!(res.data.iter().all(|p| p.version.starts_with('1')));

        let mut matrix = vec![
            versioned(),
            SearchOptions {
                sort: SortOrder::Version,
                ..versioned()
            },
            SearchOptions {
                full: true,
                ..versioned()
            },
            SearchOptions {
                limit: 0,
                ..versioned()
            },
        ];
        for offset in [0, 1, 2] {
            matrix.push(SearchOptions {
                limit: 1,
                offset,
                ..versioned()
            });
        }
        for opts in &matrix {
            assert_parity(&indexed, &fallback, opts);
        }
    }

    #[test]
    fn indexed_matches_fallback_on_literal_wildcards() {
        let (_dir, indexed, fallback) = parity_dbs();

        // Literal `%` must not act as a wildcard: only `lw%hit` matches, not the
        // `lwXhit` decoy.
        let pct = execute_search(indexed.connection(), &base("lw%")).unwrap();
        assert_eq!(
            pct.data
                .iter()
                .map(|p| p.attribute_path.as_str())
                .collect::<Vec<_>>(),
            vec!["lw%hit"]
        );
        assert_parity(&indexed, &fallback, &base("lw%"));

        // Literal `_` must not act as a wildcard: only `lw_hit` matches.
        let underscore = execute_search(indexed.connection(), &base("lw_hit")).unwrap();
        assert_eq!(
            underscore
                .data
                .iter()
                .map(|p| p.attribute_path.as_str())
                .collect::<Vec<_>>(),
            vec!["lw_hit"]
        );
        assert_parity(&indexed, &fallback, &base("lw_hit"));
    }
}
