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
//! Tokens live in the OS credential vault (one `token:login/<card_id>`
//! slot per account, see secretstore.rs); the per-family JSON file under
//! `%APPDATA%\Pane\login-accounts\` keeps only non-secrets (labels, ids,
//! expiry, extra). Legacy plaintext token entries migrate to the vault on
//! first read — the vault write must succeed before the file loses them.
//! Family-specific material that a login flow captured (idc region,
//! client ids, profile ARNs…) rides in [`LoginAccount::extra`] — the
//! store stays schema-agnostic.

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
pub const LOGIN_FAMILIES: [&str; 14] = [
    "codebuddy", "codebuddycn", "copilot", "grok", "kiro", "qoder", "trae", "windsurf", "workbuddy",
    "workbuddyai", "zed", "tokenrhythm", "xfyun", "xfyun-speech",
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

/// Vault slot for one card's tokens — the card id already carries the
/// family, so the scope is unique per account across all stores.
fn vault_scope(family: &str, account: &LoginAccount) -> String {
    format!("login/{}", card_id_for_account(family, account))
}

/// Read-through to the vault: each account's tokens come from the OS
/// credential store when it has them; a plaintext file entry migrates on
/// first sight (vault write first, then the file keeps only non-secrets).
/// A broken vault keeps the file working as the fallback.
pub fn load_from(base: &Path, family: &str) -> Vec<LoginAccount> {
    read_accounts_from(base, family, true)
}

fn read_accounts_from(base: &Path, family: &str, migrate: bool) -> Vec<LoginAccount> {
    let raw = std::fs::read_to_string(accounts_file(base, family)).unwrap_or_default();
    let mut out = parse_accounts(&raw);
    let mut strip_file: Vec<usize> = Vec::new();
    for (idx, account) in out.iter_mut().enumerate() {
        let scope = vault_scope(family, account);
        if let Some(triple) = crate::secretstore::get_triple(&scope) {
            let has_plaintext = !account.access_token.is_empty() || !account.refresh_token.is_empty() || !account.id_token.is_empty();
            account.access_token = triple.access_token;
            account.refresh_token = triple.refresh_token;
            account.id_token = triple.id_token;
            // The file copy (if any tokens linger) is stale — strip it too.
            if migrate && has_plaintext { strip_file.push(idx); }
            continue;
        }
        if migrate && !account.access_token.is_empty() {
            let triple = crate::secretstore::TokenTriple {
                access_token: account.access_token.clone(),
                refresh_token: account.refresh_token.clone(),
                id_token: account.id_token.clone(),
            };
            if crate::secretstore::put_triple(&scope, &triple) {
                strip_file.push(idx);
            }
        }
    }
    if !strip_file.is_empty() {
        let mut file_rows = parse_accounts(&raw);
        for idx in strip_file {
            if let Some(row) = file_rows.get_mut(idx) {
                row.access_token.clear();
                row.refresh_token.clear();
                row.id_token.clear();
            }
        }
        // Best-effort: a failed rewrite keeps the plaintext fallback.
        if let Ok(body) = serde_json::to_string_pretty(&file_rows) {
            let _ = crate::accounts::write_json_atomic(&accounts_file(base, family), body);
        }
    }
    out.retain(|a| !a.account_id.trim().is_empty() && !a.access_token.trim().is_empty());
    out
}

/// Vault-first save: tokens that reach the vault are stripped from the
/// persisted JSON; entries the vault refused keep their plaintext (the
/// fallback keeps working). Vault slots of cards that left the list are
/// removed so a deleted account leaves nothing behind.
///
/// Ordering is the data-safety property (2026-10-18, GitHub #3): the old
/// file's slot list is snapshotted before it is replaced, but the slots
/// themselves are only dropped AFTER the new file is durably on disk. A
/// failed write therefore leaves the old file and every old vault slot
/// intact — the next load still returns every account with its tokens. A
/// vault write that lands before a failed file write can leave a slot the
/// file does not list yet; the next successful save reconciles it — that
/// reconciliation assumes the same account list is retried, so a retry
/// with a SMALLER list than the failed save leaves the dropped card's slot
/// orphaned (an invisible credential in the OS vault — hygiene, not loss).
pub fn save_to(base: &Path, family: &str, accounts: &[LoginAccount]) -> Result<(), String> {
    let mut rows: Vec<LoginAccount> = accounts.to_vec();
    let keep_scopes: Vec<String> = rows
        .iter()
        .map(|a| vault_scope(family, a))
        .collect();
    for (row, scope) in rows.iter_mut().zip(&keep_scopes) {
        if row.access_token.is_empty() {
            continue;
        }
        let triple = crate::secretstore::TokenTriple {
            access_token: row.access_token.clone(),
            refresh_token: row.refresh_token.clone(),
            id_token: row.id_token.clone(),
        };
        if crate::secretstore::put_triple(scope, &triple) {
            row.access_token.clear();
            row.refresh_token.clear();
            row.id_token.clear();
        }
    }
    // Snapshot the slots the OLD file still owns while it is still readable;
    // the GC below runs only once the replacement file is durable.
    let stale_scopes: Vec<String> = parse_accounts(
        &std::fs::read_to_string(accounts_file(base, family)).unwrap_or_default(),
    )
    .into_iter()
    .map(|old| vault_scope(family, &old))
    .filter(|scope| !keep_scopes.contains(scope))
    .collect();
    std::fs::create_dir_all(base.join("login-accounts"))
        .map_err(|e| format!("create login-accounts dir: {e}"))?;
    // A serialization failure must surface BEFORE anything is written: the
    // old `unwrap_or_default()` would have persisted an empty file (every
    // account gone from the file) and then let the GC below wipe the vault
    // slots — total loss. Practically unreachable (all String/Value
    // fields), but the failure mode is unacceptable, so it is a hard error.
    let body = serde_json::to_string_pretty(&rows)
        .map_err(|e| format!("serialize {family} login accounts: {e}"))?;
    crate::accounts::write_json_atomic(&accounts_file(base, family), body)
        .map_err(|e| format!("write {family} login accounts: {e}"))?;
    // GC: drop vault slots for cards no longer in the file — only now that
    // the new file is safely on disk.
    for scope in stale_scopes {
        crate::secretstore::remove(&crate::secretstore::token_key(&scope));
    }
    Ok(())
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

/// Stable card id for one login account. The identity is the vendor's
/// account id — never the label, never the rotating access token.
pub fn card_id_for_account(family: &str, account: &LoginAccount) -> String {
    let mut material = account.account_id.trim().as_bytes().to_vec();
    material.push(0);
    crate::identity::stable_card_id(family, b"pane-account-id-v1\0", &material)
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

/// account_list rows for one login-backed family — the same {id, label,
/// email, maskedKey} shape the API-key lane emits, so the frontend treats
/// both stores identically. Regression guard for the 2026-10-12 bug:
/// account_list only read the API-key store, so a scanned WorkBuddy login
/// was stored yet invisible to the frontend's account cache (which then
/// purged the account card on every refresh).
pub fn account_rows(family: &str, logins: &[LoginAccount]) -> Vec<Value> {
    let display_name = crate::provider_catalog::provider_definition(family)
        .map(|d| d.display_name)
        .unwrap_or(family);
    logins
        .iter()
        .map(|login| {
            serde_json::json!({
                "id": card_id_for_account(family, login),
                "label": if login.label.trim().is_empty() {
                    if login.email.trim().is_empty() {
                        format!("{display_name} 账号")
                    } else {
                        login.email.clone()
                    }
                } else {
                    login.label.clone()
                },
                "email": login.email,
                "maskedKey": mask_token(&login.access_token),
                "baseUrl": Value::Null,
            })
        })
        .collect()
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
    let _mutation = crate::account_lifecycle::mutation_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_with_imported_single_login_from_unlocked(base, family)
}

/// Caller already owns the account transaction lock.
pub(crate) fn load_with_imported_single_login_from_unlocked(base: &Path, family: &str) -> Vec<LoginAccount> {
    merged_single_login_from(base, family, true)
}

/// Identity checks must not write a captured account list back to disk.
pub(crate) fn preview_single_login_from(base: &Path, family: &str) -> Vec<LoginAccount> {
    merged_single_login_from(base, family, false)
}

fn merged_single_login_from(base: &Path, family: &str, persist: bool) -> Vec<LoginAccount> {
    let mut accounts = read_accounts_from(base, family, persist);
    let Some(tokens) = (if persist { crate::oauth::load_from(&base.join("oauth"), family) } else { crate::oauth::peek_from(&base.join("oauth"), family) }) else {
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
    if persist { let _ = save_to(base, family, &accounts); }
    accounts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_preview_does_not_migrate_credentials_or_rewrite_account_files() {
        let base = std::env::temp_dir().join(format!("pane-login-preview-{}", std::process::id()));
        std::fs::create_dir_all(base.join("login-accounts")).unwrap();
        let row = account("preview-readonly-only", "Preview");
        let scope = crate::secretstore::token_key(&vault_scope("copilot", &row));
        crate::secretstore::remove(&scope);
        let raw = serde_json::to_string_pretty(&vec![row.clone()]).unwrap();
        let path = accounts_file(&base, "copilot");
        std::fs::write(&path, &raw).unwrap();
        let rows = read_accounts_from(&base, "copilot", false);
        assert_eq!(rows[0].access_token, row.access_token);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        assert!(matches!(crate::secretstore::get(&scope), crate::secretstore::Secret::Missing));
    }

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
    fn multi_source_import_preserves_managed_identity_and_fresh_credentials_per_family() {
        for family in ["copilot", "grok", "kiro"] {
            let base = std::env::temp_dir().join(format!("pane-source-order-{family}-{}", std::process::id()));
            std::fs::create_dir_all(base.join("oauth")).unwrap();
            // OAuth slots are provider-scoped even for a temp directory.
            // Clear the harness's in-memory slot left by earlier tests.
            let slot_key = crate::secretstore::token_key(&format!("oauth/{family}"));
            crate::secretstore::remove(&slot_key);
            let id = format!("source-order-{family}");
            save_to(&base, family, &[account(&id, "Managed")]).unwrap();
            std::fs::write(base.join("oauth").join(format!("{family}.json")), serde_json::to_string(&crate::oauth::StoredTokens {
                account_id: Some(id), access_token: "stale-slot".into(), refresh_token: "stale-refresh".into(),
                id_token: None, label: Some("Old slot".into()), expires_at: String::new(),
            }).unwrap()).unwrap();
            for _ in 0..2 {
                let rows = load_with_imported_single_login_from(&base, family);
                assert_eq!(rows.len(), 1, "{family}: duplicate source");
                assert_eq!(rows[0].label, "Managed");
                assert_eq!(rows[0].access_token, "gho_token_value");
            }
            save_to(&base, family, &[]).unwrap();
            crate::secretstore::remove(&slot_key);
            std::fs::remove_dir_all(base).unwrap();
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
        assert_eq!(load_from(&base, "copilot").len(), 1,
            "refresh-token writeback must find a durable account row");

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

    #[test]
    fn account_rows_serve_every_login_family_from_the_login_store() {
        // The 2026-10-12 bug: account_list had no login-store branch, so a
        // scanned WorkBuddy login never reached the frontend (its empty
        // account cache purged the account card on every refresh).
        for family in LOGIN_FAMILIES {
            let rows = account_rows(family, &[account("42", "Named")]);
            assert_eq!(rows.len(), 1, "{family}");
            assert_eq!(rows[0]["id"], serde_json::json!(card_id_for_account(family, &account("42", "Named"))));
            assert_eq!(rows[0]["label"], "Named");
            assert_eq!(rows[0]["maskedKey"], "…alue"); // gho_token_value
            assert!(rows[0]["baseUrl"].is_null(), "{family}");
        }
        // Empty label falls back to the email, then the display name —
        // never a blank tab.
        let by_email = account_rows("workbuddy", &[account("7", "")]);
        assert_eq!(by_email[0]["label"], "user@example.com");
        let mut bare = account("7", "");
        bare.email = String::new();
        let by_name = account_rows("workbuddy", &[bare]);
        assert_eq!(by_name[0]["label"], "WorkBuddy 账号");
    }

    #[test]
    fn catalog_login_only_families_are_all_login_families() {
        // Drift guard: a catalog family that is login-only (no API key,
        // extra accounts) but missing from LOGIN_FAMILIES would fall
        // through account_list to the API-key store and list nothing.
        let own_stores = ["cursor", "antigravity", "codex", "onenewapi"]; // their own stores/lists
        let catalog: Vec<&str> = crate::provider_catalog::provider_definitions()
            .iter()
            .filter(|d| !d.supports_api_key && d.supports_extra_accounts)
            .map(|d| d.family_id)
            .collect();
        let missing: Vec<&str> = catalog
            .iter()
            .copied()
            .filter(|f| !own_stores.contains(f) && !LOGIN_FAMILIES.contains(f))
            .collect();
        assert!(
            missing.is_empty(),
            "login-only families missing from LOGIN_FAMILIES: {missing:?}"
        );
    }

    /// The vault owns the tokens; the file keeps only non-secrets. Saving
    /// strips the file where the vault took over, a later load merges the
    /// pair back, and dropping the account GCs the vault slot.
    #[test]
    fn vault_holds_tokens_and_the_file_keeps_nonsecrets() {
        let base = std::env::temp_dir().join(format!("pane-login-vault-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let mut acct = account("9001", "Named");
        acct.access_token = "gho_vault_pair".into();
        acct.refresh_token = "rt_vault_pair".into();
        let scope = crate::secretstore::token_key(&format!(
            "login/{}",
            card_id_for_account("copilot", &acct)
        ));
        crate::secretstore::remove(&scope);
        save_to(&base, "copilot", &[acct.clone()]).unwrap();
        let file = base.join("login-accounts").join("copilot.json");
        let raw = std::fs::read_to_string(&file).unwrap();
        assert!(raw.contains("Named"), "labels stay behind: {raw}");
        assert!(!raw.contains("gho_vault_pair"), "tokens must not persist: {raw}");
        assert_eq!(
            crate::secretstore::get_triple(&format!(
                "login/{}",
                card_id_for_account("copilot", &acct)
            ))
            .map(|t| t.access_token),
            Some("gho_vault_pair".to_string())
        );
        let back = load_from(&base, "copilot");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].access_token, "gho_vault_pair");
        assert_eq!(back[0].label, "Named");
        // A card removed from the save list leaves nothing in the vault.
        save_to(&base, "copilot", &[]).unwrap();
        assert!(matches!(
            crate::secretstore::get(&scope),
            crate::secretstore::Secret::Missing
        ));
        crate::secretstore::remove(&scope);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A broken vault never loses the credential: the plaintext keeps
    /// serving reads, and the first healthy read migrates and strips.
    /// Harness-gated: the failure injection (`secretstore::fail_puts_for`)
    /// only exists in the harness build, and only there is the vault an
    /// in-memory store a test may break on purpose.
    #[cfg(feature = "harness")]
    #[test]
    fn broken_vault_keeps_plaintext_and_migrates_when_healthy() {
        let base = std::env::temp_dir().join(format!("pane-login-vaultfail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let mut acct = account("9002", "Fallback");
        acct.access_token = "gho_fallback".into();
        let scope = crate::secretstore::token_key(&format!(
            "login/{}",
            card_id_for_account("copilot", &acct)
        ));
        crate::secretstore::remove(&scope);
        std::fs::create_dir_all(base.join("login-accounts")).unwrap();
        let file = base.join("login-accounts").join("copilot.json");
        std::fs::write(&file, serde_json::to_string_pretty(&[acct.clone()]).unwrap()).unwrap();
        // Vault writes fail for THIS slot only → tokens still served, the
        // plaintext file stays intact as the fallback.
        crate::secretstore::fail_puts_for(&scope);
        let during = load_from(&base, "copilot");
        assert_eq!(during.len(), 1);
        assert_eq!(during[0].access_token, "gho_fallback");
        let raw = std::fs::read_to_string(&file).unwrap();
        assert!(raw.contains("gho_fallback"), "fallback keeps the plaintext: {raw}");
        crate::secretstore::clear_put_failures(&scope);
        // A healthy vault migrates on the next read and strips the file.
        let after = load_from(&base, "copilot");
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].access_token, "gho_fallback");
        let raw = std::fs::read_to_string(&file).unwrap();
        assert!(!raw.contains("gho_fallback"), "migrated file must be stripped: {raw}");
        crate::secretstore::remove(&scope);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// A failed file write must not cost accounts (2026-10-18, GitHub #3):
    /// the save surfaces the error, but the old file and every old vault
    /// slot stay intact, so the next load still returns all accounts with
    /// their tokens — and a retry then completes the removal for real.
    /// Harness-gated: the write failure injection only exists in the
    /// harness build, and only there is the vault an in-memory store.
    #[cfg(feature = "harness")]
    #[test]
    fn failed_file_write_keeps_every_account_recoverable() {
        let base = std::env::temp_dir().join(format!("pane-login-writefail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let a = account("7101", "Keep A");
        let b = account("7102", "Drop B");
        let scope_b = crate::secretstore::token_key(&format!(
            "login/{}",
            card_id_for_account("copilot", &b)
        ));
        crate::secretstore::remove(&scope_b);
        save_to(&base, "copilot", &[a.clone(), b.clone()]).unwrap();
        let file = base.join("login-accounts").join("copilot.json");

        // Dropping B fails at the file write: B's vault slot must survive.
        crate::accounts::fail_json_writes_for(&file);
        let err = save_to(&base, "copilot", &[a.clone()]);
        assert!(err.is_err(), "the injected write failure must surface");
        crate::accounts::clear_json_write_failure_for(&file);

        // Old file untouched and B's slot alive: a load still returns both.
        let during = load_from(&base, "copilot");
        assert_eq!(during.len(), 2, "a failed save must not lose accounts");
        assert!(during
            .iter()
            .any(|x| x.account_id == "7102" && x.access_token == "gho_token_value"));

        // The retry completes the removal for real: one account, no residue.
        save_to(&base, "copilot", &[a.clone()]).unwrap();
        let after = load_from(&base, "copilot");
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].account_id, "7101");
        assert!(matches!(
            crate::secretstore::get(&scope_b),
            crate::secretstore::Secret::Missing
        ));
        crate::secretstore::remove(&scope_b);
        let _ = std::fs::remove_dir_all(&base);
    }
}
