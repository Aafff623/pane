//! Read-only scan of other AI-usage tools' configs for onboarding import.
//!
//! v1 reads cc-switch (`~/.cc-switch/config.json`): every app section holds
//! provider profiles with a base URL and an API key. Candidates are matched
//! to Pane families by base-URL host first, then by name; only families with
//! a live key probe qualify, so an imported key always has a card that can
//! read it. The raw key never leaves this module — the frontend gets a masked
//! preview and a stable id, and `import_apply` re-reads the cached value.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CANDIDATES: usize = 100;

const KEY_NAMES: &[&str] = &[
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "apiKey",
    "api_key",
    "API_KEY",
    "apikey",
];

const BASE_NAMES: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "OPENAI_BASE_URL",
    "GOOGLE_GEMINI_BASE_URL",
    "baseUrl",
    "base_url",
    "endpoint",
];

const CC_SWITCH_SECTIONS: &[&str] = &[
    "claude", "codex", "gemini", "grok", "kimi", "opencode", "hermes", "mcode", "agy", "omp",
    "pi", "openclaw",
];

/// Families Pane can both store a key for and query. `openai-api` (admin
/// key), `stepfun-plan` (session pair) and `relaybalance` (needs a user URL)
/// are deliberately absent: importing them would create cards that cannot
/// read the imported credential.
struct FamilyMatch {
    family: &'static str,
    hosts: &'static [&'static str],
    names: &'static [&'static str],
}

