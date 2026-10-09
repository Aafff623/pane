//! Qoder login — the CLI's device login against qoder.com, ported from
//! cockpit's `qoder_oauth.rs`: the browser completes a PKCE device
//! selection, and the token arrives through Qoder's own
//! `deviceToken/poll` endpoint (there is no loopback callback — the IDE
//! redirect is the `qoder://` deep link).
//!
//! `start` opens the selection page with an S256 challenge; `poll` asks
//! openapi.qoder.sh for the token (404 = still authorizing) and stores
//! the account under its Qoder user id. The machine-id header variant
//! cockpit prefers when the Qoder app left a machine token on disk is
//! skipped: the no-machine chain is the same flow without that header
//! (cockpit's own fallback), so Pane needs nothing from the IDE install.

use base64::Engine;
use rand_core::{OsRng, RngCore};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LOGIN_BASE_URL: &str = "https://qoder.com/device/selectAccounts";
const IDE_REDIRECT_URI: &str = "qoder://aicoding.aicoding-agent/login-success";
const OPENAPI_BASE: &str = "https://openapi.qoder.sh";
const DEVICE_TOKEN_POLL_PATH: &str = "/api/v1/deviceToken/poll";
const USER_INFO_PATH: &str = "/api/v1/userinfo";
const CHALLENGE_METHOD: &str = "S256";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
/// The remote poll's own pacing — the endpoint is cheap but the UI tick
/// is 2 s, so at most one request per 1.5 s leaves the round trip room.
const REMOTE_POLL_INTERVAL: Duration = Duration::from_millis(1500);

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginStart {
    pub login_id: String,
    pub auth_url: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginPoll {
    pub done: bool,
    pub label: Option<String>,
    pub error: Option<String>,
}

struct Pending {
    login_id: String,
    nonce: String,
    verifier: String,
    next_remote_poll: Instant,
    deadline: Instant,
    done: bool,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random_token() -> String {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    b64url(&bytes)
}

fn code_challenge(verifier: &str) -> String {
    b64url(&Sha256::digest(verifier.as_bytes()))
}

fn pct_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Qoder reports the expiry in epoch MILLISECONDS (its own device token
/// shape) or occasionally RFC3339; the store keeps RFC3339.
fn expiry_to_rfc3339(raw: Option<&str>) -> String {
    let Some(text) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    if let Ok(number) = text.parse::<i64>() {
        let millis = if number > 1_000_000_000_000 { number } else { number.saturating_mul(1000) };
        return chrono::DateTime::from_timestamp_millis(millis)
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339();
    }
    chrono::DateTime::parse_from_rfc3339(text)
        .map(|t| t.to_rfc3339())
        .unwrap_or_default()
}

pub fn start() -> Result<LoginStart, String> {
    let login_id = random_token();
    let nonce = random_token();
    let verifier = random_token();
    let auth_url = format!(
        "{LOGIN_BASE_URL}?nonce={}&challenge={}&challenge_method={CHALLENGE_METHOD}&redirect_uri={}",
        pct_encode(&nonce),
        pct_encode(&code_challenge(&verifier)),
        pct_encode(IDE_REDIRECT_URI),
    );
    let mut guard = PENDING.lock().map_err(|_| "login state poisoned".to_string())?;
    *guard = Some(Pending {
        login_id: login_id.clone(),
        nonce,
        verifier,
        next_remote_poll: Instant::now(),
        deadline: Instant::now() + LOGIN_TIMEOUT,
        done: false,
    });
    Ok(LoginStart { login_id, auth_url })
}

/// One remote poll: 404 means "still authorizing"; a token payload ends
/// the login and lands in the qoder account store.
async fn poll_remote(nonce: &str, verifier: &str) -> Result<Option<String>, String> {
    let resp = crate::providers::http()
        .get(format!("{OPENAPI_BASE}{DEVICE_TOKEN_POLL_PATH}"))
        .timeout(Duration::from_secs(20))
        .query(&[
            ("nonce", nonce),
            ("verifier", verifier),
            ("challenge_method", CHALLENGE_METHOD),
        ])
        .send()
        .await
        .map_err(|e| format!("device token poll: {e}"))?;
    if resp.status().as_u16() == 404 {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(format!("device token poll: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("device token parse: {e}"))?;
    let token = doc
        .get("token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or("device token response missing token")?;

    let account_id = doc
        .get("user_id")
        .or_else(|| doc.get("userId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or("device token response missing user id")?;
    let refresh_token = doc
        .get("refresh_token")
        .or_else(|| doc.get("refreshToken"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let expires_at = expiry_to_rfc3339(
        doc.get("expires_at")
            .or_else(|| doc.get("expiresAt"))
            .and_then(|v| v.as_str()),
    );

    // /userinfo carries the email and display name; the login still
    // succeeds when it doesn't answer.
    let (email, name) = match crate::providers::http()
        .get(format!("{OPENAPI_BASE}{USER_INFO_PATH}"))
        .header("Authorization", format!("Bearer {token}"))
        .timeout(Duration::from_secs(15))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let doc: Value = resp.json().await.unwrap_or(Value::Null);
            let pick = |key: &str| {
                doc.get(key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            };
            (pick("email").unwrap_or_default(), pick("name").unwrap_or_default())
        }
        _ => (String::new(), String::new()),
    };
    let label = if !email.is_empty() {
        email.clone()
    } else if !name.is_empty() {
        name.clone()
    } else {
        format!("Qoder {}", &account_id[..account_id.len().min(12)])
    };

    crate::login_accounts::record_login(
        "qoder",
        crate::login_accounts::LoginAccount {
            account_id,
            email,
            label: label.clone(),
            access_token: token,
            refresh_token,
            id_token: String::new(),
            expires_at,
            added_at: chrono::Utc::now().timestamp(),
            extra: doc,
        },
    )?;
    Ok(Some(label))
}

pub async fn poll(login_id: &str) -> LoginPoll {
    // Snapshot the pending state; the network call happens outside the
    // lock so a 2 s UI tick never blocks the cancel path.
    let (nonce, verifier, deadline_expired, done) = {
        let Ok(mut guard) = PENDING.lock() else {
            return LoginPoll { done: false, label: None, error: Some("login state poisoned".into()) };
        };
        match guard.as_mut() {
            Some(p) if p.login_id == login_id => (
                p.nonce.clone(),
                p.verifier.clone(),
                Instant::now() > p.deadline,
                p.done,
            ),
            _ => {
                return LoginPoll { done: false, label: None, error: Some("no pending sign-in".into()) };
            }
        }
    };
    if done {
        return LoginPoll { done: true, label: None, error: None };
    }
    if deadline_expired {
        finish(login_id);
        return LoginPoll {
            done: false,
            label: None,
            error: Some("sign-in timed out — start again".into()),
        };
    }

    // Pace the remote poll internally; the UI tick is only a nudge.
    {
        let Ok(mut guard) = PENDING.lock() else {
            return LoginPoll { done: false, label: None, error: None };
        };
        let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id) else {
            return LoginPoll { done: false, label: None, error: Some("no pending sign-in".into()) };
        };
        if Instant::now() < p.next_remote_poll {
            return LoginPoll { done: false, label: None, error: None };
        }
        p.next_remote_poll = Instant::now() + REMOTE_POLL_INTERVAL;
    }

    match poll_remote(&nonce, &verifier).await {
        Ok(Some(label)) => {
            finish(login_id);
            LoginPoll { done: true, label: Some(label), error: None }
        }
        Ok(None) => LoginPoll { done: false, label: None, error: None },
        Err(e) => LoginPoll { done: false, label: None, error: Some(e) },
    }
}

fn finish(login_id: &str) {
    if let Ok(mut guard) = PENDING.lock() {
        if guard.as_ref().is_some_and(|p| p.login_id == login_id) {
            *guard = None;
        }
    }
}

pub fn cancel(login_id: &str) {
    finish(login_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s256_challenge_matches_the_rfc_vector() {
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn expiry_accepts_millis_and_rfc3339() {
        let from_ms = expiry_to_rfc3339(Some("1772323200000"));
        assert!(chrono::DateTime::parse_from_rfc3339(&from_ms).is_ok());
        assert_eq!(expiry_to_rfc3339(Some("2026-03-01T00:00:00Z")), "2026-03-01T00:00:00+00:00");
        assert_eq!(expiry_to_rfc3339(None), "");
        assert_eq!(expiry_to_rfc3339(Some("  ")), "");
    }
}
