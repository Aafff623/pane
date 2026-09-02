//! Pane's own OAuth sign-in (device code flow), one account per provider.
//!
//! Scope: Codex (OpenAI) and Grok (xAI) — the two providers whose device
//! code endpoints are proven (same public client ids the CLIs themselves
//! use). Tokens live in %APPDATA%\Pane\oauth\<provider>.json and nowhere
//! else: never in the repo, never in logs, and the CLI's own credential
//! files are never written from here (refresh writes back to Pane's file
//! only).
//!
//! Flow per provider:
//! 1. `start` asks the vendor for a device code and returns the user code
//!    + verification URL for the frontend to show/open.
//! 2. The frontend polls `poll` every few seconds; the backend enforces
//!    the server-asked interval (plus a safety margin) itself.
//! 3. On authorization the tokens are exchanged and written to disk.
//! 4. Providers call `valid_tokens`, which refreshes an expiring access
//!    token with the stored refresh token and saves the rotated pair.

use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

// --- Codex (OpenAI) — the Codex CLI's public OAuth client, same constants
// as upstream uses ---
const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CODEX_USERCODE_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/usercode";
const CODEX_DEVICE_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
const CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const CODEX_VERIFY_URL: &str = "https://auth.openai.com/codex/device";
const CODEX_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
const CODEX_SCOPE: &str = "openid profile email";
/// OpenAI documents 15 minutes for the device code.
const CODEX_DEFAULT_EXPIRES_IN: u64 = 900;

// --- Grok (xAI) — endpoints come from the OIDC discovery document so a
// protocol change doesn't strand installed builds ---
const XAI_ISSUER: &str = "https://auth.x.ai";
const XAI_DISCOVERY_URL: &str = "https://auth.x.ai/.well-known/openid-configuration";
const XAI_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
const XAI_SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";

/// Auth requests fail fast — the shared client timeout is tuned for
/// streaming model responses, far too long for a stuck network here.
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
/// Refresh this many seconds before the stored expiry.
const REFRESH_BUFFER_SECS: i64 = 60;
const DEFAULT_TOKEN_LIFETIME_SECS: i64 = 3600;
const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;
/// Poll no faster than the server-asked interval plus this margin.
const POLLING_SAFETY_MARGIN_SECS: u64 = 3;
const MAX_POLL_INTERVAL_SECS: u64 = 60;
/// One OAuth response is a few hundred bytes; this bounds a hostile or
/// broken endpoint.
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// What `oauth_start` hands the frontend.
#[derive(Serialize)]
pub struct StartResponse {
    pub device_auth_id: String,
    pub user_code: String,
    pub verify_url: String,
    pub expires_in: u64,
}

/// What `oauth_poll` hands the frontend: `done` with the account label on
/// success, `error` on terminal failure, neither while still waiting.
#[derive(Serialize)]
pub struct PollResponse {
    pub done: bool,
    pub label: Option<String>,
    pub error: Option<String>,
}

impl PollResponse {
    fn pending() -> Self {
        Self { done: false, label: None, error: None }
    }
    fn failure(error: String) -> Self {
        Self { done: false, label: None, error: Some(error) }
    }
    fn success(label: Option<String>) -> Self {
        Self { done: true, label, error: None }
    }
}

/// The persisted credential file, %APPDATA%\Pane\oauth\<provider>.json.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StoredTokens {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    /// RFC3339 expiry of the access token.
    pub expires_at: String,
    /// Human label for the UI (account email where the vendor exposes it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Codex only: the ChatGPT workspace id, sent as the
    /// chatgpt-account-id header on usage requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Codex only: kept for the plan claim on the usage card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}

/// A login flow between `start` and completion — the poll needs the user
/// code (Codex) and the discovered token endpoint (xAI) from the start
/// step, and the expiry/interval for pacing and cleanup.
struct PendingFlow {
    provider: String,
    user_code: String,
    token_endpoint: String,
    expires_at_ms: i64,
    interval_secs: u64,
    next_poll_at_ms: i64,
}

static PENDING: LazyLock<Mutex<HashMap<String, PendingFlow>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
/// xAI endpoints resolved from the discovery document, fetched once.
static XAI_ENDPOINTS: Mutex<Option<(String, String)>> = Mutex::new(None);

