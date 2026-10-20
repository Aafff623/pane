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
pub const AUTH_FAMILIES: [&str; 14] = [
    "antigravity", "codebuddy", "codebuddycn", "codex", "copilot", "cursor", "grok", "kiro", "qoder",
    "trae", "windsurf", "workbuddy", "workbuddyai", "zed",
];

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
    /// How this credential entered Pane: "qr" (scan sign-in) or "desktop"
    /// (imported from the official client's read-only auth store). Only the
    /// login stores carry it; other families omit the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The vendor's enterprise tenant name, when the credential carries one.
    #[serde(rename = "enterpriseName", skip_serializing_if = "Option::is_none")]
    pub enterprise_name: Option<String>,
    /// Which local store an already-signed-in official client was found in.
    /// Only "local" rows carry it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct AuthFamilyGroup {
    pub family: String,
    pub accounts: Vec<AuthAccountRow>,
}

/// One non-secret string from a login account's `extra` blob, or None.
fn extra_text(extra: &serde_json::Value, key: &str) -> Option<String> {
    extra
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn rows_for_family_from(base: &Path, family: &str) -> Vec<AuthAccountRow> {    match family {
        "antigravity" => antigravity_accounts::load_slots_from(base)
            .iter()
            .map(|slot| AuthAccountRow {
                id: antigravity_accounts::card_id_for_slot(slot),
                label: slot.label.clone(),
                email: None,
                masked_key: antigravity_accounts::mask_token(&slot.refresh_token),
                captured_at: Some(slot.captured_at).filter(|ts| *ts > 0),
                kind: "slot",
                source: None,
                enterprise_name: None,
                hint: None,
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
                source: None,
                enterprise_name: None,
                hint: None,
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
                source: None,
                enterprise_name: None,
                hint: None,
            })
            .collect(),
        "codebuddy" | "codebuddycn" | "workbuddy" | "workbuddyai" | "copilot" | "grok" | "kiro"
        | "qoder" | "trae" | "windsurf" | "zed" => {
            crate::login_accounts::load_with_imported_single_login_from(base, family)
                .iter()
                .map(|login| AuthAccountRow {
                    id: crate::login_accounts::card_id_for_account(family, login),
                    label: login.label.clone(),
                    email: (!login.email.trim().is_empty()).then(|| login.email.clone()),
                    masked_key: crate::login_accounts::mask_token(&login.access_token),
                    captured_at: Some(login.added_at).filter(|ts| *ts > 0),
                    kind: "account",
                    source: extra_text(&login.extra, "sourceKind"),
                    enterprise_name: extra_text(&login.extra, "enterpriseName"),
                    hint: None,
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

fn append_local_rows(rows: &mut Vec<AuthAccountRow>, candidates: Vec<crate::local_logins::LocalLogin>) {
    for candidate in candidates {
        // The same sign-in can be known twice: once as a Pane-managed
        // account and once in the official client's store. Match emails
        // first; a local row without an email is only a duplicate when the
        // group already has a Pane-managed account. CN editions are separate
        // products and always keep their own row.
        let duplicate = if candidate.always_listed {
            false
        } else if let Some(email) = candidate.email.as_deref() {
            rows.iter().any(|row| row.email.as_deref() == Some(email))
        } else {
            !rows.is_empty()
        };
        if duplicate {
            continue;
        }
        rows.push(AuthAccountRow {
            id: candidate.id.to_string(),
            label: candidate.label,
            email: candidate.email,
            masked_key: "—".into(),
            captured_at: None,
            kind: "local",
            source: None,
            enterprise_name: None,
            hint: Some(candidate.hint),
        });
    }
}

/// Every auth family with its rows, reading the stores under `base`
/// (the `_from` convention keeps tests on a temp dir).
#[cfg(test)]
pub fn collect_from(base: &Path) -> Vec<AuthFamilyGroup> {
    collect_with(base, |_| Vec::new())
}

/// Like [`collect_from`] but also lists sign-ins found in official clients'
/// local stores, discovered through `local` (injected so tests stay off the
/// real filesystem).
pub fn collect_with<F>(base: &Path, local: F) -> Vec<AuthFamilyGroup>
where
    F: Fn(&str) -> Vec<crate::local_logins::LocalLogin>,
{
    AUTH_FAMILIES
        .iter()
        .map(|family| {
            let mut accounts = rows_for_family_from(base, family);
            append_local_rows(&mut accounts, local(family));
            AuthFamilyGroup {
                family: family.to_string(),
                accounts,
            }
        })
        .collect()
}

pub fn collect() -> Vec<AuthFamilyGroup> {
    collect_with(&crate::providers::config_dir(), crate::local_logins::candidates)
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
        assert!(groups.iter().any(|g| g.family == "grok"));
        assert!(groups.iter().all(|g| g.accounts.is_empty()));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn workbuddy_and_codebuddycn_rows_come_from_the_login_store() {
        // Regression: the login-backed match arm once listed every family
        // EXCEPT workbuddy/codebuddycn, so a scanned WorkBuddy account was
        // stored, rendered on its card, and yet invisible in the auth
        // center (2026-10-12 user report).
        let base = temp_base("workbuddy");
        for family in ["workbuddy", "codebuddycn"] {
            crate::login_accounts::save_to(
                &base,
                family,
                &[crate::login_accounts::LoginAccount {
                    account_id: "uid-1".into(),
                    email: String::new(),
                    label: "1aaac1cc".into(),
                    access_token: "tok".into(),
                    refresh_token: String::new(),
                    id_token: String::new(),
                    expires_at: String::new(),
                    added_at: 1_700_000_000,
                    extra: serde_json::json!({
                        "uid": "uid-1",
                        "sourceKind": "qr",
                        "enterpriseName": "Tencent"
                    }),
                }],
            )
            .expect("seed login");
        }
        let groups = collect_from(&base);
        for family in ["workbuddy", "codebuddycn"] {
            let group = groups.iter().find(|g| g.family == family).expect("group");
            assert_eq!(group.accounts.len(), 1, "{family} must list its login");
            assert_eq!(group.accounts[0].label, "1aaac1cc");
            assert_eq!(group.accounts[0].kind, "account");
            assert_eq!(group.accounts[0].source.as_deref(), Some("qr"));
            assert_eq!(group.accounts[0].enterprise_name.as_deref(), Some("Tencent"));
        }
        // The key-API families carry no channel metadata.
        let antigravity = groups.iter().find(|g| g.family == "antigravity").unwrap();
        assert!(antigravity.accounts.iter().all(|row| row.source.is_none()));
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

    #[test]
    fn every_login_family_is_an_auth_center_family() {
        // Drift guard (2026-10-12 bug class): a family added to the login
        // store but missing from AUTH_FAMILIES would store accounts the
        // auth center never lists.
        for family in crate::login_accounts::LOGIN_FAMILIES {
            assert!(
                AUTH_FAMILIES.contains(&family),
                "{family} is a login family but not an auth-center family"
            );
        }
    }

    fn local(
        id: &'static str,
        email: Option<&str>,
        always: bool,
    ) -> crate::local_logins::LocalLogin {
        crate::local_logins::LocalLogin {
            id,
            label: id.to_string(),
            email: email.map(str::to_string),
            hint: format!("{id} local sign-in"),
            always_listed: always,
        }
    }

    #[test]
    fn local_sign_ins_fold_into_their_group_as_read_only_rows() {
        let base = temp_base("local-rows");
        let groups = collect_with(&base, |family| match family {
            "qoder" => vec![local("qoder", None, false), local("qodercn", None, true)],
            "codex" => vec![local("codex", Some("me@example.com"), false)],
            _ => Vec::new(),
        });
        let qoder = groups.iter().find(|g| g.family == "qoder").unwrap();
        assert_eq!(qoder.accounts.len(), 2);
        assert_eq!(qoder.accounts[0].id, "qoder");
        assert_eq!(qoder.accounts[1].id, "qodercn");
        assert_eq!(qoder.accounts[0].kind, "local");
        assert_eq!(qoder.accounts[0].source, None);
        assert_eq!(
            qoder.accounts[1].hint.as_deref(),
            Some("qodercn local sign-in")
        );
        let codex = groups.iter().find(|g| g.family == "codex").unwrap();
        assert_eq!(codex.accounts[0].label, "codex");
        assert_eq!(codex.accounts[0].email.as_deref(), Some("me@example.com"));

        // Families with managed rows only keep hint=None.
        let groups = collect_from(&base);
        assert!(groups.iter().flat_map(|g| &g.accounts).all(|row| row.hint.is_none()));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn managed_and_local_sign_ins_are_not_double_listed() {
        let base = temp_base("local-dedupe");
        crate::login_accounts::save_to(
            &base,
            "qoder",
            &[crate::login_accounts::LoginAccount {
                account_id: "uid-1".into(),
                email: "q@example.com".into(),
                label: String::new(),
                access_token: "tok".into(),
                refresh_token: String::new(),
                id_token: String::new(),
                expires_at: String::new(),
                added_at: 1_700_000_000,
                extra: serde_json::json!({}),
            }],
        )
        .expect("seed login");

        // Same email: the local row is the same sign-in, hide it.
        let groups = collect_with(&base, |family| {
            (family == "qoder").then(|| local("qoder", Some("q@example.com"), false)).into_iter().collect()
        });
        let qoder = groups.iter().find(|g| g.family == "qoder").unwrap();
        assert_eq!(qoder.accounts.len(), 1);
        assert_eq!(qoder.accounts[0].kind, "account");

        // No local email: any managed account in the group masks it.
        let groups = collect_with(&base, |family| {
            (family == "qoder").then(|| local("qoder", None, false)).into_iter().collect()
        });
        assert_eq!(
            groups.iter().find(|g| g.family == "qoder").unwrap().accounts.len(),
            1
        );

        // The CN edition is a different product: it keeps its row.
        let groups = collect_with(&base, |family| {
            (family == "qoder").then(|| local("qodercn", None, true)).into_iter().collect()
        });
        let qoder = groups.iter().find(|g| g.family == "qoder").unwrap();
        assert_eq!(qoder.accounts.len(), 2);
        assert_eq!(qoder.accounts[1].id, "qodercn");
        let _ = std::fs::remove_dir_all(&base);
    }
}
