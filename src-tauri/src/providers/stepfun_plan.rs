//! StepFun Step Plan subscription quota — covers BOTH billing shapes the
//! platform has run since the 2026-06-18 upgrade: the legacy Coding Plan
//! (rolling 5h/weekly windows, prompt-counted) and the Token Plan (monthly
//! Credit pool + 30-day top-up packs). Classification is by payload shape,
//! NOT by `plan_family` — CodexBar issue #2491 shows the family id lies
//! during migration.
//!
//! Credentials live in `%APPDATA%/Pane/stepfun-plan.json` (same file the
//! Settings paste-box has always written; a legacy `{apiKey}` blob is read
//! as `token`). The stored token is an Oasis-Token pair
//! "<access JWT>...<refresh JWT>" — the refresh half carries the device_id
//! the platform demands in Oasis-Webid, a bare access JWT gets rejected as
//! "embezzled".
//!
//! Autonomy ladder (no timers — renew on use, same shape as sensenova):
//!   L0  cached access JWT whose exp is still ahead → just fetch.
//!   L1  expired/401 → POST RefreshToken with the stored pair (single-flight
//!       + backoff); a rotated pair is persisted atomically.
//!   L2  refresh dead (invalid grant) but username+password are on file →
//!       the full 3-step login (INGRESSCOOKIE → RegisterDevice →
//!       SignInByPassword) — every password mistake backs off 30s → 5m →
//!       30m, then stops until the next process start.
//!   L3  nothing on file → no_credentials: fill the account in Settings.
//! HTTP 429 is throttling, never touches the ladder.

