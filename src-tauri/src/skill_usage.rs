//! Claude Skill usage, a read-only side ledger (#71). It never touches the
//! spend parser or its cache: one Skill tool_use adds one call, the row's
//! token buckets, and the row's own `costUSD` when present. Arguments and
//! prompts are never stored.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const VERSION: u32 = 1;
const CACHE_TTL_MS: i64 = 5 * 60_000;
const WINDOW_DAYS: u64 = 31;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRow {
    pub skill: String,
    pub calls: u64,
    pub tokens: f64,
    pub cost: f64,
    pub last_used_ms: i64,
}

#[derive(Serialize, Deserialize, Default)]
struct Cache {
    version: u32,
    scanned_at_ms: i64,
    rows: Vec<SkillRow>,
}

fn cache_path() -> PathBuf {
    crate::providers::config_dir().join("skill_usage_cache.json")
}

fn load_cache() -> Cache {
    fs::read_to_string(cache_path())
        .ok()
        .and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())
        .filter(|cache: &Cache| cache.version == VERSION)
        .unwrap_or_else(|| Cache {
            version: VERSION,
            scanned_at_ms: 0,
            rows: Vec::new(),
        })
}

fn save_cache(cache: &Cache) {
    let Ok(raw) = serde_json::to_string(cache) else { return };
    let temp = cache_path().with_extension("json.tmp");
    if fs::write(&temp, raw).is_ok() {
        let _ = fs::rename(&temp, cache_path());
    }
}

fn timestamp_ms(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::String(text) => chrono::DateTime::parse_from_rfc3339(text)
            .ok()
            .map(|date| date.timestamp_millis()),
        Value::Number(number) => number.as_i64(),
        _ => None,
    }
}

fn line_tokens(line: &Value) -> f64 {
    let usage = line.get("usage");
    ["input_tokens", "output_tokens", "cache_read_input_tokens", "cache_creation_input_tokens"]
        .iter()
        .filter_map(|key| usage.and_then(|usage| usage.get(key)).and_then(Value::as_f64))
        .sum()
}

/// One assistant line → (skill, tokens, cost, ts). A line may contain several
/// tool_use blocks; only `Skill` entries count. The cost is the request that
/// carried the call, not follow-up turns opened by the skill.
fn line_skills(line: &Value) -> Vec<(String, f64, f64)> {
    let Some(content) = line.pointer("/message/content").and_then(Value::as_array) else {
        return Vec::new();
    };
    let tokens = line_tokens(line);
    let cost = line.get("costUSD").and_then(Value::as_f64).unwrap_or(0.0);
    content
        .iter()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
        .filter(|block| block.get("name").and_then(Value::as_str) == Some("Skill"))
        .filter_map(|block| {
            let skill = block
                .pointer("/input/skill")
                .or_else(|| block.pointer("/input/name"))
                .and_then(Value::as_str)?
                .trim();
            (!skill.is_empty()).then(|| (skill.to_string(), tokens, cost))
        })
        .collect()
}

fn scan_file(path: &Path, rows: &mut HashMap<String, SkillRow>, seen: &mut HashSet<String>) {
    let Ok(text) = fs::read_to_string(path) else { return };
    for (index, line) in text.lines().enumerate() {
        if !line.contains("\"Skill\"") {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else { continue };
        let Some(ts) = timestamp_ms(value.get("timestamp")) else { continue };
        let request = value
            .get("requestId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let message = value
            .pointer("/message/id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let dedupe = format!("{message}|{request}|{}:{index}", path.display());
        for (skill, tokens, cost) in line_skills(&value) {
            if !seen.insert(format!("{skill}|{dedupe}")) {
                continue;
            }
            let row = rows.entry(skill.clone()).or_insert(SkillRow {
                skill,
                calls: 0,
                tokens: 0.0,
                cost: 0.0,
                last_used_ms: 0,
            });
            row.calls += 1;
            row.tokens += tokens;
            row.cost += cost;
            row.last_used_ms = row.last_used_ms.max(ts);
        }
    }
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else { return };
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(WINDOW_DAYS * 86_400))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            let recent = fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .map(|modified| modified >= cutoff)
                .unwrap_or(false);
            if recent {
                files.push(path);
            }
        }
    }
}

pub fn collect(force: bool) -> Vec<SkillRow> {
    let cached = load_cache();
    let now = chrono::Utc::now().timestamp_millis();
    if !force && now - cached.scanned_at_ms < CACHE_TTL_MS {
        return cached.rows;
    }
    let mut files = Vec::new();
    if let Some(home) = dirs::home_dir() {
        collect_files(&home.join(".claude").join("projects"), &mut files);
    }
    let mut rows = HashMap::new();
    let mut seen = HashSet::new();
    for file in files {
        scan_file(&file, &mut rows, &mut seen);
    }
    let mut rows: Vec<SkillRow> = rows.into_values().collect();
    rows.sort_by(|a, b| {
        b.cost
            .partial_cmp(&a.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.calls.cmp(&a.calls))
    });
    let cache = Cache { version: VERSION, scanned_at_ms: now, rows: rows.clone() };
    save_cache(&cache);
    rows
}

#[cfg(test)]
mod tests {
    use super::line_skills;
    use serde_json::json;

    #[test]
    fn only_skill_tool_use_counts_and_args_are_never_returned() {
        let line = json!({
            "timestamp": "2026-10-26T10:00:00Z",
            "usage": {"input_tokens": 10, "output_tokens": 5, "cache_read_input_tokens": 3},
            "costUSD": 0.25,
            "message": {"id": "m1", "content": [
                {"type": "tool_use", "name": "Skill", "input": {"skill": "superpowers:writing-plans", "args": "SECRET"}},
                {"type": "tool_use", "name": "Bash", "input": {"command": "SECRET"}}
            ]}
        });
        let rows = line_skills(&line);
        assert_eq!(rows, vec![("superpowers:writing-plans".to_string(), 18.0, 0.25)]);
        assert!(!format!("{rows:?}").contains("SECRET"));
    }

    #[test]
    fn a_line_without_skill_input_is_ignored() {
        let line = json!({"message": {"content": [
            {"type": "tool_use", "name": "Skill", "input": {}}
        ]}});
        assert!(line_skills(&line).is_empty());
    }
}
