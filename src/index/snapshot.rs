//! Snapshot parsing: one channel release's package set.
//!
//! Two wire formats, one output shape:
//!
//! - `packages.json.br` (2020-03-27 →): brotli-compressed
//!   `{"version": 2, "packages": { <attrpath>: {name, pname, version, system, meta} }}`.
//!   Parsed **streaming** — the decompressed JSON reaches ~381 MB and is never
//!   materialized; entries fold one at a time into the output vector.
//! - `nix-env -qaP --json --meta` output (pre-2020 era): the same entry shape
//!   as a bare top-level map, no envelope.
//!
//! Era tolerances (verified against 2020/2021/2023/2026 artifacts):
//! license is dict | list[dict] | str | list[str] | `[]`; homepage is str or
//! (rarely) a list; `meta.position` carries a `/build/source/` prefix in
//! 2020-era files; attr keys contain Nix-quoted segments
//! (`aspellDicts."or"`); unknown fields (identifiers, teams, outputName, …)
//! are ignored.

use crate::error::{NxvError, Result};
use serde::Deserialize;
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde_json::Value;
use std::fmt;
use std::io::Read;

/// One package observation parsed from a snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotEntry {
    /// Full dotted attribute path, segments unquoted.
    pub attribute_path: String,
    /// Normalized display name: pname for top-level attrs, the final
    /// attribute segment for nested attrs (stable across upstream pname
    /// convention drift).
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    /// JSON array of license identifiers (spdxId | shortName | fullName).
    pub license: Option<String>,
    pub homepage: Option<String>,
    /// JSON array of maintainer handles/names.
    pub maintainers: Option<String>,
    /// JSON array of platform strings.
    pub platforms: Option<String>,
    /// Source path relative to the nixpkgs root (from `meta.position`).
    pub source_path: Option<String>,
    /// JSON array of advisory strings.
    pub known_vulnerabilities: Option<String>,
}

/// Split a Nix attribute path into segments, honoring quoted segments
/// (`aspellDicts."or"` → `["aspellDicts", "or"]`).
pub fn split_attr_segments(raw: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => in_quotes = !in_quotes,
            '\\' if in_quotes => {
                // Rare escaped character inside a quoted segment.
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
            '.' if !in_quotes => {
                segments.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    segments.push(current);
    segments
}

/// Raw per-package entry as found in both wire formats.
#[derive(Debug, Deserialize)]
struct RawEntry {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    pname: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    meta: RawMeta,
}

#[derive(Debug, Default, Deserialize)]
struct RawMeta {
    #[serde(default)]
    description: Option<Value>,
    #[serde(default)]
    homepage: Option<Value>,
    #[serde(default)]
    license: Option<Value>,
    #[serde(default)]
    maintainers: Option<Value>,
    #[serde(default)]
    platforms: Option<Value>,
    #[serde(default)]
    position: Option<String>,
    #[serde(default, rename = "knownVulnerabilities")]
    known_vulnerabilities: Option<Value>,
}

/// Derive the version from a `name-version` string when `version` is absent
/// (some ancient entries). Splits at the first `-<digit>` boundary.
fn version_from_name(name: &str) -> Option<(String, String)> {
    let bytes = name.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'-' && bytes[i + 1].is_ascii_digit() {
            return Some((name[..i].to_string(), name[i + 1..].to_string()));
        }
    }
    None
}

fn value_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// license: dict | list[dict] | str | list[str] | [] → JSON array of ids.
fn normalize_license(v: &Value) -> Option<String> {
    fn one(v: &Value) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Object(o) => o
                .get("spdxId")
                .or_else(|| o.get("shortName"))
                .or_else(|| o.get("fullName"))
                .and_then(value_to_string),
            _ => None,
        }
    }

    let ids: Vec<String> = match v {
        Value::Array(items) => items.iter().filter_map(one).collect(),
        other => one(other).into_iter().collect(),
    };
    if ids.is_empty() {
        None
    } else {
        serde_json::to_string(&ids).ok()
    }
}

