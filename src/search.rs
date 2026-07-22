//! Shared search logic for CLI and API.
//!
//! This module provides common search functionality that can be reused
//! by both the CLI commands and the API server.

use crate::db::queries::{self, PackageVersion};
use crate::error::Result;
use clap::ValueEnum;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashSet};

const SUGGESTION_LIMIT: usize = 5;
const SUGGESTION_CANDIDATE_LIMIT: usize = 512;

/// Sort order for search results.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum, utoipa::ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Preserve query relevance (exact/shallow attribute paths before nested sets).
    #[default]
    Relevance,
    /// Sort by date (newest first).
    Date,
    /// Sort by version (semver-aware).
    Version,
    /// Sort by name (alphabetical).
    Name,
}

/// How a version-qualified attribute search resolved its package scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    /// Require one exact attribute path.
    Exact,
    /// Search only the shallowest attribute-path tier matching the prefix.
    Shallowest,
    /// Search every attribute-path depth matching the prefix.
    AllDepths,
}

/// A nearby attribute/version pair offered after a version miss.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchSuggestion {
    pub attribute_path: String,
    pub version: String,
}

/// Machine-readable explanation of a version-qualified search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchResolution {
    pub scope: SearchScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_depth: Option<usize>,
    pub requested_version: String,
    /// Whether the requested version existed before secondary filters.
    pub version_matched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deeper_matches_available: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<SearchSuggestion>,
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
    /// Include every matching attribute-path depth for version searches.
    pub all_depths: bool,
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
    /// - `sort`: `SortOrder::Relevance`
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
    /// assert_eq!(opts.sort, crate::search::SortOrder::Relevance);
    /// assert_eq!(opts.limit, 50);
    /// assert_eq!(opts.offset, 0);
    /// ```
    fn default() -> Self {
        Self {
            query: String::new(),
            version: None,
            exact: false,
            all_depths: false,
            desc: false,
            license: None,
            sort: SortOrder::Relevance,
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
    /// Resolution details for version-qualified searches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<SearchResolution>,
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
    let (results, resolution) = if opts.desc {
        // FTS search on description
        (queries::search_by_description(conn, &opts.query)?, None)
    } else if let Some(ref version) = opts.version {
        if opts.exact {
            execute_exact_version_search(conn, opts, version)?
        } else if opts.all_depths {
            execute_all_depths_version_search(conn, opts, version)?
        } else {
            execute_scoped_version_search(conn, opts, version)?
        }
    } else if opts.exact {
        // Exact match on attribute_path. Avoid the old prefix query + Rust
        // filter path, which materialized thousands of sibling attrs on v4 DBs.
        (queries::search_by_attr_exact(conn, &opts.query)?, None)
    } else {
        // Prefix search on attribute_path
        (queries::search_by_attr(conn, &opts.query)?, None)
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
        resolution,
    })
}

fn execute_exact_version_search(
    conn: &Connection,
    opts: &SearchOptions,
    version: &str,
) -> Result<(Vec<PackageVersion>, Option<SearchResolution>)> {
    let results = queries::search_by_attr_exact_version(conn, &opts.query, version)?;
    let version_matched = !results.is_empty();
    let attr_exists = version_matched || queries::attribute_path_exists(conn, &opts.query)?;
    let suggestions = if !version_matched && attr_exists {
        rank_suggestions(
            queries::search_by_attr_exact(conn, &opts.query)?,
            &opts.query,
            version,
        )
    } else {
        Vec::new()
    };

    Ok((
        results,
        Some(SearchResolution {
            scope: SearchScope::Exact,
            resolved_depth: attr_exists.then(|| attribute_depth(&opts.query)),
            requested_version: version.to_string(),
            version_matched,
            deeper_matches_available: None,
            suggestions,
        }),
    ))
}

fn execute_scoped_version_search(
    conn: &Connection,
    opts: &SearchOptions,
    version: &str,
) -> Result<(Vec<PackageVersion>, Option<SearchResolution>)> {
    let Some(depth) = queries::min_attribute_depth(conn, &opts.query)? else {
        return Ok((
            Vec::new(),
            Some(SearchResolution {
                scope: SearchScope::Shallowest,
                resolved_depth: None,
                requested_version: version.to_string(),
                version_matched: false,
                deeper_matches_available: Some(false),
                suggestions: Vec::new(),
            }),
        ));
    };

    let results = queries::search_by_name_version_at_depth(conn, &opts.query, version, depth)?;
    let version_matched = !results.is_empty();
    let (deeper_matches_available, suggestions) = if version_matched {
        (None, Vec::new())
    } else {
        let deeper = queries::deeper_version_match_exists(conn, &opts.query, version, depth)?;
        let candidates = queries::search_version_suggestion_candidates(
            conn,
            &opts.query,
            depth,
            &preferred_version_prefix(version),
            SUGGESTION_CANDIDATE_LIMIT,
        )?;
        (
            Some(deeper),
            rank_suggestions(candidates, &opts.query, version),
        )
    };

    Ok((
        results,
        Some(SearchResolution {
            scope: SearchScope::Shallowest,
            resolved_depth: Some(depth),
            requested_version: version.to_string(),
            version_matched,
            deeper_matches_available,
            suggestions,
        }),
    ))
}

