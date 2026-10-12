//! WorkBuddy's own desktop sign-in, read straight from the file its
//! file-authentication-storage keeps — the same source magpie reads, so this
//! route needs neither a QR scan nor any reverse engineering.
//!
//! Read-only on purpose: the desktop app owns this session and refreshes it
//! itself (the rule the Qoder CN / Trae CN readers already follow), so Pane
//! never writes back and can never sign the desktop app out.
//!
//! Store shape, `<data local>/CodeBuddyExtension/Data/Public/auth/
//! workbuddy-desktop.info`:
//!
//! ```json
//! { "account": { "uid": "…" },
//!   "auth": { "accessToken": "…", "refreshToken": "…", "expiresAt": 1794181883535,
//!             "refreshExpiresAt": 1794354683535, "domain": "www.workbuddy.cn" } }
//! ```
//!
//! With credential protection on — off by default, but this machine has it on
//! — the token fields become `{ "$wbEncrypted": 1, "envelope": "<base64>" }`
//! objects. Those are deliberately not readable here (decrypting them means
//! running the vendor's own Electron binding), so the caller degrades to its
//! other routes instead.

use serde_json::Value;
use std::path::PathBuf;

/// The session the desktop app is signed in with, as its file records it.
pub struct LocalSession {
    pub uid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub domain: String,
    /// The file stores these as epoch **milliseconds** (not seconds).
    pub expires_at_ms: Option<i64>,
    pub refresh_expires_at_ms: Option<i64>,
}

pub enum LocalRead {
    Session(Box<LocalSession>),
    /// The store is there, but its tokens are encrypted at rest.
    Encrypted,
    Missing,
    Unreadable(String),
}

/// The desktop app's auth store for the WorkBuddy build.
pub fn auth_path() -> Option<PathBuf> {
    Some(
        crate::platform::data_local_home()?
            .join("CodeBuddyExtension")
            .join("Data")
            .join("Public")
            .join("auth")
            .join("workbuddy-desktop.info"),
    )
}

/// Read the desktop app's stored session from disk.
pub fn read() -> LocalRead {
    let Some(path) = auth_path() else {
        return LocalRead::Unreadable("no local data directory".into());
    };
    match std::fs::read_to_string(&path) {
        Ok(raw) => parse_session(&raw),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => LocalRead::Missing,
        Err(error) => LocalRead::Unreadable(error.to_string()),
    }
}

/// One token field: a readable string, the encrypted envelope, or nothing.
enum Field<'a> {
    Plain(&'a str),
    Encrypted,
    Absent,
}

fn field(value: Option<&Value>) -> Field<'_> {
    match value {
        Some(Value::String(text)) if !text.trim().is_empty() => Field::Plain(text.trim()),
        Some(Value::Object(map)) if map.contains_key("$wbEncrypted") => Field::Encrypted,
        _ => Field::Absent,
    }
}

/// Parse the store's JSON. Split out from [`read`] so the shapes can be
/// tested without touching the disk.
pub fn parse_session(raw: &str) -> LocalRead {
    let doc: Value = match serde_json::from_str(raw) {
        Ok(doc) => doc,
        Err(error) => return LocalRead::Unreadable(format!("auth store is not JSON: {error}")),
    };
    let access = field(doc.pointer("/auth/accessToken"));
    let refresh = field(doc.pointer("/auth/refreshToken"));
    if matches!(access, Field::Encrypted) || matches!(refresh, Field::Encrypted) {
        return LocalRead::Encrypted;
    }
    let Field::Plain(access_token) = access else {
        return LocalRead::Unreadable("auth store has no access token".into());
    };
    let uid = doc
        .pointer("/account/uid")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if uid.is_empty() {
        return LocalRead::Unreadable("auth store has no account id".into());
    }
    LocalRead::Session(Box::new(LocalSession {
        uid,
        access_token: access_token.to_string(),
        refresh_token: match refresh {
            Field::Plain(token) => token.to_string(),
            _ => String::new(),
        },
        domain: doc
            .pointer("/auth/domain")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string(),
        expires_at_ms: doc.pointer("/auth/expiresAt").and_then(Value::as_i64),
        refresh_expires_at_ms: doc.pointer("/auth/refreshExpiresAt").and_then(Value::as_i64),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store() -> String {
        json!({
            "account": { "uid": "92aeb9b3-f354-48e8-a97b-fa20710a38fa", "uin": "17836372725" },
            "auth": {
                "accessToken": "  at-value  ",
                "refreshToken": "rt-value",
                "expiresAt": 1_794_181_883_535i64,
                "refreshExpiresAt": 1_794_354_683_535i64,
                "domain": "www.workbuddy.cn",
            },
        })
        .to_string()
    }

    #[test]
    fn a_plain_store_reads_the_session() {
        let LocalRead::Session(s) = parse_session(&store()) else {
            panic!("expected a session");
        };
        assert_eq!(s.uid, "92aeb9b3-f354-48e8-a97b-fa20710a38fa");
        assert_eq!(s.access_token, "at-value"); // trimmed
        assert_eq!(s.refresh_token, "rt-value");
        assert_eq!(s.domain, "www.workbuddy.cn");
        assert_eq!(s.expires_at_ms, Some(1_794_181_883_535));
        assert_eq!(s.refresh_expires_at_ms, Some(1_794_354_683_535));
    }

    #[test]
    fn an_encrypted_store_is_reported_not_guessed() {
        let raw = json!({
            "account": { "uid": "u-1" },
            "auth": {
                "accessToken": { "$wbEncrypted": 1, "envelope": "AAAA" },
                "refreshToken": { "$wbEncrypted": 1, "envelope": "BBBB" },
            },
        })
        .to_string();
        assert!(matches!(parse_session(&raw), LocalRead::Encrypted));
    }

    #[test]
    fn one_encrypted_field_is_enough_to_degrade() {
        let raw = json!({
            "account": { "uid": "u-1" },
            "auth": { "accessToken": "plain", "refreshToken": { "$wbEncrypted": 1 } },
        })
        .to_string();
        assert!(matches!(parse_session(&raw), LocalRead::Encrypted));
    }

    #[test]
    fn a_store_without_an_account_or_token_is_unreadable() {
        for raw in [
            json!({ "auth": { "accessToken": "t" } }).to_string(),
            json!({ "account": { "uid": "u" }, "auth": {} }).to_string(),
            json!({ "account": { "uid": "  " }, "auth": { "accessToken": "t" } }).to_string(),
        ] {
            assert!(matches!(parse_session(&raw), LocalRead::Unreadable(_)), "{raw}");
        }
    }

    #[test]
    fn a_non_json_store_is_unreadable_not_a_panic() {
        assert!(matches!(parse_session("not json"), LocalRead::Unreadable(_)));
    }

    #[test]
    fn a_session_without_a_refresh_token_still_reads() {
        let raw = json!({
            "account": { "uid": "u-1" },
            "auth": { "accessToken": "t", "domain": "www.workbuddy.cn" },
        })
        .to_string();
        let LocalRead::Session(s) = parse_session(&raw) else {
            panic!("expected a session");
        };
        assert_eq!(s.refresh_token, "");
        assert_eq!(s.expires_at_ms, None);
    }
}