use super::{http, json_body, Metric, Snapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;

const ID: &str = "stepfun-plan";
const NAME: &str = "StepFun Step Plan";
const PLATFORM: &str = "https://platform.stepfun.com";
const RATE_URL: &str =
    "https://platform.stepfun.com/api/step.openapi.devcenter.Dashboard/QueryStepPlanRateLimit";
const STATUS_URL: &str =
    "https://platform.stepfun.com/api/step.openapi.devcenter.Dashboard/GetStepPlanStatus";
const REGISTER_DEVICE_URL: &str =
    "https://platform.stepfun.com/passport/proto.api.passport.v1.PassportService/RegisterDevice";
const SIGN_IN_URL: &str =
    "https://platform.stepfun.com/passport/proto.api.passport.v1.PassportService/SignInByPassword";
const REFRESH_URL: &str =
    "https://platform.stepfun.com/passport/proto.api.passport.v1.PassportService/RefreshToken";
const APP_ID: &str = "10300";
/// The web frontend's pre-registration webid — the passport endpoints
/// 4xx without it, and it doubles as the fallback when a token pair carries
/// no device_id claim (derivation cases the platform rejects by name).
const DEFAULT_WEBID: &str = "c8a1002d2c457e758785a9979832217c7c0b884c";
/// The passport layer fingerprint-checks the UA; a non-browser agent string
/// gets a 403 on some edges.
const BROWSER_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
                          (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36";
const MAX_BODY_BYTES: usize = 64 * 1024;

const BACKOFFS_MS: [i64; 3] = [30_000, 5 * 60_000, 30 * 60_000];

/// Stored credential triple. `apiKey` is read as `token` so the file the
/// Settings paste-box always wrote keeps working untouched.
#[derive(Clone, Default, Serialize, Deserialize)]
struct Creds {
    #[serde(default, alias = "apiKey")]
    token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    username: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    password: String,
}

fn creds_path() -> PathBuf {
    super::config_dir().join("stepfun-plan.json")
}

fn load_creds() -> Creds {
    std::fs::read_to_string(creds_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Atomic persist: sibling temp file + rename, so a crash mid-write can
/// never leave a half credential file.
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

/// Backoff for the entire renewal/login ladder. FETCH_LOCK serializes callers.
#[derive(Default)]
struct Renew {
    failures: u32,
    next_attempt_ms: i64,
    last_error: Option<AuthErr>,
}

static RENEW: Mutex<Renew> = Mutex::new(Renew {
    failures: 0,
    next_attempt_ms: 0,
    last_error: None,
});

// Waiters reload credentials after the active refresh has persisted its pair.
// Holding an async guard also releases the flight when a future is cancelled.
static FETCH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Why a login/refresh failed — decides backoff and user messaging.
#[derive(Debug, Clone)]
enum AuthErr {
    /// Credential rejected (401/invalid grant/bad password): final for this
    /// attempt, back off, never silently retry the same secret.
    Invalid(String),
    /// Network/5xx/parse trouble: retryable with backoff.
    Transient(String),
}

type AuthResult<T> = Result<T, AuthErr>;

pub fn local_credential_hint() -> Option<String> {
    let c = load_creds();
    (!c.token.is_empty() || (!c.username.is_empty() && !c.password.is_empty()))
        .then(|| "Pane StepFun Step Plan session".into())
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        // `fetch` turns a rejected-but-present credential into a
        // needs_setup snapshot itself (the card must survive); what reaches
        // here is a genuine fault.
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// The "credential present, one more step needed" card: hint + whatever the
/// stored key can already read.
async fn needs_setup_snapshot() -> Snapshot {
    needs_setup_for(&load_creds().token).await
}

async fn needs_setup_for(token: &str) -> Snapshot {
    let shape = if token.trim().is_empty() {
        TokenShape::Empty
    } else {
        token_shape(token)
    };
    // The wallet read is the same Open-Platform endpoint the `stepfun`
    // family uses; a Step Plan session token cannot read it, an API key can.
    let mut metrics = Vec::new();
    let plan = if let Some(wallet) = wallet_snapshot(token).await {
        metrics = wallet.metrics;
        wallet.plan
    } else {
        None
    };
    Snapshot::needs_setup(ID, NAME, plan, metrics, &setup_hint(&shape))
}

/// Pay-as-you-go wallet via the Open-Platform accounts endpoint, when the
/// stored credential is an API key. `None` when it isn't readable.
async fn wallet_snapshot(token: &str) -> Option<Snapshot> {
    if token.trim().is_empty() || token_shape(token) != TokenShape::NotAJwt {
        return None;
    }
    let snap = crate::providers::stepfun::snapshot_with_key(token).await;
    (snap.status == "ok").then_some(snap)
}

/// Live test of pasted credentials (Customize "Test"); never saved. A pasted
/// token is used as-is; account+password runs a throwaway login.
pub async fn snapshot_with_creds(token: &str, username: &str, password: &str) -> Snapshot {
    let token = token.trim();
    if !token.is_empty() {
        return match fetch_with(token).await {
            Ok(s) => s,
            Err(FetchErr::Auth) => pasted_token_fallback(token, wallet_snapshot(token).await),
            Err(e) => Snapshot::error(ID, NAME, describe_fetch(e)),
        };
    }
    match full_login(username.trim(), password).await {
        Ok(tok) => match fetch_with(&tok).await {
            Ok(s) => s,
            Err(e) => Snapshot::error(ID, NAME, describe_fetch(e)),
        },
        Err(AuthErr::Invalid(m)) => {
            Snapshot::error(ID, NAME, format!("Step Plan sign-in failed: {m}"))
        }
        Err(AuthErr::Transient(m)) => {
            Snapshot::error(ID, NAME, format!("Step Plan sign-in unavailable: {m}"))
        }
    }
}

// A pasted key must prove connectivity before Test can say "connected".
// Stored cards may remain visible with guidance even when that proof fails.
fn pasted_token_fallback(token: &str, wallet: Option<Snapshot>) -> Snapshot {
    let hint = setup_hint(&token_shape(token));
    match wallet {
        Some(wallet) => Snapshot::needs_setup(ID, NAME, wallet.plan, wallet.metrics, &hint),
        None => Snapshot::error(ID, NAME, hint),
    }
}

fn describe_fetch(e: FetchErr) -> String {
    match e {
        FetchErr::Auth => "Step Plan session was rejected".into(),
        FetchErr::Other(m) => m,
    }
}

/// Settings "Save": persist the triple. Account+password alone triggers one
/// real login so the stored token pair is born here (and any typo in the
/// password is caught at save time, not at the next refresh).
pub async fn save_creds_interactive(
    token: &str,
    username: &str,
    password: &str,
) -> Result<(), String> {
    let _flight = FETCH_LOCK.lock().await;
    let mut creds = load_creds();
    // Non-empty fields overwrite; an empty field keeps whatever is stored —
    // filling only the account must not wipe the token pair (and vice versa).
    if !token.trim().is_empty() {
        creds.token = token.trim().to_string();
    }
    if !username.trim().is_empty() {
        creds.username = username.trim().to_string();
    }
    if !password.is_empty() {
        creds.password = password.to_string();
    }
    if creds.token.is_empty() && (!creds.username.is_empty() && !creds.password.is_empty()) {
        let tok = full_login(&creds.username, &creds.password)
            .await
            .map_err(|e| match e {
                AuthErr::Invalid(m) => format!("sign-in failed: {m}"),
                AuthErr::Transient(m) => format!("sign-in unavailable: {m}"),
            })?;
        creds.token = tok;
    }
    save_creds(&creds);
    *renew_state() = Renew::default();
    Ok(())
}

pub async fn clear_creds() {
    let _flight = FETCH_LOCK.lock().await;
    let _ = std::fs::remove_file(creds_path());
    *renew_state() = Renew::default();
}

/// What a pasted credential actually is. The console API only accepts a
/// session pair ("<access JWT>...<refresh JWT>"); an Open-Platform API key
/// looks like a key, not a JWT, and gets `token is illegal` — the user must
/// be told that difference instead of seeing a bare failure.
#[derive(Debug, PartialEq, Eq)]
enum TokenShape {
    /// "<access JWT>...<refresh JWT>" — the shape the console accepts.
    SessionPair,
    /// A single JWT (no pair): the refresh half is missing.
    SingleJwt,
    /// Not a JWT at all (an Open-Platform API key, e.g. `sk-…`/32-char key).
    NotAJwt,
    Empty,
}

fn token_shape(token: &str) -> TokenShape {
    let t = token.trim();
    if t.is_empty() {
        return TokenShape::Empty;
    }
    let halves: Vec<&str> = t.rsplit("...").collect();
    if halves.len() >= 2 && halves.iter().all(|h| looks_like_jwt(h)) {
        return TokenShape::SessionPair;
    }
    if looks_like_jwt(t) {
        return TokenShape::SingleJwt;
    }
    TokenShape::NotAJwt
}

/// Three dot-separated, non-empty chunks whose middle decodes as JSON —
/// the shape test only. It must NOT require a `device_id` claim: the access
/// half of a real Oasis pair carries no device_id (only the refresh half
/// does), and demanding one misclassified a perfectly good session pair as
/// "not a JWT" (2026-10-15).
fn looks_like_jwt(s: &str) -> bool {
    use base64::Engine;
    let s = s.trim();
    let mut parts = s.split('.');
    let (Some(head), Some(payload), Some(sig), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if head.is_empty() || payload.is_empty() || sig.is_empty() {
        return false;
    }
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload))
        .is_ok_and(|bytes| serde_json::from_slice::<Value>(&bytes).is_ok())
}

/// The user-facing remediation for a credential shape this API can't use.
fn setup_hint(shape: &TokenShape) -> String {
    match shape {
        TokenShape::SessionPair => {
            "Step Plan session was rejected — it may have expired; save the credentials again in Settings.".into()
        }
        TokenShape::SingleJwt => {
            "Step Plan needs the FULL session pair, not just the access token: open platform.stepfun.com in a browser, log in, then DevTools → Application → Cookies → copy the whole Oasis-Token value (two JWTs joined by \"...\") into Settings → Step Plan.".into()
        }
        TokenShape::NotAJwt => {
            "This is an Open-Platform API key, not a Step Plan session. An API key can only read the pay-as-you-go wallet, never the plan quota. To read Step Plan usage, open platform.stepfun.com → DevTools → Application → Cookies → copy the Oasis-Token value, or save your StepFun account + password in Settings (Pane then renews the session itself).".into()
        }
        TokenShape::Empty => "Save your StepFun Step Plan session or account+password in Settings.".into(),
    }
}

/// The stored Oasis-Webid for a pair: the refresh half's `device_id` claim
/// (reverse order — that's where the platform puts it), else the frontend's
/// default. The server cross-checks this value against the token's device
/// and rejects a mismatch as "oasis-token is embezzled".
fn webid_for_token(token: &str) -> String {
    for half in token.rsplit("...") {
        if let Some(id) = device_id_of_jwt(half).filter(|s| !s.is_empty()) {
            return id;
        }
    }
    DEFAULT_WEBID.to_string()
}

/// One claim out of a JWT's payload, when the token parses at all.
fn jwt_claim(jwt: &str, name: &str) -> Option<Value> {
    use base64::Engine;
    let payload_b64 = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload_b64))
        .ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    v.get(name).cloned()
}

fn device_id_of_jwt(jwt: &str) -> Option<String> {
    jwt_claim(jwt, "device_id")?.as_str().map(str::to_string)
}

/// The account identity the platform stamps into BOTH halves of a pair. A
/// genuine rotation preserves it; the anonymous session the passport service
/// hands back for a token it cannot use carries a different one.
fn oasis_id_of(token: &str) -> Option<String> {
    let mut identity = None;
    for half in token.split("...") {
        let Some(claim) = jwt_claim(half, "oasis_id") else {
            continue;
        };
        let id = match claim {
            Value::Number(n) => n.as_u64()?,
            Value::String(s) => s.parse::<u64>().ok()?,
            _ => return None,
        };
        if id == 0 {
            return None;
        }
        let id = id.to_string();
        if identity.as_ref().is_some_and(|old| old != &id) {
            return None;
        }
        identity = Some(id);
    }
    identity
}

/// Accept a rotated pair only when it proves it is the SAME account.
///
/// A 200 does NOT mean the rotation worked: handed a token it cannot use,
/// PassportService/RefreshToken answers 200 with a brand-new ANONYMOUS device
/// session (`activated:false`, `age:1`, `device_id` = the pre-registration
/// webid) — measured 2026-10-15. Persisting that would silently destroy the
/// user's real session and strand the card on "rejected", so the pair has to
/// carry a matching `oasis_id`; anything else leaves the stored credential
/// untouched and sends the ladder to `needs_setup`.
fn accept_rotation(stored: &str, rotated: &str) -> Result<(), AuthErr> {
    reject_anonymous_access(rotated)?;
    match (oasis_id_of(stored), oasis_id_of(rotated)) {
        (Some(old), Some(new)) if old == new => Ok(()),
        (Some(_), Some(_)) => Err(AuthErr::Invalid(
            "refresh answered with a different account's session".into(),
        )),
        _ => Err(AuthErr::Invalid(
            "refresh answered without an account identity".into(),
        )),
    }
}

/// A successful sign-in must land on a DIFFERENT identity than the throwaway
/// device session it started from: `RegisterDevice` mints an anonymous
/// account, and signing in has to switch away from it. A response that just
/// carries the device session back means the password was not accepted — the
/// same 200-without-success trap as `accept_rotation`, one step earlier.
fn accept_login(device: &str, signed_in: &str) -> Result<(), AuthErr> {
    reject_anonymous_access(signed_in)?;
    match (oasis_id_of(device), oasis_id_of(signed_in)) {
        (Some(anon), Some(user)) if anon != user => Ok(()),
        _ => Err(AuthErr::Invalid(
            "sign-in did not return a signed-in account".into(),
        )),
    }
}

fn reject_anonymous_access(token: &str) -> AuthResult<()> {
    if jwt_claim(token.split("...").next().unwrap_or_default(), "activated")
        == Some(Value::Bool(false))
    {
        return Err(AuthErr::Invalid(
            "returned an anonymous device session".into(),
        ));
    }
    Ok(())
}

/// Access-segment JWT exp as ms-from-now (60s skew floor); None when the
/// token is unparseable or the pair has no access half.
fn access_expires_in_ms(token: &str) -> Option<i64> {
    let jwt = token.split("...").next()?;
    let exp = jwt_claim(jwt, "exp")?.as_i64()?;
    Some(exp * 1000 - chrono::Utc::now().timestamp_millis())
}

async fn fetch() -> Result<Snapshot, String> {
    let _flight = FETCH_LOCK.lock().await;
    let mut creds = load_creds();
    if creds.token.is_empty() && creds.username.is_empty() {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Sign in to StepFun Step Plan in Settings (account + password keeps you signed in).",
        ));
    }

    // L0: locally-valid access JWT → straight to the quota endpoint. A 401
    // despite a valid local exp (revoked, clock skew) falls through.
    if access_expires_in_ms(&creds.token).is_some_and(|ms| ms > 60_000) {
        match fetch_with(&creds.token).await {
            Ok(s) => return Ok(s),
            Err(FetchErr::Auth) => {}
            Err(e) => return Err(describe_fetch(e)),
        }
    }

    // L1: rotate the pair via RefreshToken (single-flight + backoff).
    let refreshed = renew(&mut creds).await.map_err(|e| describe_login(&e))?;
    let token = match refreshed {
        Some(t) => t,
        None => {
            // A credential is on file but the plan endpoint refuses it
            // (typical: an Open-Platform API key, or a session without
            // its refresh half). Keep the card and say what to provide.
            return Ok(needs_setup_snapshot().await);
        }
    };
    match fetch_with(&token).await {
        Ok(s) => Ok(s),
        Err(FetchErr::Auth) => Ok(needs_setup_snapshot().await),
        Err(FetchErr::Other(m)) => Err(m),
    }
}

