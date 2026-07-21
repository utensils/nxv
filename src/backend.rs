//! Backend abstraction for local database or remote API access.
//!
//! This module provides a unified interface for querying package data,
//! regardless of whether the data comes from a local SQLite database
//! or a remote API server.
//!
//! Set the `NXV_API_URL` environment variable to use a remote server:
//! ```bash
//! export NXV_API_URL=http://localhost:8080
//! nxv search python
//! ```

use crate::client::ApiClient;
use crate::db::Database;
use crate::db::queries::{self, IndexStats, PackageVersion, VersionHistoryEntry};
use crate::error::Result;
use crate::search::{self, SearchOptions, SearchResult};

/// Returns whether `version` starts with `prefix`, case-insensitively over ASCII.
///
/// The remote backend filters versions in Rust while the local backend filters them
/// with SQLite `LIKE`. SQLite's `LIKE` is ASCII-case-insensitive by default, so this
/// deliberately mirrors that rather than using a plain [`str::starts_with`], keeping
/// `NXV_API_URL` results identical to local ones for versions with alphabetic
/// components (e.g. `1.0.0-RC1` vs `1.0.0-rc1`).
fn version_matches_prefix(version: &str, prefix: &str) -> bool {
    version.len() >= prefix.len()
        && version
            .as_bytes()
            .iter()
            .zip(prefix.as_bytes())
            .all(|(v, p)| v.eq_ignore_ascii_case(p))
}

/// Backend for data access - either local database or remote API.
pub enum Backend {
    /// Local SQLite database.
    Local(Database),
    /// Remote API server.
    Remote(ApiClient),
}

impl Backend {
    /// Check if using remote API.
    pub fn is_remote(&self) -> bool {
        matches!(self, Backend::Remote(_))
    }

    /// Searches packages using the provided search options.
    ///
    /// Uses the backend's configured data source to perform the query according to `opts`.
    ///
    /// Returns a `SearchResult` containing matches and related metadata on success.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let backend: Backend = unimplemented!();
    /// let opts: SearchOptions = unimplemented!();
    /// let _result = backend.search(&opts).unwrap();
    /// ```
    pub fn search(&self, opts: &SearchOptions) -> Result<SearchResult> {
        match self {
            Backend::Local(db) => search::execute_search(db.connection(), opts),
            Backend::Remote(client) => client.search(opts),
        }
    }

    /// Retrieve package versions whose attribute path exactly matches the provided attribute.
    ///
    /// On success returns a `Vec<PackageVersion>` containing only entries whose `attribute_path` is
    /// exactly equal to `attr`.
    ///
    /// # Examples
    ///
    /// ```
    /// // assuming `backend` is a `Backend` instance and `pkg_attr` is the attribute path string
    /// let matches = backend.get_package("example/package").unwrap();
    /// for pv in &matches {
    ///     assert_eq!(pv.attribute_path, "example/package");
    /// }
    /// ```
    #[allow(dead_code)]
    pub fn get_package(&self, attr: &str) -> Result<Vec<PackageVersion>> {
        match self {
            Backend::Local(db) => queries::search_by_attr_exact(db.connection(), attr),
            Backend::Remote(client) => client.get_package(attr),
        }
    }

