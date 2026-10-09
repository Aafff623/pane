//! Auth center (panel window, M3): grouped account rows for the
//! authorization page. Read-only assembly over the existing credential
//! stores — antigravity slots, cursor accounts, and the Pane-managed login
//! stores of codex/copilot/grok (every completed sign-in is one account
//! row; the legacy single OAuth slots import lazily). Storage logic stays
//! in the owning modules; this file only shapes what they already load.
//!
//! Kept free of Tauri types so the parse-tests harness compiles it via
//! #[path], same convention as accounts.rs.

use serde::Serialize;
use std::path::Path;

use crate::{antigravity_accounts, cursor_accounts};

/// The families the auth center groups, in display order. "grok" is the
/// xAI family id (its device-code flow lives in oauth.rs under that name).
pub const AUTH_FAMILIES: [&str; 6] = ["antigravity", "codex", "copilot", "cursor", "grok", "kiro"];

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct AuthAccountRow {
    /// The card id this account publishes under (family@<fingerprint> for
    /// parallel accounts, the bare family id for an OAuth login) — the id
    /// its usage snapshot is cached against.
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "maskedKey")]
    pub masked_key: String,
    /// Unix seconds when the credential was captured/imported. Device-code
    /// OAuth logins record no date, so the row omits the segment.
    #[serde(rename = "capturedAt", skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<i64>,
    /// "slot" (antigravity capture) | "account" (cursor import) |
    /// "oauth" (Pane device-code login).
    pub kind: &'static str,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct AuthFamilyGroup {
    pub family: String,
    pub accounts: Vec<AuthAccountRow>,
}