/// Only these two providers have a device-code flow this phase. Also the
/// path-safety gate: the provider id becomes a file name.
fn check_provider(provider: &str) -> Result<(), String> {
    match provider {
        "codex" | "grok" => Ok(()),
        _ => Err(format!("OAuth sign-in is not available for {provider}")),
    }
}

fn oauth_dir() -> PathBuf {
    crate::providers::config_dir().join("oauth")
}

fn file_path(dir: &Path, provider: &str) -> PathBuf {
    dir.join(format!("{provider}.json"))
}

pub fn load_from(dir: &Path, provider: &str) -> Option<StoredTokens> {
    let raw = std::fs::read_to_string(file_path(dir, provider)).ok()?;
    serde_json::from_str(&raw).ok()
}

/// The stored Pane OAuth credential for a provider, if any.
pub fn load(provider: &str) -> Option<StoredTokens> {
    check_provider(provider).ok()?;
    load_from(&oauth_dir(), provider)
}

/// Display label of the stored login (account email), for status chips.
pub fn label(provider: &str) -> Option<String> {
    load(provider).and_then(|t| t.label)
}

/// Atomic write (tmp + rename), like the CLI credential write-backs do.
fn save_to(dir: &Path, provider: &str, tokens: &StoredTokens) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("create oauth dir: {e}"))?;
    let path = file_path(dir, provider);
    let body = serde_json::to_string_pretty(tokens).map_err(|e| format!("serialize: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body)
        .and_then(|_| {
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            std::fs::rename(&tmp, &path)
        })
        .map_err(|e| format!("write oauth file: {e}"))
}

fn save(provider: &str, tokens: &StoredTokens) -> Result<(), String> {
    save_to(&oauth_dir(), provider, tokens)
}

/// `oauth_logout`: delete Pane's OAuth file for the provider. The CLI's
/// own credentials are untouched.
pub fn logout(provider: &str) -> Result<(), String> {
    check_provider(provider)?;
    let path = file_path(&oauth_dir(), provider);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("remove oauth file: {e}"))?;
    }
    Ok(())
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

/// The access token is unusable within REFRESH_BUFFER_SECS of its stored
/// expiry; a missing/unparseable expiry means "assume expired" so a
/// refresh gets a chance to fix it.
fn access_expired(tokens: &StoredTokens, now: DateTime<Utc>) -> bool {
    match DateTime::parse_from_rfc3339(&tokens.expires_at) {
        Ok(exp) => exp.with_timezone(&Utc) <= now + chrono::Duration::seconds(REFRESH_BUFFER_SECS),
        Err(_) => true,
    }
}

fn expires_at_rfc3339(expires_in: Option<i64>) -> String {
    (Utc::now() + chrono::Duration::seconds(expires_in.unwrap_or(DEFAULT_TOKEN_LIFETIME_SECS).max(1)))
        .to_rfc3339()
}

/// The server-asked poll interval (Codex sends it as a JSON string on
/// some tenants), clamped and given the safety margin.
fn parse_interval(v: Option<&Value>) -> u64 {
    let raw = match v {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(DEFAULT_POLL_INTERVAL_SECS),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(DEFAULT_POLL_INTERVAL_SECS),
        _ => DEFAULT_POLL_INTERVAL_SECS,
    };
    raw.clamp(1, MAX_POLL_INTERVAL_SECS) + POLLING_SAFETY_MARGIN_SECS
}

/// JWT payload as JSON — the middle base64 chunk. Used for the account
/// label and Codex's workspace/plan claims; never logged.
fn jwt_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn http() -> reqwest::Client {
    crate::providers::http()
}

/// A response body, size-capped, as JSON.
async fn read_json(resp: reqwest::Response, what: &str) -> Result<Value, String> {
    if resp.content_length().is_some_and(|n| n > MAX_RESPONSE_BYTES as u64) {
        return Err(format!("{what}: response too large"));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("{what}: {e}"))?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(format!("{what}: response too large"));
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("{what}: invalid response ({e})"))
}

/// The OAuth-standard error field of a response body, if present.
fn error_code(body: &Value) -> Option<&str> {
    body.get("error").and_then(Value::as_str)
}