fn execute_all_depths_version_search(
    conn: &Connection,
    opts: &SearchOptions,
    version: &str,
) -> Result<(Vec<PackageVersion>, Option<SearchResolution>)> {
    let results = queries::search_by_name_version(conn, &opts.query, version)?;
    let version_matched = !results.is_empty();
    let (resolved_depth, suggestions) = if version_matched {
        (None, Vec::new())
    } else if let Some(depth) = queries::min_attribute_depth(conn, &opts.query)? {
        let candidates = queries::search_version_suggestion_candidates(
            conn,
            &opts.query,
            depth,
            &preferred_version_prefix(version),
            SUGGESTION_CANDIDATE_LIMIT,
        )?;
        (
            Some(depth),
            rank_suggestions(candidates, &opts.query, version),
        )
    } else {
        (None, Vec::new())
    };

    Ok((
        results,
        Some(SearchResolution {
            scope: SearchScope::AllDepths,
            resolved_depth,
            requested_version: version.to_string(),
            version_matched,
            deeper_matches_available: None,
            suggestions,
        }),
    ))
}

fn attribute_depth(attribute_path: &str) -> usize {
    attribute_path.bytes().filter(|byte| *byte == b'.').count()
}

fn numeric_components(version: &str) -> Vec<u64> {
    version
        .split(['.', '-', '_', '+'])
        .map(|part| {
            part.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
        })
        .take_while(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect()
}

fn preferred_version_prefix(version: &str) -> String {
    let components = numeric_components(version);
    if components.len() >= 2 {
        format!("{}.{}", components[0], components[1])
    } else if let Some(component) = components.first() {
        component.to_string()
    } else {
        version
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric())
            .collect()
    }
}

fn shared_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(a, b)| a.eq_ignore_ascii_case(b))
        .count()
}

fn compare_version_distance(left: &str, right: &str, requested: &str) -> Ordering {
    let requested_numeric = numeric_components(requested);
    let left_numeric = numeric_components(left);
    let right_numeric = numeric_components(right);

    if !requested_numeric.is_empty() && !left_numeric.is_empty() && !right_numeric.is_empty() {
        let numeric_key = |candidate: &[u64]| {
            let common = requested_numeric
                .iter()
                .zip(candidate)
                .take_while(|(a, b)| a == b)
                .count();
            let distance = requested_numeric
                .iter()
                .zip(candidate)
                .find_map(|(a, b)| (a != b).then(|| a.abs_diff(*b)))
                .unwrap_or_else(|| requested_numeric.len().abs_diff(candidate.len()) as u64);
            (common, distance)
        };
        let (left_common, left_distance) = numeric_key(&left_numeric);
        let (right_common, right_distance) = numeric_key(&right_numeric);
        return right_common
            .cmp(&left_common)
            .then_with(|| left_distance.cmp(&right_distance));
    }

    shared_prefix_len(right, requested).cmp(&shared_prefix_len(left, requested))
}

fn rank_suggestions(
    candidates: Vec<PackageVersion>,
    query: &str,
    requested_version: &str,
) -> Vec<SearchSuggestion> {
    let mut candidates = candidates;
    candidates.sort_by(|left, right| {
        compare_version_distance(&left.version, &right.version, requested_version)
            .then_with(|| (left.attribute_path != query).cmp(&(right.attribute_path != query)))
            .then_with(|| left.attribute_path.len().cmp(&right.attribute_path.len()))
            .then_with(|| right.last_commit_date.cmp(&left.last_commit_date))
            .then_with(|| left.attribute_path.cmp(&right.attribute_path))
            .then_with(|| right.version.cmp(&left.version))
    });

    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|candidate| {
            seen.insert((candidate.attribute_path.clone(), candidate.version.clone()))
        })
        .take(SUGGESTION_LIMIT)
        .map(|candidate| SearchSuggestion {
            attribute_path: candidate.attribute_path,
            version: candidate.version,
        })
        .collect()
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
        // The query layer has already produced deterministic relevance order
        // using the covering search index. Preserve it without another sort.
        SortOrder::Relevance => {}
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
        assert!(!opts.all_depths);
        assert!(!opts.desc);
        assert!(!opts.full);
        assert!(!opts.reverse);
        assert_eq!(opts.sort, SortOrder::Relevance);
    }
}

