//! Cursor account store — multi-account support for the Cursor family,
//! mirroring cockpit-tools' cursor_account.rs (import formats included).
//! Token pairs live in the OS credential vault (one `token:cursor/<card_id>`
//! slot per account, see secretstore.rs); `%APPDATA%\Pane\cursor-accounts.json`
//! keeps only non-secrets plus a card-id snapshot (the card id hashes the
//! access token, so the snapshot routes the vault lookup once the token is
//! gone from the file). Each account's quota is queried independently of
//! whatever account the local editor is logged into.

use crate::cursor_oauth::CursorAccount;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn accounts_file(base: &Path) -> PathBuf {
    base.join("cursor-accounts.json")
}

/// File row: the account plus a non-secret card-id snapshot. The card id
/// hashes the access token, so once the tokens live in the vault the file
/// row can no longer derive it — the snapshot routes the vault lookup
/// (same serialized shape as before: the snapshot is one additive field).
#[derive(Serialize, Deserialize, Clone)]
struct CursorRow {
    #[serde(flatten)]
    account: CursorAccount,
    #[serde(rename = "cardId", default, skip_serializing_if = "Option::is_none")]
    card_id: Option<String>,
}

fn parse_rows(raw: &str) -> Vec<CursorRow> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
}

/// Vault slot for one card's token pair, routed by the card id (a hash of
/// the access token — non-secret, already displayed as the card identity).
fn vault_scope(card_id: &str) -> String {
    format!("cursor/{card_id}")
}

/// Read-through to the vault (see login_accounts::load_from): vault tokens
/// win, plaintext rows migrate lazily, a broken vault keeps the file
/// working. Rows whose tokens are gone everywhere are dropped, as before.
pub fn load_accounts_from(base: &Path) -> Vec<CursorAccount> {
    read_accounts_from(base, true)
}

pub(crate) fn preview_accounts_from(base: &Path) -> Vec<CursorAccount> {
    read_accounts_from(base, false)
}

fn read_accounts_from(base: &Path, migrate: bool) -> Vec<CursorAccount> {
    let raw = std::fs::read_to_string(accounts_file(base)).unwrap_or_default();
    let mut rows = parse_rows(&raw);
    // `rows` keep the merged (usable) tokens for the caller; `disk_rows` is
    // the file-shape copy written back after a migration — the returned
    // accounts must never be the stripped ones.
    let mut disk_rows = rows.clone();
    let mut dirty = false;
    for (row, disk) in rows.iter_mut().zip(disk_rows.iter_mut()) {
        let card = row
            .card_id
            .clone()
            .unwrap_or_else(|| card_id_for_account(&row.account));
        let scope = vault_scope(&card);
        if let Some(triple) = crate::secretstore::get_triple(&scope) {
            row.account.access_token = triple.access_token;
            if !triple.refresh_token.is_empty() {
                row.account.refresh_token = Some(triple.refresh_token);
            }
            if migrate && row.card_id.is_none() {
                disk.card_id = Some(card);
                dirty = true;
            }
            continue;
        }
        if migrate && !row.account.access_token.is_empty() {
            let triple = crate::secretstore::TokenTriple {
                access_token: row.account.access_token.clone(),
                refresh_token: row.account.refresh_token.clone().unwrap_or_default(),
                id_token: String::new(),
            };
            if crate::secretstore::put_triple(&scope, &triple) {
                disk.account.access_token.clear();
                disk.account.refresh_token = None;
                disk.card_id = Some(card);
                dirty = true;
            }
        }
    }
    if dirty {
        // Best-effort rewrite: keeps only the non-secret row shape.
        if let Ok(body) = serde_json::to_string_pretty(&disk_rows) {
            let _ = crate::accounts::write_json_atomic(&accounts_file(base), body);
        }
    }
    rows.into_iter()
        .map(|r| r.account)
        .filter(|a| !a.access_token.trim().is_empty())
        .collect()
}

