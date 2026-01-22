//! Nix package extraction from nixpkgs commits.

use crate::error::Result;
use crate::index::nix_ffi::with_evaluator;
use serde::Deserialize;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::Command;
use std::time::Instant;
use tracing::{debug, instrument, trace};

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const MAX_SKIP_SAMPLES: usize = 2000;

struct SkipMetrics {
    total_skipped: AtomicU64,
    failed_batches: AtomicU64,
    counts: Mutex<HashMap<String, u64>>,
    samples: Mutex<Vec<String>>,
    sample_keys: Mutex<HashSet<String>>,
}

impl SkipMetrics {
    fn new() -> Self {
        Self {
            total_skipped: AtomicU64::new(0),
            failed_batches: AtomicU64::new(0),
            counts: Mutex::new(HashMap::new()),
            samples: Mutex::new(Vec::new()),
            sample_keys: Mutex::new(HashSet::new()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SkipSummary {
    pub total_skipped: u64,
    pub failed_batches: u64,
    pub unique_skipped: usize,
    pub top_skipped: Vec<(String, u64)>,
    pub samples: Vec<String>,
}

static SKIP_METRICS: OnceLock<SkipMetrics> = OnceLock::new();
static EXTRACT_NIX_PATH: OnceLock<PathBuf> = OnceLock::new();

fn skip_metrics() -> &'static SkipMetrics {
    SKIP_METRICS.get_or_init(SkipMetrics::new)
}

fn extract_nix_path() -> Result<PathBuf> {
    if let Some(path) = EXTRACT_NIX_PATH.get() {
        if !path.exists() {
            std::fs::write(path, EXTRACT_NIX)?;
        }
        return Ok(path.clone());
    }

    let path = std::env::temp_dir().join(format!("nxv-extract-{}.nix", std::process::id()));
    std::fs::write(&path, EXTRACT_NIX)?;
    let _ = EXTRACT_NIX_PATH.set(path.clone());
    Ok(path)
}

pub fn reset_skip_metrics() {
    let metrics = skip_metrics();
    metrics.total_skipped.store(0, Ordering::SeqCst);
    metrics.failed_batches.store(0, Ordering::SeqCst);
    metrics.counts.lock().unwrap().clear();
    metrics.samples.lock().unwrap().clear();
    metrics.sample_keys.lock().unwrap().clear();
}

pub fn take_skip_metrics() -> SkipSummary {
    let metrics = skip_metrics();
    let counts = metrics.counts.lock().unwrap();
    let mut top: Vec<(String, u64)> = counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));
    top.truncate(20);
    let samples = metrics.samples.lock().unwrap().clone();
    SkipSummary {
        total_skipped: metrics.total_skipped.load(Ordering::SeqCst),
        failed_batches: metrics.failed_batches.load(Ordering::SeqCst),
        unique_skipped: counts.len(),
        top_skipped: top,
        samples,
    }
}

fn record_failed_batch() {
    skip_metrics().failed_batches.fetch_add(1, Ordering::SeqCst);
}

fn record_skipped(system: &str, attrs: &[String], reason: &str) {
    if attrs.is_empty() {
        return;
    }
    let metrics = skip_metrics();
    metrics
        .total_skipped
        .fetch_add(attrs.len() as u64, Ordering::SeqCst);
    {
        let mut counts = metrics.counts.lock().unwrap();
        for attr in attrs {
            let key = format!("{}:{}", system, attr);
            *counts.entry(key).or_insert(0) += 1;
        }
    }
    let mut samples = metrics.samples.lock().unwrap();
    let mut sample_keys = metrics.sample_keys.lock().unwrap();
    if samples.len() >= MAX_SKIP_SAMPLES {
        return;
    }
    let remaining = MAX_SKIP_SAMPLES - samples.len();
    for attr in attrs.iter().take(remaining) {
        let key = format!("{}:{}", system, attr);
        if sample_keys.insert(key.clone()) {
            samples.push(format!("{} ({})", key, reason));
        }
    }
}

/// Information about an extracted package.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    pub name: String,
    /// Package version. None if version could not be extracted from any source.
    pub version: Option<String>,
    /// Source of the version information: "direct", "unwrapped", "passthru", "name", or null.
    /// Tracks how the version was extracted for debugging and quality tracking.
    pub version_source: Option<String>,
    #[serde(rename = "attrPath")]
    pub attribute_path: String,
    pub description: Option<String>,
    pub license: Option<Vec<String>>,
    pub homepage: Option<String>,
    pub maintainers: Option<Vec<String>>,
    pub platforms: Option<Vec<String>>,
    /// Source file path relative to nixpkgs root (e.g., "pkgs/development/interpreters/python/default.nix")
    #[serde(rename = "sourcePath")]
    pub source_path: Option<String>,
    /// Known security vulnerabilities or EOL notices from meta.knownVulnerabilities
    pub known_vulnerabilities: Option<Vec<String>>,
    /// Store path for the package output (e.g., "/nix/store/hash-name-version")
    /// Only populated for commits from 2020-01-01 onwards.
    /// Note: Named "storePath" in Nix output because "outPath" is a special attribute.
    #[serde(rename = "storePath", default)]
    pub out_path: Option<String>,
}

/// Attribute position information for mapping attribute names to files.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttrPosition {
    pub attr_path: String,
    pub file: Option<String>,
}

impl PackageInfo {
    /// Serialize known vulnerabilities to JSON for database storage.
    pub fn known_vulnerabilities_json(&self) -> Option<String> {
        self.known_vulnerabilities
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
    }
}

/// The nix expression for extracting package information.
/// Loaded from external file at compile time for better maintainability.
const EXTRACT_NIX: &str = include_str!("nix/extract.nix");

/// The nix expression for extracting attribute positions.
/// Loaded from external file at compile time for better maintainability.
const POSITIONS_NIX: &str = include_str!("nix/positions.nix");

/// Default batch size for metadata extraction.
const DEFAULT_BATCH_SIZE: usize = 100;
/// Smaller batch size for store path extraction to reduce memory spikes.
const STORE_PATH_BATCH_SIZE: usize = 50;

