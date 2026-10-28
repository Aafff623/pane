//! Pane-managed Codex logins — multi-account support for the Codex family.
//!
//! Every completed Pane sign-in appends (or refreshes) one account here,
//! keyed by the ChatGPT account id, so any number of subscriptions can be
//! monitored in parallel no matter which one the CLI itself is signed into.
//! Card ids share the discovery scheme in `providers::codex`
//! (`codex@<hash8>` of the account id), so a Pane login and a discovered
//! CLI home for the SAME account never double-card, and the pre-multi-
//! account single OAuth slot imports lazily and retires after durable storage.
//!
//! Tokens live in the OS credential vault (one `token:codex/<card_id>`
//! slot per account, see secretstore.rs); `%APPDATA%\Pane\codex-accounts.json`
//! keeps only non-secrets (labels, emails, expiry). Legacy plaintext token
//! entries migrate to the vault on first read.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
    /// Lowest-priority fallback; retired permanently when a live default appears.
    #[serde(default)]
    pub imported_from_backup: bool,
    /// Bumped on every credential rotation (refresh writeback, re-login).
    /// A task waiting on the per-account refresh lock compares the
    /// generation it observed before waiting with the stored one after: an
    /// advanced generation plus a valid token means another task already
    /// rotated the pair, so the waiter reuses instead of burning the
    /// refreshed refresh_token a second time (#63-05).
    #[serde(default)]
    pub generation: u64,
}

fn accounts_file(base: &Path) -> PathBuf {
    base.join("codex-accounts.json")
}

/// Account ids the user removed from the store. A Pane backup import must not
/// resurrect a row the user deliberately deleted; re-login adds the account
/// back to the store and the normal known-set check wins.
fn suppressed_file(base: &Path) -> PathBuf {
    base.join("codex-backup-suppressed.json")
}

pub(crate) fn suppressed_ids(base: &Path) -> HashSet<String> {
    std::fs::read_to_string(suppressed_file(base))
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn save_suppressed(base: &Path, ids: &HashSet<String>) -> Result<(), String> {
    let mut list: Vec<String> = ids.iter().cloned().collect();
    list.sort();
    std::fs::create_dir_all(base).map_err(|e| format!("create config dir: {e}"))?;
    crate::accounts::write_json_atomic(
        &suppressed_file(base),
        serde_json::to_string_pretty(&list).unwrap_or_default(),
    )
}

/// A deliberate delete/archive must keep the local `.pane-bak` fallback from
/// re-importing the same ChatGPT account. Incidental list changes never
/// tombstone: a transient vault read failure must not burn the account away.
pub(crate) fn suppress_backup_import_from(base: &Path, account_id: &str) -> Result<(), String> {
    let mut suppressed = suppressed_ids(base);
    if !suppressed.insert(account_id.to_string()) {
        return Ok(());
    }
    save_suppressed(base, &suppressed)
}

/// Tolerates a UTF-8 BOM like every other Pane JSON store.
pub fn parse_accounts(raw: &str) -> Vec<CodexLogin> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
}

/// Vault slot for one card's tokens; the card id derives from the stable
/// (non-secret) ChatGPT account id, so the scope survives token stripping.
fn vault_scope(account_id: &str) -> String {
    format!("codex/{}", card_id_for_account_id(account_id))
}

/// Read-through to the vault (see login_accounts::load_from): vault tokens
/// win, plaintext file entries migrate lazily, a broken vault falls back
/// to the file.
pub fn load_accounts_from(base: &Path) -> Vec<CodexLogin> {
    read_accounts_from(base, true)
}