/// Vault-first save (see login_accounts::save_to): the pair reaches the
/// vault and the file row keeps the card-id snapshot with empty tokens;
/// entries the vault refused keep their plaintext. Vault slots of cards
/// that left the list are removed when their card id is still derivable.
pub fn save_accounts_to(base: &Path, accounts: &[CursorAccount]) -> Result<(), String> {
    let mut rows: Vec<CursorRow> = accounts
        .iter()
        .map(|a| CursorRow { account: a.clone(), card_id: Some(card_id_for_account(a)) })
        .collect();
    for row in rows.iter_mut() {
        if row.account.access_token.is_empty() {
            continue;
        }
        let card = row.card_id.clone().unwrap_or_default();
        let triple = crate::secretstore::TokenTriple {
            access_token: row.account.access_token.clone(),
            refresh_token: row.account.refresh_token.clone().unwrap_or_default(),
            id_token: String::new(),
        };
        if crate::secretstore::put_triple(&vault_scope(&card), &triple) {
            row.account.access_token.clear();
            row.account.refresh_token = None;
        }
    }
    let old_rows = parse_rows(&std::fs::read_to_string(accounts_file(base)).unwrap_or_default());
    std::fs::create_dir_all(base).map_err(|e| format!("create config dir: {e}"))?;
    crate::accounts::write_json_atomic(
        &accounts_file(base),
        serde_json::to_string_pretty(&rows).unwrap_or_default(),
    )
    .map_err(|e| format!("write cursor accounts: {e}"))?;
    for old in old_rows {
        // A pre-snapshot legacy row with tokens still names its card (via
        // the token); a stripped row without a snapshot can't — its slot
        // stays behind, inert.
        let card = old
            .card_id
            .unwrap_or_else(|| card_id_for_account(&old.account));
        if rows.iter().any(|r| r.card_id.as_deref() == Some(card.as_str())) {
            continue;
        }
        crate::secretstore::remove(&crate::secretstore::token_key(&vault_scope(&card)));
    }
    Ok(())
}

pub fn load_accounts() -> Vec<CursorAccount> {
    load_accounts_from(&crate::providers::config_dir())
}

pub fn save_accounts(accounts: &[CursorAccount]) -> Result<(), String> {
    save_accounts_to(&crate::providers::config_dir(), accounts)
}


/// Stable card id keyed on the access token (the identity Cursor mints per
/// login; refresh rotation keeps the same access until re-login). Same
/// two-lane FNV scheme as accounts.rs.
pub fn card_id_for_account(account: &CursorAccount) -> String {
    let material = account.access_token.trim().as_bytes();
    crate::identity::stable_card_id("cursor", b"pane-cursor-account-v1\0", material)
}

/// "ya29…abcd" style mask: recognizable head, ellipsis, last 4 chars.
pub fn mask_token(token: &str) -> String {
    let token = token.trim();
    let chars: Vec<char> = token.chars().collect();
    if chars.len() < 4 {
        return "…".into();
    }
    let tail: String = chars[chars.len() - 4..].iter().collect();
    if chars.len() >= 12 {
        format!("{}…{tail}", chars[..4].iter().collect::<String>())
    } else {
        format!("…{tail}")
    }
}

// ---------------------------------------------------------------------------
// Import (cockpit-compatible field aliases, cursor_account.rs:919-1030)
// ---------------------------------------------------------------------------

