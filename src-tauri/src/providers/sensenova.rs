//! SenseNova Token Plan (商汤日日新) quota — dual credit pools (default +
//! Flash-Lite dedicated), each with a rolling 5h and 7d window, plus grant
//! (返赠) balances. Quota lives behind the console session API (Bearer
//! access_token); sk- data-plane keys CANNOT read it (401 auth_type_disabled
//! — verified; see temp/handoff/20261003-145636-sensenova-tokenplan-*.md).
//!
//! Credential lifecycle — the "renew on use" ladder (no timers):
//!   L0  cached access_token whose JWT exp is still ahead → just fetch.
//!   L1  expired/401 and a refresh_token exists → single-flight
//!       POST /oauth2/token (grant_type=refresh_token, public client
//!       `nova`); a rotated refresh_token (if re-issued) is persisted
//!       atomically (temp file + rename). invalid_grant drops the token
//!       and falls through; repeated failures back off 30s → 5m → 30m,
//!       then stop auto-retrying until the next process start.
//!   L2  no/failed refresh → sign in again with one browser authorization:
//!       Settings builds a PKCE /oauth2/auth link (public client `nova`);
//!       whatever the browser yields — redirect URL, token payload, or a
//!       bare access token — is exchanged/persisted here, and the refresh
//!       token it issues is what makes L1 permanent.
//!   L3  everything failed → error snapshot; lib.rs's stale-snapshot
//!       mechanism shows the last good numbers with a warning instead of
//!       passing old data off as current.
//! HTTP 429 (TPM/RPM throttling) is a NORMAL state and never touches the
//! ladder; only authentication rejection does.

use super::{http, json_body, Metric, Snapshot};
use base64::Engine;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;

const ID: &str = "sensenova";
const NAME: &str = "SenseNova";
const MAX_BODY_BYTES: usize = 128 * 1024;
const POOL_URL: &str = "https://platform.sensenova.cn/lite/console/v1/tokenplan/pool-usage";
const TOKEN_URL: &str = "https://platform.sensenova.cn/oauth2/token";
const CLIENT_ID: &str = "nova"; // public PKCE client id, from their frontend JS
const AUTH_URL: &str = "https://platform.sensenova.cn/oauth2/auth";
const REDIRECT_URI: &str = "https://platform.sensenova.cn";
/// The redirect + scope pre-encoded for the query string (fixed values).
const REDIRECT_URI_ENCODED: &str = "https%3A%2F%2Fplatform.sensenova.cn";
const AUTH_SCOPE_ENCODED: &str = "openid%20offline%20offline_access";

const BACKOFFS_MS: [i64; 3] = [30_000, 5 * 60_000, 30 * 60_000];

/// Persisted credential slot: %APPDATA%/Pane/sensenova.json.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct Creds {
    #[serde(default, alias = "apiKey")]
    access_token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    refresh_token: String,
}

fn creds_path() -> PathBuf {
    super::config_dir().join("sensenova.json")
}

