//! CodeBuddy (Tencent) login — the IDE's scan-login handshake, ported
//! from cockpit's `codebuddy_oauth.rs`: a state request mints the login
//! page, the user signs in there, and the token is collected by polling
//! `auth/token` until the scan completes. There is no loopback callback
//! and no user code to type — the page is the whole ceremony.
//!
//! The account lands in the codebuddy login store under its `uid`, with
//! the tenant/domain headers the billing endpoints need riding in
//! `extra` (see providers::codebuddy).

use base64::Engine;
use rand_core::{OsRng, RngCore};
use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const API_ENDPOINT: &str = "https://www.codebuddy.ai";
const API_PREFIX: &str = "/v2/plugin";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
const REMOTE_POLL_INTERVAL: Duration = Duration::from_millis(1500);
/// Browser identity headers the IDE sends on every plugin call.
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

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
    /// The server-issued state the token poll keys on.
    state: String,
    next_remote_poll: Instant,
    deadline: Instant,
    done: bool,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn random_token() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn http() -> reqwest::Client {
    crate::providers::http()
}

fn plugin_url(path: &str) -> String {
    format!("{API_ENDPOINT}{API_PREFIX}{path}")
}

/// codebuddy.ai answers every plugin call with `{code, data, message}`;
/// 0 and 200 are its two success codes.
fn success_code(body: &Value) -> bool {
    matches!(body.get("code").and_then(Value::as_i64), Some(0) | Some(200))
}

fn data_of(body: &Value) -> Option<&Value> {
    body.get("data").filter(|d| d.is_object())
}