/// The renewal bookkeeping lock. A poisoned mutex must not wedge the ladder:
/// `Renew` only holds backoff state; the async flight guard releases on drop.
fn renew_state() -> std::sync::MutexGuard<'static, Renew> {
    RENEW
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Refresh, then password login only on rejection, under FETCH_LOCK.
/// Both stages share backoff; only successful pairs are persisted.
async fn renew(creds: &mut Creds) -> AuthResult<Option<String>> {
    // Scoped so the guard provably dies before the await below (the guarded
    // future must stay Send for lib.rs's refresh pool).
    {
        let guard = renew_state();
        let now = chrono::Utc::now().timestamp_millis();
        if guard.failures > 0 && now < guard.next_attempt_ms {
            return match &guard.last_error {
                Some(AuthErr::Transient(m)) => Err(AuthErr::Transient(m.clone())),
                _ if !creds.username.is_empty() && !creds.password.is_empty() => Err(guard
                    .last_error
                    .clone()
                    .unwrap_or_else(|| AuthErr::Invalid("sign-in cooling down".into()))),
                _ => Ok(None),
            };
        }
    }

    let mut result = if creds.token.is_empty() {
        Err(AuthErr::Invalid("no session pair".into()))
    } else {
        refresh_pair(&creds.token).await
    };
    // Network failures and throttling must never trigger password login.
    let can_login = !creds.username.is_empty() && !creds.password.is_empty();
    if matches!(result, Err(AuthErr::Invalid(_))) && can_login {
        result = full_login(&creds.username, &creds.password).await;
    }

    {
        let mut guard = renew_state();
        match result {
            Ok(pair) => {
                guard.failures = 0;
                guard.next_attempt_ms = 0;
                guard.last_error = None;
                creds.token = pair.clone();
                save_creds(creds);
                Ok(Some(pair))
            }
            Err(e) => {
                let step = (guard.failures as usize).min(BACKOFFS_MS.len() - 1);
                guard.next_attempt_ms = chrono::Utc::now().timestamp_millis() + BACKOFFS_MS[step];
                guard.failures = guard.failures.saturating_add(1);
                guard.last_error = Some(e.clone());
                match e {
                    AuthErr::Invalid(_) if !can_login => Ok(None),
                    e => Err(e),
                }
            }
        }
    }
}