/// homepage: str | list (take first string).
fn normalize_homepage(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Array(items) => items.iter().find_map(value_to_string),
        _ => None,
    }
}

/// maintainers: list of {github|name|email} or strings → JSON array.
fn normalize_maintainers(v: &Value) -> Option<String> {
    fn one(v: &Value) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Object(o) => o
                .get("github")
                .or_else(|| o.get("name"))
                .or_else(|| o.get("email"))
                .and_then(value_to_string),
            _ => None,
        }
    }
    let names: Vec<String> = match v {
        Value::Array(items) => items.iter().filter_map(one).collect(),
        other => one(other).into_iter().collect(),
    };
    if names.is_empty() {
        None
    } else {
        serde_json::to_string(&names).ok()
    }
}

/// platforms: list of strings / {system} attrs | single string → JSON array.
fn normalize_platforms(v: &Value) -> Option<String> {
    fn one(v: &Value) -> Option<String> {
        match v {
            Value::String(s) => Some(s.clone()),
            Value::Object(o) => o.get("system").and_then(value_to_string),
            _ => None,
        }
    }
    let platforms: Vec<String> = match v {
        Value::Array(items) => items.iter().filter_map(one).collect(),
        other => one(other).into_iter().collect(),
    };
    if platforms.is_empty() {
        None
    } else {
        serde_json::to_string(&platforms).ok()
    }
}

/// knownVulnerabilities: list of strings → JSON array.
fn normalize_vulnerabilities(v: &Value) -> Option<String> {
    let advisories: Vec<String> = match v {
        Value::Array(items) => items.iter().filter_map(value_to_string).collect(),
        _ => return None,
    };
    if advisories.is_empty() {
        None
    } else {
        serde_json::to_string(&advisories).ok()
    }
}

/// meta.position → source path relative to the nixpkgs root.
///
/// Handles `/build/source/pkgs/...` (2020-era), store-path prefixes, and the
/// `:NNN` line suffix.
fn normalize_position(position: &str) -> Option<String> {
    let no_line = position.rsplit_once(':').map_or(position, |(path, line)| {
        if line.bytes().all(|b| b.is_ascii_digit()) {
            path
        } else {
            position
        }
    });
    no_line
        .find("pkgs/")
        .map(|idx| no_line[idx..].to_string())
        .or_else(|| {
            // nixos modules etc. — keep a recognizable relative path if present
            no_line.find("nixos/").map(|idx| no_line[idx..].to_string())
        })
}

/// Fold one raw entry into a [`SnapshotEntry`]. Returns `None` for entries
/// without a usable version (non-derivation stubs).
fn fold_entry(raw_attr: &str, entry: RawEntry) -> Option<SnapshotEntry> {
    let segments = split_attr_segments(raw_attr);
    let attribute_path = segments.join(".");
    let leaf = segments.last()?.clone();
    if attribute_path.is_empty() {
        return None;
    }

    // version: explicit field, else parsed off `name`.
    let (derived_pname, derived_version) = entry
        .name
        .as_deref()
        .and_then(version_from_name)
        .map(|(p, v)| (Some(p), Some(v)))
        .unwrap_or((None, None));
    let version = entry
        .version
        .filter(|v| !v.is_empty())
        .or(derived_version)?;

    // name: pname for top-level, attr leaf for nested (stable across
    // upstream pname-convention drift, e.g. python3.10-requests → requests).
    let is_nested = segments.len() > 1;
    let name = if is_nested {
        leaf
    } else {
        entry
            .pname
            .filter(|p| !p.is_empty())
            .or(derived_pname)
            .unwrap_or(leaf)
    };

    let meta = entry.meta;
    Some(SnapshotEntry {
        attribute_path,
        name,
        version,
        description: meta.description.as_ref().and_then(value_to_string),
        license: meta.license.as_ref().and_then(normalize_license),
        homepage: meta.homepage.as_ref().and_then(normalize_homepage),
        maintainers: meta.maintainers.as_ref().and_then(normalize_maintainers),
        platforms: meta.platforms.as_ref().and_then(normalize_platforms),
        source_path: meta.position.as_deref().and_then(normalize_position),
        known_vulnerabilities: meta
            .known_vulnerabilities
            .as_ref()
            .and_then(normalize_vulnerabilities),
    })
}