/// Regression coverage for #52: `--exact` was shadowed by the version filter
/// because `execute_search` dispatched on `version` before `exact`, routing the
/// query into the prefix-matching `search_by_name_version`. These tests drive
/// the real dispatch, so they fail if the branch ordering regresses.
///
/// Both the CLI and the API server build `SearchOptions` and call
/// `execute_search`, so covering it here covers both front ends.
#[cfg(test)]
mod exact_with_version_tests {
    use super::*;
    use crate::db::Database;
    use tempfile::{TempDir, tempdir};

    /// Mirrors the issue's repro shape. Deliberately tuned so every assertion
    /// below is non-vacuous — i.e. genuinely fails if the `exact` branch is
    /// removed from `execute_search`:
    ///  * `python` is the only exact attr, and it is NEVER at 2.7.x, so the
    ///    2.7 queries must return zero rather than the siblings that do match;
    ///  * `python313Packages.django` sits at 3.11.2 so even the "matching"
    ///    3.11 query has a sibling to wrongly pick up without the fix.
    const SEED: &str = r#"
        INSERT INTO package_versions
            (name, version, first_commit_hash, first_commit_date,
             last_commit_hash, last_commit_date, attribute_path, description, license)
        VALUES
            ('python', '3.11.0', 'h1', 100, 'l1', 900, 'python',  'd', 'MIT'),
            ('python2', '2.7.18', 'h2', 100, 'l2', 800, 'python2', 'd', 'MIT'),
            ('granian', '2.7.9', 'h3', 100, 'l3', 700, 'python314Packages.granian', 'd', 'MIT'),
            ('aiohappyeyeballs', '2.7.1', 'h4', 100, 'l4', 600, 'python313Packages.aiohappyeyeballs', 'd', 'MIT'),
            ('django', '3.11.2', 'h5', 100, 'l5', 500, 'python313Packages.django', 'd', 'MIT');
    "#;

    fn seeded_db() -> (TempDir, Database) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db")).unwrap();
        db.connection().execute_batch(SEED).unwrap();
        (dir, db)
    }

    fn opts(query: &str, version: Option<&str>, exact: bool) -> SearchOptions {
        SearchOptions {
            query: query.to_string(),
            version: version.map(str::to_string),
            exact,
            limit: 0, // unlimited, matching the issue's `--limit 0`
            ..Default::default()
        }
    }

    /// The version the exact attr does carry: only `python` may come back,
    /// never `python313Packages.django`, which also sits at 3.11.x.
    #[test]
    fn exact_with_version_filter_restricts_to_the_named_attr() {
        let (_dir, db) = seeded_db();

        let res = execute_search(db.connection(), &opts("python", Some("3.11"), true)).unwrap();

        assert_eq!(
            res.total,
            1,
            "--exact leaked sibling attrs: {:?}",
            res.data
                .iter()
                .map(|p| &p.attribute_path)
                .collect::<Vec<_>>()
        );
        assert_eq!(res.data[0].attribute_path, "python");
        assert_eq!(res.data[0].version, "3.11.0");
    }

    /// The issue's own repro: `python` was never at 2.7.x, so the result must
    /// be empty even though three sibling attrs do match that version prefix.
    #[test]
    fn exact_with_version_never_held_by_the_attr_returns_no_rows() {
        let (_dir, db) = seeded_db();

        let res = execute_search(db.connection(), &opts("python", Some("2.7"), true)).unwrap();

        assert_eq!(
            res.total,
            0,
            "python was never at 2.7.x, got {:?}",
            res.data
                .iter()
                .map(|p| &p.attribute_path)
                .collect::<Vec<_>>()
        );
    }

    /// The default non-exact search stays at the shallowest matching tier.
    #[test]
    fn non_exact_with_version_filter_uses_shallowest_prefix_tier() {
        let (_dir, db) = seeded_db();

        let res = execute_search(db.connection(), &opts("python", Some("2.7"), false)).unwrap();

        assert_eq!(res.total, 1);
        assert_eq!(res.data[0].attribute_path, "python2");
        assert_eq!(res.resolution.unwrap().scope, SearchScope::Shallowest);
    }

    /// `--all-depths` preserves the legacy broad prefix search explicitly.
    #[test]
    fn all_depths_with_version_filter_keeps_prefix_siblings() {
        let (_dir, db) = seeded_db();
        let res = execute_search(
            db.connection(),
            &SearchOptions {
                all_depths: true,
                ..opts("python", Some("2.7"), false)
            },
        )
        .unwrap();

        assert_eq!(res.total, 3);
        assert!(
            res.data
                .iter()
                .any(|p| p.attribute_path == "python314Packages.granian")
        );
    }

    /// `--exact` composes with the version filter without disturbing the
    /// version's prefix semantics: a bare `3` still resolves `3.11.0`, and
    /// still excludes the 3.11.x sibling.
    #[test]
    fn exact_keeps_version_as_a_prefix_match() {
        let (_dir, db) = seeded_db();

        let res = execute_search(db.connection(), &opts("python", Some("3"), true)).unwrap();

        assert_eq!(res.total, 1);
        assert_eq!(res.data[0].attribute_path, "python");
        assert_eq!(res.data[0].version, "3.11.0");
    }
}