fn describe_login(e: &AuthErr) -> String {
    match e {
        AuthErr::Invalid(m) => format!("Step Plan sign-in failed: {m}"),
        AuthErr::Transient(m) => format!("Step Plan sign-in unavailable: {m}"),
    }
}

/// Full 3-step login (no lock — callers hold the FETCH_LOCK guard or are
/// interactive one-shots): INGRESSCOOKIE → RegisterDevice → SignInByPassword.
async fn full_login(username: &str, password: &str) -> AuthResult<String> {
    let ingress = get_ingress_cookie().await?;
    let anon = register_device(&ingress).await?;
    let webid = webid_for_token(&anon);
    let resp = http()
        .post(SIGN_IN_URL)
        .header("User-Agent", BROWSER_UA)
        .header("Content-Type", "application/json")
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", &webid)
        .header(
            "Cookie",
            format!("Oasis-Token={anon}; Oasis-Webid={webid}; INGRESSCOOKIE={ingress}"),
        )
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await
        .map_err(|e| AuthErr::Transient(format!("network: {e}")))?;
    if resp.status().as_u16() == 429 {
        return Err(AuthErr::Transient("rate-limited".into()));
    }
    let status = resp.status();
    if !status.is_success() {
        // Never include the upstream body: auth responses may echo inputs.
        return Err(auth_http_error(status.as_u16(), "sign-in"));
    }
    let doc: Value = resp
        .json()
        .await
        .map_err(|e| AuthErr::Transient(format!("body: {e}")))?;
    let pair = combine_token(&doc, "login")
        .ok_or_else(|| AuthErr::Invalid("no access token in login response".into()))?;
    accept_login(&anon, &pair)?;
    Ok(pair)
}

/// Step 1: GET the platform homepage and harvest INGRESSCOOKIE from
/// Set-Cookie.
async fn get_ingress_cookie() -> AuthResult<String> {
    let resp = http()
        .get(PLATFORM)
        .header("User-Agent", BROWSER_UA)
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", DEFAULT_WEBID)
        .send()
        .await
        .map_err(|e| AuthErr::Transient(format!("network: {e}")))?;
    if !resp.status().is_success() {
        return Err(AuthErr::Transient(format!(
            "homepage HTTP {}",
            resp.status()
        )));
    }
    for value in resp.headers().get_all("set-cookie") {
        let Ok(s) = value.to_str() else { continue };
        if let Some(rest) = s.split("INGRESSCOOKIE=").nth(1) {
            let cookie = rest.split(';').next().unwrap_or("").trim();
            if !cookie.is_empty() {
                return Ok(cookie.to_string());
            }
        }
    }
    Err(AuthErr::Transient("INGRESSCOOKIE not issued".into()))
}