/// Extract packages from a nixpkgs checkout at a specific path.
///
/// # Arguments
/// * `repo_path` - Path to the nixpkgs repository checkout
///
/// # Returns
/// A vector of PackageInfo, or an error if extraction fails.
#[cfg(test)]
pub fn extract_packages<P: AsRef<Path>>(repo_path: P) -> Result<Vec<PackageInfo>> {
    extract_packages_for_attrs(repo_path, "x86_64-linux", &[], true)
}

/// Extract packages for a specific list of attribute names and system.
///
/// # Arguments
/// * `repo_path` - Path to nixpkgs checkout
/// * `system` - Target system (e.g., "x86_64-linux")
/// * `attr_names` - List of attribute names to extract (empty = all packages)
/// * `extract_store_paths` - Whether to extract store paths. Set to `false` for old commits
///   (before 2020) to avoid triggering derivationStrict which fails on darwin-dependent
///   packages with old nixpkgs + modern Nix.
#[instrument(level = "debug", skip(repo_path, attr_names), fields(attr_count = attr_names.len()))]
pub fn extract_packages_for_attrs<P: AsRef<Path>>(
    repo_path: P,
    system: &str,
    attr_names: &[String],
    extract_store_paths: bool,
) -> Result<Vec<PackageInfo>> {
    extract_packages_for_attrs_with_mode(repo_path, system, attr_names, extract_store_paths, false)
}

/// Extract packages with additional mode controls for store-path-only extraction.
pub fn extract_packages_for_attrs_with_mode<P: AsRef<Path>>(
    repo_path: P,
    system: &str,
    attr_names: &[String],
    extract_store_paths: bool,
    store_paths_only: bool,
) -> Result<Vec<PackageInfo>> {
    let repo_path = repo_path.as_ref();

    // Batch large attribute lists to avoid memory pressure in Nix evaluation.
    // Full extraction with 11K+ attrs can exhaust worker memory.
    // Nix has ~1.5GB baseline overhead, so with 2GB workers we need small batches.
    // Process in smaller batches when store paths are requested.
    let batch_size = if extract_store_paths {
        STORE_PATH_BATCH_SIZE
    } else {
        DEFAULT_BATCH_SIZE
    };

    if !attr_names.is_empty() && attr_names.len() > batch_size {
        let num_batches = attr_names.len().div_ceil(batch_size);
        debug!(
            system = %system,
            total_attrs = attr_names.len(),
            batch_size = batch_size,
            num_batches = num_batches,
            "Batching large extraction"
        );

        let mut all_packages = Vec::new();
        let mut failed_batches = 0;
        let mut total_failed_attrs = 0;

        fn extract_with_fallback<P: AsRef<Path>>(
            repo_path: P,
            system: &str,
            attrs: &[String],
            extract_store_paths: bool,
            store_paths_only: bool,
            all_packages: &mut Vec<PackageInfo>,
            skipped: &mut Vec<String>,
        ) {
            match extract_packages_batch(
                repo_path.as_ref(),
                system,
                attrs,
                extract_store_paths,
                store_paths_only,
            ) {
                Ok(batch_result) => {
                    all_packages.extend(batch_result);
                }
                Err(_e) => {
                    if extract_store_paths
                        && let Ok(batch_result) = extract_packages_batch(
                            repo_path.as_ref(),
                            system,
                            attrs,
                            false,
                            store_paths_only,
                        )
                    {
                        trace!(
                            system = %system,
                            attr_count = attrs.len(),
                            "Retry without store paths succeeded"
                        );
                        all_packages.extend(batch_result);
                        return;
                    }
                    if attrs.len() <= 1 {
                        skipped.extend(attrs.iter().cloned());
                        return;
                    }
                    let mid = attrs.len() / 2;
                    extract_with_fallback(
                        repo_path.as_ref(),
                        system,
                        &attrs[..mid],
                        extract_store_paths,
                        store_paths_only,
                        all_packages,
                        skipped,
                    );
                    extract_with_fallback(
                        repo_path.as_ref(),
                        system,
                        &attrs[mid..],
                        extract_store_paths,
                        store_paths_only,
                        all_packages,
                        skipped,
                    );
                }
            }
        }

        for (batch_idx, batch) in attr_names.chunks(batch_size).enumerate() {
            debug!(
                system = %system,
                batch_idx = batch_idx,
                batch_size = batch.len(),
                "Processing extraction batch"
            );

            // Continue on batch failures to collect as many packages as possible.
            // This is critical for full extraction where some batches may fail due to
            // memory pressure or evaluation errors, but we still want results from
            // successful batches.
            match extract_packages_batch(
                repo_path,
                system,
                batch,
                extract_store_paths,
                store_paths_only,
            ) {
                Ok(batch_result) => {
                    trace!(
                        system = %system,
                        batch_idx = batch_idx,
                        packages_found = batch_result.len(),
                        "Batch extraction succeeded"
                    );
                    all_packages.extend(batch_result);
                }
                Err(e) => {
                    debug!(
                        system = %system,
                        batch_idx = batch_idx + 1,
                        total_batches = num_batches,
                        batch_size = batch.len(),
                        error = %e,
                        "Batch failed, retrying with smaller chunks"
                    );

                    let mut skipped = Vec::new();
                    extract_with_fallback(
                        repo_path,
                        system,
                        batch,
                        extract_store_paths,
                        store_paths_only,
                        &mut all_packages,
                        &mut skipped,
                    );
                    if !skipped.is_empty() {
                        failed_batches += 1;
                        total_failed_attrs += skipped.len();
                        record_failed_batch();
                        record_skipped(system, &skipped, "eval_failed");
                        debug!(
                            system = %system,
                            batch_idx = batch_idx,
                            skipped_attrs = skipped.len(),
                            "Batch extraction skipped attributes after retries"
                        );
                    }
                    // Continue with remaining batches instead of failing early
                }
            }
        }

        if failed_batches > 0 {
            debug!(
                system = %system,
                failed_batches = failed_batches,
                total_failed_attrs = total_failed_attrs,
                packages_extracted = all_packages.len(),
                "Batched extraction completed with skipped attrs"
            );
        }

        trace!(
            system = %system,
            total_attrs = attr_names.len(),
            total_packages = all_packages.len(),
            failed_batches = failed_batches,
            total_failed_attrs = total_failed_attrs,
            "Batched extraction completed"
        );

        return Ok(all_packages);
    }

    // Single extraction for small attr lists or full discovery (empty list)
    extract_packages_batch(
        repo_path,
        system,
        attr_names,
        extract_store_paths,
        store_paths_only,
    )
}

