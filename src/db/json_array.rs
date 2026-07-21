//! Helpers for columns that store a JSON array in a TEXT column.
//!
//! The indexer normalizes nixpkgs' loosely-typed `meta` fields (`license` is
//! dict | list[dict] | str | list[str]; `platforms` is str | list[str | attrs])
//! into a canonical JSON array which SQLite stores as TEXT — see
//! [`crate::index::snapshot`]. That string is a *storage* detail: it must not
//! leak into `--format json`, the HTTP API, or human-readable output.
//!
//! [`parse`] is the single place that decodes such a column. [`serialize_opt`]
//! emits a real JSON array on the wire, and [`deserialize_opt`] accepts both
//! that array and the legacy stringified form, so a newer CLI keeps working
//! against an older `nxv serve` (and vice versa for stored fixtures).

use serde::{Deserialize, Deserializer, Serializer};

/// Decode a JSON-array column into its elements.
///
/// Tolerates every shape the column has held historically: a JSON array, a
/// bare string (pre-v4 indexes stored `MIT` rather than `["MIT"]`), the
/// `null`/`None` sentinels some older rows carry, and malformed JSON — the
/// last of which is returned as a single element rather than being dropped, so
/// data is never silently lost.
pub fn parse(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "null" || trimmed == "None" {
        return Vec::new();
    }

    if trimmed.starts_with('[')
        && let Ok(values) = serde_json::from_str::<Vec<serde_json::Value>>(trimmed)
    {
        return values.iter().filter_map(value_to_string).collect();
    }

    vec![raw.to_string()]
}

/// Render a JSON-array column for human-readable output (table, plain, info).
pub fn join(raw: &str) -> String {
    parse(raw).join(", ")
}

/// Render an optional JSON-array column, falling back to `placeholder` when the
/// column is NULL or decodes to nothing.
pub fn join_or(raw: Option<&str>, placeholder: &str) -> String {
    let joined = raw.map(join).unwrap_or_default();
    if joined.is_empty() {
        placeholder.to_string()
    } else {
        joined
    }
}

/// Flatten a JSON value into a display string, dropping nulls.
fn value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

/// Serialize a JSON-array column as a real JSON array.
///
/// A NULL column stays `null`; anything else becomes an array, so consumers can
/// rely on the field being either `null` or a list — never a string that needs
/// a second `fromjson`.
pub fn serialize_opt<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(raw) => serializer.collect_seq(parse(raw)),
        None => serializer.serialize_none(),
    }
}

/// Deserialize a JSON-array column from either a real array or a legacy string.
///
/// An array is re-encoded to the stringified form the DB uses, so a value that
/// arrived over HTTP is indistinguishable from one read out of SQLite. A legacy
/// bare string is kept verbatim rather than wrapped — [`parse`] already accepts
/// both spellings, so every consumer sees the same elements either way.
pub fn deserialize_opt<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        Str(String),
        List(Vec<serde_json::Value>),
    }

    let repr = Option::<Repr>::deserialize(deserializer)?;
    Ok(repr.map(|repr| match repr {
        Repr::Str(s) => s,
        Repr::List(items) => {
            let items: Vec<String> = items.iter().filter_map(value_to_string).collect();
            serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Holder {
        #[serde(serialize_with = "serialize_opt", deserialize_with = "deserialize_opt")]
        field: Option<String>,
    }

    fn holder(field: Option<&str>) -> Holder {
        Holder {
            field: field.map(str::to_string),
        }
    }

    #[test]
    fn parses_json_arrays() {
        assert_eq!(parse(r#"["MIT","BSD-3-Clause"]"#), ["MIT", "BSD-3-Clause"]);
        assert_eq!(parse("[]"), Vec::<String>::new());
    }

    #[test]
    fn parses_legacy_and_sentinel_values() {
        // Pre-v4 indexes stored a bare license string.
        assert_eq!(parse("MIT"), ["MIT"]);
        assert_eq!(parse("null"), Vec::<String>::new());
        assert_eq!(parse("None"), Vec::<String>::new());
        assert_eq!(parse("   "), Vec::<String>::new());
    }

    #[test]
    fn malformed_json_is_preserved_not_dropped() {
        assert_eq!(parse(r#"["unterminated"#), [r#"["unterminated"#]);
    }

    #[test]
    fn non_string_elements_are_stringified_and_nulls_dropped() {
        assert_eq!(parse(r#"["a",null,7]"#), ["a", "7"]);
    }

    #[test]
    fn joins_for_display() {
        assert_eq!(
            join(r#"["x86_64-linux","aarch64-darwin"]"#),
            "x86_64-linux, aarch64-darwin"
        );
        assert_eq!(join_or(None, "-"), "-");
        assert_eq!(join_or(Some("[]"), "-"), "-");
        assert_eq!(join_or(Some(r#"["MIT"]"#), "-"), "MIT");
    }

    #[test]
    fn serializes_as_real_array() {
        let json = serde_json::to_string(&holder(Some(r#"["Python-2.0"]"#))).unwrap();
        assert_eq!(json, r#"{"field":["Python-2.0"]}"#);
    }

    #[test]
    fn serializes_legacy_string_as_single_element_array() {
        let json = serde_json::to_string(&holder(Some("MIT"))).unwrap();
        assert_eq!(json, r#"{"field":["MIT"]}"#);
    }

    #[test]
    fn serializes_null_column_as_null() {
        let json = serde_json::to_string(&holder(None)).unwrap();
        assert_eq!(json, r#"{"field":null}"#);
    }

    #[test]
    fn deserializes_real_array() {
        let parsed: Holder = serde_json::from_str(r#"{"field":["MIT","ISC"]}"#).unwrap();
        assert_eq!(parsed, holder(Some(r#"["MIT","ISC"]"#)));
    }

    #[test]
    fn deserializes_legacy_stringified_array_from_older_server() {
        let parsed: Holder = serde_json::from_str(r#"{"field":"[\"MIT\"]"}"#).unwrap();
        assert_eq!(parsed, holder(Some(r#"["MIT"]"#)));
    }

    #[test]
    fn deserializes_null() {
        let parsed: Holder = serde_json::from_str(r#"{"field":null}"#).unwrap();
        assert_eq!(parsed, holder(None));
    }

    #[test]
    fn round_trips_through_serialization() {
        let original = holder(Some(r#"["x86_64-linux","aarch64-darwin"]"#));
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Holder = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, original);
    }
}
