//! Indexer configuration overrides loaded from JSON.

use crate::error::{NxvError, Result};
use crate::paths;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// JSON overrides for indexer settings (advanced options).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct IndexerConfigOverrides {
    pub checkpoint_interval: Option<usize>,
    pub workers: Option<usize>,
    pub gc_interval: Option<usize>,
    pub max_range_workers: Option<usize>,
    pub max_commits: Option<usize>,
    pub full_extraction_interval: Option<u32>,
    pub full_extraction_parallelism: Option<usize>,
    pub parallel_ranges: Option<String>,
}

impl IndexerConfigOverrides {
    pub fn is_empty(&self) -> bool {
        self.checkpoint_interval.is_none()
            && self.workers.is_none()
            && self.gc_interval.is_none()
            && self.max_range_workers.is_none()
            && self.max_commits.is_none()
            && self.full_extraction_interval.is_none()
            && self.full_extraction_parallelism.is_none()
            && self.parallel_ranges.is_none()
    }
}

fn default_config_path() -> PathBuf {
    paths::get_data_dir().join("indexer.json")
}

fn parse_overrides_json(json: &str, source: &str) -> Result<IndexerConfigOverrides> {
    serde_json::from_str(json)
        .map_err(|e| NxvError::Config(format!("Failed to parse indexer config {}: {}", source, e)))
}

fn read_overrides_from_path(path: &Path) -> Result<IndexerConfigOverrides> {
    if !path.exists() {
        return Ok(IndexerConfigOverrides::default());
    }
    let raw = fs::read_to_string(path)?;
    parse_overrides_json(&raw, &path.display().to_string())
}

/// Load indexer overrides from `NXV_INDEXER_CONFIG` or the default data-dir path.
///
/// If `NXV_INDEXER_CONFIG` contains JSON (starts with `{`), it is parsed directly.
/// Otherwise it is treated as a file path.
pub fn load_indexer_overrides() -> Result<IndexerConfigOverrides> {
    if let Ok(raw) = std::env::var("NXV_INDEXER_CONFIG") {
        let trimmed = raw.trim();
        if trimmed.starts_with('{') {
            return parse_overrides_json(trimmed, "from NXV_INDEXER_CONFIG");
        }
        let path = PathBuf::from(trimmed);
        return read_overrides_from_path(&path);
    }

    read_overrides_from_path(&default_config_path())
}