/// A failed token refresh that means the login itself is dead and the
/// user must sign in again (as opposed to a transient server/network
/// error worth retrying).
fn codex_refresh_needs_relogin(status: u16, body: &Value) -> bool {
    status == 401
        || status == 403
        || matches!(
            error_code(body),
            Some("invalid_grant")
                | Some("refresh_token_expired")
                | Some("refresh_token_reused")
                | Some("refresh_token_invalidated")
        )
}

fn xai_refresh_needs_relogin(status: u16, body: Option<&Value>) -> bool {
    match body {
        // An unparseable error body on a 400 is still a dead credential.
        None => status == 400 || status == 401 || status == 403,
        Some(v) => {
            status == 401
                || status == 403
                || matches!(error_code(v), Some("invalid_grant") | Some("invalid_token"))
        }
    }
}

const RELOGIN_HINT: &str = "OAuth login expired — sign in again from the gear panel";

/// xAI's token + device-authorization endpoints from the OIDC discovery
/// document, validated to actually live on auth.x.ai over https.
async fn xai_endpoints() -> Result<(String, String), String> {
    if let Some(endpoints) = XAI_ENDPOINTS.lock().unwrap().clone() {
        return Ok(endpoints);
    }
    let resp = http()
        .get(XAI_DISCOVERY_URL)
        .timeout(HTTP_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("xAI discovery: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("xAI discovery: HTTP {}", resp.status()));
    }
    let doc = read_json(resp, "xAI discovery").await?;
    let issuer = doc.get("issuer").and_then(Value::as_str).unwrap_or_default();
    if issuer.trim_end_matches('/') != XAI_ISSUER {
        return Err("xAI discovery: unexpected issuer".into());
    }
    let endpoint = |key: &str| -> Result<String, String> {
        let url = doc
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("xAI discovery: missing {key}"))?;
        let parsed = reqwest::Url::parse(url).map_err(|_| format!("xAI discovery: bad {key}"))?;
        if parsed.scheme() != "https"
            || parsed.host_str() != Some("auth.x.ai")
            || parsed.port_or_known_default() != Some(443)
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err(format!("xAI discovery: untrusted {key}"));
        }
        Ok(url.to_string())
    };
    let endpoints = (endpoint("token_endpoint")?, endpoint("device_authorization_endpoint")?);
    *XAI_ENDPOINTS.lock().unwrap() = Some(endpoints.clone());
    Ok(endpoints)
}

// ---------------------------------------------------------------------------
// start
// ---------------------------------------------------------------------------

pub async fn start(provider: &str) -> Result<StartResponse, String> {
    check_provider(provider)?;
    match provider {
        "codex" => start_codex().await,
        _ => start_xai().await,
    }
}