fn read_accounts_from(base: &Path, migrate: bool) -> Vec<CodexLogin> {
    let raw = std::fs::read_to_string(accounts_file(base)).unwrap_or_default();
    let mut out = parse_accounts(&raw);
    let mut strip_file: Vec<usize> = Vec::new();
    for (idx, account) in out.iter_mut().enumerate() {
        let scope = vault_scope(&account.account_id);
        if let Some(triple) = crate::secretstore::get_triple(&scope) {
            let has_plaintext = !account.access_token.is_empty() || !account.refresh_token.is_empty() || !account.id_token.is_empty();
            account.access_token = triple.access_token;
            account.refresh_token = triple.refresh_token;
            account.id_token = triple.id_token;
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
        if let Ok(body) = serde_json::to_string_pretty(&file_rows) {
            let _ = crate::accounts::write_json_atomic(&accounts_file(base), body);
        }
    }
    out.retain(|a| !a.account_id.trim().is_empty() && !a.access_token.trim().is_empty());
    out
}

/// Vault-first save (see login_accounts::save_to): stripped where the
/// vault took over, plaintext kept where it refused; vault slots of cards
/// that left the list are removed.
pub fn save_accounts_to(base: &Path, accounts: &[CodexLogin]) -> Result<(), String> {
    let mut rows: Vec<CodexLogin> = accounts.to_vec();
    let keep_scopes: Vec<String> = rows
        .iter()
        .map(|a| vault_scope(&a.account_id))
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
    let previous = parse_accounts(&std::fs::read_to_string(accounts_file(base)).unwrap_or_default());
    std::fs::create_dir_all(base).map_err(|e| format!("create config dir: {e}"))?;
    crate::accounts::write_json_atomic(
        &accounts_file(base), serde_json::to_string_pretty(&rows).unwrap_or_default(),
    ).map_err(|e| format!("write codex accounts: {e}"))?;
    for old in previous {
        let scope = vault_scope(&old.account_id);
        if !keep_scopes.contains(&scope) {
            crate::secretstore::remove(&crate::secretstore::token_key(&scope));
        }
    }
    Ok(())
}

pub fn load_accounts() -> Vec<CodexLogin> {
    load_accounts_from(&crate::providers::config_dir())
}

/// Presence-only check for probes: rows in the store file, no vault reads.
pub fn has_accounts() -> bool {
    let raw = std::fs::read_to_string(accounts_file(&crate::providers::config_dir()))
        .unwrap_or_default();
    !parse_accounts(&raw).is_empty()
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

/// Drop a cached extra-account card whose fingerprint is the account the
/// bare `codex` card already renders. Live fan-out dedupes default versus
/// store by account id; the first-paint cache must apply the same rule or
/// one ChatGPT account shows as two cards until the fetch lands.
pub fn remove_default_identity_card(ids: &mut HashSet<String>, default_account_id: Option<&str>) {
    if let Some(account_id) = default_account_id {
        ids.remove(&card_id_for_account_id(account_id));
    }
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
        let promoted = existing.imported_from_backup && !incoming.imported_from_backup;
        let label = if !existing.label.trim().is_empty() && !(promoted && existing.label == format!("{} · 本机", existing.email)) {
            std::mem::take(&mut existing.label)
        } else {
            incoming.label
        };
        let added_at = existing.added_at;
        // A re-login replaces the credential pair outright; advance the
        // generation so a concurrent refresh waiter sees the rotation (#63-05).
        let generation = existing.generation.saturating_add(1);
        *existing = CodexLogin { label, added_at, generation, ..incoming };
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
        imported_from_backup: false,
        generation: 0,
    })
}

/// One completed Pane sign-in → the store (called from the OAuth poll).
pub fn record_login(tokens: &crate::oauth::StoredTokens) -> Result<(), String> {
    let _mutation = crate::account_lifecycle::mutation_lock().lock().unwrap_or_else(|e| e.into_inner());
    record_login_from(&crate::providers::config_dir(), tokens)
}

pub(crate) fn record_login_from(base: &Path, tokens: &crate::oauth::StoredTokens) -> Result<(), String> {
    let login = from_stored(tokens)?;
    let mut accounts = load_accounts_from(base);
    upsert(&mut accounts, login.clone());
    save_accounts_to(base, &accounts)?;
    crate::oauth::logout_matching_from(&base.join("oauth"), "codex", &login.account_id, &login.access_token)
}