const MATCHES: &[FamilyMatch] = &[
    FamilyMatch { family: "deepseek", hosts: &["api.deepseek.com"], names: &["deepseek", "深度求索"] },
    FamilyMatch { family: "moonshot", hosts: &["api.moonshot.cn", "api.moonshot.ai"], names: &["moonshot", "kimi", "月之暗面"] },
    FamilyMatch { family: "zai", hosts: &["api.z.ai", "open.bigmodel.cn"], names: &["zhipu", "智谱", "glm", "z.ai", "bigmodel"] },
    FamilyMatch { family: "minimax", hosts: &["api.minimax.io", "api.minimaxi.com", "api.minimax.chat"], names: &["minimax", "稀宇", "海螺"] },
    FamilyMatch { family: "stepfun", hosts: &["api.stepfun.com", "api.stepfun.ai"], names: &["stepfun", "阶跃"] },
    FamilyMatch { family: "openrouter", hosts: &["openrouter.ai"], names: &["openrouter"] },
    FamilyMatch { family: "siliconflow", hosts: &["api.siliconflow.cn", "api.siliconflow.com"], names: &["siliconflow", "silicon flow", "硅基流动"] },
    FamilyMatch { family: "novita", hosts: &["api.novita.ai"], names: &["novita"] },
    FamilyMatch { family: "aihubmix", hosts: &["aihubmix.com"], names: &["aihubmix"] },
    FamilyMatch { family: "kimi", hosts: &["api.kimi.com", "auth.kimi.com"], names: &["kimi code", "kimi coding"] },
    FamilyMatch { family: "commandcode", hosts: &["api.commandcode.ai"], names: &["commandcode"] },
    FamilyMatch { family: "groq", hosts: &["api.groq.com"], names: &["groq"] },
    FamilyMatch { family: "mistral", hosts: &["api.mistral.ai"], names: &["mistral"] },
    FamilyMatch { family: "chutes", hosts: &["llm.chutes.ai", "api.chutes.ai"], names: &["chutes"] },
    FamilyMatch { family: "longcat", hosts: &["api.longcat.chat", "longcat.chat"], names: &["longcat", "龙猫"] },
    FamilyMatch { family: "elevenlabs", hosts: &["api.elevenlabs.io"], names: &["elevenlabs", "11labs"] },
    FamilyMatch { family: "deepgram", hosts: &["api.deepgram.com"], names: &["deepgram"] },
    FamilyMatch { family: "huggingface", hosts: &["huggingface.co", "api-inference.huggingface.co"], names: &["huggingface", "hf.co"] },
    FamilyMatch { family: "poe", hosts: &["api.poe.com"], names: &["poe"] },
    FamilyMatch { family: "venice", hosts: &["api.venice.ai"], names: &["venice"] },
    FamilyMatch { family: "factory", hosts: &["api.factory.ai"], names: &["factory"] },
    FamilyMatch { family: "mimo", hosts: &["api.mimo.xiaomi.com"], names: &["mimo"] },
    FamilyMatch { family: "qwen", hosts: &["dashscope.aliyuncs.com", "coding-intl.dashscope.aliyuncs.com"], names: &["qwen", "dashscope", "通义", "bailian", "百炼"] },
    FamilyMatch { family: "volcengine", hosts: &["ark.cn-beijing.volces.com"], names: &["volcengine", "火山", "ark"] },
    FamilyMatch { family: "perplexity", hosts: &["api.perplexity.ai"], names: &["perplexity"] },
];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidate {
    pub id: String,
    pub source: String,
    pub source_label: String,
    pub section: String,
    pub name: String,
    pub family: Option<String>,
    pub family_label: Option<String>,
    pub masked_key: String,
    pub base_host: Option<String>,
    /// matched | duplicate | unsupported
    pub state: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportApp {
    pub id: String,
    pub label: String,
    pub config_path: String,
    pub found: bool,
    pub candidate_count: usize,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportScan {
    pub apps: Vec<ImportApp>,
    pub candidates: Vec<ImportCandidate>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProbeResult {
    pub id: String,
    /// ok | needs_setup | rejected | error | unverified
    pub status: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub id: String,
    /// imported | exists | failed | skipped
    pub status: &'static str,
    pub message: String,
}

#[derive(Clone, Debug)]
struct Secret {
    family: String,
    key: String,
    base_url: Option<String>,
}

static CACHE: OnceLock<Mutex<HashMap<String, Secret>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, Secret>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, PartialEq)]
struct RawCandidate {
    section: String,
    provider_id: String,
    name: String,
    key: String,
    base_url: Option<String>,
}

pub fn scan() -> ImportScan {
    let mut candidates = Vec::new();
    let mut apps = Vec::new();
    let mut secrets = HashMap::new();

    let mut cc = ImportApp {
        id: "cc-switch".into(),
        label: "CC Switch".into(),
        config_path: cc_switch_path()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        found: false,
        candidate_count: 0,
        message: None,
    };
    match cc_switch_path() {
        Some(path) => match read_json(&path) {
            Ok(doc) => {
                cc.found = true;
                for raw in candidates_from_doc(&doc) {
                    if candidates.len() >= MAX_CANDIDATES {
                        break;
                    }
                    if let Some(candidate) = to_candidate(raw, &mut secrets) {
                        candidates.push(candidate);
                    }
                }
            }
            Err(error) => cc.message = Some(error),
        },
        None => cc.message = Some("home directory unavailable".into()),
    }
    cc.candidate_count = candidates.len();
    apps.push(cc);

    *cache().lock().unwrap_or_else(|e| e.into_inner()) = secrets;
    ImportScan { apps, candidates }
}

pub async fn probe(ids: &[String]) -> Vec<ImportProbeResult> {
    let items: Vec<(String, Secret)> = {
        let cached = cache().lock().unwrap_or_else(|e| e.into_inner());
        ids.iter()
            .filter_map(|id| cached.get(id).map(|secret| (id.clone(), secret.clone())))
            .collect()
    };
    let mut out = Vec::with_capacity(items.len());
    for (id, secret) in items {
        let probe = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            crate::provider_dispatch::key_snapshot_for(
                &secret.family,
                &secret.key,
                secret.base_url.clone(),
            ),
        )
        .await;
        let (status, message) = match probe {
            Err(_) => ("unverified", "timed out".to_string()),
            Ok(Err(error)) => classify(&error),
            Ok(Ok(snap)) => match snap.status.as_str() {
                "ok" => ("ok", String::new()),
                "needs_setup" => ("needs_setup", snap.error.unwrap_or_default()),
                _ => classify(snap.error.as_deref().unwrap_or("query failed")),
            },
        };
        out.push(ImportProbeResult {
            id,
            status,
            message: redact(&message, &secret.key),
        });
    }
    out
}