async fn start_codex() -> Result<StartResponse, String> {
    let resp = http()
        .post(CODEX_USERCODE_URL)
        .timeout(HTTP_TIMEOUT)
        .json(&serde_json::json!({ "client_id": CODEX_CLIENT_ID }))
        .send()
        .await
        .map_err(|e| format!("device code request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("device code request: HTTP {}", resp.status()));
    }
    let doc = read_json(resp, "device code").await?;
    let device_auth_id = doc
        .get("device_auth_id")
        .and_then(Value::as_str)
        .ok_or("device code response missing device_auth_id")?
        .to_string();
    let user_code = doc
        .get("user_code")
        .and_then(Value::as_str)
        .ok_or("device code response missing user_code")?
        .to_string();
    let expires_in = doc
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(CODEX_DEFAULT_EXPIRES_IN);
    let interval = parse_interval(doc.get("interval"));
    register_pending(
        device_auth_id.clone(),
        PendingFlow {
            provider: "codex".into(),
            user_code: user_code.clone(),
            token_endpoint: String::new(), // Codex's poll URL is a constant.
            expires_at_ms: now_ms() + expires_in as i64 * 1000,
            interval_secs: interval,
            next_poll_at_ms: 0,
        },
    );
    Ok(StartResponse {
        device_auth_id,
        user_code,
        verify_url: CODEX_VERIFY_URL.into(),
        expires_in,
    })
}

async fn start_xai() -> Result<StartResponse, String> {
    let (token_endpoint, device_endpoint) = xai_endpoints().await?;
    let resp = http()
        .post(&device_endpoint)
        .timeout(HTTP_TIMEOUT)
        .form(&[("client_id", XAI_CLIENT_ID), ("scope", XAI_SCOPE)])
        .send()
        .await
        .map_err(|e| format!("device code request: {e}"))?;
    let status = resp.status();
    let doc = read_json(resp, "device code").await?;
    if !status.is_success() {
        return Err(format!(
            "device code request: HTTP {status}{}",
            error_code(&doc).map(|c| format!(" ({c})")).unwrap_or_default()
        ));
    }
    let device_code = doc
        .get("device_code")
        .and_then(Value::as_str)
        .ok_or("device code response missing device_code")?
        .to_string();
    let user_code = doc
        .get("user_code")
        .and_then(Value::as_str)
        .ok_or("device code response missing user_code")?
        .to_string();
    let verify_url = doc
        .get("verification_uri_complete")
        .and_then(Value::as_str)
        .or_else(|| doc.get("verification_uri").and_then(Value::as_str))
        .ok_or("device code response missing verification URI")?
        .to_string();
    let expires_in = doc.get("expires_in").and_then(Value::as_u64).unwrap_or(900);
    let interval = parse_interval(doc.get("interval"));
    register_pending(
        device_code.clone(),
        PendingFlow {
            provider: "grok".into(),
            user_code: user_code.clone(),
            token_endpoint,
            expires_at_ms: now_ms() + expires_in as i64 * 1000,
            interval_secs: interval,
            next_poll_at_ms: 0,
        },
    );
    Ok(StartResponse {
        device_auth_id: device_code,
        user_code,
        verify_url,
        expires_in,
    })
}

fn register_pending(device_auth_id: String, flow: PendingFlow) {
    let mut pending = PENDING.lock().unwrap();
    // Drop flows the user abandoned, so the map can't grow without bound.
    let now = now_ms();
    pending.retain(|_, f| f.expires_at_ms > now);
    pending.insert(device_auth_id, flow);
}

// ---------------------------------------------------------------------------
// poll
// ---------------------------------------------------------------------------

pub async fn poll(provider: &str, device_auth_id: &str) -> PollResponse {
    if check_provider(provider).is_err() {
        return PollResponse::failure(format!("OAuth sign-in is not available for {provider}"));
    }
    let flow = {
        let pending = PENDING.lock().unwrap();
        pending.get(device_auth_id).map(|f| PendingFlow {
            provider: f.provider.clone(),
            user_code: f.user_code.clone(),
            token_endpoint: f.token_endpoint.clone(),
            expires_at_ms: f.expires_at_ms,
            interval_secs: f.interval_secs,
            next_poll_at_ms: f.next_poll_at_ms,
        })
    };
    let Some(flow) = flow else {
        return PollResponse::failure("no pending login — start again".into());
    };
    if flow.provider != provider {
        return PollResponse::failure("login flow belongs to another provider".into());
    }
    let now = now_ms();
    if flow.expires_at_ms <= now {
        PENDING.lock().unwrap().remove(device_auth_id);
        return PollResponse::failure("the sign-in code expired — start again".into());
    }
    // The frontend polls on a fixed timer; the server-asked interval
    // (plus margin) is enforced here so a fast timer can't get the
    // client rate-limited.
    if flow.next_poll_at_ms > now {
        return PollResponse::pending();
    }
    if let Some(f) = PENDING.lock().unwrap().get_mut(device_auth_id) {
        f.next_poll_at_ms = now + flow.interval_secs as i64 * 1000;
    }
    match provider {
        "codex" => poll_codex(device_auth_id, &flow).await,
        _ => poll_xai(device_auth_id, &flow).await,
    }
}

async fn poll_codex(device_auth_id: &str, flow: &PendingFlow) -> PollResponse {
    let resp = match http()
        .post(CODEX_DEVICE_TOKEN_URL)
        .timeout(HTTP_TIMEOUT)
        .json(&serde_json::json!({
            "device_auth_id": device_auth_id,
            "user_code": flow.user_code,
        }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return PollResponse::failure(format!("poll request: {e}")),
    };
    let status = resp.status().as_u16();
    // 403/404: the user hasn't authorized yet — keep waiting.
    if status == 403 || status == 404 {
        return PollResponse::pending();
    }
    if status == 410 {
        PENDING.lock().unwrap().remove(device_auth_id);
        return PollResponse::failure("the sign-in code expired — start again".into());
    }
    if status != 200 {
        return PollResponse::failure(format!("poll request: HTTP {status}"));
    }
    let doc = match read_json(resp, "poll").await {
        Ok(d) => d,
        Err(e) => return PollResponse::failure(e),
    };
    let code = doc.get("authorization_code").and_then(Value::as_str).unwrap_or_default();
    let verifier = doc.get("code_verifier").and_then(Value::as_str).unwrap_or_default();
    if code.is_empty() || verifier.is_empty() {
        return PollResponse::failure("poll response missing authorization code".into());
    }
    // The device flow hands back a code + verifier; exchanging them for
    // tokens is the same call the Codex CLI makes.
    let resp = match http()
        .post(CODEX_OAUTH_TOKEN_URL)
        .timeout(HTTP_TIMEOUT)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", CODEX_REDIRECT_URI),
            ("client_id", CODEX_CLIENT_ID),
            ("code_verifier", verifier),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return PollResponse::failure(format!("token exchange: {e}")),
    };
    if !resp.status().is_success() {
        return PollResponse::failure(format!("token exchange: HTTP {}", resp.status()));
    }
    let doc = match read_json(resp, "token exchange").await {
        Ok(d) => d,
        Err(e) => return PollResponse::failure(e),
    };
    match finish_login("codex", &doc) {
        Ok(label) => {
            PENDING.lock().unwrap().remove(device_auth_id);
            PollResponse::success(label)
        }
        Err(e) => PollResponse::failure(e),
    }
}

async fn poll_xai(device_auth_id: &str, flow: &PendingFlow) -> PollResponse {
    let resp = match http()
        .post(&flow.token_endpoint)
        .timeout(HTTP_TIMEOUT)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", XAI_CLIENT_ID),
            ("device_code", device_auth_id),
        ])
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return PollResponse::failure(format!("poll request: {e}")),
    };
    let status = resp.status().as_u16();
    let doc = match read_json(resp, "poll").await {
        Ok(d) => d,
        Err(e) => return PollResponse::failure(e),
    };
    if let Some(code) = error_code(&doc) {
        return match code {
            "authorization_pending" => PollResponse::pending(),
            "slow_down" => {
                // Back off an extra 5s per slow_down, like the RFC asks.
                if let Some(f) = PENDING.lock().unwrap().get_mut(device_auth_id) {
                    f.interval_secs = (f.interval_secs + 5).min(MAX_POLL_INTERVAL_SECS);
                    f.next_poll_at_ms = now_ms() + f.interval_secs as i64 * 1000;
                }
                PollResponse::pending()
            }
            "access_denied" => {
                PENDING.lock().unwrap().remove(device_auth_id);
                PollResponse::failure("sign-in was declined".into())
            }
            "expired_token" => {
                PENDING.lock().unwrap().remove(device_auth_id);
                PollResponse::failure("the sign-in code expired — start again".into())
            }
            _ => PollResponse::failure(format!("poll request: HTTP {status} ({code})")),
        };
    }
    if status != 200 {
        return PollResponse::failure(format!("poll request: HTTP {status}"));
    }
    match finish_login("grok", &doc) {
        Ok(label) => {
            PENDING.lock().unwrap().remove(device_auth_id);
            PollResponse::success(label)
        }
        Err(e) => PollResponse::failure(e),
    }
}