/// A refresh owns only the credential version it sent. Re-login/rename and
/// deletion during the network wait must survive the eventual writeback.
pub(crate) fn save_refreshed_from(base: &Path, expected: &CodexLogin, refreshed: &CodexLogin) -> Result<CodexLogin, String> {
    let _mutation = crate::account_lifecycle::mutation_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut rows = load_accounts_from(base);
    let current = rows.iter_mut().find(|row| row.account_id == expected.account_id)
        .ok_or("Codex account was removed while refreshing — retry after adding it again")?;
    if current.access_token != expected.access_token || current.refresh_token != expected.refresh_token {
        return Ok(current.clone());
    }
    current.access_token = refreshed.access_token.clone();
    current.refresh_token = refreshed.refresh_token.clone();
    current.id_token = refreshed.id_token.clone();
    current.expires_at = refreshed.expires_at.clone();
    // The credential pair rotated: advance the generation so any task that
    // observed the old one before waiting on the refresh lock reuses this
    // result instead of consuming the rotated refresh_token again (#63-05).
    current.generation = current.generation.saturating_add(1);
    let saved = current.clone();
    save_accounts_to(base, &rows)?;
    Ok(saved)
}

/// The store plus a lazy import of the pre-multi-account single OAuth slot:
/// the old `%APPDATA%\Pane\oauth\codex.json` login becomes the first
/// account, and the legacy slot is retired only after durable account storage.
pub fn load_with_imported_single_login() -> Vec<CodexLogin> {
    load_with_imported_single_login_from(&crate::providers::config_dir())
}

/// The `_from` variant keeps tests on a temp dir (the auth-center
/// convention).
pub fn load_with_imported_single_login_from(base: &Path) -> Vec<CodexLogin> {
    let _mutation = crate::account_lifecycle::mutation_lock().lock().unwrap_or_else(|e| e.into_inner());
    load_with_imported_single_login_from_unlocked(base)
}

/// Caller already owns the account transaction lock.
pub(crate) fn load_with_imported_single_login_from_unlocked(base: &Path) -> Vec<CodexLogin> {
    merged_single_login_from(base, true)
}

/// Identity checks must not write a captured account list back to disk.
pub(crate) fn preview_single_login_from(base: &Path) -> Vec<CodexLogin> {
    merged_single_login_from(base, false)
}

fn merge_backup_logins(
    accounts: &mut Vec<CodexLogin>,
    default_id: Option<String>,
    backups: Vec<CodexLogin>,
    suppressed: &mut HashSet<String>,
) -> bool {
    let mut changed = false;
    // Upgrade only the old generated backup rows, not arbitrary same-email
    // web workspaces. Re-login replaces this marker with a managed login.
    for row in accounts.iter_mut() {
        if !row.imported_from_backup && backups.iter().any(|backup|
            backup.account_id == row.account_id
                && row.label == format!("{} · 本机", row.email)) {
            row.imported_from_backup = true;
            changed = true;
        }
    }
    if default_id.is_some() {
        // Remember even backups not imported yet; a later missing auth.json
        // must not revive a fallback that the live default already superseded.
        for id in backups.iter().map(|a| &a.account_id)
            .chain(accounts.iter().filter(|a| a.imported_from_backup).map(|a| &a.account_id)) {
            changed |= suppressed.insert(id.clone());
        }
    }
    let before = accounts.len();
    let web_ids: HashSet<String> = accounts.iter().filter(|a| !a.imported_from_backup)
        .map(|a| a.account_id.clone()).collect();
    accounts.retain(|row| !row.imported_from_backup ||
        (default_id.is_none() && !suppressed.contains(&row.account_id) && !web_ids.contains(&row.account_id)));
    changed |= before != accounts.len();
    if default_id.is_some() {
        return changed;
    }
    let mut known: HashSet<String> = accounts.iter().map(|a| a.account_id.clone()).collect();
    for mut login in backups {
        if suppressed.contains(&login.account_id) {
            continue;
        }
        if known.insert(login.account_id.clone()) {
            login.imported_from_backup = true;
            accounts.push(login);
            changed = true;
        }
    }
    changed
}