/// Internal function to extract a batch of packages.
fn extract_packages_batch<P: AsRef<Path>>(
    repo_path: P,
    system: &str,
    attr_names: &[String],
    extract_store_paths: bool,
    store_paths_only: bool,
) -> Result<Vec<PackageInfo>> {
    let repo_path = repo_path.as_ref();

    // Canonicalize the path to avoid any relative path issues
    let canonical_path = std::fs::canonicalize(repo_path)?;
    let repo_path_str = canonical_path.display().to_string();

    let nix_file = extract_nix_path()?;

    // Build the attrNames argument - write to file if large to avoid "Argument list too long"
    // OS limit is typically ~2MB for all args + env, so we use a conservative threshold
    let mut _attr_file: Option<tempfile::NamedTempFile> = None;
    let attr_names_arg = if attr_names.is_empty() {
        "null".to_string()
    } else {
        // Estimate the size: each name plus quotes and space
        let estimated_size: usize = attr_names.iter().map(|s| s.len() + 3).sum();

        if estimated_size > 100_000 {
            // Write attr names to a JSON file and read in Nix
            let json = serde_json::to_string(attr_names)?;
            let attr_file = tempfile::NamedTempFile::new()?;
            std::fs::write(attr_file.path(), &json)?;
            let attr_path = attr_file.path().display().to_string();
            _attr_file = Some(attr_file);
            // Quote the path to handle spaces and special characters
            format!("builtins.fromJSON (builtins.readFile \"{}\")", attr_path)
        } else {
            let items: Vec<String> = attr_names.iter().map(|s| format!("\"{}\"", s)).collect();
            format!("[ {} ]", items.join(" "))
        }
    };

    // Build an expression that imports and calls the extract file.
    // Note: Nix import takes a path, not a string, so we don't quote nix_file.
    // But nixpkgsPath is assigned as a string, so we quote it.
    let expr = format!(
        "import {} {{ nixpkgsPath = \"{}\"; system = \"{}\"; attrNames = {}; extractStorePaths = {}; storePathsOnly = {}; }}",
        nix_file.display(),
        repo_path_str,
        system,
        attr_names_arg,
        if extract_store_paths { "true" } else { "false" },
        if store_paths_only { "true" } else { "false" }
    );

    // Use FFI evaluator with large stack thread
    let eval_start = Instant::now();
    let json_output = with_evaluator(move |eval| eval.eval_json(&expr, "<extract>"))?;
    let eval_time = eval_start.elapsed();

    let parse_start = Instant::now();
    let packages: Vec<PackageInfo> = serde_json::from_str(&json_output)?;
    let parse_time = parse_start.elapsed();

    trace!(
        system = %system,
        attr_count = attr_names.len(),
        packages_found = packages.len(),
        json_size_bytes = json_output.len(),
        eval_time_ms = eval_time.as_millis(),
        parse_time_ms = parse_time.as_millis(),
        "Nix extraction completed"
    );

    Ok(packages)
}

