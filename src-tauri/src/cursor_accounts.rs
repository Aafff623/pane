//! Cursor account store — multi-account support for the Cursor family,
//! mirroring cockpit-tools' cursor_account.rs (import formats included).
//! Accounts live in %APPDATA%\Pane\cursor-accounts.json; each one carries
//! its own token pair so its quota is queried independently of whatever
//! account the local editor is logged into.

use crate::cursor_oauth::CursorAccount;
use std::path::{Path, PathBuf};

fn accounts_file(base: &Path) -> PathBuf {
    base.join("cursor-accounts.json")
}

/// Tolerates a UTF-8 BOM like every other Pane JSON store.
pub fn parse_accounts(raw: &str) -> Vec<CursorAccount> {
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_default()
}

pub fn load_accounts_from(base: &Path) -> Vec<CursorAccount> {
    let raw = std::fs::read_to_string(accounts_file(base)).unwrap_or_default();
    let mut out = parse_accounts(&raw);
    out.retain(|a| !a.access_token.trim().is_empty());
    out
}

pub fn save_accounts_to(base: &Path, accounts: &[CursorAccount]) -> Result<(), String> {
    std::fs::create_dir_all(base).map_err(|e| format!("create config dir: {e}"))?;
    std::fs::write(
        accounts_file(base),
        serde_json::to_string_pretty(accounts).unwrap_or_default(),
    )
    .map_err(|e| format!("write cursor accounts: {e}"))
}

pub fn load_accounts() -> Vec<CursorAccount> {
    load_accounts_from(&crate::providers::config_dir())
}

pub fn save_accounts(accounts: &[CursorAccount]) -> Result<(), String> {
    save_accounts_to(&crate::providers::config_dir(), accounts)
}

fn fnv1a(bytes: &[u8], mut hash: u64) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// Stable card id keyed on the access token (the identity Cursor mints per
/// login; refresh rotation keeps the same access until re-login). Same
/// two-lane FNV scheme as accounts.rs.
pub fn card_id_for_account(account: &CursorAccount) -> String {
    let material = account.access_token.trim().as_bytes().to_vec();
    let left = fnv1a(&material, 0xcbf2_9ce4_8422_2325);
    let mut second = b"pane-cursor-account-v1\0".to_vec();
    second.extend_from_slice(&material);
    let right = fnv1a(&second, 0x8422_2325_cbf2_9ce4);
    format!("cursor@{left:016x}{right:016x}")
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
}