fn load_creds() -> Creds {
    std::fs::read_to_string(creds_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Atomic persist: write a sibling temp file, then rename over. A crash
/// mid-write can never leave a half token file.
fn save_creds(creds: &Creds) {
    let path = creds_path();
    let tmp = path.with_extension("json.tmp");
    let ok = serde_json::to_string_pretty(creds)
        .ok()
        .and_then(|s| std::fs::write(&tmp, s).ok())
        .is_some_and(|()| std::fs::rename(&tmp, &path).is_ok());
    if !ok {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Renewal bookkeeping: consecutive failures + when the next attempt is
/// allowed. `in_flight` makes the mutex double as the single-flight lock —
/// a second renew call while one runs reports "not now" instead of piling
/// a duplicate token request onto the provider.
#[derive(Default)]
struct Renew {
    in_flight: bool,
    failures: u32,
    next_attempt_ms: i64,
}

static RENEW: Mutex<Renew> = Mutex::new(Renew {
    in_flight: false,
    failures: 0,
    next_attempt_ms: 0,
});

/// How a quota fetch failed — decides whether the credential ladder runs.
#[derive(Debug)]
enum FetchErr {
    /// 401/403: the access token was rejected → ladder.
    Auth,
    /// 429: model TPM/RPM throttling, unrelated to credentials → no ladder.
    RateLimited,
    Other(String),
}

/// Local JWT exp precheck (L0) without network: base64-decode the payload,
/// read `exp` (unix seconds), allow 60s of clock skew.
fn token_expires_in_ms(token: &str) -> Option<i64> {
    use base64::Engine;
    let payload_b64 = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload_b64))
        .ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    let exp = v.get("exp")?.as_i64()?;
    Some(exp * 1000 - chrono::Utc::now().timestamp_millis())
}

pub fn local_credential_hint() -> Option<String> {
    let c = load_creds();
    (!c.access_token.is_empty() || !c.refresh_token.is_empty())
        .then(|| "Pane SenseNova session".into())
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a pasted access token (Customize "Test"); never saved.
pub async fn snapshot_with_key(token: &str) -> Snapshot {
    match fetch_pools(token.trim()).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, describe(&e)),
    }
}

fn describe(e: &FetchErr) -> String {
    match e {
        FetchErr::Auth => "SenseNova session was rejected — sign in again in Settings".into(),
        FetchErr::RateLimited => "SenseNova rate-limited (TPM/RPM) — retrying next cycle".into(),
        FetchErr::Other(s) => s.clone(),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let mut creds = load_creds();
    if creds.access_token.is_empty() && creds.refresh_token.is_empty() {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Sign in to SenseNova Token Plan in Settings (one browser authorization keeps it renewed).",
        ));
    }

    // L0: locally-valid token → straight to the quota endpoint.
    let token_valid = token_expires_in_ms(&creds.access_token).is_some_and(|ms| ms > 60_000);
    if token_valid {
        match fetch_pools(&creds.access_token).await {
            Ok(s) => return Ok(s),
            // A 401 despite a locally-valid JWT (server-side revoke, clock
            // skew) falls through to renewal instead of hard-erroring.
            Err(FetchErr::Auth) => {}
            Err(e) => return Err(describe(&e)),
        }
    }

    // L1: refresh (single-flight + backoff inside).
    match renew(&mut creds).await {
        Some(tok) => match fetch_pools(&tok).await {
            Ok(s) => Ok(s),
            Err(e) => Err(describe(&e)),
        },
        None => Err(if creds.refresh_token.is_empty() {
            "SenseNova session expired — sign in again in Settings".to_string()
        } else {
            "SenseNova session refresh is backing off — will retry automatically".to_string()
        }),
    }
}

/// One renewal attempt under the single-flight lock with backoff. Returns
/// the new access token on success (creds already persisted).
async fn renew(creds: &mut Creds) -> Option<String> {
    if creds.refresh_token.is_empty() {
        return None;
    }
    // Scoped so the guard provably dies before the await below (the
    // guarded future must stay Send for lib.rs's refresh pool).
    {
        let mut guard = RENEW.lock().ok()?;
        if guard.in_flight {
            return None;
        }
        let now = chrono::Utc::now().timestamp_millis();
        if guard.failures > 0 && now < guard.next_attempt_ms {
            return None;
        }
        guard.in_flight = true;
    }

    let result = refresh_via_oauth(&creds.refresh_token).await;

    {
        let mut guard = RENEW.lock().ok()?;
        guard.in_flight = false;
        match result {
            Ok((access, rotated)) => {
                guard.failures = 0;
                guard.next_attempt_ms = 0;
                creds.access_token = access.clone();
                if let Some(r) = rotated {
                    creds.refresh_token = r; // rotation: replace when re-issued
                }
                save_creds(creds);
                Some(access)
            }
            Err(RefreshError::Invalid) => {
                // invalid_grant: this refresh token is dead; never retry it.
                creds.refresh_token.clear();
                save_creds(creds);
                guard.failures = 0;
                None
            }
            Err(RefreshError::Transient(_)) => {
                let step = (guard.failures as usize).min(BACKOFFS_MS.len() - 1);
                guard.next_attempt_ms = chrono::Utc::now().timestamp_millis() + BACKOFFS_MS[step];
                guard.failures += 1;
                None
            }
        }
    }
}

enum RefreshError {
    /// invalid_grant — the token itself is rejected; drop it.
    Invalid,
    /// Network/server issues — retry later with backoff.
    Transient(String),
}

