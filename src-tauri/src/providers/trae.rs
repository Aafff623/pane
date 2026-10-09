//! Trae — the international edition of ByteDance's Trae IDE (trae.ai; the
//! China edition has its own card, `traecn`).
//!
//! Same storage and crypto as the CN edition: sign-in lives in
//! `%APPDATA%\Trae\User\globalStorage\storage.json` under
//! `iCubeAuthInfo://icube.cloudide`, ByteCrypto-encrypted (see `traecn.rs`
//! for the format). The API host is stored beside the token — international
//! installs report grow-normal.trae.ai / growsg-normal.trae.ai /
//! grow-normal.traeapi.us depending on region. Pane only reads; an expired
//! token means "open Trae once", not a refresh attempt.

use super::traecn::{self, MAX_STORAGE_BYTES};
use super::Snapshot;
use serde_json::Value;
use std::path::Path;

const ID: &str = "trae";
const NAME: &str = "Trae";
const APP_DIR: &str = "Trae";
const AUTH_KEY: &str = "iCubeAuthInfo://icube.cloudide";
const DEFAULT_HOST: &str = "https://grow-normal.trae.ai";
const ENT_USAGE_PATH: &str = "/trae/api/v2/pay/ide_user_ent_usage";
const PAY_STATUS_PATH: &str = "/trae/api/v2/pay/ide_user_pay_status";
/// cockpit (the only reference querying the international API) still uses
/// v1 paths; pane's live-verified CN surface is v2. Try v2 first, v1 as
/// fallback — whichever answers wins.
const ENT_USAGE_PATH_V1: &str = "/trae/api/v1/pay/ide_user_ent_usage";
const PAY_STATUS_PATH_V1: &str = "/trae/api/v1/pay/ide_user_pay_status";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Snapshot for one Pane-managed Trae login (international edition): the
/// same entitlement query the IDE's own token drives, under the
/// account's card id.
pub async fn snapshot_with_login(login: crate::login_accounts::LoginAccount) -> Snapshot {
    let id = crate::login_accounts::card_id_for_account(ID, &login);
    let name = login_card_name(&login);
    let host = login
        .extra
        .get("host")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| s.starts_with("https://"))
        .unwrap_or(DEFAULT_HOST)
        .to_string();
    match fetch_with_token(&login.access_token, &host, &id, &name).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(&id, &name, e),
    }
}

pub(crate) fn login_card_name(login: &crate::login_accounts::LoginAccount) -> String {
    if !login.label.trim().is_empty() {
        return login.label.clone();
    }
    if !login.email.trim().is_empty() {
        return format!("Trae — {}", login.email);
    }
    format!("Trae @{}", &login.account_id[..login.account_id.len().min(12)])
}

/// Pure local probe for the Customize gear panel (no network): the Trae
/// app's storage.json carries an encrypted sign-in blob.
pub fn local_credential_hint() -> Option<String> {
    storage_path()
        .and_then(|p| super::read_small_text(&p, MAX_STORAGE_BYTES, "storage.json").ok())
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|doc| doc.get(AUTH_KEY).is_some())
        .then(|| "Trae app sign-in".to_string())
}

async fn fetch() -> Result<Snapshot, String> {
    // Same as the CN card: the path resolves regardless of existence, so a
    // missing file is "signed out / never signed in", not a raw OS error.
    let Some(path) = storage_path().filter(|p| p.is_file()) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Trae sign-in state not found — the IDE was never signed in on this machine, or the sign-in was dropped after long disuse. Open Trae, sign in again, then refresh.",
        ));
    };
    let (token, host) = read_sign_in(&path)?;
    fetch_with_token(&token, &host, ID, NAME).await
}

/// The entitlement query under one session token, publishing as
/// `id`/`name` (the bare card or one login card).
async fn fetch_with_token(token: &str, host: &str, id: &str, name: &str) -> Result<Snapshot, String> {
    let usage = match traecn::fetch_json(host, ENT_USAGE_PATH, "usage endpoint", token).await {
        Ok(usage) => usage,
        Err(e) if e.contains("HTTP 404") => {
            traecn::fetch_json(host, ENT_USAGE_PATH_V1, "usage endpoint (v1)", token).await?
        }
        Err(e) => return Err(e),
    };
    let pay_status = match traecn::fetch_json(host, PAY_STATUS_PATH, "pay-status endpoint", token).await
    {
        Ok(p) => Some(p),
        Err(e) if e.contains("HTTP 404") => {
            traecn::fetch_json(host, PAY_STATUS_PATH_V1, "pay-status endpoint (v1)", token)
                .await
                .ok()
        }
        Err(_) => None,
    };
    let plan = pay_status.and_then(|p| {
        p.get("user_pay_identity_str")
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let metrics = traecn::credit_metrics(&usage)?;
    Ok(Snapshot::ok(id, name, plan, metrics))
}

/// storage.json → (session token, API host). Same blob shape as the CN
/// card, different app directory and default host.
fn read_sign_in(path: &Path) -> Result<(String, String), String> {
    let raw = super::read_small_text(path, MAX_STORAGE_BYTES, "storage.json")?;
    let doc: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("parse storage.json: {e}"))?;
    let encrypted = doc
        .get(AUTH_KEY)
        .and_then(Value::as_str)
        .ok_or_else(|| "storage.json has no Trae sign-in blob".to_string())?;
    let auth = traecn::decode_auth_blob(encrypted)?;
    let token = auth
        .get("token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "Trae sign-in blob has no token".to_string())?
        .to_string();
    let host = auth
        .get("host")
        .and_then(Value::as_str)
        .filter(|h| h.starts_with("https://"))
        .unwrap_or(DEFAULT_HOST)
        .to_string();
    Ok((token, host))
}

fn storage_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|cfg| {
        cfg.join(APP_DIR)
            .join("User")
            .join("globalStorage")
            .join("storage.json")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a storage.json carrying a ByteCrypto-encrypted international
    /// sign-in blob and read it back through the production path.
    #[test]
    fn reads_an_international_sign_in_blob_with_its_own_host() {
        let dir = std::env::temp_dir().join(format!("pane-trae-intl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("storage.json");

        // Reuse traecn's test encryptor through the round-trip helper: the
        // blob is the same format, only the host differs.
        let plain = br#"{"token":"jwt-intl","host":"https://growsg-normal.trae.ai","userId":"7"}"#;
        let blob = traecn::tests::encrypt_blob_for_tests(plain);
        let doc = serde_json::json!({ AUTH_KEY: blob });
        std::fs::write(&path, serde_json::to_string(&doc).unwrap()).unwrap();

        let (token, host) = read_sign_in(&path).expect("sign-in");
        assert_eq!(token, "jwt-intl");
        assert_eq!(host, "https://growsg-normal.trae.ai");
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn missing_blob_or_token_is_an_error_not_a_panic() {
        let dir = std::env::temp_dir().join(format!("pane-trae-intl-err-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("storage.json");
        std::fs::write(&path, serde_json::to_string(&serde_json::json!({ "other": 1 })).unwrap())
            .unwrap();
        assert!(read_sign_in(&path).is_err());
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }
}
