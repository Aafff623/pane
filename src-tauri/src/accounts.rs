//! Extra API-key accounts for providers that support multiple key identities
//! (Phase 3.2):
//! deepseek, kimi, stepfun, siliconflow, novita, relaybalance. Each entry in
//! %APPDATA%\Pane\accounts\<provider>.json renders its own stable
//! <provider>@<fingerprint> card
//! card next to the family's main card, so one user can watch several
//! wallets at once.
//!
//! Kept free of Tauri types so the parse-tests harness (which mirrors only
//! part of the crate) can compile this file via #[path] and run its unit
//! tests. Provider-specific fetching stays in lib.rs.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One row of accounts/<provider>.json. `label` may be empty (the UI shows
/// a localized "Account N"); `base_url` is only meaningful for
/// relaybalance, whose accounts each point at their own relay host.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AccountEntry {
    #[serde(default)]
    pub label: String,
    #[serde(rename = "apiKey", default)]
    pub api_key: String,
    #[serde(rename = "baseUrl", default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ArchivedAccount {
    pub provider: String,
    pub card_id: String,
    pub label: String,
    pub archived_at: i64,
    /// Credential kept so an archived account can be restored as-is. Old
    /// records (before restore existed) simply have no key and can only be
    /// inspected, never resurrected.
    #[serde(default)]
    pub api_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Archived credentials for account stores that have their own JSON shape
/// (Antigravity OAuth slots and Cursor accounts).  The payload stays on disk
/// and is never returned to the WebView; the command layer only exposes the
/// label and stable card id.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ArchivedExternalAccount {
    pub provider: String,
    pub card_id: String,
    pub label: String,
    pub archived_at: i64,
    pub payload: serde_json::Value,
}

fn accounts_dir(base: &std::path::Path) -> PathBuf {
    base.join("accounts")
}

fn accounts_file(base: &std::path::Path, provider: &str) -> PathBuf {
    accounts_dir(base).join(format!("{provider}.json"))
}

/// Parses one accounts file. Tolerates a UTF-8 BOM (Notepad / PowerShell
/// 5.1 write one, same as config.json). Anything unreadable — a malformed
/// array, an object instead of a list — yields no accounts rather than an
/// error the refresh loop would have to handle.
pub fn parse_accounts(raw: &str) -> Vec<AccountEntry> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
}

pub fn serialize_accounts(entries: &[AccountEntry]) -> String {
    serde_json::to_string_pretty(entries).unwrap_or_default()
}

/// On-disk row: the public `AccountEntry` plus the stable vault id. Kept
/// private so the public struct stays literal-friendly; legacy files simply
/// lack the `id` key and get it captured during the plaintext migration.
#[derive(Serialize, Deserialize, Default)]
struct StoredAccount {
    #[serde(default)]
    id: String,
    #[serde(default)]
    label: String,
    #[serde(rename = "apiKey", default, skip_serializing_if = "String::is_empty")]
    api_key: String,
    #[serde(rename = "baseUrl", default, skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
}

pub fn load_accounts_from(base: &std::path::Path, provider: &str) -> Vec<AccountEntry> {
    let path = accounts_file(base, provider);
    let raw = std::fs::read_to_string(&path).unwrap_or_default();
    let stored: Vec<StoredAccount> =
        serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default();
    let mut out: Vec<AccountEntry> = Vec::with_capacity(stored.len());
    let mut rows: Vec<StoredAccount> = Vec::with_capacity(stored.len());
    let mut dirty = false;
    for mut row in stored {
        if row.api_key.trim().is_empty() {
            // Migrated row: the OS vault holds the key; the persisted id is
            // the only address back to it.
            let mut api_key = String::new();
            if !row.id.is_empty() {
                if let crate::secretstore::Secret::Found(key) =
                    crate::secretstore::get(&crate::secretstore::account_key(&row.id))
                {
                    api_key = key;
                }
            }
            if api_key.is_empty() {
                continue; // key gone — the same drop the legacy loader applied
            }
            out.push(AccountEntry {
                label: row.label.clone(),
                api_key,
                base_url: row.base_url.clone(),
            });
            rows.push(row);
            continue;
        }
        // Plaintext row: capture the stable id and move the secret into the
        // vault — the field leaves the file only after the write succeeded.
        let entry = AccountEntry {
            label: row.label.clone(),
            api_key: row.api_key.clone(),
            base_url: row.base_url.clone(),
        };
        if row.id.is_empty() {
            row.id = card_id_for_account(provider, &entry);
        }
        if crate::secretstore::put(
            &crate::secretstore::account_key(&row.id),
            entry.api_key.trim(),
        )
        .is_ok()
        {
            row.api_key.clear();
            dirty = true;
        }
        out.push(entry);
        rows.push(row);
    }
    if dirty {
        if let Ok(text) = serde_json::to_string_pretty(&rows) {
            let _ = write_json_atomic(&path, text);
        }
    }
    out
}

