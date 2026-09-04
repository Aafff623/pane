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

pub fn load_accounts_from(base: &std::path::Path, provider: &str) -> Vec<AccountEntry> {
    let raw = std::fs::read_to_string(accounts_file(base, provider)).unwrap_or_default();
    let mut out = parse_accounts(&raw);
    out.retain(|e| !e.api_key.trim().is_empty());
    out
}

pub fn save_accounts_to(
    base: &std::path::Path,
    provider: &str,
    entries: &[AccountEntry],
) -> Result<(), String> {
    let dir = accounts_dir(base);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create accounts dir: {e}"))?;
    std::fs::write(accounts_file(base, provider), serialize_accounts(entries))
        .map_err(|e| format!("write accounts file: {e}"))
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
/// Custom Balance includes its normalized base URL because the same key at
/// two relay hosts represents two different quota sources.
pub fn card_id_for_account(provider: &str, account: &AccountEntry) -> String {
    let mut material = account.api_key.trim().as_bytes().to_vec();
    material.push(0);
    if provider == "relaybalance" {
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
        ] {
            assert!(provider_takes_accounts(p));
        }
        assert!(!provider_takes_accounts("claude"));
    }

    #[test]
    fn family_display_name_comes_from_catalog() {
        assert_eq!(family_display_name("relaybalance"), "Custom Balance");
        assert_eq!(family_display_name("future-provider"), "future-provider");
    }
}
