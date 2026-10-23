//! Shared command core for Pane-managed sign-ins. No WebView/Tauri dependency:
//! the parsing harness exercises the same save/rollback paths as the commands.
use std::path::Path;
use crate::{accounts, codex_accounts, login_accounts, oauth};
use login_accounts::LoginAccount;

/// Serialize command mutations with the final snapshot/history write. Never
/// hold this guard over a network request or an async suspension point.
pub(crate) fn mutation_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    &LOCK
}

pub fn manages(family: &str) -> bool {
    family == "codex" || login_accounts::takes_login_accounts(family)
}

fn load(base: &Path, family: &str) -> Result<Vec<LoginAccount>, String> {
    if family == "codex" {
        codex_accounts::load_with_imported_single_login_from_unlocked(base).into_iter()
            .map(|a| serde_json::to_value(a).and_then(serde_json::from_value).map_err(|e| e.to_string()))
            .collect()
    } else if login_accounts::takes_login_accounts(family) {
        Ok(login_accounts::load_with_imported_single_login_from_unlocked(base, family))
    } else {
        Err("this family has no managed login store".into())
    }
}

fn save(base: &Path, family: &str, rows: &[LoginAccount]) -> Result<(), String> {
    if family == "codex" {
        let rows = rows.iter().map(|a| serde_json::to_value(a).and_then(serde_json::from_value))
            .collect::<Result<Vec<codex_accounts::CodexLogin>, _>>().map_err(|e| e.to_string())?;
        codex_accounts::save_accounts_to(base, &rows)
    } else {
        login_accounts::save_to(base, family, rows)
    }
}

fn card_id(family: &str, row: &LoginAccount) -> String {
    if family == "codex" { codex_accounts::card_id_for_account_id(&row.account_id) }
    else { login_accounts::card_id_for_account(family, row) }
}

fn retire_legacy(base: &Path, family: &str, row: &LoginAccount) -> Result<(), String> {
    if !crate::provider_catalog::is_web_session(family) {
        oauth::logout_matching_from(&base.join("oauth"), family, &row.account_id, &row.access_token)?;
    }
    Ok(())
}

pub fn delete_from(base: &Path, family: &str, id: &str) -> Result<Option<usize>, String> {
    let mut rows = load(base, family)?;
    let index = rows.iter().position(|row| card_id(family, row) == id);
    if let Some(index) = index {
        // Removing the source before replacing the active list prevents lazy
        // import from resurrecting the row. A failed save retains the active row.
        retire_legacy(base, family, &rows[index])?;
        if family == "codex" {
            codex_accounts::suppress_backup_import_from(base, &rows[index].account_id)?;
        }
        rows.remove(index);
        save(base, family, &rows)?;
    }
    accounts::remove_archived_external_from(base, id)?;
    accounts::remove_archived_from(base, id)?;
    Ok(index)
}

pub fn archive_from(base: &Path, family: &str, id: &str) -> Result<usize, String> {
    let mut rows = load(base, family)?;
    let index = rows.iter().position(|row| card_id(family, row) == id).ok_or("account no longer exists")?;
    let row = rows.remove(index);
    let archived = accounts::ArchivedExternalAccount {
        provider: family.into(), card_id: id.into(),
        label: if row.label.is_empty() { row.email.clone() } else { row.label.clone() },
        archived_at: chrono::Utc::now().timestamp_millis(),
        payload: serde_json::to_value(&row).map_err(|e| e.to_string())?,
    };
    accounts::archive_external_with(base, archived, || {
        retire_legacy(base, family, &row)?;
        if family == "codex" {
            codex_accounts::suppress_backup_import_from(base, &row.account_id)?;
        }
        save(base, family, &rows)
    })?;
    Ok(index)
}