/// Temp file + rename so a crash mid-write can never truncate a store that
/// holds plaintext credentials. Shared by the other token stores.
pub(crate) fn write_json_atomic(path: &std::path::Path, contents: String) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", tmp.display()))
}

pub fn save_accounts_to(
    base: &std::path::Path,
    provider: &str,
    entries: &[AccountEntry],
) -> Result<(), String> {
    let dir = accounts_dir(base);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create accounts dir: {e}"))?;
    // Keys move into the OS vault as the file is written; a row whose vault
    // write fails keeps its plaintext (the documented fallback).
    let mut rows: Vec<StoredAccount> = Vec::with_capacity(entries.len());
    for entry in entries {
        let mut row = StoredAccount {
            id: String::new(),
            label: entry.label.clone(),
            api_key: entry.api_key.clone(),
            base_url: entry.base_url.clone(),
        };
        if !entry.api_key.trim().is_empty() {
            row.id = card_id_for_account(provider, entry);
            if crate::secretstore::put(
                &crate::secretstore::account_key(&row.id),
                entry.api_key.trim(),
            )
            .is_ok()
            {
                row.api_key.clear();
            }
        }
        rows.push(row);
    }
    let text = serde_json::to_string_pretty(&rows).map_err(|e| e.to_string())?;
    write_json_atomic(&accounts_file(base, provider), text)
}

/// The app's real store. Keeping the dir implicit here (and explicit in the
/// `_from`/`_to` variants) is what lets the tests roundtrip a temp dir
/// instead of touching the user's config.
pub fn load_accounts(provider: &str) -> Vec<AccountEntry> {
    load_accounts_from(&crate::providers::config_dir(), provider)
}

pub fn save_accounts(provider: &str, entries: &[AccountEntry]) -> Result<(), String> {
    save_accounts_to(&crate::providers::config_dir(), provider, entries)
}

fn archive_file(base: &std::path::Path) -> PathBuf {
    base.join("archived_accounts.json")
}

fn external_archive_file(base: &std::path::Path) -> PathBuf {
    base.join("archived_external_accounts.json")
}

pub fn load_archived_external_from(base: &std::path::Path) -> Vec<ArchivedExternalAccount> {
    let raw = std::fs::read_to_string(external_archive_file(base)).unwrap_or_default();
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
}

pub fn archive_external_to(
    base: &std::path::Path,
    account: ArchivedExternalAccount,
) -> Result<(), String> {
    std::fs::create_dir_all(base).map_err(|e| format!("create config dir: {e}"))?;
    let mut list = load_archived_external_from(base);
    if !list.iter().any(|a| a.card_id == account.card_id) {
        list.push(account);
    }
    write_json_atomic(
        &external_archive_file(base),
        serde_json::to_string_pretty(&list).unwrap_or_default(),
    )
    .map_err(|e| format!("write external archive: {e}"))
}

pub fn remove_archived_external_from(
    base: &std::path::Path,
    card_id: &str,
) -> Result<(), String> {
    let mut list = load_archived_external_from(base);
    let before = list.len();
    list.retain(|a| a.card_id != card_id);
    if list.len() == before {
        return Ok(());
    }
    write_json_atomic(
        &external_archive_file(base),
        serde_json::to_string_pretty(&list).unwrap_or_default(),
    )
    .map_err(|e| format!("write external archive: {e}"))
}

pub fn load_archived_from(base: &std::path::Path) -> Vec<ArchivedAccount> {
    let raw = std::fs::read_to_string(archive_file(base)).unwrap_or_default();
    let mut list: Vec<ArchivedAccount> =
        serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default();
    // Hydrate credentials that live in the vault (migrated tombstones).
    for a in list.iter_mut() {
        if a.api_key.trim().is_empty() && !a.card_id.is_empty() {
            if let crate::secretstore::Secret::Found(key) =
                crate::secretstore::get(&crate::secretstore::archived_key(&a.card_id))
            {
                a.api_key = key;
            }
        }
    }
    list
}

