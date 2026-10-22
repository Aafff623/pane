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
pub const AUTH_FAMILIES: [&str; 15] = [
    "antigravity", "codebuddy", "codebuddycn", "codex", "copilot", "cursor", "grok", "kiro", "qoder",
    "trae", "windsurf", "workbuddy", "workbuddyai", "zed", "tokenrhythm",
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
    /// Unix milliseconds of the last successful snapshot for this card id.
    /// Filled by the command layer from last_snapshots.json; None means the
    /// card was never queried (not "queried and empty").
    #[serde(rename = "lastOkAt", skip_serializing_if = "Option::is_none")]
    pub last_ok_at: Option<i64>,
    /// Persisted by the CodeBuddy/WorkBuddy refresh path when a token
    /// refresh is rejected. None means no flag — never a claim of health.
    #[serde(rename = "needsRelogin", skip_serializing_if = "Option::is_none")]
    pub needs_relogin: Option<bool>,
    /// Coarse, non-secret class paired with needsRelogin
    /// (refresh_rejected / network / vendor / unknown).
    #[serde(rename = "lastErrorKind", skip_serializing_if = "Option::is_none")]
    pub last_error_kind: Option<String>,
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
    /// A local row whose store exposed no reliable identity (email/uid).
    /// It is shown with a "verify this is the same account" marker instead
    /// of being silently folded into an unrelated managed account.
    #[serde(
        rename = "identityUnknown",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub identity_unknown: bool,
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
                last_ok_at: None,
                needs_relogin: None,
                last_error_kind: None,
                kind: "slot",
                source: None,
                enterprise_name: None,
                hint: None,
                identity_unknown: false,
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
                last_ok_at: None,
                needs_relogin: None,
                last_error_kind: None,
                kind: "account",
                source: None,
                enterprise_name: None,
                hint: None,
                identity_unknown: false,
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
                last_ok_at: None,
                needs_relogin: None,
                last_error_kind: None,
                kind: "account",
                source: None,
                enterprise_name: None,
                hint: None,
                identity_unknown: false,
            })
            .collect(),
        "codebuddy" | "codebuddycn" | "workbuddy" | "workbuddyai" | "copilot" | "grok" | "kiro"
        | "qoder" | "trae" | "windsurf" | "zed" | "tokenrhythm" => {
            crate::login_accounts::load_with_imported_single_login_from(base, family)
                .iter()
                .map(|login| {
                    let needs_relogin = login
                        .extra
                        .get("needsRelogin")
                        .and_then(serde_json::Value::as_bool)
                        .filter(|flag| *flag);
                    AuthAccountRow {
                        id: crate::login_accounts::card_id_for_account(family, login),
                        label: login.label.clone(),
                        email: (!login.email.trim().is_empty()).then(|| login.email.clone()),
                        masked_key: crate::login_accounts::mask_token(&login.access_token),
                        captured_at: Some(login.added_at).filter(|ts| *ts > 0),
                        last_ok_at: None,
                        needs_relogin,
                        last_error_kind: needs_relogin
                            .and_then(|_| extra_text(&login.extra, "lastErrorKind")),
                        kind: "account",
                        source: extra_text(&login.extra, "sourceKind"),
                        enterprise_name: extra_text(&login.extra, "enterpriseName"),
                        hint: None,
                        identity_unknown: false,
                    }
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// A local store's email is only an identity when it actually looks like
/// one. "unknown", a blank string, or a nickname that merely contains '@'
/// must never be used to merge two accounts.
fn reliable_local_email(email: Option<&str>) -> Option<&str> {
    let email = email?.trim();
    if email.is_empty() || !email.contains('@') || email.eq_ignore_ascii_case("unknown") {
        return None;
    }
    Some(email)
}

fn append_local_rows(rows: &mut Vec<AuthAccountRow>, candidates: Vec<crate::local_logins::LocalLogin>) {
    for candidate in candidates {
        // The same sign-in can be known twice: once as a Pane-managed
        // account and once in the official client's store. Merge only on a
        // reliable identity (email today). A local row without one is NOT
        // necessarily the managed account already in this group — keep it
        // visible and mark it unverified instead of hiding a second account.
        // CN editions are separate products and always keep their own row.
        let email = reliable_local_email(candidate.email.as_deref());
        let duplicate = !candidate.always_listed
            && email.is_some_and(|email| {
                rows.iter().any(|row| {
                    row.email
                        .as_deref()
                        .is_some_and(|existing| existing.eq_ignore_ascii_case(email))
                })
            });
        if duplicate {
            continue;
        }
        rows.push(AuthAccountRow {
            id: candidate.id.to_string(),
            label: candidate.label,
            email: email.map(str::to_string),
            masked_key: "—".into(),
            captured_at: None,
            last_ok_at: None,
            needs_relogin: None,
            last_error_kind: None,
            kind: "local",
            source: None,
            enterprise_name: None,
            hint: Some(candidate.hint),
            identity_unknown: !candidate.always_listed && email.is_none(),
        });
    }
}

/// Fill each row's last successful snapshot time by card id. The snapshot
/// cache lives in lib.rs, so the command passes a lookup instead of this
/// module reaching into Tauri state.
pub fn attach_last_ok_at(
    groups: &mut [AuthFamilyGroup],
    at: impl Fn(&str) -> Option<i64>,
) {
    for group in groups {
        for row in &mut group.accounts {
            row.last_ok_at = at(&row.id);
        }
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
            let mut local_rows = local(family);
            // Codex email can span multiple workspaces. Preserve the explicit
            // read-only local source even when a Pane subscription shares it.
            if *family == "codex" { for row in &mut local_rows { row.always_listed = true; } }
            append_local_rows(&mut accounts, local_rows);
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
    fn tokenrhythm_group_reads_multiple_managed_web_sessions() {
        let base = temp_base("tokenrhythm");
        let rows: Vec<crate::login_accounts::LoginAccount> = ["web-a", "web-b"].iter().map(|id|
            serde_json::from_value(serde_json::json!({
                "account_id": id, "label": id, "access_token": format!("synthetic-{id}-cookie")
            })).unwrap()).collect();
        crate::login_accounts::save_to(&base, "tokenrhythm", &rows).unwrap();
        let group = collect_from(&base).into_iter().find(|g| g.family == "tokenrhythm").unwrap();
        assert_eq!(group.accounts.len(), 2);
        assert_ne!(group.accounts[0].id, group.accounts[1].id);
        let public = serde_json::to_string(&group).unwrap();
        assert!(!public.contains("synthetic-web-a-cookie"));
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
    fn codex_local_source_is_visible_beside_a_managed_account_with_the_same_email() {
        let base = temp_base("codex-source-coexist");
        crate::codex_accounts::save_accounts_to(&base, &[crate::codex_accounts::CodexLogin {
            account_id: "workspace-other".into(), email: "same@example.com".into(), label: "Subscription".into(),
            access_token: "synthetic-access".into(), refresh_token: "synthetic-refresh".into(),
            id_token: String::new(), expires_at: String::new(), added_at: 0,
        }]).unwrap();
        let groups = collect_with(&base, |family| (family == "codex").then(|| local("codex", Some("same@example.com"), false)).into_iter().collect());
        let rows = &groups.iter().find(|g| g.family == "codex").unwrap().accounts;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|row| row.kind == "local" && row.id == "codex"));
        assert!(rows.iter().any(|row| row.id.starts_with("codex@")));
        crate::codex_accounts::save_accounts_to(&base, &[]).unwrap();
        let _ = std::fs::remove_dir_all(base);
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
        assert!(qoder.accounts[0].identity_unknown, "no email to match on");
        assert!(!qoder.accounts[1].identity_unknown, "CN is always listed");
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

        // No reliable local email: this is not evidence of the same identity.
        // Keep both rows and mark the local one unverified instead of hiding
        // a second account behind the managed one.
        let groups = collect_with(&base, |family| {
            (family == "qoder").then(|| local("qoder", None, false)).into_iter().collect()
        });
        let qoder = groups.iter().find(|g| g.family == "qoder").unwrap();
        assert_eq!(qoder.accounts.len(), 2);
        assert_eq!(qoder.accounts[0].kind, "account");
        assert_eq!(qoder.accounts[1].kind, "local");
        assert!(qoder.accounts[1].identity_unknown);
        assert_eq!(qoder.accounts[1].email, None);

        // A malformed or placeholder email is not an identity either.
        for bad in ["unknown", "not-an-email", "  "] {
            let groups = collect_with(&base, |family| {
                (family == "qoder").then(|| local("qoder", Some(bad), false)).into_iter().collect()
            });
            let qoder = groups.iter().find(|g| g.family == "qoder").unwrap();
            assert_eq!(qoder.accounts.len(), 2, "{bad:?} must not merge");
            assert!(qoder.accounts[1].identity_unknown, "{bad:?}");
        }

        // Email identity is case-insensitive.
        let groups = collect_with(&base, |family| {
            (family == "qoder").then(|| local("qoder", Some("Q@EXAMPLE.COM"), false)).into_iter().collect()
        });
        let qoder = groups.iter().find(|g| g.family == "qoder").unwrap();
        assert_eq!(qoder.accounts.len(), 1);
        assert!(!qoder.accounts[0].identity_unknown);

        // The CN edition is a different product: it keeps its row.
        let groups = collect_with(&base, |family| {
            (family == "qoder").then(|| local("qodercn", None, true)).into_iter().collect()
        });
        let qoder = groups.iter().find(|g| g.family == "qoder").unwrap();
        assert_eq!(qoder.accounts.len(), 2);
        assert_eq!(qoder.accounts[1].id, "qodercn");
        assert!(!qoder.accounts[1].identity_unknown);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn login_health_flags_surface_on_the_row() {
        let base = temp_base("login-health");
        let account = |extra: serde_json::Value| crate::login_accounts::LoginAccount {
            account_id: "uid-health".into(),
            email: "h@example.com".into(),
            label: "Health".into(),
            access_token: "tok".into(),
            refresh_token: "ref".into(),
            id_token: String::new(),
            expires_at: String::new(),
            added_at: 1_700_000_000,
            extra,
        };
        crate::login_accounts::save_to(
            &base,
            "workbuddy",
            &[account(serde_json::json!({
                "needsRelogin": true,
                "lastErrorKind": "refresh_rejected"
            }))],
        )
        .expect("seed relogin");
        let groups = collect_from(&base);
        let row = &groups.iter().find(|g| g.family == "workbuddy").unwrap().accounts[0];
        assert_eq!(row.needs_relogin, Some(true));
        assert_eq!(row.last_error_kind.as_deref(), Some("refresh_rejected"));
        assert_eq!(row.last_ok_at, None);

        crate::login_accounts::save_to(&base, "workbuddy", &[account(serde_json::json!({}))])
            .expect("seed healthy");
        let groups = collect_from(&base);
        let row = &groups.iter().find(|g| g.family == "workbuddy").unwrap().accounts[0];
        assert_eq!(row.needs_relogin, None);
        assert_eq!(row.last_error_kind, None);

        // A stale lastErrorKind without the flag is not a relogin state.
        crate::login_accounts::save_to(
            &base,
            "workbuddy",
            &[account(serde_json::json!({ "lastErrorKind": "network" }))],
        )
        .expect("seed stale kind");
        let groups = collect_from(&base);
        let row = &groups.iter().find(|g| g.family == "workbuddy").unwrap().accounts[0];
        assert_eq!(row.needs_relogin, None);
        assert_eq!(row.last_error_kind, None);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn last_ok_at_attaches_by_card_id() {
        let base = temp_base("last-ok");
        crate::login_accounts::save_to(
            &base,
            "workbuddy",
            &[crate::login_accounts::LoginAccount {
                account_id: "uid-ok".into(),
                email: String::new(),
                label: "OK".into(),
                access_token: "tok".into(),
                refresh_token: String::new(),
                id_token: String::new(),
                expires_at: String::new(),
                added_at: 1_700_000_000,
                extra: serde_json::json!({}),
            }],
        )
        .expect("seed");
        let mut groups = collect_from(&base);
        let id = groups
            .iter()
            .find(|g| g.family == "workbuddy")
            .unwrap()
            .accounts[0]
            .id
            .clone();
        attach_last_ok_at(&mut groups, |card| (card == id).then_some(1_700_000_123_000));
        let row = &groups.iter().find(|g| g.family == "workbuddy").unwrap().accounts[0];
        assert_eq!(row.last_ok_at, Some(1_700_000_123_000));

        attach_last_ok_at(&mut groups, |_| None);
        let row = &groups.iter().find(|g| g.family == "workbuddy").unwrap().accounts[0];
        assert_eq!(row.last_ok_at, None);
        let _ = std::fs::remove_dir_all(&base);
    }
}