fn rows_for_family_from(base: &Path, family: &str) -> Vec<AuthAccountRow> {
    match family {
        "antigravity" => antigravity_accounts::load_slots_from(base)
            .iter()
            .map(|slot| AuthAccountRow {
                id: antigravity_accounts::card_id_for_slot(slot),
                label: slot.label.clone(),
                email: None,
                masked_key: antigravity_accounts::mask_token(&slot.refresh_token),
                captured_at: Some(slot.captured_at).filter(|ts| *ts > 0),
                kind: "slot",
            })
            .collect(),
        "cursor" => cursor_accounts::load_accounts_from(base)
            .iter()
            .map(|acct| AuthAccountRow {
                id: cursor_accounts::card_id_for_account(acct),
                label: acct.label.clone(),
                email: (!acct.email.trim().is_empty()).then(|| acct.email.clone()),
                masked_key: cursor_accounts::mask_token(&acct.access_token),
                captured_at: Some(acct.captured_at).filter(|ts| *ts > 0),
                kind: "account",
            })
            .collect(),
        "codex" => crate::codex_accounts::load_with_imported_single_login_from(base)
            .iter()
            .map(|login| AuthAccountRow {
                id: crate::codex_accounts::card_id_for_account(login),
                label: login.label.clone(),
                email: (!login.email.trim().is_empty()).then(|| login.email.clone()),
                masked_key: crate::codex_accounts::mask_token(&login.access_token),
                captured_at: Some(login.added_at).filter(|ts| *ts > 0),
                kind: "account",
            })
            .collect(),
        "copilot" | "grok" | "kiro" => {
            crate::login_accounts::load_with_imported_single_login_from(base, family)
                .iter()
                .map(|login| AuthAccountRow {
                    id: crate::login_accounts::card_id_for_account(family, login),
                    label: login.label.clone(),
                    email: (!login.email.trim().is_empty()).then(|| login.email.clone()),
                    masked_key: crate::login_accounts::mask_token(&login.access_token),
                    captured_at: Some(login.added_at).filter(|ts| *ts > 0),
                    kind: "account",
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Every auth family with its rows, reading the stores under `base`
/// (the `_from` convention keeps tests on a temp dir).
pub fn collect_from(base: &Path) -> Vec<AuthFamilyGroup> {
    AUTH_FAMILIES
        .iter()
        .map(|family| AuthFamilyGroup {
            family: family.to_string(),
            accounts: rows_for_family_from(base, family),
        })
        .collect()
}

pub fn collect() -> Vec<AuthFamilyGroup> {
    collect_from(&crate::providers::config_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth;
    use crate::cursor_oauth::CursorAccount;

    fn temp_base(tag: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("pane-auth-center-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        base
    }

    #[test]
    fn empty_store_yields_all_families_with_no_rows() {
        let base = temp_base("empty");
        let groups = collect_from(&base);
        assert_eq!(groups.len(), AUTH_FAMILIES.len());
        assert_eq!(groups[0].family, "antigravity");
        assert_eq!(groups[4].family, "grok");
        assert!(groups.iter().all(|g| g.accounts.is_empty()));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn rows_cover_slots_cursor_accounts_and_oauth_logins() {
        let base = temp_base("full");
        antigravity_accounts::save_slots_to(
            &base,
            &[antigravity_accounts::AgSlot {
                label: "pro-a".into(),
                refresh_token: "1//refresh-token-a".into(),
                access_token: "ya29.aaa".into(),
                expires_at: None,
                captured_at: 1_700_000_000,
            }],
        )
        .expect("slots");
        cursor_accounts::save_accounts_to(
            &base,
            &[CursorAccount {
                label: "work".into(),
                email: "w@example.com".into(),
                auth_id: None,
                access_token: "cursor-access-token".into(),
                refresh_token: None,
                membership: None,
                captured_at: 1_700_000_100,
            }],
        )
        .expect("cursor");
        let oauth_dir = base.join("oauth");
        std::fs::create_dir_all(&oauth_dir).expect("oauth dir");
        std::fs::write(
            oauth_dir.join("codex.json"),
            serde_json::to_string_pretty(&oauth::StoredTokens {
                access_token: "codex-access-token".into(),
                refresh_token: String::new(),
                expires_at: "2030-01-01T00:00:00Z".into(),
                label: Some("me@example.com".into()),
                account_id: Some("acct-1234".into()),
                id_token: None,
            })
            .unwrap(),
        )
        .expect("codex tokens");

        let groups = collect_from(&base);
        let by_family = |f: &str| groups.iter().find(|g| g.family == f).unwrap();

        let ag = &by_family("antigravity").accounts;
        assert_eq!(ag.len(), 1);
        assert_eq!(ag[0].kind, "slot");
        assert_eq!(ag[0].label, "pro-a");
        assert_eq!(ag[0].captured_at, Some(1_700_000_000));
        assert!(ag[0].id.starts_with("antigravity@"));

        let cursor = &by_family("cursor").accounts;
        assert_eq!(cursor.len(), 1);
        assert_eq!(cursor[0].email.as_deref(), Some("w@example.com"));
        assert!(cursor[0].id.starts_with("cursor@"));

        let codex = &by_family("codex").accounts;
        assert_eq!(codex.len(), 1);
        // A codex login is an ACCOUNT row now (multi-account family): its
        // card id follows the codex@<hash8> scheme the fetch path mints.
        assert_eq!(codex[0].kind, "account");
        assert_eq!(codex[0].id, "codex@acct1234");
        assert_eq!(codex[0].label, "me@example.com");
        assert_eq!(codex[0].captured_at.is_some(), true);

        // copilot/grok: the legacy single OAuth slot imports lazily as an
        // ACCOUNT row with a fingerprint card id (multi-account families).
        std::fs::write(
            oauth_dir.join("grok.json"),
            serde_json::to_string_pretty(&oauth::StoredTokens {
                access_token: "grok-access-token".into(),
                refresh_token: "grok-refresh".into(),
                expires_at: "2030-01-01T00:00:00Z".into(),
                label: Some("grok@x.ai".into()),
                account_id: Some("xai-sub-77".into()),
                id_token: None,
            })
            .unwrap(),
        )
        .expect("grok tokens");
        let groups = collect_from(&base);
        let grok = &groups.iter().find(|g| g.family == "grok").unwrap().accounts;
        assert_eq!(grok.len(), 1);
        assert_eq!(grok[0].kind, "account");
        assert!(grok[0].id.starts_with("grok@"));
        assert_ne!(grok[0].id, "grok");

        // Families without a stored login stay empty.
        assert!(by_family("copilot").accounts.is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn zero_capture_dates_are_omitted() {
        let base = temp_base("zero-date");
        antigravity_accounts::save_slots_to(
            &base,
            &[antigravity_accounts::AgSlot {
                label: String::new(),
                refresh_token: "1//refresh-token-b".into(),
                access_token: String::new(),
                expires_at: None,
                captured_at: 0,
            }],
        )
        .expect("slots");
        let groups = collect_from(&base);
        let ag = &groups.iter().find(|g| g.family == "antigravity").unwrap().accounts;
        assert_eq!(ag[0].captured_at, None);
        let _ = std::fs::remove_dir_all(&base);
    }
}
