use crate::model::{FileKind, StructuredField};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

const SENSITIVE_TERMS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "token",
    "secret",
    "api_key",
    "apikey",
    "authorization",
    "auth_token",
    "access_token",
    "refresh_token",
    "cookie",
    "session_id",
    "sessionid",
];

pub fn classify(path: &Path, bytes: &[u8]) -> FileKind {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "json" => FileKind::Json,
        "ini" | "cfg" | "conf" | "properties" => FileKind::KeyValue,
        "txt" | "xml" | "yaml" | "yml" | "toml" | "log" => FileKind::Text,
        _ if std::str::from_utf8(bytes).is_ok() => FileKind::Text,
        _ => FileKind::Binary,
    }
}

pub fn parse_structured(
    kind: &FileKind,
    bytes: &[u8],
) -> Result<BTreeMap<String, StructuredField>, String> {
    match kind {
        FileKind::Json => parse_json(bytes),
        FileKind::KeyValue => parse_key_value(bytes),
        FileKind::Text | FileKind::Binary => Ok(BTreeMap::new()),
    }
}

pub fn is_sensitive_key(key: &str) -> bool {
    let normalized = normalize_sensitive_key(key);

    SENSITIVE_TERMS.iter().any(|term| {
        normalized == *term
            || normalized.starts_with(&format!("{term}_"))
            || normalized.ends_with(&format!("_{term}"))
            || normalized.contains(&format!("_{term}_"))
    })
}

fn normalize_sensitive_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    let mut normalized = String::with_capacity(chars.len() + 4);

    for (index, ch) in chars.iter().copied().enumerate() {
        if !ch.is_ascii_alphanumeric() {
            if !normalized.is_empty() && !normalized.ends_with('_') {
                normalized.push('_');
            }
            continue;
        }

        let previous = index
            .checked_sub(1)
            .and_then(|value| chars.get(value))
            .copied();
        let next = chars.get(index + 1).copied();

        let camel_boundary = ch.is_ascii_uppercase()
            && previous.is_some_and(|value| {
                value.is_ascii_lowercase()
                    || value.is_ascii_digit()
                    || (value.is_ascii_uppercase()
                        && next.is_some_and(|next_value| next_value.is_ascii_lowercase()))
            });

        if camel_boundary && !normalized.is_empty() && !normalized.ends_with('_') {
            normalized.push('_');
        }

        normalized.push(ch.to_ascii_lowercase());
    }

    normalized.trim_matches('_').to_owned()
}

fn parse_json(bytes: &[u8]) -> Result<BTreeMap<String, StructuredField>, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid json: {error}"))?;

    let mut fields = BTreeMap::new();
    flatten_json("", &value, &mut fields);
    Ok(fields)
}

fn flatten_json(prefix: &str, value: &Value, fields: &mut BTreeMap<String, StructuredField>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                let next = format!("{prefix}/{escaped}");
                flatten_json(&next, child, fields);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let next = format!("{prefix}/{index}");
                flatten_json(&next, child, fields);
            }
        }
        _ => {
            let key = if prefix.is_empty() { "/" } else { prefix };
            let raw = scalar_display(value);
            fields.insert(key.to_owned(), field_for(key, &raw));
        }
    }
}

fn parse_key_value(bytes: &[u8]) -> Result<BTreeMap<String, StructuredField>, String> {
    let text = std::str::from_utf8(bytes).map_err(|error| format!("not utf-8: {error}"))?;
    let mut section = String::new();
    let mut fields = BTreeMap::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();

        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(';')
            || line.starts_with("//")
        {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') && line.len() > 2 {
            section = line[1..line.len() - 1].trim().to_owned();
            continue;
        }

        let separator = line.find('=').or_else(|| line.find(':'));

        let Some(index) = separator else {
            continue;
        };

        let key = line[..index].trim();
        let value = line[index + 1..].trim();

        if key.is_empty() {
            continue;
        }

        let full_key = if section.is_empty() {
            key.to_owned()
        } else {
            format!("{section}.{key}")
        };

        fields.insert(full_key.clone(), field_for(&full_key, value));
    }

    Ok(fields)
}

fn field_for(key: &str, raw: &str) -> StructuredField {
    let sensitive = key
        .split(['/', '.', '[', ']'])
        .filter(|segment| !segment.is_empty())
        .any(is_sensitive_key);

    StructuredField {
        display: if sensitive {
            "<redacted>".to_owned()
        } else {
            raw.to_owned()
        },
        value_sha256: sha256_text(raw),
        sensitive,
    }
}

fn scalar_display(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => unreachable!("containers are flattened first"),
    }
}

fn sha256_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_flattened_and_sensitive_value_is_redacted() {
        let fields = parse_json(
            br#"{"graphics":{"hdr":true,"width":3840},"account":{"access_token":"abc123"}}"#,
        )
        .unwrap();

        assert_eq!(fields["/graphics/hdr"].display, "true");
        assert_eq!(fields["/graphics/width"].display, "3840");
        assert_eq!(fields["/account/access_token"].display, "<redacted>");
        assert!(fields["/account/access_token"].sensitive);
        assert_ne!(fields["/account/access_token"].value_sha256, "abc123");
    }

    #[test]
    fn ini_sections_are_preserved() {
        let fields = parse_key_value(
            b"; comment\n[Video]\nWidth=3840\nHeight:2160\n[Account]\npassword=hunter2\n",
        )
        .unwrap();

        assert_eq!(fields["Video.Width"].display, "3840");
        assert_eq!(fields["Video.Height"].display, "2160");
        assert_eq!(fields["Account.password"].display, "<redacted>");
        assert!(fields["Account.password"].sensitive);
    }

    #[test]
    fn ordinary_key_names_are_not_over_redacted() {
        assert!(!is_sensitive_key("keybind_forward"));
        assert!(!is_sensitive_key("keyboard_layout"));
        assert!(!is_sensitive_key("tokenizer_model"));
        assert!(is_sensitive_key("api_key"));
        assert!(is_sensitive_key("user_password"));
    }

    #[test]
    fn camel_and_pascal_case_secret_keys_are_redacted() {
        assert!(is_sensitive_key("apiToken"));
        assert!(is_sensitive_key("accessToken"));
        assert!(is_sensitive_key("refreshToken"));
        assert!(is_sensitive_key("clientSecret"));
        assert!(is_sensitive_key("sessionId"));
        assert!(is_sensitive_key("APIKey"));
    }

    #[test]
    fn classifier_recognizes_common_config_types() {
        assert_eq!(
            classify(Path::new("settings.json"), br#"{"x":1}"#),
            FileKind::Json
        );
        assert_eq!(
            classify(Path::new("settings.ini"), b"x=1"),
            FileKind::KeyValue
        );
        assert_eq!(classify(Path::new("notes.txt"), b"hello"), FileKind::Text);
    }
}
