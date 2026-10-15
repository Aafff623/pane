//! OS credential vault for provider and account API keys.
//!
//! Keys live in the platform's native store (Windows Credential Manager via
//! keyring-rs; Keychain / Secret Service elsewhere); the JSON files keep
//! only non-secret fields (labels, base URLs, stable ids). Reads fall back
//! to the legacy plaintext file and migrate it on first sight — the vault
//! write must succeed BEFORE the field is dropped from disk, so a broken
//! vault can never lose a credential.
//!
//! The parse-tests harness swaps in an in-memory store (feature `harness`):
//! the GNU test binary has no OS credential service, and tests must never
//! touch the developer's real vault.

use serde_json::Value;
use std::path::Path;

/// One lookup result. `Missing` and `Unavailable` must stay distinct: a
/// broken vault means "fall back to the file", NOT "no credential".
pub enum Secret {
    Found(String),
    Missing,
    Unavailable,
}

pub fn provider_key(family: &str) -> String {
    format!("provider:{family}")
}

/// Account rows key on the stable card id (`<family>@<fnv1a>`), never on
/// the array index — removing one account must not shift the others' keys.
pub fn account_key(card_id: &str) -> String {
    format!("account:{card_id}")
}

pub fn archived_key(card_id: &str) -> String {
    format!("archived:{card_id}")
}

/// Login/OAuth token blobs (refresh tokens, account token pairs) live under
/// their own namespace so they group separately from API keys in the OS
/// vault UI. The scope is a stable, unique slug (`oauth/codex`,
/// `login/<family>@<fnv1a>`, `searchkey/<id>`, …).
pub fn token_key(scope: &str) -> String {
    format!("token:{scope}")
}

/// The login-style secret triple one account card keeps in the vault.
/// Account JSON files persist the same record with these fields emptied —
/// the vault owns the secrets, the file owns labels, ids and expiry.
#[derive(serde::Serialize, serde::Deserialize, Default, Clone, PartialEq, Eq, Debug)]
pub struct TokenTriple {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub id_token: String,
}

/// One card's triple from the vault, if present.
pub fn get_triple(scope: &str) -> Option<TokenTriple> {
    match get(&token_key(scope)) {
        Secret::Found(raw) => serde_json::from_str(&raw).ok(),
        Secret::Missing | Secret::Unavailable => None,
    }
}

/// Vault-first triple write: `true` = the vault took it and the caller may
/// strip the file copy; `false` = vault unavailable, the caller keeps the
/// plaintext fallback so the credential keeps working.
pub fn put_triple(scope: &str, triple: &TokenTriple) -> bool {
    serde_json::to_string(triple)
        .ok()
        .and_then(|blob| put(&token_key(scope), &blob).ok())
        .is_some()
}

pub fn get(key: &str) -> Secret {
    backend::get(key)
}

pub fn put(key: &str, secret: &str) -> Result<(), String> {
    backend::put(key, secret)
}

pub fn remove(key: &str) {
    backend::remove(key)
}

/// Keyring-first provider lookup with lazy plaintext migration. The file
/// loses its `apiKey` only after the vault write succeeded; on any vault
/// failure the plaintext file stays and keeps working as the fallback.
pub fn resolve_provider_key(family: &str, path: &Path) -> Option<String> {
    match get(&provider_key(family)) {
        Secret::Found(k) => return Some(k),
        Secret::Missing | Secret::Unavailable => {}
    }
    let raw = std::fs::read_to_string(path).ok()?;
    let doc: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok()?;
    let key = doc.get("apiKey")?.as_str()?.trim().to_string();
    if key.is_empty() {
        return None;
    }
    if put(&provider_key(family), &key).is_ok() {
        strip_api_key_field(path);
    }
    Some(key)
}

/// Rewrites a key file without its `apiKey`; a document left with no fields
/// at all removes the file. Best-effort — a failed rewrite keeps the
/// plaintext file (still readable through the fallback), never drops it.
pub fn strip_api_key_field(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(mut doc) = serde_json::from_str::<Value>(raw.trim_start_matches('\u{feff}')) else {
        return false;
    };
    let Some(obj) = doc.as_object_mut() else {
        return false;
    };
    if obj.remove("apiKey").is_none() {
        return false;
    }
    if obj.is_empty() {
        return std::fs::remove_file(path).is_ok();
    }
    match serde_json::to_string_pretty(&doc) {
        Ok(text) => crate::accounts::write_json_atomic(path, text).is_ok(),
        Err(_) => false,
    }
}