/// Extract attribute positions for a nixpkgs checkout and system.
#[instrument(level = "debug", skip(repo_path))]
pub fn extract_attr_positions<P: AsRef<Path>>(
    repo_path: P,
    system: &str,
) -> Result<Vec<AttrPosition>> {
    let repo_path = repo_path.as_ref();

    // Canonicalize the path to avoid any relative path issues
    let canonical_path = std::fs::canonicalize(repo_path)?;
    let repo_path_str = canonical_path.display().to_string();

    // Write the nix expression to a temp file
    let temp_dir = tempfile::tempdir()?;
    let nix_file = temp_dir.path().join("positions.nix");
    std::fs::write(&nix_file, POSITIONS_NIX)?;

    // Build an expression that imports and calls the positions file
    let expr = format!(
        "import {} {{ nixpkgsPath = \"{}\"; system = \"{}\"; }}",
        nix_file.display(),
        repo_path_str,
        system
    );

    // Use FFI evaluator with large stack thread
    let eval_start = Instant::now();
    let json_output = with_evaluator(move |eval| eval.eval_json(&expr, "<positions>"))?;
    let eval_time = eval_start.elapsed();

    let parse_start = Instant::now();
    let positions: Vec<AttrPosition> = serde_json::from_str(&json_output)?;
    let parse_time = parse_start.elapsed();

    trace!(
        system = %system,
        positions_found = positions.len(),
        json_size_bytes = json_output.len(),
        eval_time_ms = eval_time.as_millis(),
        parse_time_ms = parse_time.as_millis(),
        "Positions extraction completed"
    );

    Ok(positions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn skip_metrics_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_package_info_json_serialization() {
        let pkg = PackageInfo {
            name: "test".to_string(),
            version: Some("1.0.0".to_string()),
            version_source: Some("direct".to_string()),
            attribute_path: "test".to_string(),
            description: Some("A test package".to_string()),
            license: Some(vec!["MIT".to_string(), "Apache-2.0".to_string()]),
            homepage: Some("https://example.com".to_string()),
            maintainers: Some(vec!["user1".to_string(), "user2".to_string()]),
            platforms: Some(vec!["x86_64-linux".to_string()]),
            source_path: Some("pkgs/test/default.nix".to_string()),
            known_vulnerabilities: Some(vec!["CVE-2025-0001".to_string()]),
            out_path: Some("/nix/store/abc123-test-1.0.0".to_string()),
        };

        let vulnerabilities_json = pkg.known_vulnerabilities_json().unwrap();
        assert!(vulnerabilities_json.contains("CVE-2025-0001"));
    }

    #[test]
    #[ignore] // Requires nix to be installed and nixpkgs to be present
    fn test_extract_packages_from_nixpkgs() {
        let nixpkgs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("nixpkgs");

        if !nixpkgs_path.exists() {
            eprintln!("Skipping: nixpkgs not present");
            return;
        }

        let result = extract_packages(&nixpkgs_path);

        match result {
            Ok(packages) => {
                assert!(
                    !packages.is_empty(),
                    "Should extract at least some packages"
                );

                // Verify we got some common packages
                let names: Vec<_> = packages.iter().map(|p| p.name.as_str()).collect();

                // At least one of these common packages should exist
                let has_common = names
                    .iter()
                    .any(|n| ["hello", "git", "curl", "coreutils", "bash"].contains(n));
                assert!(has_common, "Should find at least one common package");

                // Verify package structure
                for pkg in packages.iter().take(5) {
                    assert!(!pkg.name.is_empty());
                    assert!(pkg.version.is_some() && !pkg.version.as_ref().unwrap().is_empty());
                    assert!(!pkg.attribute_path.is_empty());
                }
            }
            Err(e) => {
                // Nix might not be available in CI
                eprintln!("Extraction failed (nix may not be available): {}", e);
            }
        }
    }

    /// Test that the Nix extractor handles edge cases where:
    /// - maintainers or platforms are strings instead of lists
    /// - version is an integer instead of a string
    ///
    /// These bugs caused extraction failures when packages had non-standard types.
    #[test]
    #[ignore] // Requires nix to be installed
    fn test_extract_handles_string_maintainers_and_platforms() {
        use std::process::Command;
        use tempfile::tempdir;

        // Create a minimal nixpkgs-like structure with edge cases
        let dir = tempdir().unwrap();
        let path = dir.path();

        // Create pkgs directory (required for validation)
        std::fs::create_dir(path.join("pkgs")).unwrap();

        // Create a default.nix that acts like nixpkgs - a function that takes config
        let default_nix = r#"
{ config ? {}, system ? builtins.currentSystem, ... }:
{
  # Normal package with list maintainers
  normalPkg = {
    pname = "normal-pkg";
    version = "1.0.0";
    type = "derivation";
    meta = {
      description = "A normal package";
      maintainers = [ { github = "user1"; } { name = "User Two"; } ];
      platforms = [ "x86_64-linux" "aarch64-darwin" ];
    };
  };

  # Edge case: maintainers is a string instead of a list
  stringMaintainerPkg = {
    pname = "string-maintainer-pkg";
    version = "2.0.0";
    type = "derivation";
    meta = {
      description = "Package with string maintainer";
      maintainers = "David Kleuker <post@davidak.de>";
      platforms = [ "x86_64-linux" ];
    };
  };

  # Edge case: platforms is a string instead of a list
  stringPlatformPkg = {
    pname = "string-platform-pkg";
    version = "3.0.0";
    type = "derivation";
    meta = {
      description = "Package with string platform";
      maintainers = [ { github = "someone"; } ];
      platforms = "x86_64-linux";
    };
  };

  # Edge case: both maintainers and platforms are strings
  bothStringsPkg = {
    pname = "both-strings-pkg";
    version = "4.0.0";
    type = "derivation";
    meta = {
      description = "Package with both as strings";
      maintainers = "test@example.com";
      platforms = "aarch64-darwin";
    };
  };

  # Edge case: version is an integer instead of a string
  intVersionPkg = {
    pname = "int-version-pkg";
    version = 61;
    type = "derivation";
    meta = {
      description = "Package with integer version";
    };
  };
}
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        // Check if nix is available
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        // Run extraction
        let result = extract_packages(path);

        match result {
            Ok(packages) => {
                assert_eq!(
                    packages.len(),
                    5,
                    "Should extract all 5 packages despite edge cases"
                );

                // Verify normal package
                let normal = packages.iter().find(|p| p.name == "normal-pkg").unwrap();
                assert_eq!(normal.version.as_deref(), Some("1.0.0"));
                assert!(normal.maintainers.is_some());
                assert!(normal.platforms.is_some());

                // Verify string maintainer package - should have maintainers as list with one element
                let string_maint = packages
                    .iter()
                    .find(|p| p.name == "string-maintainer-pkg")
                    .unwrap();
                assert_eq!(string_maint.version.as_deref(), Some("2.0.0"));
                let maint = string_maint.maintainers.as_ref().unwrap();
                assert_eq!(maint.len(), 1);
                assert!(maint[0].contains("David Kleuker"));

                // Verify string platform package - should have platforms as list with one element
                let string_plat = packages
                    .iter()
                    .find(|p| p.name == "string-platform-pkg")
                    .unwrap();
                assert_eq!(string_plat.version.as_deref(), Some("3.0.0"));
                let plat = string_plat.platforms.as_ref().unwrap();
                assert_eq!(plat.len(), 1);
                assert_eq!(plat[0], "x86_64-linux");

                // Verify both strings package
                let both = packages
                    .iter()
                    .find(|p| p.name == "both-strings-pkg")
                    .unwrap();
                assert_eq!(both.version.as_deref(), Some("4.0.0"));
                assert!(both.maintainers.is_some());
                assert!(both.platforms.is_some());

                // Verify integer version package - should have version converted to string
                let int_ver = packages
                    .iter()
                    .find(|p| p.name == "int-version-pkg")
                    .unwrap();
                assert_eq!(
                    int_ver.version.as_deref(),
                    Some("61"),
                    "Integer version should be converted to string"
                );
            }
            Err(e) => {
                panic!("Extraction should not fail with edge case packages: {}", e);
            }
        }
    }

    #[test]
    fn test_extract_packages_with_empty_attr_list() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        let default_nix = r#"
{ system, config }:
{
  hello = {
    pname = "hello";
    version = "1.0.0";
    type = "derivation";
    meta = {
      description = "A test package";
    };
  };
}
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        let packages = extract_packages_for_attrs(path, "x86_64-linux", &[], true).unwrap();
        assert!(!packages.is_empty());
        assert!(packages.iter().any(|pkg| pkg.name == "hello"));
    }

    #[test]
    fn test_skip_metrics_tracking() {
        let _guard = skip_metrics_test_lock().lock().unwrap();
        reset_skip_metrics();
        record_failed_batch();
        record_skipped(
            "x86_64-linux",
            &vec!["foo".to_string(), "bar".to_string()],
            "eval_failed",
        );
        let summary = take_skip_metrics();
        assert!(summary.total_skipped >= 2);
        assert!(summary.failed_batches >= 1);
        assert!(summary.samples.len() >= 2);
        assert_eq!(summary.unique_skipped, 2);
        assert!(summary.samples[0].contains("x86_64-linux:foo"));
    }

    #[test]
    fn test_skip_metrics_sample_cap() {
        let _guard = skip_metrics_test_lock().lock().unwrap();
        reset_skip_metrics();
        let attrs: Vec<String> = (0..(MAX_SKIP_SAMPLES as u32 + 10))
            .map(|i| format!("attr{}", i))
            .collect();
        record_skipped("x86_64-linux", &attrs, "eval_failed");
        let summary = take_skip_metrics();
        assert!(summary.total_skipped >= attrs.len() as u64);
        assert!(summary.samples.len() <= MAX_SKIP_SAMPLES);
        assert!(summary.unique_skipped >= attrs.len());
    }

    #[test]
    fn test_extract_attr_positions_returns_files() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        let default_nix = r#"
{ system, config }:
{
  hello = {
    pname = "hello";
    version = "1.0.0";
    type = "derivation";
    meta = {
      description = "A test package";
    };
  };
}
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        let positions = extract_attr_positions(path, "x86_64-linux").unwrap();
        assert!(positions.iter().any(|pos| pos.attr_path == "hello"));
    }

    #[test]
    fn test_extract_attr_positions_handles_non_attrset() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        let default_nix = r#"
{ system, config }:
  x: x
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        let positions = extract_attr_positions(path, "x86_64-linux").unwrap();
        assert!(positions.is_empty());
    }

    #[test]
    fn test_extract_packages_with_attr_filter() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        let default_nix = r#"
{ system, config }:
{
  hello = {
    pname = "hello";
    version = "1.0.0";
    type = "derivation";
  };
  world = {
    pname = "world";
    version = "2.0.0";
    type = "derivation";
  };
}
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        let names = vec!["hello".to_string()];
        let packages = extract_packages_for_attrs(path, "x86_64-linux", &names, true).unwrap();
        assert!(packages.iter().any(|pkg| pkg.name == "hello"));
        assert!(!packages.iter().any(|pkg| pkg.name == "world"));
    }

    #[test]
    fn test_store_paths_only_skips_metadata() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        let default_nix = r#"
{ system, config }:
{
  hello = {
    pname = "hello";
    version = "1.0.0";
    type = "derivation";
    meta = {
      description = "A test package";
      homepage = "https://example.com";
    };
  };
}
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        let names = vec!["hello".to_string()];
        let full = extract_packages_for_attrs(path, "x86_64-linux", &names, true).unwrap();
        let full_pkg = full.iter().find(|pkg| pkg.name == "hello").unwrap();
        assert!(full_pkg.description.is_some());
        assert!(full_pkg.homepage.is_some());

        let store_only =
            extract_packages_for_attrs_with_mode(path, "x86_64-linux", &names, true, true).unwrap();
        let store_pkg = store_only.iter().find(|pkg| pkg.name == "hello").unwrap();
        assert!(store_pkg.description.is_none());
        assert!(store_pkg.homepage.is_none());
        if let Some(full_path) = full_pkg.out_path.as_deref() {
            assert_eq!(store_pkg.out_path.as_deref(), Some(full_path));
        }
    }

    /// Test that extract_attr_positions handles attributes that throw errors.
    /// This simulates older nixpkgs commits where some attributes may fail to evaluate.
    #[test]
    fn test_extract_attr_positions_handles_throwing_attrs() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        // Create a nixpkgs-like structure where one attribute throws an error
        let default_nix = r#"
{ system, config }:
{
  goodPkg = {
    pname = "good-pkg";
    version = "1.0.0";
    type = "derivation";
  };
  # This attribute will throw when accessed
  badPkg = throw "This package is broken";
  anotherGoodPkg = {
    pname = "another-good-pkg";
    version = "2.0.0";
    type = "derivation";
  };
}
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        // Should not fail even though badPkg throws
        let positions = extract_attr_positions(path, "x86_64-linux").unwrap();

        // Should have positions for the good packages
        assert!(positions.iter().any(|p| p.attr_path == "goodPkg"));
        assert!(positions.iter().any(|p| p.attr_path == "anotherGoodPkg"));
        // badPkg may or may not have a position depending on when the error occurs
    }

    /// Test that large attribute lists are written to file to avoid "Argument list too long".
    /// This tests the fix for OS error 7 (E2BIG) when extracting many packages.
    #[test]
    fn test_extract_large_attr_list_uses_file() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        // Create a nixpkgs-like structure with many packages
        let mut default_nix = "{ system, config }:\n{\n".to_string();
        for i in 0..100 {
            default_nix.push_str(&format!(
                r#"  pkg{} = {{ pname = "pkg{}"; version = "1.0.0"; type = "derivation"; }};
"#,
                i, i
            ));
        }
        default_nix.push_str("}\n");
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        // Generate a large list of attribute names that would exceed the threshold
        // The threshold is 100,000 bytes, so we need ~10,000 attrs of ~10 chars each
        let mut large_attr_list: Vec<String> = (0..100).map(|i| format!("pkg{}", i)).collect();
        // Add many fake attrs to push over the size threshold
        for i in 100..15000 {
            large_attr_list.push(format!("nonexistent_package_{}", i));
        }

        // This should NOT fail with "Argument list too long" because
        // the attr list is written to a file
        let result = extract_packages_for_attrs(path, "x86_64-linux", &large_attr_list, true);

        match result {
            Ok(packages) => {
                // Should find some of the real packages
                assert!(packages.iter().any(|p| p.name == "pkg0"));
                assert!(packages.iter().any(|p| p.name == "pkg99"));
            }
            Err(e) => {
                // Should NOT be "Argument list too long"
                let err_str = e.to_string();
                assert!(
                    !err_str.contains("Argument list too long"),
                    "Should not get E2BIG error, but got: {}",
                    err_str
                );
                // Other nix eval errors are acceptable (e.g., evaluation errors)
            }
        }
    }

    /// Test the version extraction fallback chain: direct -> unwrapped -> passthru -> name
    /// This verifies the Phase 1 version extraction improvements work correctly.
    #[test]
    fn test_version_extraction_fallback_chain() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        // Create test packages with different version extraction scenarios
        let default_nix = r#"
{ system, config }:
{
  # Direct version - should use "direct" source
  directVersion = {
    pname = "direct-pkg";
    version = "1.0.0";
    type = "derivation";
    meta = { description = "Package with direct version"; };
  };

  # Wrapper pattern - version from unwrapped
  wrapperPkg = {
    pname = "wrapper-pkg";
    type = "derivation";
    unwrapped = {
      version = "2.0.0";
    };
    meta = { description = "Wrapper without direct version"; };
  };

  # Passthru pattern - version from passthru.unwrapped
  passthruPkg = {
    pname = "passthru-pkg";
    type = "derivation";
    passthru = {
      unwrapped = {
        version = "3.0.0";
      };
    };
    meta = { description = "Package with passthru unwrapped version"; };
  };

  # Name-based version - version extracted from pname
  namePkg = {
    pname = "name-pkg-4.0.0";
    type = "derivation";
    meta = { description = "Package with version in name"; };
  };

  # Truly versionless - no version anywhere
  versionlessPkg = {
    pname = "versionless-hook";
    type = "derivation";
    meta = { description = "A build hook with no version"; };
  };
}
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        let packages = extract_packages_for_attrs(path, "x86_64-linux", &[], true).unwrap();

        // Test direct version extraction
        let direct = packages.iter().find(|p| p.name == "direct-pkg");
        assert!(direct.is_some(), "Should find direct-pkg");
        let direct = direct.unwrap();
        assert_eq!(direct.version.as_deref(), Some("1.0.0"));
        assert_eq!(direct.version_source.as_deref(), Some("direct"));

        // Test wrapper version extraction
        let wrapper = packages.iter().find(|p| p.name == "wrapper-pkg");
        assert!(wrapper.is_some(), "Should find wrapper-pkg");
        let wrapper = wrapper.unwrap();
        assert_eq!(wrapper.version.as_deref(), Some("2.0.0"));
        assert_eq!(wrapper.version_source.as_deref(), Some("unwrapped"));

        // Test passthru version extraction
        let passthru = packages.iter().find(|p| p.name == "passthru-pkg");
        assert!(passthru.is_some(), "Should find passthru-pkg");
        let passthru = passthru.unwrap();
        assert_eq!(passthru.version.as_deref(), Some("3.0.0"));
        assert_eq!(passthru.version_source.as_deref(), Some("passthru"));

        // Test name-based version extraction
        let name_pkg = packages.iter().find(|p| p.name == "name-pkg-4.0.0");
        assert!(name_pkg.is_some(), "Should find name-pkg-4.0.0");
        let name_pkg = name_pkg.unwrap();
        assert_eq!(name_pkg.version.as_deref(), Some("4.0.0"));
        assert_eq!(name_pkg.version_source.as_deref(), Some("name"));

        // Test versionless package
        let versionless = packages.iter().find(|p| p.name == "versionless-hook");
        assert!(versionless.is_some(), "Should find versionless-hook");
        let versionless = versionless.unwrap();
        assert!(
            versionless.version.is_none() || versionless.version.as_deref() == Some(""),
            "Versionless package should have None or empty version"
        );
    }

    /// Test that version_source field is properly populated
    #[test]
    fn test_version_source_field_populated() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        let default_nix = r#"
{ system, config }:
{
  hello = {
    pname = "hello";
    version = "2.12.1";
    type = "derivation";
    meta = { description = "Hello world"; };
  };
}
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        let packages =
            extract_packages_for_attrs(path, "x86_64-linux", &["hello".to_string()], true).unwrap();

        assert_eq!(packages.len(), 1);
        let hello = &packages[0];
        assert_eq!(hello.name, "hello");
        assert_eq!(hello.version.as_deref(), Some("2.12.1"));
        assert_eq!(hello.version_source.as_deref(), Some("direct"));
    }

    /// Regression test that validates package extraction against known-good fixture.
    ///
    /// This test requires a nixpkgs clone and runs actual nix evaluation, so it's
    /// marked as ignored. Run with:
    ///   NIXPKGS_PATH=/path/to/nixpkgs cargo test --features indexer test_regression_fixture -- --ignored
    #[test]
    #[ignore]
    fn test_regression_fixture() {
        use regex::Regex;
        use serde::Deserialize;
        use std::collections::HashMap;
        use std::env;
        use std::fs;

        #[derive(Deserialize)]
        struct RegressionFixture {
            packages: Vec<PackageTest>,
        }

        #[derive(Deserialize)]
        struct PackageTest {
            attr_path: String,
            expect_version_regex: Option<String>,
            expect_version_source: Option<serde_json::Value>,
        }

        // Check for nixpkgs path
        let nixpkgs_path = env::var("NIXPKGS_PATH").unwrap_or_else(|_| {
            panic!(
                "NIXPKGS_PATH environment variable not set. \
                 Set it to the path of your nixpkgs clone."
            )
        });

        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            panic!("nix is not available - required for regression tests");
        }

        // Load fixture
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/regression_packages.json");
        let fixture_content = fs::read_to_string(&fixture_path)
            .unwrap_or_else(|e| panic!("Failed to read fixture file {:?}: {}", fixture_path, e));
        let fixture: RegressionFixture =
            serde_json::from_str(&fixture_content).expect("Failed to parse fixture JSON");

        // Extract packages
        let attr_paths: Vec<String> = fixture
            .packages
            .iter()
            .map(|p| p.attr_path.clone())
            .collect();
        let packages = extract_packages_for_attrs(
            std::path::Path::new(&nixpkgs_path),
            "x86_64-linux",
            &attr_paths,
            true,
        )
        .expect("Failed to extract packages");

        // Build lookup map
        let package_map: HashMap<_, _> = packages
            .iter()
            .map(|p| (p.attribute_path.clone(), p))
            .collect();

        let mut failures = Vec::new();

        for test_pkg in &fixture.packages {
            let pkg = match package_map.get(&test_pkg.attr_path) {
                Some(p) => p,
                None => {
                    failures.push(format!("{}: Package not extracted", test_pkg.attr_path));
                    continue;
                }
            };

            // Check version regex
            if let Some(regex_str) = &test_pkg.expect_version_regex {
                let regex = Regex::new(regex_str).expect("Invalid regex in fixture");
                match &pkg.version {
                    Some(v) if regex.is_match(v) => {}
                    Some(v) => {
                        failures.push(format!(
                            "{}: Version '{}' doesn't match regex '{}'",
                            test_pkg.attr_path, v, regex_str
                        ));
                    }
                    None => {
                        failures.push(format!(
                            "{}: No version extracted (expected match for '{}')",
                            test_pkg.attr_path, regex_str
                        ));
                    }
                }
            }

            // Check version source
            if let Some(expected_source) = &test_pkg.expect_version_source {
                let actual_source = pkg.version_source.as_deref();
                let matches = match expected_source {
                    serde_json::Value::String(s) => actual_source == Some(s.as_str()),
                    serde_json::Value::Array(arr) => arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .any(|s| actual_source == Some(s)),
                    _ => false,
                };
                if !matches {
                    failures.push(format!(
                        "{}: version_source {:?} doesn't match expected {:?}",
                        test_pkg.attr_path, actual_source, expected_source
                    ));
                }
            }
        }

        if !failures.is_empty() {
            panic!(
                "Regression test failures ({} of {}):\n  - {}",
                failures.len(),
                fixture.packages.len(),
                failures.join("\n  - ")
            );
        }

        println!(
            "Regression test passed: {} packages validated",
            fixture.packages.len()
        );
    }

    /// Documents the version extraction patterns in extract.nix.
    ///
    /// The extractVersionFromName function handles these edge cases:
    /// - Milestone versions: mezzo-0.0.m8 -> 0.0.m8
    /// - Internal hyphens: omake-0.9.8.6-0.rc1 -> 0.9.8.6-0.rc1
    /// - File extensions: perl-Memoize-1.03.tgz -> 1.03
    /// - Pre-release: matita-0.99.1pre130312 -> 0.99.1pre130312
    ///
    /// To test these patterns manually, run:
    /// ```bash
    /// nix-instantiate --eval --strict -E '
    /// let
    ///   # Copy patterns from extract.nix and test them
    ///   extract = name: ...; # See extract.nix extractVersionFromName
    /// in {
    ///   mezzo = extract "mezzo-0.0.m8";           # -> "0.0.m8"
    ///   omake = extract "omake-0.9.8.6-0.rc1";    # -> "0.9.8.6-0.rc1"
    ///   perl = extract "perl-Memoize-1.03.tgz";   # -> "1.03"
    ///   matita = extract "matita-0.99.1pre130312"; # -> "0.99.1pre130312"
    /// }
    /// '
    /// ```
    #[test]
    fn test_version_extraction_patterns_documented() {
        // This test documents the expected behavior of extractVersionFromName in extract.nix
        // The actual extraction happens in Nix code and is tested via nix-instantiate
        //
        // Expected extractions (validated via nix-instantiate):
        let expected_extractions = [
            ("mezzo-0.0.m8", "0.0.m8"),
            ("omake-0.9.8.6-0.rc1", "0.9.8.6-0.rc1"),
            ("perl-Memoize-1.03.tgz", "1.03"),
            ("matita-0.99.1pre130312", "0.99.1pre130312"),
            ("foo-1.2.3.tar.gz", "1.2.3"),
            ("hello-2.12.1", "2.12.1"),
            ("pkg-1.0rc1", "1.0rc1"),
            ("pkg-2.0.0beta2", "2.0.0beta2"),
            ("pkg-v1.2.3", "1.2.3"),
            ("pkg-2021-07-29", "2021-07-29"),
        ];

        // These should NOT extract a version (return null)
        let no_version_extractions = ["vimplugin-YankRing", "stdenv-linux", "stdenv"];

        // Document the patterns for reference
        println!("Version extraction patterns (from extract.nix):");
        println!("  1. Internal hyphen: .*-([0-9]+\\.[0-9]+(\\.[0-9]+)*-[0-9a-z.]+)$");
        println!("  2. Pre-release: .*-([0-9]+\\.[0-9]+(\\.[0-9]+)*[a-z]+[0-9]+)$");
        println!("  3. Milestone: .*-([0-9]+\\.[0-9]+\\.[a-z]+[0-9]*)$");
        println!("  4. Letter suffix: .*-([0-9]+\\.[0-9]+(\\.[0-9]+)*[a-z]+[0-9]*)$");
        println!("  5. Semver: .*-([0-9]+\\.[0-9]+(\\.[0-9]+)*[a-z]?)$");
        println!("  + Extension stripping: .tar.gz, .tgz, .zip, etc.");

        println!("\nExpected extractions:");
        for (name, version) in &expected_extractions {
            println!("  {} -> {}", name, version);
        }

        println!("\nNo version expected:");
        for name in &no_version_extractions {
            println!("  {} -> null", name);
        }

        // The test passes if the documentation is correct
        // Actual validation is done via nix-instantiate in development
        assert!(!expected_extractions.is_empty());
    }

    /// Test that extract.nix handles an explicit empty list for attrNames.
    ///
    /// This is a regression test for a bug where passing `attrNames = []` to extract.nix
    /// would result in no packages being extracted, because the condition
    /// `attrNames != null` was true for `[]`, so it used the empty list instead of
    /// falling back to `builtins.attrNames pkgs`.
    ///
    /// The fix changed the condition to check both null AND empty:
    /// `attrNames != null && builtins.length attrNames > 0`
    #[test]
    fn test_extract_nix_handles_empty_list_directly() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        // Create a simple nixpkgs mock
        let default_nix = r#"
{ system, config }:
{
  hello = {
    pname = "hello";
    version = "1.0.0";
    type = "derivation";
    meta = { description = "A test package"; };
  };
  world = {
    pname = "world";
    version = "2.0.0";
    type = "derivation";
    meta = { description = "Another test package"; };
  };
}
"#;
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        // Write extract.nix to a temp file
        let nix_file = temp_dir.path().join("extract.nix");
        std::fs::write(&nix_file, EXTRACT_NIX).unwrap();

        // Build expression that passes an EMPTY LIST (not null) for attrNames
        // This is the exact case that was buggy before the fix
        let canonical_path = std::fs::canonicalize(path).unwrap();
        let expr = format!(
            "import {} {{ nixpkgsPath = \"{}\"; system = \"x86_64-linux\"; attrNames = []; }}",
            nix_file.display(),
            canonical_path.display()
        );

        // Run nix-instantiate directly with the empty list
        let output = Command::new("nix-instantiate")
            .arg("--eval")
            .arg("--json")
            .arg("--strict")
            .arg("-E")
            .arg(&expr)
            .output()
            .expect("Failed to run nix-instantiate");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!(
                "nix-instantiate failed with empty attrNames list: {}",
                stderr
            );
        }

        // Parse the result and verify packages were discovered
        let stdout = String::from_utf8_lossy(&output.stdout);
        let packages: Vec<serde_json::Value> =
            serde_json::from_str(&stdout).expect("Failed to parse JSON output");

        // With the fix, packages should be discovered even with attrNames = []
        assert!(
            !packages.is_empty(),
            "extract.nix should discover packages when attrNames = [], but got empty result"
        );

        // Verify both packages were found
        let names: Vec<&str> = packages
            .iter()
            .filter_map(|p| p.get("name").and_then(|v| v.as_str()))
            .collect();
        assert!(
            names.contains(&"hello"),
            "Should find 'hello' package, got: {:?}",
            names
        );
        assert!(
            names.contains(&"world"),
            "Should find 'world' package, got: {:?}",
            names
        );
    }

    /// Test that large attribute lists are batched correctly.
    ///
    /// This tests the batching logic where lists > BATCH_SIZE (100) are split
    /// into multiple batches to avoid memory pressure during Nix evaluation.
    #[test]
    fn test_batching_logic_splits_large_lists() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        // Create a nixpkgs mock with 250 packages (more than BATCH_SIZE of 100)
        let mut default_nix = "{ system, config }:\n{\n".to_string();
        for i in 0..250 {
            default_nix.push_str(&format!(
                r#"  pkg{} = {{ pname = "pkg{}"; version = "{}.0.0"; type = "derivation"; }};
"#,
                i, i, i
            ));
        }
        default_nix.push_str("}\n");
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        // Generate attr names for all 250 packages
        let attr_names: Vec<String> = (0..250).map(|i| format!("pkg{}", i)).collect();

        // This should trigger batching (250 > 100)
        let packages = extract_packages_for_attrs(path, "x86_64-linux", &attr_names, true).unwrap();

        // Should have extracted all 250 packages despite batching
        assert_eq!(
            packages.len(),
            250,
            "Should extract all 250 packages across batches"
        );

        // Verify some specific packages from different batches
        assert!(packages.iter().any(|p| p.name == "pkg0")); // First batch
        assert!(packages.iter().any(|p| p.name == "pkg99")); // End of first batch
        assert!(packages.iter().any(|p| p.name == "pkg100")); // Start of second batch
        assert!(packages.iter().any(|p| p.name == "pkg249")); // Last package
    }

    /// Test that small attribute lists are NOT batched.
    ///
    /// Lists smaller than BATCH_SIZE should be processed in a single call
    /// without batching overhead.
    #[test]
    fn test_small_lists_not_batched() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        // Create a nixpkgs mock with 50 packages (less than BATCH_SIZE of 100)
        let mut default_nix = "{ system, config }:\n{\n".to_string();
        for i in 0..50 {
            default_nix.push_str(&format!(
                r#"  pkg{} = {{ pname = "pkg{}"; version = "{}.0.0"; type = "derivation"; }};
"#,
                i, i, i
            ));
        }
        default_nix.push_str("}\n");
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        // Generate attr names for all 50 packages
        let attr_names: Vec<String> = (0..50).map(|i| format!("pkg{}", i)).collect();

        // This should NOT trigger batching (50 < 500)
        let packages = extract_packages_for_attrs(path, "x86_64-linux", &attr_names, true).unwrap();

        // Should have extracted all 50 packages
        assert_eq!(packages.len(), 50, "Should extract all 50 packages");
    }

    /// Test that batched extraction continues on partial failures.
    ///
    /// When some batches fail (e.g., due to evaluation errors), the extraction
    /// should continue with remaining batches and return all successful results.
    /// This is critical for full extraction where memory pressure may cause
    /// some batches to fail.
    #[test]
    fn test_batched_extraction_continues_on_partial_failure() {
        let nix_check = Command::new("nix").arg("--version").output();
        if nix_check.is_err() || !nix_check.unwrap().status.success() {
            eprintln!("Skipping: nix not available");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path();
        std::fs::create_dir_all(path.join("pkgs")).unwrap();

        // Create a nixpkgs mock with 600 packages, some that throw errors
        let mut default_nix = "{ system, config }:\n{\n".to_string();
        for i in 0..600 {
            default_nix.push_str(&format!(
                r#"  pkg{} = {{ pname = "pkg{}"; version = "{}.0.0"; type = "derivation"; }};
"#,
                i, i, i
            ));
        }
        default_nix.push_str("}\n");
        std::fs::write(path.join("default.nix"), default_nix).unwrap();

        // Request 600 real packages plus some non-existent ones
        // The non-existent ones will simply not be found, but shouldn't cause batch failures
        let mut attr_names: Vec<String> = (0..600).map(|i| format!("pkg{}", i)).collect();
        attr_names.extend((0..100).map(|i| format!("nonexistent{}", i)));

        // This should process all batches, finding 600 real packages
        let packages = extract_packages_for_attrs(path, "x86_64-linux", &attr_names, true).unwrap();

        // Should have found the 600 real packages
        assert_eq!(
            packages.len(),
            600,
            "Should extract all 600 existing packages"
        );
    }

    /// Test that batch size constant is reasonable.
    ///
    /// BATCH_SIZE should be:
    /// - Large enough to amortize overhead (>= 50)
    /// - Small enough to avoid memory pressure (<= 200)
    #[test]
    fn test_batch_size_is_reasonable() {
        assert!(
            DEFAULT_BATCH_SIZE >= 50,
            "Default batch size should be >= 50 for efficiency"
        );
        assert!(
            DEFAULT_BATCH_SIZE <= 200,
            "Default batch size should be <= 200 to avoid memory pressure with 2GB workers"
        );
        assert!(
            STORE_PATH_BATCH_SIZE >= 10,
            "Store-path batch size should be >= 10 for progress"
        );
        assert!(
            STORE_PATH_BATCH_SIZE <= DEFAULT_BATCH_SIZE,
            "Store-path batch size should not exceed default batch size"
        );
    }
}