/// Step 2: anonymous device registration → "access...refresh" pair.
async fn register_device(ingress: &str) -> AuthResult<String> {
    let resp = http()
        .post(REGISTER_DEVICE_URL)
        .header("User-Agent", BROWSER_UA)
        .header("Content-Type", "application/json")
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", DEFAULT_WEBID)
        .header("Cookie", format!("INGRESSCOOKIE={ingress}"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| AuthErr::Transient(format!("network: {e}")))?;
    if !resp.status().is_success() {
        return Err(AuthErr::Transient(format!(
            "device registration HTTP {}",
            resp.status()
        )));
    }
    let doc: Value = resp
        .json()
        .await
        .map_err(|e| AuthErr::Transient(format!("body: {e}")))?;
    combine_token(&doc, "device").ok_or_else(|| AuthErr::Transient("no device token".into()))
}

/// Rotate the stored pair. A 4xx body ("token is expired"/"illegal") means
/// the pair is dead for good — that routes the caller to full login. A 2xx is
/// NOT sufficient proof of success (see `accept_rotation`).
async fn refresh_pair(token: &str) -> AuthResult<String> {
    let webid = webid_for_token(token);
    let resp = http()
        .post(REFRESH_URL)
        .header("User-Agent", BROWSER_UA)
        .header("Content-Type", "application/json")
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", &webid)
        .header("Oasis-Token", token)
        .header(
            "Cookie",
            format!("Oasis-Token={token}; Oasis-Webid={webid}"),
        )
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| AuthErr::Transient(format!("network: {e}")))?;
    if !resp.status().is_success() {
        return Err(auth_http_error(resp.status().as_u16(), "refresh"));
    }
    let doc: Value = resp
        .json()
        .await
        .map_err(|e| AuthErr::Transient(format!("body: {e}")))?;
    let pair = combine_token(&doc, "refresh")
        .ok_or_else(|| AuthErr::Invalid("no token in refresh".into()))?;
    accept_rotation(token, &pair)?;
    Ok(pair)
}

fn auth_http_error(status: u16, operation: &str) -> AuthErr {
    let message = format!("{operation} HTTP {status}");
    if status == 429 || status >= 500 {
        AuthErr::Transient(message)
    } else {
        AuthErr::Invalid(message)
    }
}

/// {accessToken:{raw}, refreshToken:{raw}} → "<access>...<refresh>" — the
/// pair form the platform's device_id check expects.
fn combine_token(doc: &Value, _what: &str) -> Option<String> {
    let access = doc
        .get("accessToken")
        .and_then(|t| t.get("raw"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())?;
    match doc
        .get("refreshToken")
        .and_then(|t| t.get("raw"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        Some(refresh) => Some(format!("{access}...{refresh}")),
        None => Some(access.to_string()),
    }
}

/// How a quota fetch failed — 401/403 sends the ladder running, 429 is
/// plain throttling that must not.
enum FetchErr {
    Auth,
    Other(String),
}

impl FetchErr {
    fn from_status(code: u16) -> Option<Self> {
        match code {
            401 | 403 => Some(FetchErr::Auth),
            429 => Some(FetchErr::Other(
                "Step Plan rate-limited — retrying next cycle".into(),
            )),
            _ => None,
        }
    }
}

async fn post_usage(token: &str, url: &str) -> Result<Value, FetchErr> {
    let webid = webid_for_token(token);
    let mut req = http()
        .post(url)
        .header("User-Agent", BROWSER_UA)
        .header("Content-Type", "application/json")
        .header("oasis-appid", APP_ID)
        .header("oasis-platform", "web")
        .header("oasis-webid", &webid)
        .header(
            "Cookie",
            format!("Oasis-Token={token}; Oasis-Webid={webid}"),
        );
    if url != SIGN_IN_URL {
        req = req.header("Oasis-Token", token);
    }
    let r = req
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| FetchErr::Other(format!("Step Plan request: {e}")))?;
    if let Some(e) = FetchErr::from_status(r.status().as_u16()) {
        return Err(e);
    }
    if !r.status().is_success() {
        return Err(FetchErr::Other(format!(
            "Step Plan endpoint: HTTP {}",
            r.status()
        )));
    }
    json_body(r, MAX_BODY_BYTES, "Step Plan")
        .await
        .map_err(FetchErr::Other)
}

async fn fetch_with(token: &str) -> Result<Snapshot, FetchErr> {
    let rate = post_usage(token, RATE_URL).await?;
    let status = post_usage(token, STATUS_URL).await.ok();
    parse_snapshot(&rate, status.as_ref()).map_err(FetchErr::Other)
}

fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(|x| x.as_f64().or_else(|| x.as_str()?.parse().ok()))
}

fn ts(v: Option<&Value>) -> Option<i64> {
    num(v).map(|s| {
        let secs = s as i64;
        if secs > 0 && secs < 2_000_000_000_000 {
            secs * 1000
        } else {
            secs
        }
    })
}