/// Streaming seed for the `"packages"` map: folds entries one at a time.
struct PackagesMapSeed<'a> {
    out: &'a mut Vec<SnapshotEntry>,
}

impl<'de> DeserializeSeed<'de> for PackagesMapSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MapVisitor<'a> {
            out: &'a mut Vec<SnapshotEntry>,
        }

        impl<'de> Visitor<'de> for MapVisitor<'_> {
            type Value = ();

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map of attribute paths to package entries")
            }

            fn visit_map<A>(self, mut map: A) -> std::result::Result<(), A::Error>
            where
                A: MapAccess<'de>,
            {
                while let Some(attr) = map.next_key::<String>()? {
                    let entry: RawEntry = map.next_value()?;
                    if let Some(folded) = fold_entry(&attr, entry) {
                        self.out.push(folded);
                    }
                }
                Ok(())
            }
        }

        deserializer.deserialize_map(MapVisitor { out: self.out })
    }
}

/// Parse a `packages.json` stream (the envelope format). The reader should
/// already be decompressed (see [`parse_packages_json_br`]).
pub fn parse_packages_json<R: Read>(reader: R) -> Result<Vec<SnapshotEntry>> {
    struct FileVisitor {
        entries: Vec<SnapshotEntry>,
        version_seen: Option<u64>,
    }

    impl<'de> Visitor<'de> for &mut FileVisitor {
        type Value = ();

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a packages.json document")
        }

        fn visit_map<A>(self, mut map: A) -> std::result::Result<(), A::Error>
        where
            A: MapAccess<'de>,
        {
            while let Some(key) = map.next_key::<String>()? {
                match key.as_str() {
                    "version" => self.version_seen = Some(map.next_value::<u64>()?),
                    "packages" => map.next_value_seed(PackagesMapSeed {
                        out: &mut self.entries,
                    })?,
                    _ => {
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                }
            }
            Ok(())
        }
    }

    let mut visitor = FileVisitor {
        entries: Vec::new(),
        version_seen: None,
    };
    let mut deserializer = serde_json::Deserializer::from_reader(std::io::BufReader::new(reader));
    serde::Deserializer::deserialize_map(&mut deserializer, &mut visitor)
        .map_err(NxvError::Json)?;
    deserializer.end().map_err(NxvError::Json)?;

    match visitor.version_seen {
        Some(2) => Ok(visitor.entries),
        Some(other) => Err(NxvError::Config(format!(
            "unsupported packages.json format version {other} (expected 2)"
        ))),
        None => Err(NxvError::Config(
            "packages.json has no format version field".to_string(),
        )),
    }
}

/// Parse a brotli-compressed `packages.json.br` stream.
pub fn parse_packages_json_br<R: Read>(reader: R) -> Result<Vec<SnapshotEntry>> {
    let decompressor = brotli::Decompressor::new(reader, 64 * 1024);
    parse_packages_json(decompressor)
}