fn merged_single_login_from(base: &Path, persist: bool) -> Vec<CodexLogin> {
    let mut accounts = read_accounts_from(base, persist);
    let single = (if persist {
        crate::oauth::load_from(&base.join("oauth"), "codex")
    } else {
        crate::oauth::peek_from(&base.join("oauth"), "codex")
    })
    .and_then(|tokens| from_stored(&tokens).ok());

    let mut changed = false;
    if let Some(login) = single.clone() {
        if accounts.iter().any(|a| a.account_id == login.account_id) {
            // The store's copy wins: it is what cards read, and it may carry a
            // user label plus fresher rotated tokens.
            if persist {
                let _ = crate::oauth::logout_matching_from(
                    &base.join("oauth"),
                    "codex",
                    &login.account_id,
                    &login.access_token,
                );
            }
        } else {
            accounts.push(login);
            changed = true;
        }
    }

    // A desktop/CLI login whose live auth.json was renamed to auth.json.pane-bak
    // is still a local configuration account. Import it once; account_id
    // dedup keeps the same ChatGPT account from double-carding across the
    // desktop file, an extra CODEX_HOME, and the Pane web-OAuth store.
    // Unit tests use temp account stores; never pull the developer's real
    // ~/.codex backups into them. Production keeps the local import.
    let backups = if cfg!(test) {
        Vec::new()
    } else {
        crate::providers::codex::backup_auth_logins()
    };
    let mut suppressed = suppressed_ids(base);
    if merge_backup_logins(
        &mut accounts,
        crate::providers::codex::default_identity(),
        backups,
        &mut suppressed,
    ) {
        changed = true;
    }

    if persist && changed && save_suppressed(base, &suppressed).is_ok() {
        // Retiring the source slot is only safe once the store owns the
        // credential; a failed write must leave the single OAuth slot intact.
        if save_accounts_to(base, &accounts).is_ok() {
            if let Some(login) = single {
                if accounts.iter().any(|a| a.account_id == login.account_id) {
                    let _ = crate::oauth::logout_matching_from(
                        &base.join("oauth"),
                        "codex",
                        &login.account_id,
                        &login.access_token,
                    );
                }
            }
        }
    }
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
            imported_from_backup: false,
            generation: 0,
        }
    }

    #[test]
    fn backup_import_live_default_retires_legacy_backup_but_preserves_web() {
        let backup = login("old-backup", "user@example.com · 本机");
        let web = login("web-workspace", "user@example.com");
        let mut rows = vec![web, backup.clone()];
        assert!(merge_backup_logins(&mut rows, Some("desktop-live".into()), vec![backup.clone()], &mut HashSet::new()));
        assert_eq!(rows.len(), 1, "live desktop must retire the old backup, even across account ids");
        assert_eq!(rows[0].account_id, "web-workspace");
        let mut suppressed = HashSet::from([backup.account_id.clone()]);
        assert!(!merge_backup_logins(&mut rows, Some("desktop-live".into()), vec![backup], &mut suppressed));
    }

    #[test]
    fn backup_import_live_default_blocks_new_backup() {
        let mut rows = vec![login("web", "Web")];
        let mut suppressed = HashSet::new();
        let backup = login("old", "Backup");
        assert!(merge_backup_logins(&mut rows, Some("live".into()), vec![backup.clone()], &mut suppressed));
        assert_eq!(rows.len(), 1);
        assert!(suppressed.contains("old"));
        assert!(!merge_backup_logins(&mut rows, None, vec![backup], &mut suppressed));
        assert_eq!(rows.len(), 1, "missing live auth must not resurrect a superseded backup");
    }

    #[test]
    fn backup_import_retirement_survives_restart_and_missing_live_default() {
        let base = std::env::temp_dir().join(format!("pane-backup-retirement-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let backup = login("retirement-backup", "user@example.com · 本机");
        save_accounts_to(&base, &[backup.clone(), login("retirement-web", "Web")]).unwrap();
        let mut rows = load_accounts_from(&base);
        let mut suppressed = HashSet::new();
        assert!(merge_backup_logins(&mut rows, Some("new-desktop".into()), vec![backup.clone()], &mut suppressed));
        save_accounts_to(&base, &rows).unwrap();
        save_suppressed(&base, &suppressed).unwrap();
        assert!(suppressed_ids(&base).contains(&backup.account_id));
        let mut restarted = load_accounts_from(&base);
        assert!(!merge_backup_logins(&mut restarted, None, vec![backup], &mut suppressed_ids(&base)));
        assert_eq!(restarted.len(), 1);
        save_accounts_to(&base, &[]).unwrap();
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn incidental_list_shrink_never_tombstones_a_surviving_account() {
        let base = std::env::temp_dir().join(format!("pane-save-no-tombstone-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let kept = login("survivor", "user@example.com");
        let dropped = login("vault-blip", "other@example.com");
        save_accounts_to(&base, &[kept.clone(), dropped.clone()]).unwrap();
        // A refresh that could not read the vault hands back a shorter list.
        // It may rewrite the file, but it must not burn the backup fallback.
        save_accounts_to(&base, &[kept]).unwrap();
        assert!(!suppressed_ids(&base).contains(&dropped.account_id));
        save_accounts_to(&base, &[]).unwrap();
        let _ = std::fs::remove_dir_all(suppressed_file(&base));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn backup_import_fallback_promotes_to_web_without_losing_workspace() {
        let mut rows = Vec::new();
        assert!(merge_backup_logins(&mut rows, None, vec![login("same", "user@example.com · 本机")], &mut HashSet::new()));
        assert!(rows[0].imported_from_backup);
        let mut web = login("same", "user@example.com");
        web.access_token = "web-fresh".into();
        upsert(&mut rows, web);
        assert!(!rows[0].imported_from_backup);
        assert_eq!(rows[0].label, "user@example.com");
        assert!(merge_backup_logins(&mut rows, Some("desktop".into()), vec![login("same", "user@example.com · 本机")], &mut HashSet::new()));
        assert_eq!(rows[0].access_token, "web-fresh");
    }

    #[test]
    fn backup_import_dedupes_by_chatgpt_account_id() {
        let mut accounts = vec![login("acct-web", "web")];
        let backups = vec![
            login("acct-web", "web duplicate"),
            login("acct-desktop", "desktop"),
        ];
        assert!(super::merge_backup_logins(
            &mut accounts,
            None,
            backups,
            &mut HashSet::new(),
        ));
        assert_eq!(accounts.len(), 2);
        assert!(accounts.iter().any(|a| a.account_id == "acct-desktop"));
        assert_eq!(accounts.iter().filter(|a| a.account_id == "acct-web").count(), 1);
        assert!(!super::merge_backup_logins(
            &mut accounts,
            None,
            vec![login("acct-web", "again")],
            &mut HashSet::new(),
        ));
    }

    #[test]
    fn removing_an_account_suppresses_backup_reimport() {
        let base = std::env::temp_dir().join(format!(
            "pane-codex-suppress-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let a = login("acct-a", "A");
        let b = login("acct-b", "B");
        save_accounts_to(&base, &[a.clone(), b.clone()]).unwrap();
        // Mirrors account_lifecycle::delete_from: tombstone first, then persist
        // the shortened list. A bare list change must not suppress on its own.
        suppress_backup_import_from(&base, "acct-b").unwrap();
        save_accounts_to(&base, &[a.clone()]).unwrap();
        assert!(suppressed_ids(&base).contains("acct-b"));
        let mut accounts = load_accounts_from(&base);
        assert!(!merge_backup_logins(
            &mut accounts,
            None,
            vec![b],
            &mut suppressed_ids(&base),
        ));
        assert!(!accounts.iter().any(|row| row.account_id == "acct-b"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn upsert_dedupes_by_account_and_keeps_user_labels() {
        let mut accounts = vec![login("acct-1", "Work")];
        upsert(&mut accounts, login("acct-1", ""));
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].label, "Work"); // empty incoming keeps stored
        upsert(&mut accounts, login("acct-1", "Renamed"));
        assert_eq!(accounts[0].label, "Work"); // OAuth email never overwrites a custom label
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
    fn refresh_writeback_preserves_newer_login_label_and_deletion() {
        let base = std::env::temp_dir().join(format!("pane-codex-refresh-{}", std::process::id()));
        let original = login("refresh-race-unique", "Work");
        save_accounts_to(&base, &[original.clone()]).unwrap();
        let mut refreshed = original.clone();
        refreshed.access_token = "rotated-access".into();
        refreshed.refresh_token = "rotated-refresh".into();
        let mut renamed = original.clone(); renamed.label = "My subscription".into();
        save_accounts_to(&base, &[renamed]).unwrap();
        let saved = save_refreshed_from(&base, &original, &refreshed).unwrap();
        assert_eq!(saved.label, "My subscription");
        assert_eq!(saved.access_token, "rotated-access");
        let mut relogged = saved.clone(); relogged.access_token = "new-login".into();
        save_accounts_to(&base, &[relogged]).unwrap();
        assert_eq!(save_refreshed_from(&base, &original, &refreshed).unwrap().access_token, "new-login");
        save_accounts_to(&base, &[]).unwrap();
        assert!(save_refreshed_from(&base, &original, &refreshed).is_err());
        assert!(load_accounts_from(&base).is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn credential_rotation_advances_the_generation() {
        let base = std::env::temp_dir().join(format!("pane-codex-generation-{}", std::process::id()));
        let original = login("generation-unique", "Work");
        save_accounts_to(&base, &[original.clone()]).unwrap();
        let mut refreshed = original.clone();
        refreshed.access_token = "rotated-access".into();
        refreshed.refresh_token = "rotated-refresh".into();
        // A successful refresh writeback rotates the pair → generation moves.
        let saved = save_refreshed_from(&base, &original, &refreshed).unwrap();
        assert_eq!(saved.generation, original.generation + 1);
        // A stale writeback (CAS on the already-rotated tokens) must not
        // move the generation a second time.
        let stale = save_refreshed_from(&base, &original, &refreshed).unwrap();
        assert_eq!(stale.generation, saved.generation);
        // A re-login replaces the credential pair too → generation moves.
        let mut relogin = saved.clone();
        relogin.access_token = "relogin-access".into();
        let mut rows = load_accounts_from(&base);
        upsert(&mut rows, relogin);
        assert_eq!(rows[0].generation, saved.generation + 1);
        save_accounts_to(&base, &[]).unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[cfg(feature = "harness")]
    #[test]
    fn failed_delete_save_keeps_the_previous_vault_credentials() {
        let base = std::env::temp_dir().join(format!("pane-codex-failed-delete-{}", std::process::id()));
        let account = login("failed-delete-unique", "Keep me");
        save_accounts_to(&base, &[account]).unwrap();
        crate::accounts::fail_json_writes_for(&accounts_file(&base));
        let result = save_accounts_to(&base, &[]);
        crate::accounts::clear_json_write_failure_for(&accounts_file(&base));
        assert!(result.is_err());
        let rows = load_accounts_from(&base);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].refresh_token, "rt-1");
        save_accounts_to(&base, &[]).unwrap();
        let _ = std::fs::remove_dir_all(base);
    }

    #[cfg(feature = "harness")]
    #[test]
    fn failed_store_write_keeps_the_legacy_single_slot() {
        let base = std::env::temp_dir().join(format!("pane-codex-slot-retain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("oauth")).unwrap();
        let slot_key = crate::secretstore::token_key("oauth/codex");
        crate::secretstore::remove(&slot_key);
        std::fs::write(
            base.join("oauth").join("codex.json"),
            serde_json::to_string(&crate::oauth::StoredTokens {
                account_id: Some("slot-retain".into()), access_token: "slot-access".into(),
                refresh_token: "slot-refresh".into(), id_token: None,
                label: Some("Slot".into()), expires_at: String::new(),
            }).unwrap(),
        ).unwrap();
        crate::accounts::fail_json_writes_for(&accounts_file(&base));
        let rows = load_with_imported_single_login_from(&base);
        crate::accounts::clear_json_write_failure_for(&accounts_file(&base));
        assert_eq!(rows.len(), 1);
        assert!(crate::oauth::peek_from(&base.join("oauth"), "codex").is_some(),
            "the slot is retired only after the store durably owns the credential");
        crate::secretstore::remove(&slot_key);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn completed_login_updates_only_its_account_and_keeps_remarks() {
        let base = std::env::temp_dir().join(format!("pane-codex-relogin-{}", std::process::id()));
        save_accounts_to(&base, &[login("relogin-a", "Custom name"), login("relogin-b", "Other subscription")]).unwrap();
        let tokens = crate::oauth::StoredTokens {
            account_id: Some("relogin-a".into()), access_token: "new-access".into(),
            refresh_token: "new-refresh".into(), id_token: None,
            label: Some("mail@example.com".into()), expires_at: String::new(),
        };
        record_login_from(&base, &tokens).unwrap();
        let rows = load_with_imported_single_login_from(&base);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "Custom name");
        assert_eq!(rows[0].refresh_token, "new-refresh");
        assert_eq!(rows[1].label, "Other subscription");
        save_accounts_to(&base, &[]).unwrap();
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn cached_extra_card_for_the_default_identity_is_removed() {
        let mut ids: HashSet<String> = [
            card_id_for_account_id("97fa8404-8e34-49ea-90f1-86d48d06f9c0"),
            card_id_for_account_id("3581686f-4744-4c2b-8ffb-87e8f8321371"),
        ]
        .into_iter()
        .collect();
        remove_default_identity_card(&mut ids, Some("97fa8404-8e34-49ea-90f1-86d48d06f9c0"));
        assert_eq!(ids.len(), 1);
        assert!(ids.contains("codex@3581686f"));
        // No default identity (missing/broken auth.json) removes nothing.
        remove_default_identity_card(&mut ids, None);
        assert_eq!(ids.len(), 1);
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

    /// The vault owns the tokens; the file keeps only non-secrets, and a
    /// card removed from the save list leaves nothing in the vault.
    #[test]
    fn vault_holds_tokens_and_the_file_keeps_nonsecrets() {
        let base = std::env::temp_dir().join(format!("pane-codex-vault-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let account = login("cx-vault-1", "Vaulted");
        let scope = crate::secretstore::token_key(&vault_scope("cx-vault-1"));
        crate::secretstore::remove(&scope);
        save_accounts_to(&base, &[account]).unwrap();
        let raw = std::fs::read_to_string(accounts_file(&base)).unwrap();
        assert!(raw.contains("Vaulted"), "labels stay behind: {raw}");
        assert!(!raw.contains("eyJhbGciOi.access.sig"), "tokens must not persist: {raw}");
        let back = load_accounts_from(&base);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].access_token, "eyJhbGciOi.access.sig");
        // GC: an emptied save list drops the vault slot too.
        save_accounts_to(&base, &[]).unwrap();
        assert!(matches!(
            crate::secretstore::get(&scope),
            crate::secretstore::Secret::Missing
        ));
        crate::secretstore::remove(&scope);
        let _ = std::fs::remove_dir_all(&base);
    }
}