fn as_str(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// The tenant headers the billing endpoints need when a CodeBuddy
/// account belongs to an enterprise tenant.
pub struct Tenant {
    pub uid: Option<String>,
    pub enterprise_id: Option<String>,
    pub domain: Option<String>,
}

impl Tenant {
    /// Read the tenant fields out of a stored account's `extra` blob.
    pub fn from_extra(extra: &Value) -> Tenant {
        Tenant {
            uid: as_str(extra, "uid"),
            enterprise_id: as_str(extra, "enterpriseId").or_else(|| as_str(extra, "enterprise_id")),
            domain: as_str(extra, "domain"),
        }
    }
}

pub async fn start() -> Result<LoginStart, String> {
    let resp = http()
        .post(plugin_url("/auth/state?platform=ide"))
        .header("User-Agent", UA)
        .header("X-No-Authorization", "true")
        .header("X-No-User-Id", "true")
        .header("X-No-Enterprise-Id", "true")
        .header("X-No-Department-Info", "true")
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| format!("auth/state request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("auth/state: HTTP {}", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| format!("auth/state parse: {e}"))?;
    let data = data_of(&body).ok_or("auth/state response has no data")?;
    let state = as_str(data, "state").ok_or("auth/state response has no state")?;
    let auth_url = as_str(data, "authUrl")
        .or_else(|| as_str(data, "auth_url"))
        .or_else(|| as_str(data, "url"))
        .unwrap_or_else(|| format!("{API_ENDPOINT}/login?state={state}"));

    let login_id = random_token();
    let mut guard = PENDING.lock().map_err(|_| "login state poisoned".to_string())?;
    *guard = Some(Pending {
        login_id: login_id.clone(),
        state,
        next_remote_poll: Instant::now(),
        deadline: Instant::now() + LOGIN_TIMEOUT,
        done: false,
    });
    Ok(LoginStart { login_id, auth_url })
}

pub async fn poll(login_id: &str) -> LoginPoll {
    let (state, expired, done, too_soon) = {
        let Ok(mut guard) = PENDING.lock() else {
            return LoginPoll { done: false, label: None, error: Some("login state poisoned".into()) };
        };
        let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id) else {
            return LoginPoll { done: false, label: None, error: Some("no pending sign-in".into()) };
        };
        let too_soon = Instant::now() < p.next_remote_poll;
        if !too_soon {
            p.next_remote_poll = Instant::now() + REMOTE_POLL_INTERVAL;
        }
        (p.state.clone(), Instant::now() > p.deadline, p.done, too_soon)
    };
    if done {
        return LoginPoll { done: true, label: None, error: None };
    }
    if expired {
        finish(login_id);
        return LoginPoll { done: false, label: None, error: Some("sign-in timed out — start again".into()) };
    }
    if too_soon {
        return LoginPoll { done: false, label: None, error: None };
    }

    let resp = match http()
        .get(plugin_url(&format!("/auth/token?state={state}")))
        .header("User-Agent", UA)
        .header("X-No-Authorization", "true")
        .header("X-No-User-Id", "true")
        .header("X-No-Enterprise-Id", "true")
        .header("X-No-Department-Info", "true")
        .send()
        .await
    {
        Ok(resp) => resp,
        // A hiccup mid-scan is not a failure — the next tick retries.
        Err(_) => return LoginPoll { done: false, label: None, error: None },
    };
    let Ok(body) = resp.json::<Value>().await else {
        return LoginPoll { done: false, label: None, error: None };
    };
    if !success_code(&body) {
        return LoginPoll { done: false, label: None, error: None }; // scan not finished
    }
    let Some(data) = data_of(&body) else {
        return LoginPoll { done: false, label: None, error: Some("token response has no data".into()) };
    };
    let access_token = match as_str(data, "accessToken").or_else(|| as_str(data, "access_token")) {
        Some(t) => t,
        None => return LoginPoll { done: false, label: None, error: None }, // code ok, no token yet
    };
    let refresh_token =
        as_str(data, "refreshToken").or_else(|| as_str(data, "refresh_token")).unwrap_or_default();
    let domain = as_str(data, "domain");
    // The IDE reports an absolute expiry in epoch seconds or millis.
    let expires_at = data
        .get("expiresAt")
        .or_else(|| data.get("expireTime"))
        .and_then(Value::as_i64)
        .map(|raw| {
            let millis = if raw > 1_000_000_000_000 { raw } else { raw.saturating_mul(1000) };
            chrono::DateTime::from_timestamp_millis(millis)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339()
        })
        .unwrap_or_default();

    // Account profile: uid / nickname / email / tenant.
    let profile = fetch_account(&access_token, &state, domain.as_deref()).await;
    let uid = profile
        .as_ref()
        .and_then(|p| as_str(p, "uid"))
        .unwrap_or_default();
    let nickname = profile.as_ref().and_then(|p| as_str(p, "nickname")).unwrap_or_default();
    let email = profile.as_ref().and_then(|p| as_str(p, "email")).unwrap_or_default();
    let enterprise_id = profile
        .as_ref()
        .and_then(|p| as_str(p, "enterpriseId").or_else(|| as_str(p, "enterprise_id")))
        .unwrap_or_default();

    let account_id = if !uid.is_empty() {
        uid.clone()
    } else if !email.is_empty() {
        email.clone()
    } else {
        finish(login_id);
        return LoginPoll { done: false, label: None, error: Some("sign-in returned no account identity".into()) };
    };
    let label = if !email.is_empty() {
        email.clone()
    } else if !nickname.is_empty() {
        nickname.clone()
    } else {
        account_id.clone()
    };
    let enterprise_name = profile
        .as_ref()
        .and_then(|p| as_str(p, "enterpriseName").or_else(|| as_str(p, "enterprise_name")))
        .unwrap_or_default();

    let mut extra = serde_json::Map::new();
    extra.insert("uid".into(), Value::String(uid));
    if !enterprise_id.is_empty() {
        extra.insert("enterpriseId".into(), Value::String(enterprise_id));
    }
    if !enterprise_name.is_empty() {
        extra.insert("enterpriseName".into(), Value::String(enterprise_name));
    }
    if let Some(d) = &domain {
        extra.insert("domain".into(), Value::String(d.clone()));
    }

    let poll_label = label.clone();
    let stored = crate::login_accounts::record_login(
        "codebuddy",
        crate::login_accounts::LoginAccount {
            account_id,
            email,
            label,
            access_token,
            refresh_token,
            id_token: String::new(),
            expires_at,
            added_at: chrono::Utc::now().timestamp(),
            extra: Value::Object(extra),
        },
    );
    finish(login_id);
    match stored {
        Ok(()) => LoginPoll { done: true, label: Some(poll_label), error: None },
        Err(e) => LoginPoll { done: false, label: None, error: Some(e) },
    }
}

async fn fetch_account(access_token: &str, state: &str, domain: Option<&str>) -> Option<Value> {
    let mut req = http()
        .get(plugin_url(&format!("/login/account?state={state}")))
        .header("User-Agent", UA)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("X-No-User-Id", "true")
        .header("X-No-Enterprise-Id", "true")
        .header("X-No-Department-Info", "true");
    if let Some(d) = domain {
        req = req.header("X-Domain", d);
    }
    let resp = req.send().await.ok()?;
    let body: Value = resp.json().await.ok()?;
    data_of(&body).cloned()
}

/// One refresh round-trip: the IDE carries the refresh token in its own
/// header (not the body) — returns the rotated token data.
pub async fn refresh(access_token: &str, refresh_token: &str, domain: Option<&str>) -> Result<Value, String> {
    let mut req = http()
        .post(plugin_url("/auth/token/refresh"))
        .header("User-Agent", UA)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("X-Refresh-Token", refresh_token)
        .header("X-Auth-Refresh-Source", "ide-main");
    if let Some(d) = domain {
        req = req.header("X-Domain", d);
    }
    let resp = req.send().await.map_err(|e| format!("token refresh: {e}"))?;
    let body: Value = resp.json().await.map_err(|e| format!("token refresh parse: {e}"))?;
    if !success_code(&body) {
        let msg = body
            .get("message")
            .or_else(|| body.get("msg"))
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(format!("token refresh rejected (code): {msg}"));
    }
    data_of(&body)
        .cloned()
        .ok_or_else(|| "token refresh response has no data".to_string())
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
    use serde_json::json;

    #[test]
    fn success_codes_cover_zero_and_200() {
        assert!(success_code(&json!({ "code": 0 })));
        assert!(success_code(&json!({ "code": 200 })));
        assert!(!success_code(&json!({ "code": 401 })));
        assert!(!success_code(&json!({})));
    }

    #[test]
    fn tenant_reads_the_stored_extra_blob() {
        let extra = json!({ "uid": "u-1", "enterpriseId": "e-9", "domain": "d.example" });
        let tenant = Tenant::from_extra(&extra);
        assert_eq!(tenant.uid.as_deref(), Some("u-1"));
        assert_eq!(tenant.enterprise_id.as_deref(), Some("e-9"));
        assert_eq!(tenant.domain.as_deref(), Some("d.example"));
        // A blob with nothing in it yields an all-empty tenant.
        let empty = Tenant::from_extra(&json!({}));
        assert!(empty.uid.is_none() && empty.enterprise_id.is_none() && empty.domain.is_none());
    }

    #[test]
    fn data_gate_rejects_non_objects() {
        assert!(data_of(&json!({ "data": { "a": 1 } })).is_some());
        assert!(data_of(&json!({ "data": "nope" })).is_none());
        assert!(data_of(&json!({})).is_none());
    }
}