/// Regression coverage for scoped version searches such as `python 2.7.3`.
#[cfg(test)]
mod relevance_order_tests {
    use super::*;
    use crate::db::Database;
    use tempfile::tempdir;

    const SEED: &str = r#"
        INSERT INTO package_versions
            (name, version, first_commit_hash, first_commit_date,
             last_commit_hash, last_commit_date, attribute_path, description)
        VALUES
            ('python',   '2.7.18', 'h1', 100, 'l1', 200, 'python', 'interpreter'),
            ('python',   '2.7.12', 'h4',  50, 'l4', 150, 'python', 'interpreter'),
            ('python',   '2.7.18', 'h2', 300, 'l2', 400, 'python27', 'interpreter alias'),
            ('python',   '3.11.4', 'h5', 700, 'l5', 800, 'python311', 'interpreter'),
            ('icontract','2.7.3',  'h3', 800, 'l3', 900, 'python314Packages.icontract', 'library'),
            ('anytree',  '2.7.3',  'h6', 500, 'l6', 600, 'python27Packages.anytree', 'library');
    "#;

    fn opts() -> SearchOptions {
        SearchOptions {
            query: "python".to_string(),
            version: Some("2.7".to_string()),
            limit: 0,
            ..Default::default()
        }
    }

    #[test]
    fn default_search_returns_only_the_shallowest_matching_tier() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db")).unwrap();
        db.connection().execute_batch(SEED).unwrap();

        let result = execute_search(db.connection(), &opts()).unwrap();
        let attrs: Vec<_> = result
            .data
            .iter()
            .map(|package| package.attribute_path.as_str())
            .collect();

        assert_eq!(attrs, ["python27", "python", "python"]);
        assert_eq!(result.resolution.unwrap().resolved_depth, Some(0));
    }

    #[test]
    fn explicit_date_sort_stays_within_the_resolved_tier() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db")).unwrap();
        db.connection().execute_batch(SEED).unwrap();

        let result = execute_search(
            db.connection(),
            &SearchOptions {
                sort: SortOrder::Date,
                ..opts()
            },
        )
        .unwrap();

        assert_eq!(result.data[0].attribute_path, "python27");
        assert!(
            result
                .data
                .iter()
                .all(|package| !package.attribute_path.contains('.'))
        );
    }

    #[test]
    fn exact_repro_is_a_precise_miss_with_nearby_versions() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db")).unwrap();
        db.connection().execute_batch(SEED).unwrap();

        let result = execute_search(
            db.connection(),
            &SearchOptions {
                version: Some("2.7.3".to_string()),
                ..opts()
            },
        )
        .unwrap();

        assert!(result.data.is_empty());
        let resolution = result.resolution.unwrap();
        assert_eq!(resolution.scope, SearchScope::Shallowest);
        assert_eq!(resolution.resolved_depth, Some(0));
        assert!(!resolution.version_matched);
        assert_eq!(resolution.deeper_matches_available, Some(true));
        assert_eq!(
            resolution.suggestions[0],
            SearchSuggestion {
                attribute_path: "python".to_string(),
                version: "2.7.12".to_string(),
            }
        );
    }

    #[test]
    fn package_set_prefix_resolves_its_member_depth() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db")).unwrap();
        db.connection().execute_batch(SEED).unwrap();

        let result = execute_search(
            db.connection(),
            &SearchOptions {
                query: "python27Packages".to_string(),
                version: Some("2.7.3".to_string()),
                limit: 0,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.data[0].attribute_path, "python27Packages.anytree");
        assert_eq!(result.resolution.unwrap().resolved_depth, Some(1));
    }

    #[test]
    fn all_depths_restores_nested_matches_for_the_repro() {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db")).unwrap();
        db.connection().execute_batch(SEED).unwrap();

        let result = execute_search(
            db.connection(),
            &SearchOptions {
                version: Some("2.7.3".to_string()),
                all_depths: true,
                ..opts()
            },
        )
        .unwrap();

        let attrs: Vec<_> = result
            .data
            .iter()
            .map(|package| package.attribute_path.as_str())
            .collect();
        assert_eq!(
            attrs,
            ["python314Packages.icontract", "python27Packages.anytree"]
        );
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
                Just(SortOrder::Relevance),
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
                Just(SortOrder::Relevance),
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
        for sort in [
            SortOrder::Relevance,
            SortOrder::Date,
            SortOrder::Version,
            SortOrder::Name,
        ] {
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
