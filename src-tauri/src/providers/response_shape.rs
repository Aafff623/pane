//! Opt-in console response-shape capture, for calibrating parsers against a
//! vendor envelope that has no published schema (#38 Spark-X2.6, #47 speech
//! packs). The only way to align a parser with the real answer is to look at
//! one real answer — this records its *shape* locally so calibration never
//! needs a cookie, a token or a person's name pasted into a chat.
//!
//! Off by default: set `PANE_CONSOLE_PROBE=1` in the environment before
//! launching Pane. Each provider owns its own capped file under the app
//! config directory. Nothing here is ever transmitted — this app has no
//! telemetry.

use serde_json::{Map, Value};
use std::path::Path;

const MAX_ENTRIES: usize = 40;
const MAX_DEPTH: usize = 5;
/// A value longer than this is a token/URL-shaped string, not a quota
/// reading; only its type is recorded even when the key is allowlisted.
const MAX_SCALAR_CHARS: usize = 40;

pub fn enabled() -> bool {
    std::env::var("PANE_CONSOLE_PROBE")
        .map(|raw| {
            let v = raw.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        })
        .unwrap_or(false)
}

/// Reduce a response to its shape. `allow` names the keys whose *string* value
/// is quota semantics rather than identity — status enums and date formats are
/// the whole point of calibrating against an undocumented envelope. Every
/// other scalar becomes a type name, so a profile endpoint carrying a name, an
/// ID-card number or a phone contributes no PII to the log.
pub fn skeleton(value: &Value, allow: &[&str], depth: usize) -> Value {
    if depth > MAX_DEPTH {
        return Value::String("…".into());
    }
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (key, child) in map {
                let allowed = allow.iter().any(|a| a.eq_ignore_ascii_case(key));
                out.insert(
                    key.clone(),
                    if allowed {
                        allowed_text(child)
                    } else {
                        skeleton(child, allow, depth + 1)
                    },
                );
            }
            Value::Object(out)
        }
        // Only the first element: a 3000-row usage page must not become a
        // 3000-entry log record.
        Value::Array(items) => {
            let mut out = Map::new();
            out.insert("[]".into(), Value::Number(items.len().into()));
            if let Some(first) = items.first() {
                out.insert("0".into(), skeleton(first, allow, depth + 1));
            }
            Value::Object(out)
        }
        other => Value::String(type_name(other).into()),
    }
}

/// An allowlisted value: short strings verbatim, everything else as a type.
/// Numbers and booleans are typed rather than copied because calibration needs
/// the field's presence, name and type — not the user's own quota figure.
fn allowed_text(value: &Value) -> Value {
    match value {
        Value::String(text) if text.chars().count() > MAX_SCALAR_CHARS => {
            Value::String("str(long)".into())
        }
        Value::String(text) => Value::String(text.clone()),
        other => Value::String(type_name(other).into()),
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "str",
        Value::Number(_) => "num",
        Value::Bool(_) => "bool",
        Value::Null => "null",
        Value::Array(_) => "arr",
        Value::Object(_) => "obj",
    }
}

/// Append one record, keeping the file capped and atomically written (the
/// same discipline as `kimi_probe_log.json`). A corrupt log restarts fresh
/// instead of silently disabling calibration forever.
pub fn note(path: &Path, record: Value) {
    let mut log = serde_json::from_str::<Vec<Value>>(
        &std::fs::read_to_string(path).unwrap_or_else(|_| "[]".into()),
    )
    .unwrap_or_default();
    log.push(record);
    let excess = log.len().saturating_sub(MAX_ENTRIES);
    log.drain(0..excess);
    if let Ok(raw) = serde_json::to_string(&log) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, raw).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ALLOW: &[&str] = &["code", "flag", "status", "total", "left", "expiresAt"];

    #[test]
    fn identity_and_pii_keys_keep_only_their_type() {
        let doc = json!({
            "code": 0,
            "flag": true,
            "data": {
                "name": "张三",
                "mobile": "13800000000",
                "idCard": "110101199001011234",
                "total": 1000,
                "left": 250,
                "status": "EXHAUSTED",
                "expiresAt": "2026-12-31 23:59:59"
            }
        });
        let shape = skeleton(&doc, ALLOW, 0);
        let data = &shape["data"];
        assert_eq!(data["name"], "str", "a name is a type, never a value");
        assert_eq!(data["mobile"], "str");
        assert_eq!(data["idCard"], "str");
        assert_eq!(data["total"], "num", "allowlisted numbers stay numeric-typed");
        assert_eq!(data["status"], "EXHAUSTED", "status enums are the point of calibration");
        assert_eq!(data["expiresAt"], "2026-12-31 23:59:59", "date format must be visible");
        assert_eq!(shape["flag"], "bool");
    }

    #[test]
    fn long_values_and_deep_nesting_are_bounded() {
        let long = "a".repeat(120);
        let doc = json!({ "status": long, "a": {"b": {"c": {"d": {"e": {"f": {"g": 1}}}}}} });
        let shape = skeleton(&doc, ALLOW, 0);
        assert_eq!(shape["status"], "str(long)", "a token-shaped string is not copied");
        assert_eq!(shape["a"]["b"]["c"]["d"]["e"]["f"], "…", "depth is capped");
    }

    #[test]
    fn arrays_record_length_and_one_sample() {
        let doc = json!({ "data": { "data": [
            {"requestId": "r-1", "credit": 3, "input": "secret prompt text"},
            {"requestId": "r-2", "credit": 4}
        ]}});
        let shape = skeleton(&doc, ALLOW, 0);
        let rows = &shape["data"]["data"];
        assert_eq!(rows["[]"], 2);
        assert_eq!(rows["0"]["requestId"], "str", "row ids are identity, not quota");
        assert_eq!(rows["0"]["credit"], "num");
        assert!(rows.get("1").is_none(), "only the first row is recorded");
    }

    #[test]
    fn note_caps_the_log_and_survives_a_corrupt_file() {
        let dir = std::env::temp_dir().join(format!("pane-shape-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("probe.json");
        std::fs::write(&path, "{not json").unwrap();
        note(&path, json!({"n": 1}));
        let log: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(log.len(), 1, "a corrupt log restarts instead of growing");
        for i in 0..60 {
            note(&path, json!({"n": i}));
        }
        let log: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(log.len(), MAX_ENTRIES, "the log stays capped");
        assert_eq!(log[MAX_ENTRIES - 1]["n"], 59, "newest entries win");
        assert!(!dir.join("probe.json.tmp").exists(), "no temp file is left behind");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capture_is_off_unless_the_environment_asks_for_it() {
        // The default must be off: a shape log is a calibration tool, not a
        // background recorder. (Read-only assertion; the env var is process
        // wide, so this only pins the unset case.)
        std::env::remove_var("PANE_CONSOLE_PROBE");
        assert!(!enabled());
    }
}
