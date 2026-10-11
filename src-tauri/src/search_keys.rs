//! Pane-managed plaintext store for MCP / search-service keys.
//!
//! Bocha, Tavily, Firecrawl and Brave are deliberately NOT kept in the
//! encrypted vault. A vault entry is unreadable until the user unlocks it —
//! and the v3 session key lives only in memory, so the vault is locked again
//! on every launch. Their cards would then show a "🔒 保险箱已锁定" placeholder
//! instead of quota, which is exactly what the user rejected.
//!
//! They live in `search-keys.json` instead: the user adds them in Settings,
//! they are plaintext on disk, and they are readable on every launch with no
//! unlock step. Every other service stays in the vault.
//!
//! These keys are read from nowhere else — no environment variables, no
//! third-party MCP client config. What the user adds here is what Pane queries.
//!
//! Every public function delegates to a `*_at` core that takes the store
//! path, so the tests exercise real reads/writes against a temp file instead
//! of the user's config directory.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Services whose keys live in this file rather than the vault.
pub const SEARCH_SERVICES: &[&str] = &["bocha", "tavily", "firecrawl", "brave"];

const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SearchKey {
    pub id: String,
    pub service: String,
    #[serde(default)]
    pub label: String,
    pub key: String,
    #[serde(default)]
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Default)]
struct File {
    version: u32,
    keys: Vec<SearchKey>,
}

pub fn is_search_service(service: &str) -> bool {
    SEARCH_SERVICES
        .iter()
        .any(|s| s.eq_ignore_ascii_case(service.trim()))
}

fn store_path() -> PathBuf {
    crate::providers::config_dir().join("search-keys.json")
}

fn read_from(path: &Path) -> File {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<File>(&raw).ok())
        .filter(|f| f.version <= VERSION)
        .map(|mut f| {
            f.version = VERSION;
            f
        })
        .unwrap_or_default()
}

fn write_to(path: &Path, file: &File) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| format!("write search-keys.json: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("write search-keys.json: {e}"))
}

/// Every stored key, in insertion order.
pub fn list() -> Vec<SearchKey> {
    read_from(&store_path()).keys
}

/// (label, key) for one service. A key stored without a label falls back to
/// its mask so the per-key rows stay distinguishable.
pub fn for_service(service: &str) -> Vec<(String, String)> {
    let keys = read_from(&store_path()).keys;
    rows_for(&keys, service)
}

fn rows_for(keys: &[SearchKey], service: &str) -> Vec<(String, String)> {
    keys.iter()
        .filter(|k| k.service.eq_ignore_ascii_case(service.trim()))
        .map(|k| {
            let label = if k.label.trim().is_empty() {
                crate::keyvault::mask_key(&k.key)
            } else {
                k.label.clone()
            };
            (label, k.key.clone())
        })
        .collect()
}

/// Add or update one key (idempotent: the same service+key is one entry).
pub fn add(service: &str, label: &str, key: &str) -> Result<Vec<SearchKey>, String> {
    let path = store_path();
    let mut file = read_from(&path);
    add_at(&path, &mut file, service, label, key)?;
    Ok(file.keys)
}

fn add_at(
    path: &Path,
    file: &mut File,
    service: &str,
    label: &str,
    key: &str,
) -> Result<(), String> {
    let service = service.trim().to_lowercase();
    let key = key.trim();
    if service.is_empty() || key.is_empty() {
        return Err("service and key are required".into());
    }
    if !is_search_service(&service) {
        return Err(format!("{service} is not a search service"));
    }
    let id = crate::keyvault::entry_id(&service, key);
    match file.keys.iter_mut().find(|k| k.id == id) {
        Some(existing) => existing.label = label.trim().to_string(),
        None => file.keys.push(SearchKey {
            id,
            service,
            label: label.trim().to_string(),
            key: key.to_string(),
            created_at: chrono::Utc::now().timestamp_millis(),
        }),
    }
    write_to(path, file)
}

pub fn remove(id: &str) -> Result<Vec<SearchKey>, String> {
    let path = store_path();
    let mut file = read_from(&path);
    let before = file.keys.len();
    file.keys.retain(|k| k.id != id);
    if file.keys.len() == before {
        return Err("no such key".into());
    }
    write_to(&path, &file)?;
    Ok(file.keys)
}

pub fn reveal(id: &str) -> Result<String, String> {
    read_from(&store_path())
        .keys
        .into_iter()
        .find(|k| k.id == id)
        .map(|k| k.key)
        .ok_or_else(|| "no such key".into())
}

pub fn service_of(id: &str) -> Option<String> {
    read_from(&store_path())
        .keys
        .into_iter()
        .find(|k| k.id == id)
        .map(|k| k.service)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "pane-searchkeys-{}-{tag}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&p);
        p
    }

    fn add_temp(path: &Path, service: &str, label: &str, key: &str) -> Vec<SearchKey> {
        let mut file = read_from(path);
        add_at(path, &mut file, service, label, key).unwrap();
        file.keys
    }

    #[test]
    fn add_is_idempotent_and_reads_back_per_service() {
        let path = temp("add");
        add_temp(&path, "firecrawl", "one", "fc-aaaa");
        let keys = add_temp(&path, "firecrawl", "one again", "fc-aaaa");
        add_temp(&path, "tavily", "t", "tvly-bbbb");

        let file = read_from(&path);
        assert_eq!(file.keys.len(), 2, "same service+key must be one entry");
        let fc = rows_for(&file.keys, "firecrawl");
        assert_eq!(fc.len(), 1);
        assert_eq!(fc[0].0, "one again", "re-add updates the label");
        assert!(rows_for(&file.keys, "brave").is_empty());
        assert_eq!(keys.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn blank_label_falls_back_to_the_mask() {
        let path = temp("mask");
        add_temp(&path, "brave", "", "BSA75LabcdefghijVBJs");
        let file = read_from(&path);
        assert_eq!(rows_for(&file.keys, "brave")[0].0, "BSA75L…VBJs");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn non_search_services_are_rejected() {
        let path = temp("reject");
        let mut file = read_from(&path);
        assert!(add_at(&path, &mut file, "claude", "c", "sk-ant-xxxx").is_err());
        assert!(!is_search_service("claude"));
        assert!(is_search_service("Firecrawl"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn persisted_file_round_trips_ids() {
        let path = temp("round");
        add_temp(&path, "bocha", "b", "sk-829abcdefghij7243");
        let file = read_from(&path);
        let id = file.keys[0].id.clone();
        assert_eq!(file.keys[0].key, "sk-829abcdefghij7243");
        assert!(id.starts_with("bocha-"));
        let _ = std::fs::remove_file(&path);
    }
}