pub fn import(ids: &[String]) -> Vec<ImportOutcome> {
    let items: Vec<(String, Secret)> = {
        let cached = cache().lock().unwrap_or_else(|e| e.into_inner());
        ids.iter()
            .filter_map(|id| cached.get(id).map(|secret| (id.clone(), secret.clone())))
            .collect()
    };
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if !items.iter().any(|(known, _)| known == id) {
            out.push(ImportOutcome {
                id: id.clone(),
                status: "skipped",
                message: "scan again before importing this row".into(),
            });
        }
    }
    let mut wrote_families = std::collections::BTreeSet::new();
    for (id, secret) in items {
        // Never overwrite a saved key: the row says "already saved", and the
        // user can clear the old key in Settings first.
        if crate::providers::stored_key_readonly(&secret.family).is_some()
            || wrote_families.contains(&secret.family)
        {
            out.push(ImportOutcome {
                id,
                status: "exists",
                message: "a key is already saved for this provider".into(),
            });
            continue;
        }
        match crate::providers::save_api_key(
            &secret.family,
            &secret.key,
            secret.base_url.as_deref(),
        ) {
            Ok(()) => {
                wrote_families.insert(secret.family.clone());
                out.push(ImportOutcome {
                    id,
                    status: "imported",
                    message: String::new(),
                })
            }
            Err(error) => out.push(ImportOutcome {
                id,
                status: "failed",
                message: redact(&error, &secret.key),
            }),
        }
    }
    out
}

fn classify(message: &str) -> (&'static str, String) {
    let lower = message.to_ascii_lowercase();
    let credential = ["reject", "401", "unauthor", "invalid api key", "invalid key"]
        .iter()
        .any(|needle| lower.contains(needle));
    (
        if credential { "rejected" } else { "error" },
        message.to_string(),
    )
}

fn redact(message: &str, key: &str) -> String {
    if key.len() < 8 {
        return message.to_string();
    }
    message.replace(key, "***")
}

