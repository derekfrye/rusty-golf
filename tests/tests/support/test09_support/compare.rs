use anyhow::{Result, anyhow};
use serde_json::Value;
use std::collections::BTreeSet;

pub(crate) fn assert_values_match(expected: &Value, actual: &Value, path: &str) -> Result<()> {
    match (expected, actual) {
        (Value::Object(expected_map), Value::Object(actual_map)) => {
            let expected_keys: BTreeSet<&str> = expected_map.keys().map(String::as_str).collect();
            let actual_keys: BTreeSet<&str> = actual_map.keys().map(String::as_str).collect();
            if expected_keys != actual_keys {
                return Err(anyhow!(
                    "object keys mismatch at {path}: expected {expected_keys:?}, got {actual_keys:?}"
                ));
            }
            for key in expected_map.keys() {
                assert_values_match(
                    &expected_map[key],
                    &actual_map[key],
                    &format!("{path}.{key}"),
                )?;
            }
            Ok(())
        }
        (Value::Array(expected_arr), Value::Array(actual_arr)) => {
            if expected_arr.len() != actual_arr.len() {
                return Err(anyhow!(
                    "array length mismatch at {path}: expected {}, got {}",
                    expected_arr.len(),
                    actual_arr.len()
                ));
            }
            let mut expected_items: Vec<String> =
                expected_arr.iter().map(canonicalize_value).collect();
            let mut actual_items: Vec<String> = actual_arr.iter().map(canonicalize_value).collect();
            expected_items.sort();
            actual_items.sort();
            if expected_items == actual_items {
                Ok(())
            } else {
                Err(anyhow!("array values mismatch at {path}"))
            }
        }
        _ if expected == actual => Ok(()),
        _ => Err(anyhow!(
            "value mismatch at {path}: expected {expected}, got {actual}"
        )),
    }
}

fn canonicalize_value(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value).unwrap_or_default()
        }
        Value::Array(items) => {
            let mut normalized: Vec<String> = items.iter().map(canonicalize_value).collect();
            normalized.sort();
            format!("[{}]", normalized.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap_or_default(),
                        canonicalize_value(map.get(key).unwrap_or(&Value::Null))
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}