#[cfg(feature = "harness")]
mod backend {
    use super::Secret;
    use std::collections::HashMap;
    use std::sync::Mutex;

    static MAP: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
    /// Failure injection for the "broken vault" path: puts whose key starts
    /// with this prefix fail, everything else (and every other test) is
    /// unaffected.
    pub(super) static FAIL_PUT_PREFIX: Mutex<Option<String>> = Mutex::new(None);

    pub fn get(key: &str) -> Secret {
        let guard = MAP.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref().and_then(|m| m.get(key)) {
            Some(v) => Secret::Found(v.clone()),
            None => Secret::Missing,
        }
    }

    pub fn put(key: &str, secret: &str) -> Result<(), String> {
        if let Some(prefix) = FAIL_PUT_PREFIX
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            if key.starts_with(prefix.as_str()) {
                return Err("injected vault failure (test)".into());
            }
        }
        MAP.lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_or_insert_with(HashMap::new)
            .insert(key.to_string(), secret.to_string());
        Ok(())
    }

    pub fn remove(key: &str) {
        if let Some(map) = MAP.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            map.remove(key);
        }
    }
}

/// Test-only failure injection (harness backend): puts whose key starts
/// with `prefix` fail so the broken-vault fallback paths are exercisable.
#[cfg(feature = "harness")]
pub fn fail_puts_for(prefix: &str) {
    *backend::FAIL_PUT_PREFIX.lock().unwrap_or_else(|e| e.into_inner()) = Some(prefix.to_string());
}

/// Undoes [`fail_puts_for`].
#[cfg(feature = "harness")]
pub fn clear_put_failures() {
    *backend::FAIL_PUT_PREFIX.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

#[cfg(not(feature = "harness"))]
mod backend {
    use super::Secret;
    use keyring::{Entry, Error};

    /// One service name, so every entry groups under "Pane" in the OS UI.
    const SERVICE: &str = "Pane";

    pub fn get(key: &str) -> Secret {
        let Ok(entry) = Entry::new(SERVICE, key) else {
            return Secret::Unavailable;
        };
        match entry.get_password() {
            Ok(v) => Secret::Found(v),
            Err(Error::NoEntry) => Secret::Missing,
            Err(_) => Secret::Unavailable,
        }
    }

    pub fn put(key: &str, secret: &str) -> Result<(), String> {
        Entry::new(SERVICE, key)
            .map_err(|e| e.to_string())?
            .set_password(secret)
            .map_err(|e| e.to_string())
    }

    pub fn remove(key: &str) {
        if let Ok(entry) = Entry::new(SERVICE, key) {
            let _ = entry.delete_credential();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_backend_roundtrips_and_removes() {
        let key = "test:roundtrip";
        assert!(matches!(get(key), Secret::Missing));
        put(key, "sk-1").expect("put");
        assert!(matches!(get(key), Secret::Found(v) if v == "sk-1"));
        remove(key);
        assert!(matches!(get(key), Secret::Missing));
    }

    #[test]
    fn provider_migration_strips_the_file_only_after_the_vault_write() {
        let base = std::env::temp_dir().join(format!("pane-secretstore-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let path = base.join("deepseek.json");
        std::fs::write(&path, r##"{"apiKey":"sk-mig","baseUrl":"https://relay.example.com"}"##)
            .unwrap();
        let key = resolve_provider_key("deepseek", &path).expect("key resolves");
        assert_eq!(key, "sk-mig");
        // The secret moved to the vault; baseUrl stays behind.
        assert!(matches!(get(&provider_key("deepseek")), Secret::Found(v) if v == "sk-mig"));
        let left = std::fs::read_to_string(&path).expect("file remains");
        assert!(!left.contains("apiKey"));
        assert!(left.contains("baseUrl"));
        // Second resolve answers from the vault alone.
        assert_eq!(resolve_provider_key("deepseek", &path).as_deref(), Some("sk-mig"));
        // A key-only file disappears entirely after migration.
        let bare = base.join("moonshot.json");
        std::fs::write(&bare, r#"{"apiKey":"sk-bare"}"#).unwrap();
        resolve_provider_key("moonshot", &bare).expect("key resolves");
        assert!(!bare.exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
