//! Pane-managed login accounts — the multi-account store for OAuth-style
//! families beyond Codex (copilot/grok now; kiro/qoder/trae/zed/windsurf/
//! codebuddy as their login flows land).
//!
//! Every completed Pane sign-in appends (or refreshes) one account here,
//! keyed by the vendor's stable account id, so any number of subscriptions
//! can be monitored in parallel regardless of which one the CLI/IDE is
//! signed into. Card ids follow the generic account-card scheme
//! `family@<fnv1a32(account_id)>` (the same fingerprint lanes as
//! accounts.rs), so usage history and layout stay attached across
//! re-logins and renames.
//!
//! Tokens live in `%APPDATA%\Pane\login-accounts\<family>.json`, the same
//! file-based shape as the sibling codex store. Family-specific material
//! that a login flow captured (idc region, client ids, profile ARNs…)
//! rides in [`LoginAccount::extra`] — the store stays schema-agnostic.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct LoginAccount {
    /// The vendor's stable account identity (GitHub user id, xAI `sub`, …).
    /// Labels never participate: renaming must not move a card's history.
    pub account_id: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub label: String,
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub id_token: String,
    /// RFC3339 expiry of the access token; empty = non-expiring token.
    #[serde(default)]
    pub expires_at: String,
    #[serde(default)]
    pub added_at: i64,
    /// Family-specific captured material (never parsed by this store).
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra: Value,
}

/// The families this store serves. Every entry has a login flow that
/// records here on completion; the gate keeps stray family ids from
/// minting files under `%APPDATA%\Pane\login-accounts\`.
pub const LOGIN_FAMILIES: [&str; 8] = [
    "codebuddy", "copilot", "grok", "kiro", "qoder", "trae", "windsurf", "zed",
];

pub fn takes_login_accounts(family: &str) -> bool {
    LOGIN_FAMILIES.contains(&family)
}

fn accounts_file(base: &Path, family: &str) -> PathBuf {
    base.join("login-accounts").join(format!("{family}.json"))
}

/// Tolerates a UTF-8 BOM like every other Pane JSON store.
pub fn parse_accounts(raw: &str) -> Vec<LoginAccount> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
}

pub fn load_from(base: &Path, family: &str) -> Vec<LoginAccount> {
    let raw = std::fs::read_to_string(accounts_file(base, family)).unwrap_or_default();
    let mut out = parse_accounts(&raw);
    out.retain(|a| !a.account_id.trim().is_empty() && !a.access_token.trim().is_empty());
    out
}

pub fn save_to(base: &Path, family: &str, accounts: &[LoginAccount]) -> Result<(), String> {
    std::fs::create_dir_all(base.join("login-accounts"))
        .map_err(|e| format!("create login-accounts dir: {e}"))?;
    crate::accounts::write_json_atomic(
        &accounts_file(base, family),
        serde_json::to_string_pretty(accounts).unwrap_or_default(),
    )
    .map_err(|e| format!("write {family} login accounts: {e}"))
}

pub fn load(family: &str) -> Vec<LoginAccount> {
    load_from(&crate::providers::config_dir(), family)
}

pub fn save(family: &str, accounts: &[LoginAccount]) -> Result<(), String> {
    save_to(&crate::providers::config_dir(), family, accounts)
}

// FNV-1a lanes copied from accounts.rs: the store mints the SAME card-id
// shape (`family@<32hex>`) the API-key lane uses, so every consumer
// (cache filter, layout, usage history) treats both identically.
fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Stable card id for one login account. The identity is the vendor's
/// account id — never the label, never the rotating access token.
pub fn card_id_for_account(family: &str, account: &LoginAccount) -> String {
    let mut material = account.account_id.trim().as_bytes().to_vec();
    material.push(0);
    let left = fnv1a(&material, 0xcbf2_9ce4_8422_2325);
    let mut second = b"pane-account-id-v1\0".to_vec();
    second.extend_from_slice(&material);
    let right = fnv1a(&second, 0x8422_2325_cbf2_9ce4);
    format!("{family}@{left:016x}{right:016x}")
}