pub fn restore_from(base: &Path, family: &str, id: &str) -> Result<(), String> {
    let archived = accounts::load_archived_external_from(base).into_iter()
        .find(|a| a.provider == family && a.card_id == id).ok_or("archived account no longer exists")?;
    let row: LoginAccount = serde_json::from_value(archived.payload).map_err(|e| format!("restore login: {e}"))?;
    if card_id(family, &row) != id { return Err("archived account identity mismatch".into()); }
    let mut rows = load(base, family)?;
    if rows.iter().any(|r| card_id(family, r) == id) { return Err("account already exists".into()); }
    let old = rows.clone();
    rows.push(row);
    save(base, family, &rows)?;
    if let Err(error) = accounts::remove_archived_external_from(base, id) {
        save(base, family, &old).map_err(|rollback| format!("{error}; restore rollback failed: {rollback}"))?;
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(tag: &str) -> (std::path::PathBuf, LoginAccount, String) {
        let base = std::env::temp_dir().join(format!("pane-lifecycle-{tag}-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let row = LoginAccount {
            account_id: format!("lifecycle-{tag}"), email: "synthetic@example.test".into(),
            label: "Test account".into(), access_token: format!("synthetic-secret-{tag}"),
            refresh_token: "synthetic-csrf".into(), id_token: String::new(),
            expires_at: String::new(), added_at: 1, extra: json!(null),
        };
        let id = card_id("tokenrhythm", &row);
        (base, row, id)
    }

    #[test]
    fn web_accounts_archive_restore_delete_without_exposing_sessions() {
        let (base, a, id) = fixture("roundtrip");
        let mut b = a.clone(); b.account_id.push_str("-other"); b.access_token = "other-cookie".into();
        save(&base, "tokenrhythm", &[a.clone(), b.clone()]).unwrap();
        assert_eq!(archive_from(&base, "tokenrhythm", &id).unwrap(), 0);
        assert_eq!(load(&base, "tokenrhythm").unwrap()[0].account_id, b.account_id);
        let raw = std::fs::read_to_string(base.join("archived_external_accounts.json")).unwrap();
        assert!(!raw.contains(&a.access_token));
        assert!(!raw.contains(&a.refresh_token));
        restore_from(&base, "tokenrhythm", &id).unwrap();
        let rows = load(&base, "tokenrhythm").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].access_token, a.access_token);
        assert!(accounts::load_archived_external_from(&base).is_empty());
        assert_eq!(delete_from(&base, "tokenrhythm", &id).unwrap(), Some(1));
        let other = card_id("tokenrhythm", &b);
        delete_from(&base, "tokenrhythm", &other).unwrap();
        assert!(load(&base, "tokenrhythm").unwrap().is_empty(), "last account stays deleted");
        assert!(matches!(crate::secretstore::get(&crate::secretstore::token_key(&format!("login/{id}"))), crate::secretstore::Secret::Missing));
    }

    #[test]
    fn codex_delete_suppresses_the_local_backup_fallback_from_returning() {
        let (base, row, _) = fixture("codex-suppress");
        let id = card_id("codex", &row);
        save(&base, "codex", &[row.clone()]).unwrap();
        assert_eq!(delete_from(&base, "codex", &id).unwrap(), Some(0));
        assert!(codex_accounts::suppressed_ids(&base).contains(&row.account_id),
            "a deliberate delete must tombstone the .pane-bak import");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    #[cfg(feature = "harness")]
    fn failed_archive_keeps_active_and_preserves_an_existing_tombstone() {
        let (base, a, id) = fixture("archive-fail");
        save(&base, "tokenrhythm", &[a.clone()]).unwrap();
        let file = base.join("login-accounts/tokenrhythm.json");
        accounts::fail_json_writes_for(&file);
        let result = archive_from(&base, "tokenrhythm", &id);
        accounts::clear_json_write_failure_for(&file);
        assert!(result.is_err());
        assert_eq!(load(&base, "tokenrhythm").unwrap().len(), 1);
        assert!(accounts::load_archived_external_from(&base).is_empty());
        let tombstone = accounts::ArchivedExternalAccount {
            provider: "tokenrhythm".into(), card_id: id.clone(), label: "Earlier archive".into(),
            archived_at: 1, payload: serde_json::to_value(&a).unwrap(),
        };
        accounts::archive_external_to(&base, tombstone).unwrap();
        accounts::fail_json_writes_for(&file);
        let result = archive_from(&base, "tokenrhythm", &id);
        accounts::clear_json_write_failure_for(&file);
        assert!(result.is_err());
        assert_eq!(accounts::load_archived_external_from(&base)[0].label, "Earlier archive");
    }

    #[test]
    #[cfg(feature = "harness")]
    fn failed_restore_rolls_back_active_and_retains_credential() {
        let (base, a, id) = fixture("restore-fail");
        save(&base, "tokenrhythm", &[a.clone()]).unwrap();
        archive_from(&base, "tokenrhythm", &id).unwrap();
        let file = base.join("archived_external_accounts.json");
        accounts::fail_json_writes_for(&file);
        let result = restore_from(&base, "tokenrhythm", &id);
        accounts::clear_json_write_failure_for(&file);
        assert!(result.is_err());
        assert!(load(&base, "tokenrhythm").unwrap().is_empty());
        assert_eq!(accounts::load_archived_external_from(&base)[0].payload["access_token"], a.access_token);
        restore_from(&base, "tokenrhythm", &id).unwrap();
        assert_eq!(load(&base, "tokenrhythm").unwrap().len(), 1);
    }

    #[test]
    fn deleting_login_retires_only_its_matching_legacy_slot() {
        for family in ["codex", "copilot"] {
            let (base, a, _) = fixture(family);
            save(&base, family, &[a.clone()]).unwrap();
            let dir = base.join("oauth"); std::fs::create_dir_all(&dir).unwrap();
            let tokens = json!({"access_token": a.access_token, "refresh_token": "synthetic",
                "account_id": a.account_id, "expires_at": ""});
            std::fs::write(dir.join(format!("{family}.json")), tokens.to_string()).unwrap();
            let id = card_id(family, &a);
            delete_from(&base, family, &id).unwrap();
            assert!(load(&base, family).unwrap().is_empty(), "legacy must not resurrect {family}");
            assert!(oauth::load_from(&dir, family).is_none());
        }
    }
}