fn extract_string<'a>(obj: &'a serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = obj.get(*key).and_then(|v| v.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn payload_from_import_value(raw: &serde_json::Value) -> Result<CursorAccount, String> {
    let obj = raw
        .as_object()
        .ok_or_else(|| "Cursor 导入 JSON 必须是对象".to_string())?;
    // email is best-effort: a bare token paste has none — the label
    // (or the token fingerprint) is the display identity then.
    let email = extract_string(obj, &["email", "cachedEmail", "cursor_email"]).unwrap_or_default();
    let access_token = extract_string(
        obj,
        &["access_token", "accessToken", "token", "cursor_access_token"],
    )
    .ok_or_else(|| "缺少 access_token 字段".to_string())?;
    let label = extract_string(obj, &["name", "displayName", "label"]).unwrap_or_else(|| email.clone());
    let refresh_token = extract_string(
        obj,
        &["refresh_token", "refreshToken", "cursor_refresh_token"],
    );
    let auth_id = extract_string(obj, &["auth_id", "authId", "workos_id", "workosId"]);
    let membership = extract_string(
        obj,
        &[
            "membership_type",
            "membershipType",
            "stripeMembershipType",
            "plan",
        ],
    )
    .map(|m| normalize_membership(&m));

    Ok(CursorAccount {
        label,
        email,
        auth_id,
        access_token,
        refresh_token,
        membership,
        captured_at: chrono::Utc::now().timestamp(),
    })
}

/// cockpit's normalizeCursorMembershipType: student→pro, business/team→
/// enterprise, case/underscore-insensitive.
fn normalize_membership(raw: &str) -> String {
    let normalized = raw.to_ascii_lowercase().trim().to_string();
    match normalized.as_str() {
        "pro_student" | "pro_trial" => "Pro".into(),
        "business" | "team" => "Enterprise".into(),
        "free" | "free_trial" => "Free".into(),
        "pro" => "Pro".into(),
        "pro_plus" => "Pro+".into(),
        "ultra" => "Ultra".into(),
        other if other.is_empty() => "Unknown".into(),
        other => {
            // Title-case a freeform tier name.
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => "Unknown".into(),
            }
        }
    }
}