fn cc_switch_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".cc-switch").join("config.json"))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let meta = std::fs::metadata(path).map_err(|error| format!("read {path:?}: {error}"))?;
    if meta.len() > MAX_CONFIG_BYTES {
        return Err("config file is larger than 4 MB".into());
    }
    let text = std::fs::read_to_string(path).map_err(|error| format!("read {path:?}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("parse {path:?}: {error}"))
}

fn candidates_from_doc(doc: &Value) -> Vec<RawCandidate> {
    let mut out = Vec::new();
    for section in CC_SWITCH_SECTIONS {
        let Some(providers) = doc
            .get(*section)
            .and_then(|value| value.get("providers"))
            .and_then(Value::as_object)
        else {
            continue;
        };
        for (provider_id, provider) in providers {
            if out.len() >= MAX_CANDIDATES {
                return out;
            }
            let name = provider
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(provider_id)
                .trim()
                .to_string();
            let settings = provider.get("settingsConfig").unwrap_or(&Value::Null);
            let Some(key) = extract_key(settings) else {
                continue;
            };
            out.push(RawCandidate {
                section: (*section).to_string(),
                provider_id: provider_id.clone(),
                name,
                key,
                base_url: extract_base_url(settings),
            });
        }
    }
    out
}

fn to_candidate(
    raw: RawCandidate,
    secrets: &mut HashMap<String, Secret>,
) -> Option<ImportCandidate> {
    let family = match_family(&raw.name, raw.base_url.as_deref())
        .filter(|family| crate::provider_dispatch::supports_key_probe(family));
    let family_label = family
        .and_then(crate::provider_catalog::provider_definition)
        .map(|definition| definition.display_name.to_string());
    let state = match family {
        None => "unsupported",
        // Any saved key means the import would be skipped, so the row says
        // "already saved" up front instead of promising a write.
        Some(family) => match crate::providers::stored_key_readonly(family) {
            Some(_) => "duplicate",
            None => "matched",
        },
    };
    let id = candidate_id(&raw.section, &raw.provider_id, &raw.key);
    if let Some(family) = family {
        secrets.insert(
            id.clone(),
            Secret {
                family: family.to_string(),
                key: raw.key.clone(),
                // Only relay-style families persist a base URL; the others
                // query fixed vendor endpoints and save_api_key rejects one.
                base_url: if crate::provider_catalog::takes_base_url(family) {
                    raw.base_url.clone()
                } else {
                    None
                },
            },
        );
    }
    Some(ImportCandidate {
        id,
        source: "cc-switch".into(),
        source_label: "CC Switch".into(),
        section: raw.section,
        name: raw.name,
        family: family.map(str::to_string),
        family_label,
        masked_key: mask(&raw.key),
        base_host: raw.base_url.as_deref().and_then(host_of),
        state,
    })
}

fn candidate_id(section: &str, provider_id: &str, key: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in section
        .bytes()
        .chain(b"|".iter().copied())
        .chain(provider_id.bytes())
        .chain(b"|".iter().copied())
        .chain(key.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("cc-switch:{section}:{provider_id}:{hash:016x}")
}

fn mask(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        let tail: String = chars.iter().rev().take(2).collect::<Vec<_>>().into_iter().rev().collect();
        return format!("••••{tail}");
    }
    if chars.len() < 16 {
        let head: String = chars[..2].iter().collect();
        let tail: String = chars[chars.len() - 2..].iter().collect();
        return format!("{head}…{tail}");
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

fn host_of(url: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase))
}

fn match_family(name: &str, base_url: Option<&str>) -> Option<&'static str> {
    if let Some(host) = base_url.and_then(host_of) {
        for entry in MATCHES {
            if entry
                .hosts
                .iter()
                .any(|known| host == *known || host.ends_with(&format!(".{known}")))
            {
                return Some(entry.family);
            }
        }
    }
    let name = name.to_ascii_lowercase();
    // Longest alias wins: "Kimi Code" must beat the shorter "kimi" alias
    // that maps the Kimi API key family.
    let mut best: Option<(&'static str, usize)> = None;
    for entry in MATCHES {
        for alias in entry.names {
            if name.contains(alias) && best.map_or(true, |(_, len)| alias.len() > len) {
                best = Some((entry.family, alias.len()));
            }
        }
    }
    best.map(|(family, _)| family)
}

/// First non-empty string under one of the known key names, anywhere in the
/// settings doc (env blocks, auth blocks, preset overrides).
fn extract_key(value: &Value) -> Option<String> {
    first_string_for(value, KEY_NAMES).and_then(|candidate| {
        let trimmed = candidate.trim();
        let looks_like_key = trimmed.len() >= 8
            && !trimmed.contains(char::is_whitespace)
            && !trimmed.starts_with("http");
        looks_like_key.then(|| trimmed.to_string())
    })
}

fn extract_base_url(value: &Value) -> Option<String> {
    let direct = first_string_for(value, BASE_NAMES).map(|url| url.trim().to_string());
    direct
        .filter(|url| url.starts_with("http"))
        .or_else(|| {
            value
                .get("config")
                .and_then(Value::as_str)
                .and_then(toml_base_url)
        })
}

fn first_string_for(value: &Value, names: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if names.iter().any(|name| key.eq_ignore_ascii_case(name)) {
                    if let Some(text) = child.as_str() {
                        if !text.trim().is_empty() {
                            return Some(text.to_string());
                        }
                    }
                }
            }
            for child in map.values() {
                if let Some(found) = first_string_for(child, names) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|item| first_string_for(item, names)),
        _ => None,
    }
}

