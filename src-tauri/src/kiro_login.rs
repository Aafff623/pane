//! Browser PKCE login for Kiro — the flow app.kiro.dev's own sign-in page
//! drives, ported from cockpit's `kiro_oauth.rs` (trimmed to Pane's
//! single-pending-login model, like codex_login.rs).
//!
//! `start` binds one of the fixed callback ports, builds the portal URL
//! with an S256 challenge, and spawns a listener thread; the browser lands
//! back on `/oauth/callback` (or `/signin/callback`) with `code` +
//! `state`, and `poll` exchanges the code at Kiro's desktop token
//! endpoint. Social logins (Google/GitHub) return a usable code —
//! BuilderID/enterprise (IDC) logins need the client's follow-up
//! handshake, so a callback without a code is answered with a clear error
//! instead of a broken account.
//!
//! A completed sign-in becomes one account in the kiro login store
//! (login_accounts), keyed by the profileArn the auth payload carries —
//! the same ARN the usage endpoint scopes to.

use base64::Engine;
use rand_core::{OsRng, RngCore};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const AUTH_PORTAL_URL: &str = "https://app.kiro.dev/signin";
const TOKEN_ENDPOINT: &str = "https://prod.us-east-1.auth.desktop.kiro.dev/oauth/token";
const REFRESH_ENDPOINT: &str = "https://prod.us-east-1.auth.desktop.kiro.dev/refreshToken";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
/// The Kiro IDE registers its callback on one of these; the same ladder
/// keeps Pane's port compatible with the portal's expectations.
const CALLBACK_PORT_CANDIDATES: [u16; 10] = [
    3128, 4649, 6588, 8008, 9091, 49153, 50153, 51153, 52153, 53153,
];

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

#[derive(Clone)]
struct Callback {
    login_option: String,
    code: Option<String>,
}

struct Pending {
    login_id: String,
    state: String,
    verifier: String,
    /// The loopback port this login's redirect_uri binds — the token
    /// exchange must repeat it exactly.
    callback_port: u16,
    captured: Option<Result<Callback, String>>,
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

fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The auth service wraps its payloads in `{ "data": { … } }`; unwrap when
/// present (cockpit's `unwrap_token_response`).
pub(crate) fn unwrap_data(mut doc: Value) -> Value {
    if let Some(data) = doc
        .as_object_mut()
        .and_then(|obj| obj.remove("data"))
        .filter(|v| v.is_object())
    {
        return data;
    }
    doc
}

/// Auth payloads carry `expiresIn`; the store wants an absolute
/// `expiresAt` (RFC3339) so the account refresh can decide.
pub(crate) fn ensure_expires_at(doc: &mut Value) {
    let Some(obj) = doc.as_object_mut() else { return };
    if obj.contains_key("expiresAt") {
        return;
    }
    let Some(expires_in) = obj
        .get("expiresIn")
        .or_else(|| obj.get("expires_in"))
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())))
        .filter(|s| *s > 0)
    else {
        return;
    };
    obj.insert(
        "expiresAt".into(),
        Value::String((chrono::Utc::now() + chrono::Duration::seconds(expires_in)).to_rfc3339()),
    );
}

fn callback_html() -> &'static str {
    "<!doctype html><html><head><meta charset=\"utf-8\"><title>Pane</title></head>\
     <body style=\"font-family:sans-serif;background:#18181b;color:#f4f4f5;display:flex;\
     align-items:center;justify-content:center;height:100vh;margin:0\">\
     <p>Sign-in complete — you can close this tab and return to Pane.</p></body></html>"
}