async fn refresh_via_oauth(refresh: &str) -> Result<(String, Option<String>), RefreshError> {
    let resp = http()
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh),
        ])
        .send()
        .await
        .map_err(|e| RefreshError::Transient(format!("network: {e}")))?;
    // Hydra-family stacks answer invalid_grant with 400 + that code —
    // and a dead refresh token must not be retried, so treat it as final.
    if resp.status().as_u16() == 400 {
        return Err(RefreshError::Invalid);
    }
    if !resp.status().is_success() {
        return Err(RefreshError::Transient(format!("HTTP {}", resp.status())));
    }
    let doc: Value = resp
        .json()
        .await
        .map_err(|e| RefreshError::Transient(format!("body: {e}")))?;
    let access = doc
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(RefreshError::Transient("no access_token in response".into()))?
        .to_string();
    let rotated = doc
        .get("refresh_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && *s != refresh)
        .map(str::to_string);
    Ok((access, rotated))
}

// ── Browser sign-in (PKCE authorization code) ──────────────────────────────

/// 32 bytes of entropy: RandomState is OS-seeded and a monotonic counter
/// keeps successive calls distinct — no new dependency (same recipe as
/// cursor_oauth.rs).
fn random_bytes() -> [u8; 32] {
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seed = std::collections::hash_map::RandomState::new().build_hasher().finish();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        let mut x = seed
            ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ ((i as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *byte = (x & 0xff) as u8;
    }
    out
}

fn generate_code_verifier() -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes())
}

fn generate_code_challenge(code_verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize())
}

/// PKCE material for the in-flight browser authorization. Memory only: a
/// restart invalidates it and the user just reopens the link.
struct PendingAuth {
    verifier: String,
    state: String,
}

static PENDING: Mutex<Option<PendingAuth>> = Mutex::new(None);

fn auth_url(challenge: &str, state: &str) -> String {
    format!(
        "{AUTH_URL}?response_type=code&client_id={CLIENT_ID}&code_challenge_method=S256\
         &code_challenge={challenge}&redirect_uri={REDIRECT_URI_ENCODED}\
         &scope={AUTH_SCOPE_ENCODED}&state={state}&lang=zh-CN"
    )
}

/// Settings "Sign in": build the PKCE link and remember its verifier/state.
pub fn oauth_start() -> String {
    let verifier = generate_code_verifier();
    let state = generate_code_verifier();
    let challenge = generate_code_challenge(&verifier);
    if let Ok(mut guard) = PENDING.lock() {
        *guard = Some(PendingAuth { verifier, state: state.clone() });
    }
    auth_url(&challenge, &state)
}

fn looks_like_jwt(s: &str) -> bool {
    s.starts_with("eyJ") && s.matches('.').count() == 2 && !s.contains(char::is_whitespace)
}

/// A token payload copied from DevTools (camelCase or snake_case accepted).
fn tokens_from_payload(doc: &Value) -> Option<(String, Option<String>)> {
    let access = doc
        .get("access_token")
        .or_else(|| doc.get("accessToken"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())?
        .to_string();
    let refresh = doc
        .get("refresh_token")
        .or_else(|| doc.get("refreshToken"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    Some((access, refresh))
}

/// Redirect URL → (code, state). A bare code passes through; an error
/// redirect is rejected with its reason.
fn split_code_and_state(input: &str) -> Result<(String, Option<String>), String> {
    let Some((_, query)) = input.split_once('?') else {
        if input.contains('/') || input.contains(char::is_whitespace) {
            return Err(
                "that does not look like an authorization code — paste the redirect URL".into(),
            );
        }
        return Ok((input.to_string(), None));
    };
    let mut code = None;
    let mut state = None;
    let mut error = None;
    for pair in query.split(|c| c == '&' || c == '#') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        match key {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            "error" => error = Some(value.to_string()),
            _ => {}
        }
    }
    if let Some(e) = error {
        return Err(format!("authorization was rejected: {e}"));
    }
    match code.filter(|c| !c.is_empty()) {
        Some(c) => Ok((c, state)),
        None => Err("no code in the pasted URL — rerun the authorization link".into()),
    }
}

fn save_oauth_tokens(access: &str, refresh: Option<&str>) {
    let mut creds = load_creds();
    creds.access_token = access.trim().to_string();
    if let Some(r) = refresh.map(str::trim).filter(|r| !r.is_empty()) {
        creds.refresh_token = r.to_string();
    }
    save_creds(&creds);
}

/// Exchange the authorization code under our stored PKCE verifier.
async fn exchange_code(code: &str, state: Option<&str>) -> Result<String, String> {
    let pending = PENDING.lock().ok().and_then(|mut guard| guard.take());
    let Some(pending) = pending else {
        return Err("authorization session expired — open the link again".into());
    };
    if let Some(s) = state.filter(|s| !s.is_empty()) {
        if s != pending.state {
            return Err("authorization state mismatch — start the sign-in again".into());
        }
    }
    let resp = http()
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("code_verifier", pending.verifier.as_str()),
            ("state", pending.state.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("token request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token endpoint: HTTP {}", resp.status()));
    }
    let doc = json_body(resp, MAX_BODY_BYTES, "SenseNova token").await?;
    let (access, refresh) =
        tokens_from_payload(&doc).ok_or("token response carried no access_token")?;
    save_oauth_tokens(&access, refresh.as_deref());
    Ok(access)
}

/// Settings "Finish sign-in": take whatever the browser/DevTools yielded —
/// a redirect URL, the token payload JSON, a bare access token (JWT), or a
/// raw code — persist the credential pair, and return a live snapshot.
pub async fn oauth_finish(input: &str) -> Result<Snapshot, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("paste the redirect URL, token payload, or authorization code".into());
    }
    if input.starts_with('{') {
        let doc: Value =
            serde_json::from_str(input).map_err(|e| format!("not valid JSON: {e}"))?;
        let (access, refresh) =
            tokens_from_payload(&doc).ok_or("the pasted JSON carries no access_token")?;
        save_oauth_tokens(&access, refresh.as_deref());
        return fetch_pools(&access).await.map_err(|e| describe(&e));
    }
    if looks_like_jwt(input) {
        save_oauth_tokens(input, None);
        return fetch_pools(input).await.map_err(|e| describe(&e));
    }
    let (code, state) = split_code_and_state(input)?;
    let access = exchange_code(&code, state.as_deref()).await?;
    fetch_pools(&access).await.map_err(|e| describe(&e))
}

