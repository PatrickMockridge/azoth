//! The key-value evidence lookup the review engines share.
//!
//! NeqSim's `OpenDrainReviewItem` reads a caller's evidence map by alias: the first
//! **present** key wins, a string that does not parse as a number yields the default, and
//! the boolean tokens are `true/yes/y/1` and `false/no/n/0`. This is that arithmetic once.

use std::collections::HashMap;

/// One evidence value: a string, a string list, or a present key with no value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EvidenceValue {
    /// The key is present and carries no value; Java's `null`.
    Null,
    /// One string value.
    Text(String),
    /// A list value, as `getStringList` reads it.
    List(Vec<String>),
}

/// A review item's evidence map.
pub(crate) type Evidence = HashMap<String, EvidenceValue>;

/// Java's `Double.parseDouble` on a trimmed string, with the type suffix it accepts.
pub(crate) fn parse_java_double(raw: &str) -> Option<f64> {
    let text = raw.trim();
    let lower = text.to_ascii_lowercase();
    if lower == "nan" {
        return Some(f64::NAN);
    }
    if lower == "infinity" || lower == "+infinity" {
        return Some(f64::INFINITY);
    }
    if lower == "-infinity" {
        return Some(f64::NEG_INFINITY);
    }
    let body = text.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(text);
    body.parse::<f64>().ok()
}

/// The class's boolean tokens.
pub(crate) fn parse_bool(raw: &str) -> Option<bool> {
    let text = raw.trim();
    let lower = text.to_ascii_lowercase();
    if lower == "true" || lower == "yes" || lower == "y" || text == "1" {
        return Some(true);
    }
    if lower == "false" || lower == "no" || lower == "n" || text == "0" {
        return Some(false);
    }
    None
}

/// Whether the key is present, whatever its value.
pub(crate) fn has(evidence: &Evidence, key: &str) -> bool {
    evidence.contains_key(key)
}

/// `hasAny`, in the class's key order.
pub(crate) fn has_any(evidence: &Evidence, keys: &[&str]) -> bool {
    keys.iter().any(|key| evidence.contains_key(*key))
}

/// `getDouble`: the first present string that parses, or the default.
pub(crate) fn get_double(evidence: &Evidence, default: f64, keys: &[&str]) -> f64 {
    for key in keys {
        match evidence.get(*key) {
            Some(EvidenceValue::Text(text)) => return parse_java_double(text).unwrap_or(default),
            Some(EvidenceValue::Null | EvidenceValue::List(_)) | None => continue,
        }
    }
    default
}

/// `getBooleanObject`: the first present string that parses, or `None`.
pub(crate) fn get_bool(evidence: &Evidence, keys: &[&str]) -> Option<bool> {
    for key in keys {
        match evidence.get(*key) {
            Some(EvidenceValue::Text(text)) => {
                if let Some(value) = parse_bool(text) {
                    return Some(value);
                }
            }
            Some(EvidenceValue::Null | EvidenceValue::List(_)) | None => continue,
        }
    }
    None
}

/// `getBoolean`: `getBooleanObject` with the class's default.
pub(crate) fn get_bool_default(evidence: &Evidence, default: bool, keys: &[&str]) -> bool {
    get_bool(evidence, keys).unwrap_or(default)
}

/// `getStringList`: the first present list, a present string as a singleton, else empty.
pub(crate) fn get_string_list(evidence: &Evidence, keys: &[&str]) -> Vec<String> {
    for key in keys {
        match evidence.get(*key) {
            Some(EvidenceValue::List(items)) => return items.clone(),
            Some(EvidenceValue::Text(text)) => return vec![text.clone()],
            Some(EvidenceValue::Null) | None => continue,
        }
    }
    Vec::new()
}

/// `containsIgnoreCase`.
pub(crate) fn contains_ignore_case(text: &str, token: &str) -> bool {
    text.to_lowercase().contains(&token.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(rows: &[(&str, EvidenceValue)]) -> Evidence {
        rows.iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn the_first_present_key_wins_and_an_unparseable_value_does_not_fall_through() {
        let map = evidence(&[
            ("a", EvidenceValue::Text("nope".into())),
            ("b", EvidenceValue::Text("2.5".into())),
        ]);
        assert_eq!(get_double(&map, 7.0, &["a", "b"]), 7.0);
    }

    #[test]
    fn a_present_null_key_is_present_but_not_numeric() {
        let map = evidence(&[
            ("a", EvidenceValue::Null),
            ("b", EvidenceValue::Text("2.5".into())),
        ]);
        assert!(has_any(&map, &["a"]));
        assert_eq!(get_double(&map, 7.0, &["a", "b"]), 2.5);
        assert_eq!(get_bool(&map, &["a", "missing"]), None);
    }

    #[test]
    fn a_string_list_reads_as_a_singleton_and_a_list_as_itself() {
        let map = evidence(&[
            ("a", EvidenceValue::Text("S-001".into())),
            (
                "b",
                EvidenceValue::List(vec!["P-002".into(), "ISO 13702".into()]),
            ),
        ]);
        assert_eq!(get_string_list(&map, &["a"]), vec!["S-001"]);
        assert_eq!(
            get_string_list(&map, &["missing", "b"]),
            vec!["P-002", "ISO 13702"]
        );
        assert!(get_string_list(&map, &["missing"]).is_empty());
    }

    #[test]
    fn the_boolean_tokens_are_the_classes() {
        assert_eq!(parse_bool(" YES "), Some(true));
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool("N"), Some(false));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("maybe"), None);
    }
}