pub fn start() -> Result<LoginStart, String> {
    let port = CALLBACK_PORT_CANDIDATES
        .into_iter()
        .find(|p| TcpListener::bind(("127.0.0.1", *p)).is_ok())
        .ok_or("no free Kiro callback port — close apps holding ports 3128–53153 and retry")?;
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("bind callback port: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| format!("listener setup: {e}"))?;

    let login_id = random_token();
    let state = random_token();
    let verifier = random_token();
    let callback_url = format!("http://localhost:{port}");
    let auth_url = format!(
        "{AUTH_PORTAL_URL}?state={}&code_challenge={}&code_challenge_method=S256&redirect_uri={}&redirect_from=KiroIDE",
        pct_encode(&state),
        pct_encode(&code_challenge(&verifier)),
        pct_encode(&callback_url),
    );
    {
        let mut guard = PENDING.lock().map_err(|_| "login state poisoned".to_string())?;
        *guard = Some(Pending {
            login_id: login_id.clone(),
            state,
            verifier,
            callback_port: port,
            captured: None,
            deadline: Instant::now() + LOGIN_TIMEOUT,
            done: false,
        });
    }

    let thread_login = login_id.clone();
    std::thread::spawn(move || loop {
        {
            let guard = PENDING.lock().unwrap();
            match guard.as_ref() {
                Some(p) if p.login_id == thread_login && !p.done && Instant::now() <= p.deadline => {}
                _ => return, // cancelled, superseded, done or expired
            }
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buf = [0u8; 8192];
                let read = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..read]);
                let line = req.lines().next().unwrap_or_default().to_string();
                let html = callback_html();
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html.len(),
                        html
                    )
                    .as_bytes(),
                );
                let path = line.split_whitespace().nth(1).unwrap_or_default();
                if path == "/cancel" {
                    if let Ok(mut guard) = PENDING.lock() {
                        if let Some(p) = guard.as_mut().filter(|p| p.login_id == thread_login) {
                            p.captured = Some(Err("login cancelled".into()));
                        }
                    }
                    return;
                }
                let query = path.split_once('?').map(|(_, q)| q).unwrap_or_default();
                let mut params: Vec<(String, String)> = Vec::new();
                for pair in query.split('&') {
                    let Some((key, value)) = pair.split_once('=') else { continue };
                    params.push((pct_decode(key), pct_decode(value)));
                }
                let get = |name: &str| {
                    params
                        .iter()
                        .find(|(k, _)| k == name)
                        .map(|(_, v)| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                };
                if let Ok(mut guard) = PENDING.lock() {
                    if let Some(p) = guard.as_mut().filter(|p| p.login_id == thread_login) {
                        p.captured = Some(match get("error") {
                            Some(error) => Err(format!(
                                "authorization failed: {error}{}",
                                get("error_description").map(|d| format!(" ({d})")).unwrap_or_default()
                            )),
                            None if get("state").as_deref() != Some(p.state.as_str()) => {
                                Err("state mismatch — start the sign-in again".into())
                            }
                            None => Ok(Callback {
                                login_option: get("login_option")
                                    .or_else(|| get("loginOption"))
                                    .unwrap_or_default()
                                    .to_ascii_lowercase(),
                                code: get("code"),
                            }),
                        });
                    }
                }
                return;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(_) => return,
        }
    });

    Ok(LoginStart { login_id, auth_url })
}