/// Moves an archived credential into the OS vault (keyed by its stable card
/// id) and returns the row ready for disk — the plaintext stays in the
/// written copy only when the vault write failed. Idempotent, so it can
/// also strip re-hydrated rows before a rewrite.
fn without_plaintext_key(mut account: ArchivedAccount) -> ArchivedAccount {
    if !account.api_key.trim().is_empty()
        && crate::secretstore::put(
            &crate::secretstore::archived_key(&account.card_id),
            account.api_key.trim(),
        )
        .is_ok()
    {
        account.api_key.clear();
    }
    account
}

pub fn archive_account(account: ArchivedAccount) -> Result<(), String> {
    archive_account_to(&crate::providers::config_dir(), account)
}

pub fn archive_account_to(base: &std::path::Path, account: ArchivedAccount) -> Result<(), String> {
    std::fs::create_dir_all(base).map_err(|e| format!("create config dir: {e}"))?;
    let account = without_plaintext_key(account);
    let mut list = load_archived_from(base);
    if !list.iter().any(|a| a.card_id == account.card_id) {
        list.push(account);
    }
    let list: Vec<ArchivedAccount> = list.into_iter().map(without_plaintext_key).collect();
    write_json_atomic(
        &archive_file(base),
        serde_json::to_string_pretty(&list).unwrap_or_default(),
    )
    .map_err(|e| format!("write archived accounts: {e}"))
}

/// Drops a tombstone from the archive store (after a restore reaped its
/// credential back into the active accounts file), and with it the vault
/// copy of the credential.
pub fn remove_archived(card_id: &str) -> Result<(), String> {
    remove_archived_from(&crate::providers::config_dir(), card_id)
}

pub fn remove_archived_from(base: &std::path::Path, card_id: &str) -> Result<(), String> {
    crate::secretstore::remove(&crate::secretstore::archived_key(card_id));
    let mut list = load_archived_from(base);
    let before = list.len();
    list.retain(|a| a.card_id != card_id);
    if list.len() == before {
        return Ok(());
    }
    let list: Vec<ArchivedAccount> = list.into_iter().map(without_plaintext_key).collect();
    write_json_atomic(
        &archive_file(base),
        serde_json::to_string_pretty(&list).unwrap_or_default(),
    )
    .map_err(|e| format!("write archived accounts: {e}"))
}

pub fn provider_takes_accounts(provider: &str) -> bool {
    crate::provider_catalog::supports_extra_accounts(provider)
}

/// Legacy positional card id retained only for old parse-test coverage. New
/// runtime cards use `card_id_for_account`; using #n as an identity lets a
/// delete/reorder operation attach an old cache or layout to another key.
#[allow(dead_code)]
pub fn card_id(provider: &str, n: usize) -> String {
    format!("{provider}@{n}")
}

// FNV-1a is used only to derive a stable local card identity. It is not a
// credential-hiding primitive, and the resulting fingerprint never crosses
// the telemetry boundary. Two lanes make accidental collisions vanishingly
// unlikely without adding a new hashing dependency to this small module.
fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Stable instance id for an API-key account. Labels intentionally do not
/// participate: renaming an account must not discard its cache or layout.
/// Custom Balance and Linkso include their normalized base URL because the
/// same key at two relay hosts represents two different quota sources.
pub fn card_id_for_account(provider: &str, account: &AccountEntry) -> String {
    let mut material = account.api_key.trim().as_bytes().to_vec();
    material.push(0);
    if crate::provider_catalog::takes_base_url(provider) {
        material.extend_from_slice(
            account
                .base_url
                .as_deref()
                .map(str::trim)
                .map(|url| url.trim_end_matches('/'))
                .unwrap_or("")
                .as_bytes(),
        );
    }
    let left = fnv1a(&material, 0xcbf2_9ce4_8422_2325);
    let mut second = b"pane-account-id-v1\0".to_vec();
    second.extend_from_slice(&material);
    let right = fnv1a(&second, 0x8422_2325_cbf2_9ce4);
    format!("{provider}@{left:016x}{right:016x}")
}

/// Inverse of the legacy positional `card_id`: the (family, n) behind an
/// account-scoped card id. Stable fingerprint ids are deliberately opaque
/// and are not parsed. (Tested in the parse-tests harness; the runtime only
/// formats ids, never parses them.)
#[allow(dead_code)]
pub fn parse_card_id(id: &str) -> Option<(&str, usize)> {
    let (family, n) = id.split_once('@')?;
    if family.is_empty() {
        return None;
    }
    let n = n.parse::<usize>().ok()?;
    if n == 0 {
        return None;
    }
    Some((family, n))
}