async fn fetch_pools(token: &str) -> Result<Snapshot, FetchErr> {
    let resp = http()
        .get(POOL_URL)
        .bearer_auth(token)
        .header("Accept-Language", "zh-CN")
        .send()
        .await
        .map_err(|e| FetchErr::Other(format!("SenseNova request: {e}")))?;
    if resp.status().as_u16() == 429 {
        return Err(FetchErr::RateLimited);
    }
    if matches!(resp.status().as_u16(), 401 | 403) {
        return Err(FetchErr::Auth);
    }
    if !resp.status().is_success() {
        return Err(FetchErr::Other(format!("SenseNova endpoint: HTTP {}", resp.status())));
    }
    let doc = json_body(resp, MAX_BODY_BYTES, "SenseNova")
        .await
        .map_err(FetchErr::Other)?;
    parse_snapshot(&doc).map_err(FetchErr::Other)
}

/// All numeric fields are STRING decimals ("1.12"); reset_at is unix
/// SECONDS as a string (→ ms inside). Pools are identified by `pool_type`
/// ("default" | "dedicated") — never by display name, which is localized.
fn parse_snapshot(doc: &Value) -> Result<Snapshot, String> {
    let pools = doc
        .get("pools")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .ok_or("SenseNova response has no pools")?;
    let plan = doc
        .pointer("/plan/name")
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut metrics = Vec::new();
    for pool in pools {
        let pool_type = pool.get("pool_type").and_then(Value::as_str).unwrap_or("");
        let (label_5h, label_7d) = match pool_type {
            "default" => ("Credits", "Credits Weekly"),
            "dedicated" => ("Flash-Lite", "Flash-Lite Weekly"),
            // Unknown pool shape (plan tier change): ignore, don't guess.
            _ => continue,
        };
        for (key, label, period_ms) in [
            ("window_5h", label_5h, 5 * 3_600_000i64),
            ("window_7d", label_7d, 7 * 86_400_000i64),
        ] {
            let Some(w) = pool.get(key) else { continue };
            let (Some(used), Some(limit)) = (dec(w.get("used")), dec(w.get("limit"))) else {
                continue; // malformed window: skip the row, keep the card
            };
            if limit <= 0.0 {
                continue;
            }
            let pct = (used / limit * 100.0).clamp(0.0, 100.0);
            let reset = w
                .get("reset_at")
                .and_then(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                .filter(|s| *s > 0)
                .map(|s| s * 1000);
            metrics.push(
                Metric::progress(label, pct, Some(format!("{used:.2} / {limit:.0} credits")))
                    .with_reset(reset, Some(period_ms)),
            );
        }
        // Grant (返赠) balance rides as a side row when non-zero.
        if let Some(grant) = dec(pool.get("grant_balance")).filter(|g| *g > 0.0) {
            let expiry = pool
                .get("nearest_grant_expiry")
                .and_then(|v| v.as_str().and_then(|s| s.parse::<i64>().ok()))
                .filter(|s| *s > 0)
                .and_then(|s| chrono::DateTime::from_timestamp(s, 0))
                .map(|d| d.format("%m-%d").to_string())
                .unwrap_or_default();
            metrics.push(Metric::text(
                "Grant",
                format!("{grant:.0} credits{}", if expiry.is_empty() { String::new() } else { format!(" · {expiry}") }),
            ));
        }
    }
    if metrics.is_empty() {
        return Err("SenseNova response had no usable pools".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

fn dec(v: Option<&Value>) -> Option<f64> {
    v?.as_f64().or_else(|| v?.as_str()?.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim live response captured 2026-10-03 (string decimals, unix
    /// second resets, dual pools) — inlined so the harness carries its own
    /// fixture instead of reaching into gitignored temp/.
    const REAL_SAMPLE: &str = r#"{"plan":{"id":"free","name":"Free Plan","type":"TOKEN_PLAN_PLAN_TYPE_FREE"},"pools":[{"id":"pool_f2f9196e-3009-42dd-a044-46d30cc9143a","name":"通用积分池","model_ids":["glm-5.2"],"window_5h":{"limit":"60000","used":"1.12","remaining":"59998.88","reset_at":"1791012792"},"window_7d":{"limit":"600000","used":"70679.296","remaining":"529320.704","reset_at":"1791181992"},"grant_balance":"0","nearest_grant_expiry":"0","nearest_grant_expiring_balance":"0","pool_type":"default"},{"id":"pool_2afaf48e-1974-4f3e-a466-9b61f297d177","name":"Flash-Lite积分池","model_ids":["sensenova-6.8-flash-lite"],"window_5h":{"limit":"60000","used":"0.198","remaining":"59999.802","reset_at":"1791012792"},"window_7d":{"limit":"600000","used":"0.38","remaining":"599999.62","reset_at":"1791181992"},"grant_balance":"0","nearest_grant_expiry":"0","nearest_grant_expiring_balance":"0","pool_type":"dedicated"}]}"#;

    #[test]
    fn parses_the_real_dual_pool_shape() {
        let doc: Value = serde_json::from_str(REAL_SAMPLE).unwrap();
        let snap = parse_snapshot(&doc).expect("parse");
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan.as_deref(), Some("Free Plan"));
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["Credits", "Credits Weekly", "Flash-Lite", "Flash-Lite Weekly"]
        );
        // 1.12 / 60000 → ~0.0019%
        let five_h = &snap.metrics[0];
        assert!((five_h.used_percent.unwrap() - 0.0018666).abs() < 0.0001);
        assert_eq!(five_h.period_ms, Some(5 * 3_600_000));
        assert_eq!(five_h.resets_at, Some(1791012792 * 1000));
        // Weekly default pool: 70679.296 / 600000 → ~11.78%
        assert!((snap.metrics[1].used_percent.unwrap() - 11.7799).abs() < 0.001);
        // Dedicated pool is matched by pool_type, not the localized name.
        assert!((snap.metrics[2].used_percent.unwrap() - 0.00033).abs() < 0.0001);
        // Zero grants → no Grant side rows.
        assert!(!snap.metrics.iter().any(|m| m.label == "Grant"));
    }

    #[test]
    fn grant_rows_appear_only_when_nonzero() {
        let doc = serde_json::json!({
            "plan": {"name": "Free Plan", "type": "TOKEN_PLAN_PLAN_TYPE_FREE"},
            "pools": [{
                "pool_type": "default",
                "window_5h": {"limit": "60000", "used": "100", "remaining": "59900", "reset_at": "1791012792"},
                "window_7d": {"limit": "600000", "used": "100", "remaining": "599900", "reset_at": "1791181992"},
                "grant_balance": "250",
                "nearest_grant_expiry": "1792000000",
                "nearest_grant_expiring_balance": "250"
            }]
        });
        let snap = parse_snapshot(&doc).expect("parse");
        let grant = snap.metrics.iter().find(|m| m.label == "Grant").expect("grant row");
        assert!(grant.value.as_deref().unwrap().contains("250 credits"));
        assert!(grant.value.as_deref().unwrap().contains("·"));
    }

    #[test]
    fn unknown_pool_types_and_malformed_windows_are_tolerated() {
        let doc = serde_json::json!({
            "pools": [
                {"pool_type": "future_tier", "window_5h": {"limit": "10", "used": "1"}},
                {"pool_type": "default", "window_5h": {"limit": "0", "used": "0"}},
                {"pool_type": "default", "window_5h": {"used": "5"}, "window_7d": {"limit": "100", "used": "5", "reset_at": "0"}}
            ]
        });
        let snap = parse_snapshot(&doc).expect("parse");
        // Only the well-formed window survived; reset_at "0" means unknown.
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].resets_at, None);
    }

    #[test]
    fn no_pools_is_an_error() {
        assert!(parse_snapshot(&serde_json::json!({"pools": []})).is_err());
        assert!(parse_snapshot(&serde_json::json!({})).is_err());
    }

    #[test]
    fn jwt_exp_precheck_decodes_a_real_shape() {
        use base64::Engine;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"exp":9999999999,"scp":["openid"]}"#);
        let token = format!("h.{}.s", payload);
        let ms = token_expires_in_ms(&token).expect("decodes");
        assert!(ms > 3_000_000_000_000); // far future
        assert_eq!(token_expires_in_ms("garbage"), None);
        assert_eq!(token_expires_in_ms(""), None);
    }

    #[test]
    fn creds_roundtrip_keeps_missing_refresh_empty() {
        let c = Creds { access_token: "a".into(), refresh_token: String::new() };
        let s = serde_json::to_string(&c).unwrap();
        assert!(!s.contains("refresh_token"));
        let back: Creds = serde_json::from_str(&s).unwrap();
        assert_eq!(back.access_token, "a");
        assert_eq!(back.refresh_token, "");
    }

    #[test]
    fn backoff_ladder_progression() {
        // The schedule itself: 30s, 5m, 30m (then stays at the last rung).
        assert_eq!(BACKOFFS_MS, [30_000, 300_000, 1_800_000]);
    }

    #[test]
    fn pkce_challenge_matches_the_rfc_7636_vector() {
        let challenge = generate_code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk");
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        let verifier = generate_code_verifier();
        assert!(verifier.len() >= 43);
        assert!(!verifier.contains('+') && !verifier.contains('/') && !verifier.contains('='));
    }

    #[test]
    fn auth_url_carries_pkce_and_the_offline_scope() {
        let url = auth_url("CHALLENGE", "STATE");
        assert!(url.starts_with("https://platform.sensenova.cn/oauth2/auth?"));
        assert!(url.contains("client_id=nova"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("code_challenge=CHALLENGE"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fplatform.sensenova.cn"));
        assert!(url.contains("offline_access"));
        assert!(url.contains("state=STATE"));
    }

    #[test]
    fn redirect_urls_and_bare_codes_split_cleanly() {
        let (code, state) =
            split_code_and_state("https://platform.sensenova.cn/?code=abc123&state=xyz&lang=zh-CN")
                .unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state.as_deref(), Some("xyz"));
        assert_eq!(split_code_and_state("abc123").unwrap(), ("abc123".into(), None));
        let err = split_code_and_state("https://platform.sensenova.cn/?error=access_denied")
            .err()
            .unwrap();
        assert!(err.contains("access_denied"));
        assert!(split_code_and_state("https://platform.sensenova.cn/?state=xyz").is_err());
        assert!(split_code_and_state("not a code").is_err());
    }

    #[test]
    fn token_payloads_accept_both_spellings_and_a_bare_jwt() {
        let snake = serde_json::json!({"access_token": "a", "refresh_token": "r"});
        assert_eq!(tokens_from_payload(&snake), Some(("a".into(), Some("r".into()))));
        let camel = serde_json::json!({"accessToken": "a", "refreshToken": "r"});
        assert_eq!(tokens_from_payload(&camel), Some(("a".into(), Some("r".into()))));
        assert_eq!(tokens_from_payload(&serde_json::json!({"refresh_token": "r"})), None);
        assert!(looks_like_jwt("eyJhbGciOiJub25lIn0.eyJleHAiOjF9.sig"));
        assert!(!looks_like_jwt("abc"));
        assert!(!looks_like_jwt("https://platform.sensenova.cn/?code=x"));
    }

    #[test]
    fn legacy_api_key_blob_reads_as_access_token() {
        let creds: Creds = serde_json::from_str(r#"{"apiKey":"tok"}"#).unwrap();
        assert_eq!(creds.access_token, "tok");
        assert!(creds.refresh_token.is_empty());
    }
}