/// Exchange the code, build the account, store it. Identity: the profile
/// ARN (unique per AWS account profile); label: the email the payload
/// carries.
async fn complete(callback: Callback, verifier: &str, callback_port: u16) -> Result<String, String> {
    let code = callback
        .code
        .filter(|c| !c.trim().is_empty())
        .ok_or_else(|| match callback.login_option.as_str() {
            "builderid" | "awsidc" | "internal" | "external_idp" => {
                "this Kiro login mode needs the Kiro client's follow-up handshake — sign in with Google or GitHub instead".to_string()
            }
            _ => "callback carried no authorization code".to_string(),
        })?;
    let redirect_uri = format!(
        "http://localhost:{callback_port}/oauth/callback?login_option={}",
        pct_encode(&callback.login_option)
    );
    let resp = crate::providers::http()
        .post(TOKEN_ENDPOINT)
        .header("Content-Type", "application/json")
        .json(&json!({
            "code": code,
            "code_verifier": verifier,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .map_err(|e| format!("token exchange: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token exchange: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("token exchange parse: {e}"))?;
    let mut token = unwrap_data(doc);
    ensure_expires_at(&mut token);

    let pick = |keys: &[&str]| -> Option<String> {
        keys.iter()
            .find_map(|k| token.get(*k).and_then(Value::as_str))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let access_token = pick(&["accessToken", "access_token", "accessTokenJwt"])
        .ok_or("token response missing accessToken")?;
    let refresh_token = pick(&["refreshToken", "refresh_token", "refreshTokenJwt"]).unwrap_or_default();
    let expires_at = pick(&["expiresAt", "expires_at"]).unwrap_or_default();
    let profile_arn = pick(&["profileArn", "profile_arn", "arn"])
        .ok_or("token response missing profileArn — cannot identify the account")?;
    let email = pick(&["email", "userEmail"]).unwrap_or_default();
    let label = (!email.is_empty()).then(|| email.clone()).unwrap_or_else(|| {
        // Fall back to the ARN's profile id tail — recognizable, never secret.
        profile_arn
            .rsplit('/')
            .next()
            .unwrap_or("kiro")
            .chars()
            .take(12)
            .collect::<String>()
    });

    let account = crate::login_accounts::LoginAccount {
        account_id: profile_arn.clone(),
        email,
        label: label.clone(),
        access_token,
        refresh_token,
        id_token: String::new(),
        expires_at,
        added_at: chrono::Utc::now().timestamp(),
        extra: token,
    };
    crate::login_accounts::record_login("kiro", account)?;
    Ok(label)
}

/// One refresh round-trip for a stored kiro login (cockpit's
/// refresh_token_via_remote): returns the refreshed auth payload so the
/// caller can update the store.
pub async fn refresh(refresh_token: &str) -> Result<Value, String> {
    let resp = crate::providers::http()
        .post(REFRESH_ENDPOINT)
        .header("Content-Type", "application/json")
        .json(&json!({ "refreshToken": refresh_token }))
        .send()
        .await
        .map_err(|e| format!("refresh request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("refresh: HTTP {}", resp.status()));
    }
    let mut doc: Value = resp.json().await.map_err(|e| format!("refresh parse: {e}"))?;
    doc = unwrap_data(doc);
    ensure_expires_at(&mut doc);
    Ok(doc)
}

pub async fn poll(login_id: &str) -> LoginPoll {
    enum Take {
        Waiting,
        Done,
        Failed(String),
        Ready(Callback, String, u16),
    }
    let take = {
        let Ok(mut guard) = PENDING.lock() else {
            return LoginPoll { done: false, label: None, error: Some("login state poisoned".into()) };
        };
        match guard.as_mut() {
            Some(p) if p.login_id == login_id => {
                if p.done {
                    Take::Done
                } else if Instant::now() > p.deadline {
                    p.done = true;
                    Take::Failed("sign-in timed out — start again".into())
                } else {
                    match p.captured.take() {
                        Some(Ok(cb)) => Take::Ready(cb, p.verifier.clone(), p.callback_port),
                        Some(Err(e)) => {
                            p.done = true;
                            Take::Failed(e)
                        }
                        None => Take::Waiting,
                    }
                }
            }
            _ => Take::Failed("no pending sign-in".into()),
        }
    };
    match take {
        Take::Waiting => LoginPoll { done: false, label: None, error: None },
        Take::Done => LoginPoll { done: true, label: None, error: None },
        Take::Failed(error) => LoginPoll { done: false, label: None, error: Some(error) },
        Take::Ready(callback, verifier, callback_port) => {
            let mark_done = |done_ok: bool| {
                if let Ok(mut guard) = PENDING.lock() {
                    if let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id) {
                        p.done = done_ok;
                    }
                }
            };
            match complete(callback, &verifier, callback_port).await {
                Ok(label) => {
                    mark_done(true);
                    LoginPoll { done: true, label: Some(label), error: None }
                }
                Err(e) => {
                    mark_done(true);
                    LoginPoll { done: false, label: None, error: Some(e) }
                }
            }
        }
    }
}

pub fn cancel(login_id: &str) {
    if let Ok(mut guard) = PENDING.lock() {
        if guard.as_ref().is_some_and(|p| p.login_id == login_id) {
            *guard = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s256_challenge_is_the_rfc_shape() {
        // RFC 7636 appendix B — the same vector codex_login asserts.
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn auth_url_carries_the_portal_parameters() {
        let url = format!(
            "{AUTH_PORTAL_URL}?state=st&code_challenge=ch&code_challenge_method=S256&redirect_uri={}&redirect_from=KiroIDE",
            pct_encode("http://localhost:3128")
        );
        assert!(url.starts_with("https://app.kiro.dev/signin?"));
        assert!(url.contains("redirect_from=KiroIDE"));
        assert!(url.contains(&pct_encode("http://localhost:3128")));
    }

    #[test]
    fn data_envelope_is_unwrapped_and_expiry_materialized() {
        let mut doc = unwrap_data(json!({ "data": { "accessToken": "at", "expiresIn": 3600 } }));
        assert_eq!(doc.get("accessToken").and_then(Value::as_str), Some("at"));
        ensure_expires_at(&mut doc);
        let expires_at = doc.get("expiresAt").and_then(Value::as_str).unwrap_or_default();
        assert!(chrono::DateTime::parse_from_rfc3339(expires_at).is_ok());
        // A flat payload passes through untouched.
        assert_eq!(unwrap_data(json!({ "accessToken": "flat" })).get("accessToken"), Some(&json!("flat")));
    }
}