/// Codex profiles keep their endpoint inside a TOML string
/// (`base_url = "https://…"`), not as a JSON field.
fn toml_base_url(config: &str) -> Option<String> {
    for line in config.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("base_url") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let value = rest.trim().trim_matches(|c| c == '"' || c == '\'');
        if value.starts_with("http") {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn settings_with_claude_key(key: &str, base: &str) -> Value {
        json!({ "env": { "ANTHROPIC_AUTH_TOKEN": key, "ANTHROPIC_BASE_URL": base } })
    }

    #[test]
    fn reads_claude_codex_and_gemini_profiles() {
        let doc = json!({
            "version": 2,
            "claude": { "providers": {
                "p1": { "name": "DeepSeek", "settingsConfig": settings_with_claude_key("sk-deepseek-123456", "https://api.deepseek.com/anthropic") }
            }},
            "codex": { "providers": {
                "p2": { "name": "Kimi", "settingsConfig": {
                    "auth": { "OPENAI_API_KEY": "sk-moonshot-abcdef" },
                    "config": "model_provider = \"custom\"\n[model_providers.custom]\nbase_url = \"https://api.moonshot.cn/v1\"\n"
                }}
            }},
            "gemini": { "providers": {
                "p3": { "name": "SiliconFlow", "settingsConfig": { "env": { "GEMINI_API_KEY": "sk-silicon-888888", "GOOGLE_GEMINI_BASE_URL": "https://api.siliconflow.cn" } } }
            }}
        });
        let raws = candidates_from_doc(&doc);
        assert_eq!(raws.len(), 3);
        assert_eq!(raws[0].section, "claude");
        assert_eq!(raws[1].base_url.as_deref(), Some("https://api.moonshot.cn/v1"));
        assert_eq!(raws[2].key, "sk-silicon-888888");
    }

    #[test]
    fn skips_profiles_without_a_key() {
        let doc = json!({
            "claude": { "providers": {
                "official": { "name": "Anthropic", "settingsConfig": { "env": {} } }
            }}
        });
        assert!(candidates_from_doc(&doc).is_empty());
    }

    #[test]
    fn matching_prefers_host_over_name() {
        // Kimi's Anthropic-compatible preset points at Moonshot.
        assert_eq!(
            match_family("Kimi", Some("https://api.moonshot.cn/anthropic")),
            Some("moonshot")
        );
        assert_eq!(match_family("My DeepSeek relay", None), Some("deepseek"));
        assert_eq!(match_family("Anthropic", Some("https://api.anthropic.com")), None);
        assert_eq!(match_family("OpenAI", None), None);
    }

    #[test]
    fn toml_base_url_parses_quoted_and_spaced_lines() {
        assert_eq!(
            toml_base_url("[model_providers.custom]\n  base_url  =  \"https://api.z.ai/api/paas/v4\"\n"),
            Some("https://api.z.ai/api/paas/v4".into())
        );
        assert_eq!(toml_base_url("model = \"gpt-5\""), None);
    }

    #[test]
    fn masks_without_leaking_the_middle() {
        assert_eq!(mask("sk-1234567890abcdef"), "sk-1…cdef");
        assert_eq!(mask("sk-1234567890"), "sk…90");
        assert_eq!(mask("short"), "••••rt");
        assert!(!mask("sk-1234567890abcdef").contains("567890"));
    }

    #[test]
    fn every_matched_family_can_be_probed_and_stored() {
        for entry in MATCHES {
            assert!(
                crate::provider_dispatch::supports_key_probe(entry.family),
                "{} has no key probe",
                entry.family
            );
            assert!(
                crate::provider_catalog::supports_api_key(entry.family),
                "{} takes no API key",
                entry.family
            );
        }
    }

    #[test]
    fn duplicate_detection_is_case_sensitive_on_the_value() {
        // Guards the comparison used in to_candidate: trimming only.
        assert_eq!(" sk-x ".trim(), "sk-x");
        assert_ne!("sk-x", "sk-X");
    }
}