/// Shape decides the rows: a live rolling window means the legacy Coding
/// Plan (its reset_time is in the future); otherwise, if the credit pool is
/// present, this is the Token Plan. Both missing → no usable data.
fn parse_snapshot(rate: &Value, status: Option<&Value>) -> Result<Snapshot, String> {
    let mut m = Vec::new();
    let plan = status
        .and_then(|v| v.pointer("/subscription/name").and_then(Value::as_str))
        .map(str::to_owned)
        .or_else(|| Some("Step Plan".into()));

    let window_live = ts(rate.get("five_hour_usage_reset_time")).is_some_and(|ms| ms > 0)
        || ts(rate.get("weekly_usage_reset_time")).is_some_and(|ms| ms > 0);

    if window_live {
        if let Some(left) = num(rate.get("five_hour_usage_left_rate")) {
            m.push(
                Metric::progress("Session", (1.0 - left) * 100.0, None).with_reset(
                    ts(rate.get("five_hour_usage_reset_time")),
                    Some(5 * 60 * 60 * 1000),
                ),
            );
        }
        if let Some(left) = num(rate.get("weekly_usage_left_rate")) {
            m.push(
                Metric::progress("Weekly", (1.0 - left) * 100.0, None).with_reset(
                    ts(rate.get("weekly_usage_reset_time")),
                    Some(7 * 86_400_000),
                ),
            );
        }
    } else if let Some(c) = rate.get("plan_credit_rate_limit") {
        if let Some(left) = num(c.get("subscription_credit_left_rate")) {
            m.push(
                Metric::progress(
                    "Credit",
                    (1.0 - left) * 100.0,
                    Some(format!("{:.1}% remaining", left * 100.0)),
                )
                .with_reset(
                    ts(c.get("subscription_credit_reset_time")),
                    Some(30 * 86_400_000),
                ),
            );
        }
        if num(c.get("topup_credit_left_rate")).is_some_and(|r| r > 0.0) {
            let left = num(c.get("topup_credit_left_rate")).unwrap_or(0.0);
            m.push(
                Metric::progress(
                    "Top-up",
                    (1.0 - left) * 100.0,
                    Some(format!("{:.1}% remaining", left * 100.0)),
                )
                .with_reset(None, Some(30 * 86_400_000)),
            );
        }
    }

    if m.is_empty() {
        return Err("Step Plan returned no quota data".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_identity_rejects_null_zero_and_conflicting_halves() {
        let valid = jwt(r#"{"oasis_id":42}"#);
        for claim in [
            r#"{"oasis_id":null}"#,
            r#"{"oasis_id":0}"#,
            r#"{"oasis_id":true}"#,
            r#"{"oasis_id":[]}"#,
        ] {
            let invalid = jwt(claim);
            assert!(accept_rotation(&invalid, &invalid).is_err());
            assert!(accept_login(&invalid, &valid).is_err());
        }
        let conflicting = pair(&valid, &jwt(r#"{"oasis_id":43}"#));
        assert!(accept_rotation(&valid, &conflicting).is_err());
        let string_id = jwt(r#"{"oasis_id":"42"}"#);
        assert!(accept_rotation(&valid, &string_id).is_ok());
        assert!(accept_login(&valid, &string_id).is_err());
    }

    #[test]
    fn auth_throttling_and_server_errors_are_retryable_not_rejections() {
        for status in [429, 500, 502, 503] {
            assert!(matches!(
                auth_http_error(status, "refresh"),
                AuthErr::Transient(_)
            ));
        }
        for status in [400, 401, 403] {
            assert!(matches!(
                auth_http_error(status, "refresh"),
                AuthErr::Invalid(_)
            ));
        }
    }

    #[test]
    fn pasted_key_needs_a_successful_wallet_read_to_count_as_connected() {
        let token = "synthetic-api-key-not-a-jwt";
        assert_eq!(pasted_token_fallback(token, None).status, "error");
        let wallet = Snapshot::ok(
            "stepfun",
            "StepFun",
            None,
            vec![Metric::text("Balance", "¥1.00".into())],
        );
        let setup = pasted_token_fallback(token, Some(wallet));
        assert_eq!(setup.status, "needs_setup");
        assert_eq!(setup.metrics[0].value.as_deref(), Some("¥1.00"));
    }

    /// A syntactically valid JWT carrying exactly `claims`.
    fn jwt(claims: &str) -> String {
        use base64::Engine;
        let head = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256"}"#);
        let body = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.as_bytes());
        format!("{head}.{body}.c2ln")
    }

    /// "<access>...<refresh>" — the pair form the console accepts.
    fn pair(access: &str, refresh: &str) -> String {
        format!("{access}...{refresh}")
    }

    /// Synthetic account and anonymous-session claims, preserving the observed
    /// field shapes without retaining personal account or device identifiers.
    const REAL_ACCESS: &str = r#"{"activated":true,"age":171,"exp":1792078025,"mode":2,"oasis_id":123456789012345678,"real_name_status":1}"#;
    const REAL_REFRESH: &str = r#"{"app_id":10300,"device_id":"synthetic-signed-in-device","exp":1794099614,"oasis_id":123456789012345678}"#;
    const ANON_ACCESS: &str = r#"{"activated":false,"age":1,"exp":1792079183,"mode":1,"oasis_id":234567890123456789,"real_name_status":0}"#;
    const ANON_REFRESH: &str = r#"{"app_id":10300,"device_id":"c8a1002d2c457e758785a9979832217c7c0b884c","exp":1794669383,"oasis_id":234567890123456789}"#;

    /// Verbatim shape from a CodexBar fixture (2026-10) — string decimals,
    /// pool + one 30-day bucket. topup_credit_left_rate = 0 → no Top-up row.
    const CREDIT_SAMPLE: &str = r#"{"plan_credit_rate_limit":{
        "subscription_credit_left_rate":0.9641096,
        "subscription_credit_reset_time":"1786288293",
        "topup_credit_left_rate":0,
        "credit_buckets":[{"type":1,"credit_total":"400000000","credit_residual":"385643840","expire_at":"1789000000","next_reset_at":"0"}]
    }}"#;

    /// Legacy Coding Plan: live windows (reset timestamps in the future).
    const WINDOW_SAMPLE: &str = r#"{
        "plan_family":1,
        "five_hour_usage_left_rate":0.99781543,
        "five_hour_usage_reset_time":"1791012792",
        "weekly_usage_left_rate":0.85,
        "weekly_usage_reset_time":"1791181992"
    }"#;

    #[test]
    fn parses_credit_pool_shape() {
        let doc: Value = serde_json::from_str(CREDIT_SAMPLE).unwrap();
        let snap = parse_snapshot(&doc, None).expect("parse");
        assert_eq!(snap.status, "ok");
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["Credit"]);
        let c = &snap.metrics[0];
        // 0.9641096 left → ~3.59% used
        assert!((c.used_percent.unwrap() - 3.589).abs() < 0.01);
        assert_eq!(c.resets_at, Some(1786288293 * 1000));
        assert_eq!(c.period_ms, Some(30 * 86_400_000));
    }

    #[test]
    fn topup_row_appears_only_when_positive() {
        let doc = serde_json::json!({
            "plan_credit_rate_limit": {
                "subscription_credit_left_rate": 0.5,
                "subscription_credit_reset_time": "1786288293",
                "topup_credit_left_rate": 0.5
            }
        });
        let snap = parse_snapshot(&doc, None).expect("parse");
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["Credit", "Top-up"]);
    }

    #[test]
    fn live_window_beats_credit_pool_during_migration() {
        // Migration decks can carry both; a live reset_time wins.
        let doc = serde_json::json!({
            "plan_family": 2,
            "five_hour_usage_left_rate": 0.4,
            "five_hour_usage_reset_time": "1791012792",
            "weekly_usage_left_rate": 0.9,
            "weekly_usage_reset_time": "1791181992",
            "plan_credit_rate_limit": { "subscription_credit_left_rate": 0.9 }
        });
        let snap = parse_snapshot(&doc, None).expect("parse");
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["Session", "Weekly"]);
    }

    #[test]
    fn parses_window_shape() {
        let doc: Value = serde_json::from_str(WINDOW_SAMPLE).unwrap();
        let snap = parse_snapshot(&doc, None).expect("parse");
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["Session", "Weekly"]);
        // 0.99781543 left → 0.218% used
        assert!((snap.metrics[0].used_percent.unwrap() - 0.218).abs() < 0.01);
    }

    #[test]
    fn empty_payload_is_an_error() {
        assert!(parse_snapshot(&serde_json::json!({"plan_family": 2}), None).is_err());
    }

    #[test]
    fn webid_prefers_refresh_half_device_id_then_default() {
        use base64::Engine;
        let access =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"exp":9999999999}"#);
        let refresh = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"device_id":"abc123","exp":9999999999}"#);
        let pair = format!("h.{access}.s...h.{refresh}.s");
        assert_eq!(webid_for_token(&pair), "abc123");
        // No device claim anywhere → frontend default.
        assert_eq!(webid_for_token("h.payload.s"), DEFAULT_WEBID);
    }

    #[test]
    fn access_exp_prechecks_the_access_half() {
        use base64::Engine;
        let far_future =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"exp":9999999999}"#);
        let pair = format!("h.{far_future}.s...h.{far_future}.s");
        assert!(access_expires_in_ms(&pair).is_some_and(|ms| ms > 3_000_000_000_000));
        // A single bare JWT parses as its own access half.
        assert!(access_expires_in_ms(&format!("h.{far_future}.s")).is_some());
        assert_eq!(access_expires_in_ms("garbage"), None);
    }

    #[test]
    fn combine_token_builds_the_pair_form() {
        let doc = serde_json::json!({
            "accessToken": {"raw": "ACCESS"},
            "refreshToken": {"raw": "REFRESH"}
        });
        assert_eq!(
            combine_token(&doc, "login").as_deref(),
            Some("ACCESS...REFRESH")
        );
        let doc = serde_json::json!({"accessToken": {"raw": "ACCESS"}});
        assert_eq!(combine_token(&doc, "device").as_deref(), Some("ACCESS"));
        let doc = serde_json::json!({"refreshToken": {"raw": "X"}});
        assert_eq!(combine_token(&doc, "login"), None);
    }

    #[test]
    fn legacy_api_key_blob_reads_as_token() {
        let json = r#"{"apiKey":"HEADER.payload.sig"}"#;
        let creds: Creds = serde_json::from_str(json).unwrap();
        assert_eq!(creds.token, "HEADER.payload.sig");
        assert!(creds.username.is_empty() && creds.password.is_empty());
        // The new shape round-trips and omits empty secrets.
        let c = Creds {
            token: "t".into(),
            username: "u".into(),
            password: String::new(),
        };
        let s = serde_json::to_string(&c).unwrap();
        assert!(!s.contains("password"));
    }

    #[test]
    fn separate_id() {
        assert_eq!(ID, "stepfun-plan");
        assert_eq!(BACKOFFS_MS, [30_000, 300_000, 1_800_000]);
    }

    /// Real shapes: an Open-Platform API key, a lone access JWT, and the
    /// "<access>...<refresh>" pair the console accepts. Critically, the
    /// access half has NO `device_id` (only the refresh half does) — the
    /// pair must still classify as a session.
    #[test]
    fn token_shape_classifies_the_three_real_inputs() {
        // A synthetic opaque API key, never a real credential.
        assert_eq!(
            token_shape("test-open-platform-key-not-a-jwt"),
            TokenShape::NotAJwt
        );
        // The access half of a real Oasis pair: a JWT with no device_id.
        let access = jwt(r#"{"exp":1900000000,"user_id":123}"#);
        assert_eq!(token_shape(&access), TokenShape::SingleJwt);
        assert!(
            !access.contains("device_id"),
            "fixture must model the real access half"
        );
        let pair = format!(
            "{access}...{}",
            jwt(r#"{"device_id":"dev-1","exp":1900000000}"#)
        );
        assert_eq!(
            token_shape(&pair),
            TokenShape::SessionPair,
            "a real session pair must not be mistaken for an API key"
        );
        assert_eq!(token_shape("   "), TokenShape::Empty);
    }

    /// The regression that cost the owner his pinned card (2026-10-15): the
    /// endpoint answers a token it cannot use with HTTP 200 and a fresh
    /// ANONYMOUS session. Accepting that as a rotation overwrites the real
    /// credential — the guard must reject anything but the same account.
    #[test]
    fn a_rotation_must_prove_the_same_account() {
        let real = pair(&jwt(REAL_ACCESS), &jwt(REAL_REFRESH));
        let anon = pair(&jwt(ANON_ACCESS), &jwt(ANON_REFRESH));
        let rotated = pair(
            &jwt(r#"{"activated":true,"exp":1792080000,"oasis_id":123456789012345678}"#),
            &jwt(r#"{"device_id":"dev-1","exp":1795000000,"oasis_id":123456789012345678}"#),
        );
        // Same account, fresh expiries → a genuine rotation.
        assert!(accept_rotation(&real, &rotated).is_ok());
        // The anonymous session handed back for an unusable token.
        assert!(accept_rotation(&real, &anon).is_err());
        // A stored API key carries no identity → never a valid rotation. This
        // is exactly the state the card must survive, not overwrite.
        assert!(accept_rotation(
            "test-open-platform-key-not-a-jwt",
            &anon
        )
        .is_err());
        // A rotated pair with no identity cannot prove anything.
        let opaque = pair(&jwt(r#"{"exp":1792080000}"#), &jwt(r#"{"exp":1795000000}"#));
        assert!(accept_rotation(&real, &opaque).is_err());
        // Continuity alone cannot turn an anonymous session into a login.
        assert!(accept_rotation(&anon, &anon).is_err());
    }

    /// The same trap one step earlier: a sign-in response that merely carries
    /// the throwaway device session back must not count as a signed-in account.
    #[test]
    fn a_sign_in_must_leave_the_anonymous_device_behind() {
        let device = pair(&jwt(ANON_ACCESS), &jwt(ANON_REFRESH));
        let user = pair(&jwt(REAL_ACCESS), &jwt(REAL_REFRESH));
        assert!(accept_login(&device, &user).is_ok());
        // The response echoed the device session → the password was refused.
        assert!(accept_login(&device, &device).is_err());
        let other_anon = pair(
            &jwt(r#"{"activated":false,"oasis_id":345678901234567890}"#),
            &jwt(r#"{"oasis_id":345678901234567890}"#),
        );
        assert!(accept_login(&device, &other_anon).is_err());
        // No identity to compare against → cannot be trusted either way.
        let opaque = pair(&jwt(r#"{"exp":1}"#), &jwt(r#"{"exp":2}"#));
        assert!(accept_login(&device, &opaque).is_err());
        assert!(accept_login("not-a-jwt", &user).is_err());
    }

    /// The anon session is a well-formed pair, so the user must get the
    /// "session was rejected" guidance — not the "this is an API key" one.
    #[test]
    fn the_anonymous_session_reads_as_a_session_not_an_api_key() {
        let anon = pair(&jwt(ANON_ACCESS), &jwt(ANON_REFRESH));
        assert_eq!(token_shape(&anon), TokenShape::SessionPair);
        assert_ne!(
            oasis_id_of(&anon),
            oasis_id_of(&pair(&jwt(REAL_ACCESS), &jwt(REAL_REFRESH)))
        );
    }

    #[test]
    fn oasis_id_is_read_from_whichever_half_carries_it() {
        let expected = Some("123456789012345678".to_string());
        assert_eq!(
            oasis_id_of(&pair(&jwt(REAL_ACCESS), &jwt(r#"{"exp":1}"#))),
            expected
        );
        assert_eq!(
            oasis_id_of(&pair(&jwt(r#"{"exp":1}"#), &jwt(REAL_REFRESH))),
            expected
        );
        assert_eq!(oasis_id_of("not-a-jwt"), None);
    }

    /// Every shape must produce a hint that says what to do — a bare failure
    /// is exactly what the user asked us to stop showing.
    #[test]
    fn every_shape_has_actionable_guidance() {
        for shape in [
            TokenShape::SessionPair,
            TokenShape::SingleJwt,
            TokenShape::NotAJwt,
            TokenShape::Empty,
        ] {
            let hint = setup_hint(&shape);
            assert!(hint.len() > 40, "hint too thin for {shape:?}: {hint}");
        }
        // The API-key case must name the concrete path, not just "sign in".
        let api_key_hint = setup_hint(&TokenShape::NotAJwt);
        assert!(api_key_hint.contains("Oasis-Token"), "{api_key_hint}");
        assert!(api_key_hint.contains("password"), "{api_key_hint}");
    }

    /// A credential-present-but-unreadable snapshot must stay on the board
    /// with a stated status, not degrade to an error card.
    #[test]
    fn needs_setup_is_not_an_error_status() {
        let snap = Snapshot::needs_setup(
            ID,
            NAME,
            Some("Pay as you go".into()),
            vec![Metric::text("Balance", "¥-0.18".into())],
            "hint",
        );
        assert_eq!(snap.status, "needs_setup");
        assert_eq!(snap.metrics.len(), 1);
        assert!(snap.error.is_some());
    }
}