/// "…abcd" style mask: recognizable tail, never the whole token.
pub fn mask_token(token: &str) -> String {
    let token = token.trim();
    let chars: Vec<char> = token.chars().collect();
    if chars.len() < 4 {
        return "…".into();
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("…{tail}")
}

/// Upsert by account id; an incoming empty label keeps the stored one so a
/// re-login never wipes a user-set name.
pub fn upsert(accounts: &mut Vec<LoginAccount>, incoming: LoginAccount) {
    if let Some(existing) = accounts.iter_mut().find(|a| a.account_id == incoming.account_id) {
        let label = if incoming.label.trim().is_empty() {
            std::mem::take(&mut existing.label)
        } else {
            incoming.label
        };
        let email = if incoming.email.trim().is_empty() {
            std::mem::take(&mut existing.email)
        } else {
            incoming.email
        };
        let extra = if incoming.extra.is_null() {
            std::mem::take(&mut existing.extra)
        } else {
            incoming.extra
        };
        let added_at = existing.added_at;
        *existing = LoginAccount { label, email, added_at, extra, ..incoming };
        return;
    }
    accounts.push(incoming);
}

/// One completed Pane sign-in → the store (called from the login flows).
pub fn record_login(family: &str, account: LoginAccount) -> Result<(), String> {
    if !takes_login_accounts(family) {
        return Err(format!("no login-account store for {family}"));
    }
    let mut accounts = load(family);
    upsert(&mut accounts, account);
    save(family, &accounts)
}

/// StoredTokens (oauth.rs) → a store entry for copilot/grok. Identity: the
/// stored account id first, then the id_token's `sub` (grok); a sign-in
/// that names neither has no identity and never becomes a card.
pub fn from_stored(tokens: &crate::oauth::StoredTokens) -> Result<LoginAccount, String> {
    let claims = tokens
        .id_token
        .as_deref()
        .and_then(crate::oauth::jwt_claims);
    let account_id = tokens
        .account_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            claims
                .as_ref()?
                .get("sub")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
        })
        .ok_or("sign-in carried no account id")?;
    let email = claims
        .as_ref()
        .and_then(|c| c.get("email"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok(LoginAccount {
        account_id,
        email,
        label: tokens.label.clone().unwrap_or_default(),
        access_token: tokens.access_token.clone(),
        refresh_token: tokens.refresh_token.clone(),
        id_token: tokens.id_token.clone().unwrap_or_default(),
        expires_at: tokens.expires_at.clone(),
        added_at: chrono::Utc::now().timestamp(),
        extra: Value::Null,
    })
}

/// The store plus a lazy import of the pre-multi-account single OAuth slot:
/// the old `%APPDATA%\Pane\oauth\<family>.json` login becomes the first
/// account, and the file stays put (the bare family card's fallback still
/// reads it). Codex-account parity.
pub fn load_with_imported_single_login(family: &str) -> Vec<LoginAccount> {
    load_with_imported_single_login_from(&crate::providers::config_dir(), family)
}

/// The `_from` variant keeps tests on a temp dir.
pub fn load_with_imported_single_login_from(base: &Path, family: &str) -> Vec<LoginAccount> {
    let mut accounts = load_from(base, family);
    let Some(tokens) = crate::oauth::load_from(&base.join("oauth"), family) else {
        return accounts;
    };
    let Ok(login) = from_stored(&tokens) else {
        return accounts;
    };
    if accounts.iter().any(|a| a.account_id == login.account_id) {
        // The store's copy wins: it is what cards read, and it may carry a
        // user label plus fresher rotated tokens.
        return accounts;
    }
    accounts.push(login);
    let _ = save_to(base, family, &accounts);
    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(account_id: &str, label: &str) -> LoginAccount {
        LoginAccount {
            account_id: account_id.into(),
            email: "user@example.com".into(),
            label: label.into(),
            access_token: "gho_token_value".into(),
            refresh_token: String::new(),
            id_token: String::new(),
            expires_at: String::new(),
            added_at: 1,
            extra: Value::Null,
        }
    }

    #[test]
    fn upsert_dedupes_by_account_and_keeps_user_labels() {
        let mut accounts = vec![account("12345", "Work")];
        upsert(&mut accounts, account("12345", ""));
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].label, "Work"); // empty incoming keeps stored
        upsert(&mut accounts, account("12345", "Renamed"));
        assert_eq!(accounts[0].label, "Renamed");
        upsert(&mut accounts, account("67890", ""));
        assert_eq!(accounts.len(), 2);
    }

    #[test]
    fn card_ids_are_stable_and_opaque() {
        let a = account("12345", "ignored");
        let b = account("12345", "renamed");
        assert_eq!(card_id_for_account("copilot", &a), card_id_for_account("copilot", &b));
        assert_eq!(card_id_for_account("copilot", &a).len(), "copilot@".len() + 32);
        assert_ne!(card_id_for_account("copilot", &a), card_id_for_account("grok", &a));
        assert_ne!(card_id_for_account("copilot", &a), card_id_for_account("copilot", &account("12346", "")));
    }

    #[test]
    fn from_stored_takes_account_id_then_sub() {
        let tokens = crate::oauth::StoredTokens {
            access_token: "at".into(),
            refresh_token: String::new(),
            expires_at: String::new(),
            label: Some("me@x.com".into()),
            account_id: Some("gh-user-9".into()),
            id_token: None,
        };
        assert_eq!(from_stored(&tokens).unwrap().account_id, "gh-user-9");

        // No account id: the id_token's sub is the fallback identity. Build
        // a real JWT-shaped middle chunk so oauth::jwt_claims parses it.
        use base64::Engine;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&serde_json::json!({
                "sub": "xai-sub-1", "email": "a@b.c"
            })).unwrap());
        let mut sub_only = tokens;
        sub_only.account_id = None;
        sub_only.id_token = Some(format!("header.{payload}.sig"));
        let login = from_stored(&sub_only).unwrap();
        assert_eq!(login.account_id, "xai-sub-1");
        assert_eq!(login.email, "a@b.c");

        // Neither id nor id_token → no identity, never a card.
        let mut no_identity = sub_only;
        no_identity.id_token = None;
        assert!(from_stored(&no_identity).is_err());
    }

    #[test]
    fn store_roundtrips_on_a_temp_dir() {
        let base = std::env::temp_dir().join(format!("pane-login-accts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let accounts = vec![account("1", "A"), account("2", "B")];
        save_to(&base, "copilot", &accounts).unwrap();
        let back = load_from(&base, "copilot");
        assert_eq!(back.len(), 2);
        assert_eq!(back[1].label, "B");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn lazy_import_bridges_the_single_oauth_slot() {
        let base = std::env::temp_dir().join(format!("pane-login-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let oauth_dir = base.join("oauth");
        std::fs::create_dir_all(&oauth_dir).unwrap();
        std::fs::write(
            oauth_dir.join("copilot.json"),
            serde_json::to_string_pretty(&crate::oauth::StoredTokens {
                access_token: "gho_old".into(),
                refresh_token: "gho_old".into(),
                expires_at: "2030-01-01T00:00:00Z".into(),
                label: Some("legacy-login".into()),
                account_id: Some("42".into()),
                id_token: None,
            })
            .unwrap(),
        )
        .unwrap();

        // Empty store: the slot imports as the first account, once.
        let imported = load_with_imported_single_login_from(&base, "copilot");
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].account_id, "42");

        // A store copy of the SAME account wins over the slot import.
        save_to(&base, "copilot", &[account("42", "Named")]).unwrap();
        let merged = load_with_imported_single_login_from(&base, "copilot");
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].label, "Named");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn mask_keeps_only_a_tail() {
        assert_eq!(mask_token("gho_abcdefgh"), "…efgh");
        assert_eq!(mask_token("ab"), "…");
    }
}
