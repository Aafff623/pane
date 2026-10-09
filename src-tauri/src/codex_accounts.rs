//! Pane-managed Codex logins — multi-account support for the Codex family.
//!
//! Every completed Pane sign-in appends (or refreshes) one account here,
//! keyed by the ChatGPT account id, so any number of subscriptions can be
//! monitored in parallel no matter which one the CLI itself is signed into.
//! Card ids share the discovery scheme in `providers::codex`
//! (`codex@<hash8>` of the account id), so a Pane login and a discovered
//! CLI home for the SAME account never double-card, and the pre-multi-
//! account single OAuth slot imports lazily (its file stays — the default
//! card still falls back to it).
//!
//! Tokens live in `%APPDATA%\Pane\codex-accounts.json`, the same
//! file-based shape as the sibling Cursor account store.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct CodexLogin {
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
    /// RFC3339 expiry of the access token (the shape oauth.rs stores).
    #[serde(default)]
    pub expires_at: String,
    #[serde(default)]
    pub added_at: i64,
}

fn accounts_file(base: &Path) -> PathBuf {
    base.join("codex-accounts.json")
}

/// Tolerates a UTF-8 BOM like every other Pane JSON store.
pub fn parse_accounts(raw: &str) -> Vec<CodexLogin> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
}

pub fn load_accounts_from(base: &Path) -> Vec<CodexLogin> {
    let raw = std::fs::read_to_string(accounts_file(base)).unwrap_or_default();
    let mut out = parse_accounts(&raw);
    out.retain(|a| !a.account_id.trim().is_empty() && !a.access_token.trim().is_empty());
    out
}

pub fn save_accounts_to(base: &Path, accounts: &[CodexLogin]) -> Result<(), String> {
    std::fs::create_dir_all(base).map_err(|e| format!("create config dir: {e}"))?;
    crate::accounts::write_json_atomic(
        &accounts_file(base),
        serde_json::to_string_pretty(accounts).unwrap_or_default(),
    )
    .map_err(|e| format!("write codex accounts: {e}"))
}

pub fn load_accounts() -> Vec<CodexLogin> {
    load_accounts_from(&crate::providers::config_dir())
}

pub fn save_accounts(accounts: &[CodexLogin]) -> Result<(), String> {
    save_accounts_to(&crate::providers::config_dir(), accounts)
}

/// The discovery id fragment — identical rule to `providers::codex`'s scan
/// so both sources mint the same `codex@<hash8>` for one account.
pub fn hash8(account_id: &str) -> String {
    account_id.chars().filter(|c| *c != '-').take(8).collect()
}

pub fn card_id_for_account_id(account_id: &str) -> String {
    format!("codex@{}", hash8(account_id))
}

pub fn card_id_for_account(login: &CodexLogin) -> String {
    card_id_for_account_id(&login.account_id)
}

/// "sk-…abcd" style mask: recognizable tail, never the whole token.
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
pub fn upsert(accounts: &mut Vec<CodexLogin>, incoming: CodexLogin) {
    if let Some(existing) = accounts.iter_mut().find(|a| a.account_id == incoming.account_id) {
        let label = if incoming.label.trim().is_empty() {
            std::mem::take(&mut existing.label)
        } else {
            incoming.label
        };
        let added_at = existing.added_at;
        *existing = CodexLogin { label, added_at, ..incoming };
        return;
    }
    accounts.push(incoming);
}