/// A masked key for account_list: a recognizable head, an ellipsis, and
/// the last 4 characters — "sk-…abcd". Short keys reveal proportionally
/// less; nothing under 4 characters shows anything at all.
pub fn mask_key(key: &str) -> String {
    let key = key.trim();
    let chars: Vec<char> = key.chars().collect();
    if chars.len() < 4 {
        return "…".into();
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    if chars.len() >= 12 {
        format!("{}…{tail}", chars[..3].iter().collect::<String>())
    } else {
        format!("…{tail}")
    }
}

/// Display name for the family, used to prefix account card names the way
/// claude's extra accounts read "Claude — Org".
pub fn family_display_name(provider: &str) -> String {
    crate::provider_catalog::provider_definition(provider)
        .map(|definition| definition.display_name.to_string())
        .unwrap_or_else(|| provider.to_string())
}

/// The label an account with an empty stored label shows: "账号 N" for a
/// Chinese UI, "Аккаунт N" for Russian, "Account N" otherwise. The frontend
/// mirrors this for account_list rows; the backend needs its own copy
/// because the snapshot name is painted by Rust.
pub fn default_label(n: usize, locale: &str) -> String {
    match locale {
        "zh" => format!("账号 {n}"),
        "ru" => format!("Аккаунт {n}"),
        _ => format!("Account {n}"),
    }
}

/// The label actually displayed: the stored one when non-empty, else the
/// localized default for position n (1-based).
pub fn display_label(stored: &str, n: usize, locale: &str) -> String {
    let stored = stored.trim();
    if stored.is_empty() {
        default_label(n, locale)
    } else {
        stored.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounts_file_roundtrips_through_a_temp_dir() {
        let base = std::env::temp_dir().join(format!("pane-accts-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let entries = vec![
            AccountEntry { label: "work".into(), api_key: "sk-aaa".into(), base_url: None },
            AccountEntry {
                label: String::new(),
                api_key: "sk-bbb".into(),
                base_url: Some("https://relay.example.com".into()),
            },
        ];
        save_accounts_to(&base, "deepseek", &entries).expect("write");
        let loaded = load_accounts_from(&base, "deepseek");
        assert_eq!(loaded, entries);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn parse_tolerates_bom_and_ignores_junk() {
        let doc = "\u{feff}[{\"label\":\"a\",\"apiKey\":\"k1\"},{\"label\":\"b\",\"apiKey\":\"k2\",\"baseUrl\":\"https://x\"}]";
        let parsed = parse_accounts(doc);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[1].base_url.as_deref(), Some("https://x"));
        assert!(parse_accounts("not json").is_empty());
        assert!(parse_accounts("{\"apiKey\":\"k\"}").is_empty());
    }

    #[test]
    fn archive_store_keeps_credential_and_restores_by_card_id() {
        let base = std::env::temp_dir().join(format!("pane-archive-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        // Archive two accounts; the credential and base_url must survive.
        let first = ArchivedAccount {
            provider: "deepseek".into(),
            card_id: "deepseek@aa11".into(),
            label: "burnt".into(),
            archived_at: 1,
            api_key: "sk-gone".into(),
            base_url: None,
        };
        let second = ArchivedAccount {
            provider: "deepseek".into(),
            card_id: "deepseek@bb22".into(),
            label: "relay".into(),
            archived_at: 2,
            api_key: "sk-relay".into(),
            base_url: Some("https://relay.example.com".into()),
        };
        archive_account_to(&base, first.clone()).expect("archive 1");
        archive_account_to(&base, second.clone()).expect("archive 2");
        // Archiving the same card id again must not duplicate the tombstone.
        archive_account_to(&base, first.clone()).expect("archive dup");
        let archived = load_archived_from(&base);
        assert_eq!(archived.len(), 2);
        assert_eq!(archived[1].api_key, "sk-relay");
        assert_eq!(archived[1].base_url.as_deref(), Some("https://relay.example.com"));
        // Restoring reaps the tombstone, keeps the sibling.
        remove_archived_from(&base, "deepseek@aa11").expect("restore-reap");
        let remaining = load_archived_from(&base);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].card_id, "deepseek@bb22");
        // Removing an unknown id is a no-op, not an error.
        remove_archived_from(&base, "deepseek@zz99").expect("no-op");
        assert_eq!(load_archived_from(&base).len(), 1);
        // Old records without a key still parse (serde default).
        std::fs::write(
            archive_file(&base),
            "[{\"provider\":\"kimi\",\"card_id\":\"kimi@old\",\"label\":\"legacy\",\"archived_at\":0}]",
        )
        .expect("write legacy");
        let legacy = load_archived_from(&base);
        assert_eq!(legacy.last().unwrap().api_key, "");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn load_drops_entries_without_a_key() {
        let base = std::env::temp_dir().join(format!("pane-accts-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        save_accounts_to(
            &base,
            "stepfun",
            &[
                AccountEntry { label: "x".into(), api_key: String::new(), base_url: None },
                AccountEntry { label: "y".into(), api_key: "sk-1".into(), base_url: None },
            ],
        )
        .expect("write");
        let loaded = load_accounts_from(&base, "stepfun");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].label, "y");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn missing_file_loads_as_empty() {
        let base = std::env::temp_dir().join(format!("pane-accts-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        assert!(load_accounts_from(&base, "novita").is_empty());
    }

    #[test]
    fn card_ids_roundtrip_and_reject_junk() {
        assert_eq!(card_id("deepseek", 1), "deepseek@1");
        assert_eq!(parse_card_id("deepseek@12"), Some(("deepseek", 12)));
        assert_eq!(parse_card_id("claude"), None);
        assert_eq!(parse_card_id("claude@ab12cd34"), None); // claude's hash ids are not ours
        assert_eq!(parse_card_id("@3"), None);
        assert_eq!(parse_card_id("deepseek@0"), None);
        assert_eq!(parse_card_id("deepseek@x"), None);
    }

    #[test]
    fn account_card_id_is_stable_across_labels_and_positions() {
        let work = AccountEntry {
            label: "work".into(),
            api_key: "sk-account-one".into(),
            base_url: None,
        };
        let renamed = AccountEntry {
            label: "personal".into(),
            ..work.clone()
        };
        assert_eq!(
            card_id_for_account("deepseek", &work),
            card_id_for_account("deepseek", &renamed)
        );
        assert_ne!(
            card_id_for_account(
                "deepseek",
                &AccountEntry {
                    api_key: "sk-account-two".into(),
                    ..work.clone()
                }
            ),
            card_id_for_account("deepseek", &work)
        );
    }

    #[test]
    fn relay_card_id_includes_the_normalized_base_url() {
        let first = AccountEntry {
            label: String::new(),
            api_key: "sk-relay".into(),
            base_url: Some("https://relay.example.com/".into()),
        };
        let same_host = AccountEntry {
            base_url: Some(" https://relay.example.com ".into()),
            ..first.clone()
        };
        let other_host = AccountEntry {
            base_url: Some("https://other.example.com".into()),
            ..first.clone()
        };
        assert_eq!(
            card_id_for_account("relaybalance", &first),
            card_id_for_account("relaybalance", &same_host)
        );
        assert_ne!(
            card_id_for_account("relaybalance", &first),
            card_id_for_account("relaybalance", &other_host)
        );
    }

    #[test]
    fn masking_shows_head_and_last_four() {
        assert_eq!(mask_key("sk-1234567890abcd"), "sk-…abcd");
        assert_eq!(mask_key("shortkey"), "…tkey");
        // Nothing revealable under 4 characters.
        assert_eq!(mask_key("abc"), "…");
        assert_eq!(mask_key(""), "…");
    }

    #[test]
    fn labels_default_per_locale_and_positions() {
        assert_eq!(display_label("", 2, "zh"), "账号 2");
        assert_eq!(display_label("", 1, "ru"), "Аккаунт 1");
        assert_eq!(display_label("", 3, "en"), "Account 3");
        assert_eq!(display_label(" work ", 1, "zh"), "work");
    }

    #[test]
    fn the_six_key_providers_take_accounts() {
        for p in [
            "deepseek",
            "kimi",
            "stepfun",
            "siliconflow",
            "novita",
            "relaybalance",
            "deepgram",
            "groq",
            "mistral",
            "volcengine",
        ] {
            assert!(provider_takes_accounts(p));
        }
        assert!(!provider_takes_accounts("claude"));
    }

    #[test]
    fn family_display_name_comes_from_catalog() {
        assert_eq!(family_display_name("relaybalance"), "Custom Relay");
        assert_eq!(family_display_name("future-provider"), "future-provider");
    }
}