/// Imports one account or a JSON array/object of accounts (cockpit's exact
/// accepted shapes: a single object, an array, or an object wrapping
/// `accounts`/`items`).
pub fn import_from_json(json_content: &str) -> Result<Vec<CursorAccount>, String> {
    let value: serde_json::Value = serde_json::from_str(json_content.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("导入 JSON 解析失败: {e}"))?;
    let payloads: Vec<CursorAccount> = match &value {
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return Err("导入数组为空".into());
            }
            items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    payload_from_import_value(item)
                        .map_err(|e| format!("第 {} 条 Cursor 账号解析失败: {}", i + 1, e))
                })
                .collect::<Result<_, _>>()?
        }
        serde_json::Value::Object(_) => {
            if let Ok(payload) = payload_from_import_value(&value) {
                vec![payload]
            } else if let Some(accounts) = value
                .get("accounts")
                .or_else(|| value.get("items"))
                .and_then(|v| v.as_array())
            {
                if accounts.is_empty() {
                    return Err("导入数组为空".into());
                }
                accounts
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        payload_from_import_value(item)
                            .map_err(|e| format!("第 {} 条 Cursor 账号解析失败: {}", i + 1, e))
                    })
                    .collect::<Result<_, _>>()?
            } else {
                return Err("无法解析 Cursor 导入对象".into());
            }
        }
        _ => return Err("Cursor 导入 JSON 必须是对象或数组".into()),
    };

    // Dedupe against existing accounts by token fingerprint.
    let mut accounts = load_accounts();
    let mut added = 0;
    for payload in payloads {
        let id = card_id_for_account(&payload);
        if accounts.iter().any(|a| card_id_for_account(a) == id) {
            continue;
        }
        accounts.push(payload);
        added += 1;
    }
    save_accounts(&accounts)?;
    if added == 0 {
        return Err("导入的账号都已存在".into());
    }
    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "harness")]
    fn a_failed_active_save_does_not_delete_the_cursor_vault_slot() {
        let base = std::env::temp_dir().join(format!("pane-cursor-write-failure-{}", std::process::id()));
        let account = CursorAccount {
            label: "Synthetic".into(), email: "synthetic@example.test".into(), auth_id: None,
            access_token: "synthetic-cursor-write-failure".into(),
            refresh_token: Some("synthetic-refresh".into()), membership: None, captured_at: 0,
        };
        save_accounts_to(&base, &[account.clone()]).unwrap();
        let path = accounts_file(&base);
        crate::accounts::fail_json_writes_for(&path);
        let result = save_accounts_to(&base, &[]);
        crate::accounts::clear_json_write_failure_for(&path);
        assert!(result.is_err());
        let rows = load_accounts_from(&base);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].access_token, account.access_token);
    }

    #[test]
    fn json_import_accepts_cockpit_shapes() {
        let base = std::env::temp_dir().join(format!("pane-cursor-accts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let _ = save_accounts_to(&base, &[]);

        // Single object, camelCase aliases.
        let single = r#"{"email":"a@x.com","accessToken":"tok-a","stripeMembershipType":"pro"}"#;
        let parsed: serde_json::Value = serde_json::from_str(single).unwrap();
        let payload = payload_from_import_value(&parsed).unwrap();
        assert_eq!(payload.email, "a@x.com");
        assert_eq!(payload.membership.as_deref(), Some("Pro"));

        // Array shape.
        let array = r#"[{"email":"a@x.com","access_token":"tok-a"},{"email":"b@x.com","token":"tok-b","plan":"free"}]"#;
        let parsed: serde_json::Value = serde_json::from_str(array).unwrap();
        let items = match &parsed {
            serde_json::Value::Array(items) => items.clone(),
            _ => unreachable!(),
        };
        let payloads: Vec<_> = items
            .iter()
            .map(|i| payload_from_import_value(i).unwrap())
            .collect();
        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[1].membership.as_deref(), Some("Free"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn card_ids_are_stable_and_distinct() {
        let a = CursorAccount {
            label: String::new(),
            email: "a@x.com".into(),
            auth_id: None,
            access_token: "tok-a".into(),
            refresh_token: None,
            membership: None,
            captured_at: 0,
        };
        let b = CursorAccount { access_token: "tok-b".into(), ..a.clone() };
        assert_eq!(card_id_for_account(&a), card_id_for_account(&a.clone()));
        assert_ne!(card_id_for_account(&a), card_id_for_account(&b));
    }

    #[test]
    fn membership_normalization_matches_cockpit() {
        assert_eq!(normalize_membership("pro_student"), "Pro");
        assert_eq!(normalize_membership("PRO"), "Pro");
        assert_eq!(normalize_membership("pro_plus"), "Pro+");
        assert_eq!(normalize_membership("business"), "Enterprise");
        assert_eq!(normalize_membership("free"), "Free");
    }

    #[test]
    fn masking_shows_head_and_tail() {
        assert_eq!(mask_token("abcd1234567890wxyz"), "abcd…wxyz");
        assert_eq!(mask_token("ab"), "…");
    }

    /// The vault owns the token pair; the file row keeps the non-secrets
    /// plus the card-id snapshot, and the merged account still mints the
    /// same card id (the snapshot routes the lookup once the token left
    /// the file).
    #[test]
    fn vault_roundtrip_keeps_the_card_snapshot() {
        let base = std::env::temp_dir().join(format!("pane-cursor-vault-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let account = CursorAccount {
            label: "CursorVault".into(),
            email: "cv@x.com".into(),
            auth_id: Some("au-cv-1".into()),
            access_token: "tok-cursor-vault-1".into(),
            refresh_token: Some("rt-cv-1".into()),
            membership: None,
            captured_at: 0,
        };
        let card = card_id_for_account(&account);
        let scope = crate::secretstore::token_key(&vault_scope(&card));
        crate::secretstore::remove(&scope);
        save_accounts_to(&base, &[account]).unwrap();
        let raw = std::fs::read_to_string(accounts_file(&base)).unwrap();
        assert!(raw.contains("\"cardId\""), "snapshot rides in the file row: {raw}");
        assert!(raw.contains(&card), "snapshot must carry the card id: {raw}");
        assert!(!raw.contains("tok-cursor-vault-1"), "tokens must not persist: {raw}");
        assert!(raw.contains("CursorVault"), "labels stay behind: {raw}");
        let back = load_accounts_from(&base);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].access_token, "tok-cursor-vault-1");
        assert_eq!(back[0].refresh_token.as_deref(), Some("rt-cv-1"));
        assert_eq!(card_id_for_account(&back[0]), card, "card identity must be stable");
        crate::secretstore::remove(&scope);
        let _ = std::fs::remove_dir_all(&base);
    }
}