    /// Search for package versions by package name with an optional version filter.
    ///
    /// In both the versioned and unversioned cases this attempts an exact attribute-path
    /// match first and only falls back to a name-prefix search when the exact path is
    /// unknown. That keeps precise lookups such as `python311` from being contaminated by
    /// unrelated prefix siblings (`python311Full`, `python311Packages.*`) while still
    /// letting partial names resolve.
    ///
    /// If `version` is provided it narrows the results as a version *prefix*, so `3.11`
    /// matches `3.11.4`.
    ///
    /// # Returns
    ///
    /// A `Vec<PackageVersion>` containing the matching package versions (empty if no matches).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crate::backend::Backend;
    /// # use crate::models::PackageVersion;
    /// # fn example(backend: Backend) -> anyhow::Result<()> {
    /// let results: Vec<PackageVersion> = backend.search_by_name_version("example/pkg", None)?;
    /// println!("found {} versions", results.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn search_by_name_version(
        &self,
        package: &str,
        version: Option<&str>,
    ) -> Result<Vec<PackageVersion>> {
        match self {
            Backend::Local(db) => {
                if let Some(v) = version {
                    // Try exact attribute path first so a precise lookup is not
                    // contaminated by prefix siblings that happen to share a version.
                    let by_attr =
                        queries::search_by_attr_exact_version(db.connection(), package, v)?;

                    if !by_attr.is_empty() {
                        Ok(by_attr)
                    } else if queries::attribute_path_exists(db.connection(), package)? {
                        // The attribute path is known, it just never had this version.
                        // Widening to a prefix search here would answer a question the
                        // user did not ask, so report the precise miss instead.
                        Ok(Vec::new())
                    } else {
                        // Unknown attribute path: treat it as a partial name.
                        queries::search_by_name_version(db.connection(), package, v)
                    }
                } else {
                    // Try exact attribute path first.
                    let by_attr = queries::search_by_attr_exact(db.connection(), package)?;

                    if !by_attr.is_empty() {
                        Ok(by_attr)
                    } else {
                        // Fall back to name prefix search
                        queries::search_by_name(db.connection(), package, false)
                    }
                }
            }
            Backend::Remote(client) => {
                if let Some(v) = version {
                    // Exact attribute path first, narrowed to the version prefix locally.
                    // The `/packages/{attr}` endpoint is already an exact-path lookup, so
                    // this needs no server-side `exact` support.
                    let all_versions = client.get_package(package)?;

                    if !all_versions.is_empty() {
                        // Known attribute path: return only its matching versions, even if
                        // that is none. Mirrors the local backend's precise-miss behavior.
                        Ok(all_versions
                            .into_iter()
                            .filter(|pv| version_matches_prefix(&pv.version, v))
                            .collect())
                    } else {
                        // Unknown attribute path: treat it as a partial name.
                        client.search_by_name_version(package, version, None)
                    }
                } else {
                    // Try exact attribute path first
                    let by_attr = client.get_package(package)?;
                    if !by_attr.is_empty() {
                        Ok(by_attr)
                    } else {
                        // Fall back to name prefix search
                        client.search_by_name(package, false)
                    }
                }
            }
        }
    }

    /// Search packages by name, using either an exact match or a prefix match.
    ///
    /// If `exact` is `true`, only packages whose attribute path exactly equals `name` are returned.
    /// If `exact` is `false`, packages whose names start with `name` are returned.
    ///
    /// # Returns
    ///
    /// A vector of `PackageVersion` entries that match the provided name and match mode.
    ///
    /// # Examples
    ///
    /// ```
    /// // Search for packages whose name starts with "libfoo"
    /// let results = backend.search_by_name("libfoo", false).unwrap();
    /// // Search for the package whose attribute path is exactly "libfoo/pkg"
    /// let exact = backend.search_by_name("libfoo/pkg", true).unwrap();
    /// ```
    pub fn search_by_name(&self, name: &str, exact: bool) -> Result<Vec<PackageVersion>> {
        match self {
            Backend::Local(db) => queries::search_by_name(db.connection(), name, exact),
            Backend::Remote(client) => client.search_by_name(name, exact),
        }
    }

    /// Get first occurrence of a specific version.
    ///
    /// This method is part of the public API for library consumers and mirrors
    /// the `/api/v1/packages/{attr}/versions/{version}/first` endpoint.
    /// Not currently used by the CLI but provided for API completeness.
    #[allow(dead_code)]
    pub fn get_first_occurrence(
        &self,
        attr: &str,
        version: &str,
    ) -> Result<Option<PackageVersion>> {
        match self {
            Backend::Local(db) => queries::get_first_occurrence(db.connection(), attr, version),
            Backend::Remote(client) => client.get_first_occurrence(attr, version),
        }
    }

    /// Retrieve the most recent `PackageVersion` entry for the given attribute path and version.
    ///
    /// This method is part of the public API for library consumers and mirrors
    /// the `/api/v1/packages/{attr}/versions/{version}/last` endpoint.
    /// Not currently used by the CLI but provided for API completeness.
    ///
    /// - `attr`: attribute path (attribute identifier) to search for.
    /// - `version`: package version to match.
    ///
    /// # Returns
    ///
    /// `Some(PackageVersion)` containing the most recent occurrence for that attribute and version, `None` if no match is found.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given a `Backend` instance `backend`, fetch the last occurrence:
    /// let last = backend.get_last_occurrence("example/pkg", "1.2.3")?;
    /// if let Some(pkg) = last {
    ///     println!("Found: {}", pkg.attribute_path);
    /// }
    /// ```
    #[allow(dead_code)]
    pub fn get_last_occurrence(&self, attr: &str, version: &str) -> Result<Option<PackageVersion>> {
        match self {
            Backend::Local(db) => queries::get_last_occurrence(db.connection(), attr, version),
            Backend::Remote(client) => client.get_last_occurrence(attr, version),
        }
    }

    /// Retrieve the version history for the package identified by `attr`.
    ///
    /// # Parameters
    ///
    /// - `attr`: The package attribute path (e.g., `"nxv:package/name"`) to fetch version history for.
    ///
    /// # Returns
    ///
    /// A vector of `VersionHistoryEntry` records for the package, or an error if the underlying
    /// data source query fails.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `backend` is an initialized `Backend` (Local or Remote):
    /// let history = backend.get_version_history("nxv:example/package").unwrap();
    /// assert!(history.len() >= 0);
    /// ```
    pub fn get_version_history(&self, attr: &str) -> Result<Vec<VersionHistoryEntry>> {
        match self {
            Backend::Local(db) => queries::get_version_history(db.connection(), attr),
            Backend::Remote(client) => client.get_version_history(attr),
        }
    }

    /// Fetches aggregated index statistics for this backend.
    ///
    /// # Returns
    ///
    /// `IndexStats` containing aggregated counts and metadata about the index.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::backend::Backend;
    /// let backend: Backend = /* obtain Backend::Local or Backend::Remote */;
    /// let stats = backend.get_stats().unwrap();
    /// ```
    pub fn get_stats(&self) -> Result<IndexStats> {
        match self {
            Backend::Local(db) => queries::get_stats(db.connection()),
            Backend::Remote(client) => client.get_stats(),
        }
    }

    /// Fetches a metadata value for the given key from the backend.
    ///
    /// For a local backend this returns the stored metadata entry for `key`.
    /// For a remote backend this returns the remote health-derived value only for
    /// the `"last_indexed_commit"` key; other keys return `None`.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key to retrieve. The remote backend recognizes `"last_indexed_commit"`.
    ///
    /// # Returns
    ///
    /// `Some(String)` with the metadata value if present, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // `backend` can be a Local or Remote Backend instance.
    /// let value = backend.get_meta("last_indexed_commit")?;
    /// match value {
    ///     Some(commit) => println!("Last indexed commit: {}", commit),
    ///     None => println!("No metadata available for that key"),
    /// }
    /// ```
    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        match self {
            Backend::Local(db) => db.get_meta(key),
            Backend::Remote(client) => {
                // For remote, we can get some info from health endpoint
                if key == "last_indexed_commit" {
                    let health = client.get_health()?;
                    Ok(health.index_commit)
                } else {
                    Ok(None)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Builds a local backend seeded with an exact package plus prefix siblings that
    /// share its version, which is the shape that produced issue #53.
    fn local_backend() -> (tempfile::TempDir, Backend) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("test.db")).unwrap();
        db.connection()
            .execute(
                r#"
            INSERT INTO package_versions (name, version, first_commit_hash, first_commit_date,
                last_commit_hash, last_commit_date, attribute_path, description)
            VALUES
                ('python311-3.11.4', '3.11.4', 'a1', 1700000000, 'b1', 1700100000, 'python311', 'Python'),
                ('python311Full-3.11.4', '3.11.4', 'a2', 1700000000, 'b2', 1700100000, 'python311Full', 'Python full'),
                ('tkinter-3.11.4', '3.11.4', 'a3', 1700000000, 'b3', 1700100000, 'python311Packages.tkinter', 'tkinter'),
                ('bigquery-3.11.4', '3.11.4', 'a4', 1700000000, 'b4', 1700100000, 'python311Packages.google-cloud-bigquery', 'bigquery'),
                -- A sibling at 2.7.x, so a fallback to prefix search for `python311 2.7`
                -- would visibly return the wrong package rather than nothing.
                ('b2sdk-2.7.0', '2.7.0', 'a5', 1700000000, 'b5', 1700100000, 'python311Packages.b2sdk', 'b2sdk')
            "#,
                [],
            )
            .unwrap();
        (dir, Backend::Local(db))
    }

    /// The issue #53 repro: an exact attribute path plus a version must return only
    /// that package, not every prefix sibling sharing the version.
    #[test]
    fn test_info_versioned_lookup_is_exact_on_attribute_path() {
        let (_dir, backend) = local_backend();
        let results = backend
            .search_by_name_version("python311", Some("3.11.4"))
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attribute_path, "python311");
    }

    /// A known package asked for at a version it never had is a precise miss, not an
    /// invitation to widen into a prefix search.
    #[test]
    fn test_info_known_package_missing_version_does_not_fall_back() {
        let (_dir, backend) = local_backend();
        let results = backend
            .search_by_name_version("python311", Some("2.7"))
            .unwrap();

        assert!(
            results.is_empty(),
            "expected a precise miss, got {:?}",
            results
                .iter()
                .map(|p| &p.attribute_path)
                .collect::<Vec<_>>()
        );
    }

    /// An unknown attribute path is still treated as a partial name, so discovery works.
    #[test]
    fn test_info_unknown_attribute_path_falls_back_to_prefix_search() {
        let (_dir, backend) = local_backend();
        let results = backend
            .search_by_name_version("python311Packages.tk", Some("3.11"))
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attribute_path, "python311Packages.tkinter");
    }

    /// The unversioned path already behaved correctly and must stay that way.
    #[test]
    fn test_info_unversioned_lookup_still_exact() {
        let (_dir, backend) = local_backend();
        let results = backend.search_by_name_version("python311", None).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attribute_path, "python311");
    }

    #[test]
    fn test_version_matches_prefix() {
        assert!(version_matches_prefix("3.11.4", "3.11"));
        assert!(version_matches_prefix("3.11.4", "3.11.4"));
        assert!(!version_matches_prefix("3.11.4", "3.12"));
        // Shorter than the prefix cannot match.
        assert!(!version_matches_prefix("3.1", "3.11"));
        // Mirrors SQLite LIKE's ASCII case-insensitivity so remote matches local.
        assert!(version_matches_prefix("1.0.0-RC1", "1.0.0-rc"));
        assert!(version_matches_prefix("1.0.0-rc1", "1.0.0-RC"));
    }
}