/// Shared tail of both polls: a successful token response becomes the
/// stored credential file.
fn finish_login(provider: &str, doc: &Value) -> Result<Option<String>, String> {
    let access_token = doc
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("token response missing access_token")?
        .to_string();
    let refresh_token = doc
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if refresh_token.is_empty() {
        return Err("token response missing refresh_token".into());
    }
    let id_token = doc
        .get("id_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let claims = id_token.as_deref().and_then(jwt_claims);
    let label = claims
        .as_ref()
        .and_then(|c| c.get("email"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let account_id = if provider == "codex" {
        claims
            .as_ref()
            .and_then(|c| {
                c.pointer("/https:~1~1api.openai.com~1auth/chatgpt_account_id")
                    .or_else(|| c.get("chatgpt_account_id"))
            })
            .and_then(Value::as_str)
            .map(str::to_string)
    } else {
        None
    };
    let tokens = StoredTokens {
        access_token,
        refresh_token,
        expires_at: expires_at_rfc3339(doc.get("expires_in").and_then(Value::as_i64)),
        label: label.clone(),
        account_id,
        id_token,
    };
    save(provider, &tokens)?;
    Ok(label)
}

// ---------------------------------------------------------------------------
// Token resolution for providers (read + refresh + write-back)
// ---------------------------------------------------------------------------

/// The stored OAuth credential with a usable access token, refreshing and
/// writing back to Pane's own file when it has expired. `Ok(None)` means
/// no Pane OAuth login exists. A dead refresh token is a clear
/// sign-in-again error; the file is kept so the UI can still show the
/// account until the user logs out or re-logs.
pub async fn valid_tokens(provider: &str) -> Result<Option<StoredTokens>, String> {
    let Some(stored) = load(provider) else {
        return Ok(None);
    };
    if !stored.access_token.is_empty() && !access_expired(&stored, Utc::now()) {
        return Ok(Some(stored));
    }
    if stored.refresh_token.is_empty() {
        return Err(RELOGIN_HINT.into());
    }
    let refreshed = refresh(provider, &stored.refresh_token).await?;
    // Fold the refreshed token into the stored record: keep the label and,
    // for Codex, adopt a rotated id_token / workspace when one comes back.
    let mut next = stored.clone();
    next.access_token = refreshed.access_token;
    if !refreshed.refresh_token.is_empty() {
        next.refresh_token = refreshed.refresh_token;
    }
    next.expires_at = refreshed.expires_at;
    if let Some(id_token) = refreshed.id_token {
        if let Some(claims) = jwt_claims(&id_token) {
            if provider == "codex" {
                if let Some(account_id) = claims
                    .pointer("/https:~1~1api.openai.com~1auth/chatgpt_account_id")
                    .or_else(|| claims.get("chatgpt_account_id"))
                    .and_then(Value::as_str)
                {
                    next.account_id = Some(account_id.to_string());
                }
            }
            if let Some(email) = claims.get("email").and_then(Value::as_str) {
                next.label = Some(email.to_string());
            }
        }
        next.id_token = Some(id_token);
    }
    save(provider, &next)?;
    Ok(Some(next))
}

/// One refresh round-trip; the rotated pair comes back unsaved.
async fn refresh(provider: &str, refresh_token: &str) -> Result<StoredTokens, String> {
    match provider {
        "codex" => {
            let resp = http()
                .post(CODEX_OAUTH_TOKEN_URL)
                .timeout(HTTP_TIMEOUT)
                .form(&[
                    ("grant_type", "refresh_token"),
                    ("refresh_token", refresh_token),
                    ("client_id", CODEX_CLIENT_ID),
                    ("scope", CODEX_SCOPE),
                ])
                .send()
                .await
                .map_err(|e| format!("token refresh: {e}"))?;
            let status = resp.status().as_u16();
            let body = read_json(resp, "token refresh").await.unwrap_or(Value::Null);
            if !status_is_success(status) {
                if codex_refresh_needs_relogin(status, &body) {
                    return Err(RELOGIN_HINT.into());
                }
                return Err(format!("token refresh: HTTP {status}"));
            }
            parse_refreshed(body)
        }
        _ => {
            let (token_endpoint, _) = xai_endpoints().await?;
            let resp = http()
                .post(&token_endpoint)
                .timeout(HTTP_TIMEOUT)
                .form(&[
                    ("grant_type", "refresh_token"),
                    ("refresh_token", refresh_token),
                    ("client_id", XAI_CLIENT_ID),
                    ("scope", XAI_SCOPE),
                ])
                .send()
                .await
                .map_err(|e| format!("token refresh: {e}"))?;
            let status = resp.status().as_u16();
            let body = read_json(resp, "token refresh").await.ok();
            if !status_is_success(status) || body.as_ref().and_then(error_code).is_some() {
                if xai_refresh_needs_relogin(status, body.as_ref()) {
                    return Err(RELOGIN_HINT.into());
                }
                let code = body.as_ref().and_then(error_code).unwrap_or("");
                return Err(format!("token refresh: HTTP {status}{}", if code.is_empty() { String::new() } else { format!(" ({code})") }));
            }
            parse_refreshed(body.unwrap_or(Value::Null))
        }
    }
}

fn status_is_success(status: u16) -> bool {
    (200..300).contains(&status)
}

fn parse_refreshed(doc: Value) -> Result<StoredTokens, String> {
    let access_token = doc
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("refresh response missing access_token")?
        .to_string();
    Ok(StoredTokens {
        access_token,
        refresh_token: doc
            .get("refresh_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        expires_at: expires_at_rfc3339(doc.get("expires_in").and_then(Value::as_i64)),
        label: None,
        account_id: None,
        id_token: doc
            .get("id_token")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn fake_jwt(claims: Value) -> String {
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&claims).unwrap());
        format!("x.{payload}.y")
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pane-oauth-test-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn endpoint_constants_match_the_cli_clients() {
        assert_eq!(CODEX_CLIENT_ID, "app_EMoamEEZ73f0CkXaXp7hrann");
        assert_eq!(
            CODEX_USERCODE_URL,
            "https://auth.openai.com/api/accounts/deviceauth/usercode"
        );
        assert_eq!(
            CODEX_DEVICE_TOKEN_URL,
            "https://auth.openai.com/api/accounts/deviceauth/token"
        );
        assert_eq!(CODEX_OAUTH_TOKEN_URL, "https://auth.openai.com/oauth/token");
        assert_eq!(CODEX_VERIFY_URL, "https://auth.openai.com/codex/device");
        assert_eq!(
            CODEX_REDIRECT_URI,
            "https://auth.openai.com/deviceauth/callback"
        );
        assert_eq!(XAI_CLIENT_ID, "b1a00492-073a-47ea-816f-4c329264a828");
        assert_eq!(
            XAI_DISCOVERY_URL,
            "https://auth.x.ai/.well-known/openid-configuration"
        );
    }

    #[test]
    fn provider_gate_is_exact() {
        assert!(check_provider("codex").is_ok());
        assert!(check_provider("grok").is_ok());
        for bad in ["claude", "Codex", "", "../codex", "codex.json", "codex/evil"] {
            assert!(check_provider(bad).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn access_expired_reads_rfc3339_with_buffer() {
        let tokens = |expires_at: &str| StoredTokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: expires_at.into(),
            label: None,
            account_id: None,
            id_token: None,
        };
        let now = Utc::now();
        let future = (now + chrono::Duration::hours(1)).to_rfc3339();
        let soon = (now + chrono::Duration::seconds(REFRESH_BUFFER_SECS - 1)).to_rfc3339();
        let past = (now - chrono::Duration::hours(1)).to_rfc3339();
        assert!(!access_expired(&tokens(&future), now));
        // Inside the refresh buffer counts as expired.
        assert!(access_expired(&tokens(&soon), now));
        assert!(access_expired(&tokens(&past), now));
        // Garbage or empty expiry: assume expired so a refresh can fix it.
        assert!(access_expired(&tokens("not-a-date"), now));
        assert!(access_expired(&tokens(""), now));
    }

    #[test]
    fn save_load_logout_roundtrip() {
        let dir = tmp_dir("roundtrip");
        let tokens = StoredTokens {
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            expires_at: "2026-09-03T12:00:00+00:00".into(),
            label: Some("user@example.com".into()),
            account_id: Some("acct-1".into()),
            id_token: None,
        };
        save_to(&dir, "codex", &tokens).unwrap();
        assert_eq!(load_from(&dir, "codex"), Some(tokens));
        assert!(load_from(&dir, "grok").is_none());
        // Corrupt JSON reads as absent, never as a panic.
        std::fs::write(file_path(&dir, "grok"), "{not json").unwrap();
        assert!(load_from(&dir, "grok").is_none());
        std::fs::remove_file(file_path(&dir, "codex")).unwrap();
        assert!(load_from(&dir, "codex").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn poll_interval_parsing_and_margin() {
        assert_eq!(parse_interval(None), DEFAULT_POLL_INTERVAL_SECS + POLLING_SAFETY_MARGIN_SECS);
        assert_eq!(parse_interval(Some(&Value::from(5))), 8);
        // Some tenants send the interval as a string.
        assert_eq!(parse_interval(Some(&Value::from("10"))), 13);
        // Clamped to a sane range before the margin is added.
        assert_eq!(parse_interval(Some(&Value::from(0))), 1 + POLLING_SAFETY_MARGIN_SECS);
        assert_eq!(
            parse_interval(Some(&Value::from(9999))),
            MAX_POLL_INTERVAL_SECS + POLLING_SAFETY_MARGIN_SECS
        );
        assert_eq!(
            parse_interval(Some(&Value::from("junk"))),
            DEFAULT_POLL_INTERVAL_SECS + POLLING_SAFETY_MARGIN_SECS
        );
    }

    #[test]
    fn codex_relogin_classification() {
        let invalid_grant = serde_json::json!({"error": "invalid_grant"});
        let reused = serde_json::json!({"error": "refresh_token_reused"});
        let other = serde_json::json!({"error": "temporarily_unavailable"});
        let empty = Value::Null;
        assert!(codex_refresh_needs_relogin(401, &empty));
        assert!(codex_refresh_needs_relogin(403, &empty));
        assert!(codex_refresh_needs_relogin(400, &invalid_grant));
        assert!(codex_refresh_needs_relogin(400, &reused));
        assert!(!codex_refresh_needs_relogin(400, &other));
        assert!(!codex_refresh_needs_relogin(500, &empty));
        assert!(!codex_refresh_needs_relogin(400, &empty));
    }

    #[test]
    fn xai_relogin_classification() {
        let invalid_grant = serde_json::json!({"error": "invalid_grant"});
        let other = serde_json::json!({"error": "server_error"});
        assert!(xai_refresh_needs_relogin(401, None));
        assert!(xai_refresh_needs_relogin(403, None));
        // An unparseable 400 body is still a dead credential.
        assert!(xai_refresh_needs_relogin(400, None));
        assert!(xai_refresh_needs_relogin(400, Some(&invalid_grant)));
        assert!(!xai_refresh_needs_relogin(400, Some(&other)));
        assert!(!xai_refresh_needs_relogin(500, None));
    }

    #[test]
    fn jwt_claims_reads_the_payload_chunk() {
        let jwt = fake_jwt(serde_json::json!({"email": "u@x.ai"}));
        assert_eq!(
            jwt_claims(&jwt).and_then(|c| c.get("email").and_then(Value::as_str).map(str::to_string)),
            Some("u@x.ai".into())
        );
        assert!(jwt_claims("not-a-jwt").is_none());
        assert!(jwt_claims("a.!!!.c").is_none());
    }

    /// The JSON pointer finish_login/valid_tokens use to pull Codex's
    /// workspace id out of the id_token claims.
    #[test]
    fn codex_workspace_claim_pointer() {
        let id_token = fake_jwt(serde_json::json!({
            "email": "me@corp.com",
            "https://api.openai.com/auth": {"chatgpt_account_id": "acct-9"}
        }));
        let claims = jwt_claims(&id_token).unwrap();
        assert_eq!(
            claims.get("email").and_then(Value::as_str),
            Some("me@corp.com")
        );
        assert_eq!(
            claims
                .pointer("/https:~1~1api.openai.com~1auth/chatgpt_account_id")
                .and_then(Value::as_str),
            Some("acct-9")
        );
    }

    #[test]
    fn parse_refreshed_requires_access_token() {
        let ok = serde_json::json!({"access_token": "new", "refresh_token": "rotated", "expires_in": 3600});
        let parsed = parse_refreshed(ok).unwrap();
        assert_eq!(parsed.access_token, "new");
        assert_eq!(parsed.refresh_token, "rotated");
        assert!(DateTime::parse_from_rfc3339(&parsed.expires_at).is_ok());
        // Missing/empty access token is an error, not an empty credential.
        assert!(parse_refreshed(serde_json::json!({"refresh_token": "r"})).is_err());
        assert!(parse_refreshed(serde_json::json!({"access_token": ""})).is_err());
        // Missing refresh token keeps the old one (empty = "unchanged").
        let no_rotation = parse_refreshed(serde_json::json!({"access_token": "a"})).unwrap();
        assert!(no_rotation.refresh_token.is_empty());
    }
}