/// StoredTokens (oauth.rs) → a store entry. Identity: the stored account id
/// first, the id_token's ChatGPT claim as the fallback; a sign-in that names
/// neither has no identity and never becomes a card.
pub fn from_stored(tokens: &crate::oauth::StoredTokens) -> Result<CodexLogin, String> {
    let claims = tokens.id_token.as_deref().and_then(crate::providers::codex::jwt_claims);
    let account_id = tokens
        .account_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            claims
                .as_ref()?
                .pointer("/https:~1~1api.openai.com~1auth/chatgpt_account_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .ok_or("Codex sign-in carried no ChatGPT account id")?;
    let email = claims
        .as_ref()
        .and_then(|c| c.get("email"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok(CodexLogin {
        account_id,
        email,
        label: tokens.label.clone().unwrap_or_default(),
        access_token: tokens.access_token.clone(),
        refresh_token: tokens.refresh_token.clone(),
        id_token: tokens.id_token.clone().unwrap_or_default(),
        expires_at: tokens.expires_at.clone(),
        added_at: chrono::Utc::now().timestamp(),
    })
}

/// One completed Pane sign-in → the store (called from the OAuth poll).
pub fn record_login(tokens: &crate::oauth::StoredTokens) -> Result<(), String> {
    let login = from_stored(tokens)?;
    let mut accounts = load_accounts();
    upsert(&mut accounts, login);
    save_accounts(&accounts)
}

/// The store plus a lazy import of the pre-multi-account single OAuth slot:
/// the old `%APPDATA%\Pane\oauth\codex.json` login becomes the first
/// account, and the file stays put (the default card's fallback needs it).
pub fn load_with_imported_single_login() -> Vec<CodexLogin> {
    load_with_imported_single_login_from(&crate::providers::config_dir())
}

/// The `_from` variant keeps tests on a temp dir (the auth-center
/// convention).
pub fn load_with_imported_single_login_from(base: &Path) -> Vec<CodexLogin> {
    let mut accounts = load_accounts_from(base);
    let Some(tokens) = crate::oauth::load_from(&base.join("oauth"), "codex") else {
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
    let _ = save_accounts_to(base, &accounts);
    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login(account_id: &str, label: &str) -> CodexLogin {
        CodexLogin {
            account_id: account_id.into(),
            email: "user@example.com".into(),
            label: label.into(),
            access_token: "eyJhbGciOi.access.sig".into(),
            refresh_token: "rt-1".into(),
            id_token: String::new(),
            expires_at: String::new(),
            added_at: 1,
        }
    }

    #[test]
    fn upsert_dedupes_by_account_and_keeps_user_labels() {
        let mut accounts = vec![login("acct-1", "Work")];
        upsert(&mut accounts, login("acct-1", ""));
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].label, "Work"); // empty incoming keeps stored
        upsert(&mut accounts, login("acct-1", "Renamed"));
        assert_eq!(accounts[0].label, "Renamed");
        upsert(&mut accounts, login("acct-2", ""));
        assert_eq!(accounts.len(), 2);
    }

    #[test]
    fn card_ids_match_the_discovery_scheme() {
        // Same rule as providers::codex::discover_extra_accounts.
        assert_eq!(card_id_for_account_id("2f8c1d10-aaaa-bbbb"), "codex@2f8c1d10");
        // The SAME account from either source must mint ONE card id.
        assert_eq!(card_id_for_account_id("abc-123"), "codex@abc123");
        assert_ne!(card_id_for_account_id("abc-123"), card_id_for_account_id("abc-124"));
    }

    #[test]
    fn from_stored_needs_an_identity() {
        let mut tokens = crate::oauth::StoredTokens {
            access_token: "at".into(),
            refresh_token: "rt".into(),
            expires_at: String::new(),
            label: Some("me@x.com".into()),
            account_id: Some("acct-9".into()),
            id_token: None,
        };
        let login = from_stored(&tokens).unwrap();
        assert_eq!(login.account_id, "acct-9");
        assert_eq!(login.label, "me@x.com");
        // No account id and no id_token → no identity, never a card.
        tokens.account_id = None;
        assert!(from_stored(&tokens).is_err());
    }

    #[test]
    fn mask_keeps_only_a_tail() {
        assert_eq!(mask_token("sk-abcdefgh"), "…efgh");
        assert_eq!(mask_token("ab"), "…");
    }

    #[test]
    fn store_roundtrips_on_a_temp_dir() {
        let base = std::env::temp_dir().join(format!("pane-codex-accts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let accounts = vec![login("acct-1", "A"), login("acct-2", "B")];
        save_accounts_to(&base, &accounts).unwrap();
        let back = load_accounts_from(&base);
        assert_eq!(back.len(), 2);
        assert_eq!(back[1].label, "B");
        let _ = std::fs::remove_dir_all(&base);
    }
}