/// Parse `nix-env -qaP --json --meta` output: a bare attr→entry map.
pub fn parse_nix_env_json<R: Read>(reader: R) -> Result<Vec<SnapshotEntry>> {
    let mut entries = Vec::new();
    let mut deserializer = serde_json::Deserializer::from_reader(std::io::BufReader::new(reader));
    PackagesMapSeed { out: &mut entries }
        .deserialize(&mut deserializer)
        .map_err(NxvError::Json)?;
    deserializer.end().map_err(NxvError::Json)?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_attr_segments_quoted() {
        assert_eq!(split_attr_segments("firefox"), vec!["firefox"]);
        assert_eq!(
            split_attr_segments("python313Packages.requests"),
            vec!["python313Packages", "requests"]
        );
        assert_eq!(
            split_attr_segments("aspellDicts.\"or\""),
            vec!["aspellDicts", "or"]
        );
        assert_eq!(
            split_attr_segments("chickenPackages_5.chickenEggs.\"7off\""),
            vec!["chickenPackages_5", "chickenEggs", "7off"]
        );
        assert_eq!(
            split_attr_segments("emacsPackages.\"@\""),
            vec!["emacsPackages", "@"]
        );
    }

    #[test]
    fn test_modern_packages_json_with_era_2026_fields() {
        let json = r#"{
          "version": 2,
          "packages": {
            "firefox": {
              "name": "firefox-146.0.1",
              "pname": "firefox",
              "version": "146.0.1",
              "system": "x86_64-linux",
              "outputName": "out",
              "outputs": {"out": null},
              "meta": {
                "description": "A web browser",
                "homepage": "https://www.mozilla.org/firefox/",
                "license": {"fullName": "Mozilla Public License 2.0", "spdxId": "MPL-2.0", "free": true},
                "maintainers": [{"github": "mweinelt", "name": "Martin Weinelt"}],
                "teams": [{"members": []}],
                "identifiers": {"cpe": "cpe:x"},
                "platforms": ["x86_64-linux", "aarch64-darwin"],
                "position": "pkgs/applications/networkers/firefox/packages.nix:14"
              }
            },
            "python313Packages.requests": {
              "name": "python3.13-requests-2.32.3",
              "pname": "requests",
              "version": "2.32.3",
              "system": "x86_64-linux",
              "meta": {
                "license": [{"spdxId": "Apache-2.0"}],
                "knownVulnerabilities": []
              }
            },
            "aspellDicts.\"or\"": {
              "name": "aspell-dict-or-0.03-1",
              "pname": "aspell-dict-or",
              "version": "0.03-1",
              "system": "x86_64-linux",
              "meta": {"license": []}
            },
            "not-a-derivation-stub": {
              "name": null,
              "pname": null,
              "version": null,
              "system": null,
              "meta": {}
            }
          }
        }"#;

        let entries = parse_packages_json(json.as_bytes()).unwrap();
        assert_eq!(entries.len(), 3, "version-less stub must be dropped");

        let firefox = entries
            .iter()
            .find(|e| e.attribute_path == "firefox")
            .unwrap();
        assert_eq!(firefox.name, "firefox");
        assert_eq!(firefox.version, "146.0.1");
        assert_eq!(firefox.license.as_deref(), Some(r#"["MPL-2.0"]"#));
        assert_eq!(firefox.maintainers.as_deref(), Some(r#"["mweinelt"]"#));
        assert_eq!(
            firefox.source_path.as_deref(),
            Some("pkgs/applications/networkers/firefox/packages.nix")
        );

        let requests = entries
            .iter()
            .find(|e| e.attribute_path == "python313Packages.requests")
            .unwrap();
        assert_eq!(requests.name, "requests", "nested name = attr leaf");
        assert_eq!(requests.license.as_deref(), Some(r#"["Apache-2.0"]"#));
        assert!(
            requests.known_vulnerabilities.is_none(),
            "empty list -> NULL"
        );

        let aspell = entries
            .iter()
            .find(|e| e.attribute_path == "aspellDicts.or")
            .unwrap();
        assert_eq!(aspell.name, "or", "quoted nested leaf unquoted");
        assert!(aspell.license.is_none(), "empty license list -> NULL");
    }

    #[test]
    fn test_2020_era_position_prefix_and_pname_drift() {
        let json = r#"{
          "version": 2,
          "packages": {
            "python38Packages.requests": {
              "name": "python3.8-requests-2.23.0",
              "pname": "python3.8-requests",
              "version": "2.23.0",
              "system": "x86_64-linux",
              "meta": {
                "position": "/build/source/pkgs/development/python-modules/requests/default.nix:5",
                "homepage": ["https://requests.readthedocs.io/"],
                "license": "MIT"
              }
            }
          }
        }"#;

        let entries = parse_packages_json(json.as_bytes()).unwrap();
        let e = &entries[0];
        assert_eq!(
            e.name, "requests",
            "nested attrs use the leaf, not the era-dependent pname"
        );
        assert_eq!(
            e.source_path.as_deref(),
            Some("pkgs/development/python-modules/requests/default.nix"),
            "/build/source/ prefix must be stripped"
        );
        assert_eq!(
            e.homepage.as_deref(),
            Some("https://requests.readthedocs.io/"),
            "list-valued homepage takes the first entry"
        );
        assert_eq!(e.license.as_deref(), Some(r#"["MIT"]"#));
    }

    #[test]
    fn test_envelope_version_is_asserted() {
        let bad = r#"{"version": 3, "packages": {}}"#;
        assert!(parse_packages_json(bad.as_bytes()).is_err());

        let missing = r#"{"packages": {}}"#;
        assert!(parse_packages_json(missing.as_bytes()).is_err());
    }

    #[test]
    fn test_nix_env_bare_map_and_version_from_name() {
        let json = r#"{
          "firefox": {
            "name": "firefox-52.0.1",
            "system": "x86_64-linux",
            "meta": {
              "description": "A web browser",
              "platforms": ["x86_64-linux", {"system": "i686-linux"}],
              "knownVulnerabilities": ["CVE-2017-0001"]
            }
          },
          "haskellPackages.lens": {
            "name": "lens-4.15.1",
            "system": "x86_64-linux",
            "meta": {"license": {"shortName": "bsd3"}}
          }
        }"#;

        let entries = parse_nix_env_json(json.as_bytes()).unwrap();
        assert_eq!(entries.len(), 2);

        let firefox = entries
            .iter()
            .find(|e| e.attribute_path == "firefox")
            .unwrap();
        assert_eq!(firefox.version, "52.0.1", "version parsed from name");
        assert_eq!(firefox.name, "firefox");
        assert_eq!(
            firefox.platforms.as_deref(),
            Some(r#"["x86_64-linux","i686-linux"]"#),
            "attrset platforms entries use .system"
        );
        assert_eq!(
            firefox.known_vulnerabilities.as_deref(),
            Some(r#"["CVE-2017-0001"]"#)
        );

        let lens = entries
            .iter()
            .find(|e| e.attribute_path == "haskellPackages.lens")
            .unwrap();
        assert_eq!(lens.version, "4.15.1");
        assert_eq!(lens.license.as_deref(), Some(r#"["bsd3"]"#));
    }

    #[test]
    fn test_brotli_roundtrip_and_truncation() {
        let json = br#"{"version": 2, "packages": {"hello": {"pname": "hello", "version": "2.12", "meta": {}}}}"#;

        let mut compressed = Vec::new();
        {
            let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 5, 22);
            std::io::Write::write_all(&mut writer, json).unwrap();
        }

        let entries = parse_packages_json_br(&compressed[..]).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, "2.12");

        // Truncated stream must error, never return partial data silently.
        let truncated = &compressed[..compressed.len() / 2];
        assert!(parse_packages_json_br(truncated).is_err());
    }

    #[test]
    fn test_version_from_name() {
        assert_eq!(
            version_from_name("firefox-52.0.1"),
            Some(("firefox".to_string(), "52.0.1".to_string()))
        );
        assert_eq!(
            version_from_name("gst-plugins-base-1.18"),
            Some(("gst-plugins-base".to_string(), "1.18".to_string()))
        );
        assert_eq!(version_from_name("no-version-here"), None);
    }
}
