//! Unified API key vault — Pane as the single place keys live so tools
//! and projects never need re-configuring. Storage is the same
//! plaintext-on-local-disk model every Pane credential file uses:
//! `%APPDATA%/Pane/keyvault.json`. Local projects read that file (or the
//! user copies from Settings); nothing is exposed over HTTP.

use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultEntry {
    pub id: String,
    pub service: String,
    pub label: String,
    /// The raw secret. Never rendered by the UI — commands hand back a
    /// masked form for listing and the raw value only for clipboard copy.
    pub key: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub created_at: i64,
}

fn vault_path() -> PathBuf {
    crate::providers::config_dir().join("keyvault.json")
}

fn load() -> Vec<VaultEntry> {
    std::fs::read_to_string(vault_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(entries: &[VaultEntry]) -> Result<(), String> {
    let path = vault_path();
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?)
        .and_then(|()| std::fs::rename(&tmp, &path))
        .map_err(|e| e.to_string())
}

/// Stable id: service + short digest of the key — seeding twice never
/// duplicates an entry, and re-adding the same key is a no-op update.
fn entry_id(service: &str, key: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{}-{:016x}", service.to_lowercase().replace([' ', '/'], "-"), h)
}

/// `sk-abc…wxyz` — head and tail visible, middle hidden.
pub fn mask_key(key: &str) -> String {
    let k = key.trim();
    if k.len() <= 10 {
        return "*".repeat(k.len().max(3));
    }
    format!("{}…{}", &k[..6], &k[k.len() - 4..])
}

fn upsert(entries: &mut Vec<VaultEntry>, service: &str, label: &str, key: &str, note: &str) {
    let key = key.trim();
    if key.is_empty() {
        return;
    }
    let id = entry_id(service, key);
    if let Some(existing) = entries.iter_mut().find(|e| e.id == id) {
        existing.label = label.to_string();
        return;
    }
    entries.push(VaultEntry {
        id,
        service: service.trim().to_string(),
        label: label.trim().to_string(),
        key: key.to_string(),
        note: note.to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
    });
}

// ── seeding: one-time import of keys already living elsewhere ──────────────

fn read_provider_key(file: &str) -> Option<String> {
    let p = crate::providers::config_dir().join(file);
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
    v.get("apiKey")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn env_nonempty(var: &str) -> Option<String> {
    std::env::var(var).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// The active Firecrawl key: ZCode CLI's MCP config carries the literal
/// current key; env may still hold a legacy one.
fn firecrawl_seed_key() -> Option<String> {
    let cfg = dirs::home_dir()?.join(".zcode").join("cli").join("config.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(cfg).ok()?).ok()?;
    v.pointer("/mcp/servers/firecrawl/env/FIRECRAWL_API_KEY")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| env_nonempty("FIRECRAWL_FIRECRAWL_API_KEY"))
        .or_else(|| env_nonempty("FIRECRAWL_API_KEY"))
}

fn seed_entries(entries: &mut Vec<VaultEntry>) {
    // Tavily multi-key file
    let tavily_path = crate::providers::config_dir().join("tavily-keys.json");
    if let Ok(raw) = std::fs::read_to_string(&tavily_path) {
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            for (i, k) in v
                .get("keys")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .enumerate()
            {
                upsert(entries, "tavily", &format!("Tavily key {}", i + 1), k, "imported");
            }
        }
    }
    if let Some(k) = read_provider_key("apigoto.json") {
        upsert(entries, "apigoto", "APIGOTO (RouterCode Free)", &k, "imported");
    }
    if let Some(k) = read_provider_key("clinepass.json") {
        upsert(entries, "clinepass", "ClinePass", &k, "imported");
    }
    if let Some(k) = env_nonempty("BOCHA_API_KEY") {
        upsert(entries, "bocha", "BochaAI", &k, "imported from env");
    }
    if let Some(k) = firecrawl_seed_key() {
        upsert(entries, "firecrawl", "Firecrawl (active)", &k, "imported");
    }
}

// ── public API (called by thin #[tauri::command] wrappers in lib.rs) ─────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct VaultRow {
    pub id: String,
    pub service: String,
    pub label: String,
    pub masked: String,
    pub note: String,
}

pub fn list() -> Vec<VaultRow> {
    let mut entries = load();
    // Seed lazily on first listing; idempotent by construction.
    let before = entries.len();
    seed_entries(&mut entries);
    if entries.len() != before {
        let _ = save(&entries);
    }
    entries
        .into_iter()
        .map(|e| VaultRow {
            masked: mask_key(&e.key),
            id: e.id,
            service: e.service,
            label: e.label,
            note: e.note,
        })
        .collect()
}

pub fn add(service: &str, label: &str, key: &str, note: &str) -> Result<Vec<VaultRow>, String> {
    if service.trim().is_empty() || key.trim().is_empty() {
        return Err("service and key are required".into());
    }
    let mut entries = load();
    upsert(&mut entries, service, label, key, note);
    save(&entries)?;
    Ok(entries
        .into_iter()
        .map(|e| VaultRow { masked: mask_key(&e.key), id: e.id, service: e.service, label: e.label, note: e.note })
        .collect())
}

pub fn remove(id: &str) -> Result<Vec<VaultRow>, String> {
    let mut entries = load();
    let before = entries.len();
    entries.retain(|e| e.id != id);
    if entries.len() == before {
        return Err("no such key".into());
    }
    save(&entries)?;
    Ok(entries
        .into_iter()
        .map(|e| VaultRow { masked: mask_key(&e.key), id: e.id, service: e.service, label: e.label, note: e.note })
        .collect())
}

/// Raw key for the clipboard — the only path that un-masks, and only the
/// in-app frontend can call it.
pub fn copy(id: &str) -> Result<String, String> {
    load()
        .into_iter()
        .find(|e| e.id == id)
        .map(|e| e.key)
        .ok_or_else(|| "no such key".to_string())
}

/// (label, key) pairs for one service, in stored order — the read side for
/// quota providers. Multiple keys of the same service stay independent
/// pools; the label is the user-facing name shown on per-key rows.
pub fn keys_for_service(service: &str) -> Vec<(String, String)> {
    load()
        .into_iter()
        .filter(|e| e.service.eq_ignore_ascii_case(service))
        .map(|e| (e.label, e.key))
        .collect()
}

/// Which service an entry id belongs to (needed before removal, so cache
/// invalidation knows which provider to refetch).
pub fn service_of(id: &str) -> Option<String> {
    load().into_iter().find(|e| e.id == id).map(|e| e.service)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_keeps_head_and_tail_only() {
        assert_eq!(mask_key("sk-abcdefghijklmnop"), "sk-abc…mnop");
        assert_eq!(mask_key("short"), "*****");
        assert_eq!(mask_key(""), "***");
    }

    #[test]
    fn ids_are_stable_and_service_scoped() {
        assert_eq!(entry_id("Tavily", "k1"), entry_id("tavily", "k1"));
        assert_ne!(entry_id("tavily", "k1"), entry_id("tavily", "k2"));
        assert_ne!(entry_id("tavily", "k1"), entry_id("bocha", "k1"));
    }

    #[test]
    fn upsert_is_idempotent_and_updates_label() {
        let mut v = Vec::new();
        upsert(&mut v, "tavily", "A", "key-one", "");
        upsert(&mut v, "tavily", "B", "key-one", "");
        upsert(&mut v, "tavily", "C", "key-two", "");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].label, "B");
        // blank keys never land
        upsert(&mut v, "x", "y", "   ", "");
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn vault_rows_always_mask() {
        let mut v = Vec::new();
        upsert(&mut v, "svc", "l", "sk-verylongsecretkey1234", "n");
        assert!(mask_key(&v[0].key).starts_with("sk-ver"));
        assert!(!mask_key(&v[0].key).contains("secret"));
    }
}
