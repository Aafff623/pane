//! Windsurf login — the editor's browser sign-in, ported from cockpit's
//! `windsurf_oauth.rs`: the auth page uses an implicit flow
//! (`response_type=token`), so the loopback callback carries the
//! Firebase ID token itself in the query string; that token is then
//! exchanged at register.windsurf.com for the long-lived API key every
//! quota call needs.
//!
//! The callback port is ephemeral (the page accepts any loopback
//! redirect), and a completed sign-in becomes one account in the
//! windsurf login store, keyed by the Firebase `sub`.

use base64::Engine;
use rand_core::{OsRng, RngCore};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const AUTH_BASE_URL: &str = "https://www.windsurf.com";
const REGISTER_API_BASE_URL: &str = "https://register.windsurf.com";
pub const DEFAULT_API_SERVER_URL: &str = "https://server.codeium.com";
const CLIENT_ID: &str = "3GUryQ7ldAeKEuD2obYnppsnmj58eP5u";
const APP_USER_AGENT: &str = "pane";
const CALLBACK_PATH: &str = "/windsurf-auth-callback";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);

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
    state: String,
    captured: Option<Result<String, String>>,
    deadline: Instant,
    done: bool,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn random_token() -> String {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
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

fn callback_html(ok: bool) -> String {
    let (bg, fg, text) = if ok {
        ("#18181b", "#4ade80", "Sign-in complete — you can close this tab and return to Pane.")
    } else {
        ("#18181b", "#f87171", "Sign-in failed — return to Pane and try again.")
    };
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Pane</title></head>\
         <body style=\"font-family:sans-serif;background:{bg};color:{fg};display:flex;\
         align-items:center;justify-content:center;height:100vh;margin:0\"><p>{text}</p></body></html>"
    )
}

/// The SeatManagement service path every Windsurf account call shares.
pub fn seat_url(base: &str, method: &str) -> String {
    format!(
        "{}/exa.seat_management_pb.SeatManagementService/{}",
        base.trim().trim_end_matches('/'),
        method
    )
}

pub async fn post_seat(base: &str, method: &str, body: Value) -> Result<Value, String> {
    let resp = crate::providers::http()
        .post(seat_url(base, method))
        .header("User-Agent", APP_USER_AGENT)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("seat {method}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("seat {method}: HTTP {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| format!("seat {method} read: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("seat {method} parse: {e}"))
}

/// The Firebase ID token's claims — `sub` is the account identity.
pub fn firebase_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn start() -> Result<LoginStart, String> {
    let probe = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| format!("bind callback port: {e}"))?;
    let port = probe
        .local_addr()
        .map_err(|e| format!("callback port lookup: {e}"))?
        .port();
    drop(probe);
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("bind callback port: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| format!("listener setup: {e}"))?;

    let login_id = random_token();
    let state = random_token();
    let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");
    let auth_url = format!(
        "{AUTH_BASE_URL}/windsurf/signin?response_type=token&client_id={CLIENT_ID}&redirect_uri={}&state={}&prompt=login&redirect_parameters_type=query&workflow=onboarding",
        pct_encode(&redirect_uri),
        pct_encode(&state),
    );
    {
        let mut guard = PENDING.lock().map_err(|_| "login state poisoned".to_string())?;
        *guard = Some(Pending {
            login_id: login_id.clone(),
            state,
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
                _ => return,
            }
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buf = [0u8; 8192];
                let read = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..read]);
                let line = req.lines().next().unwrap_or_default().to_string();
                let path = line.split_whitespace().nth(1).unwrap_or_default();
                let query = path.split_once('?').map(|(_, q)| q).unwrap_or_default();
                let mut params: Vec<(String, String)> = Vec::new();
                for pair in query.split('&') {
                    if let Some((k, v)) = pair.split_once('=') {
                        params.push((pct_decode(k), pct_decode(v)));
                    }
                }
                let get = |name: &str| {
                    params
                        .iter()
                        .find(|(k, _)| k == name)
                        .map(|(_, v)| v.trim().to_string())
                };
                let outcome = if !path.starts_with(CALLBACK_PATH) && !path.starts_with("/cancel") {
                    None
                } else if path.starts_with("/cancel") {
                    Some(Err("login cancelled".into()))
                } else {
                    Some(match get("error") {
                        Some(error) => Err(format!(
                            "authorization failed: {error}{}",
                            get("error_description").map(|d| format!(" ({d})")).unwrap_or_default()
                        )),
                        _ => match get("access_token") {
                            Some(token) if !token.is_empty() => Ok(token),
                            _ => Err("callback carried no access_token".into()),
                        },
                    })
                };
                let ok = outcome.as_ref().map(|r| r.is_ok()).unwrap_or(false);
                let html = callback_html(ok);
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 {} OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        if ok { "200" } else { "400" },
                        html.len(),
                        html
                    )
                    .as_bytes(),
                );
                if let Some(result) = outcome {
                    if let Ok(mut guard) = PENDING.lock() {
                        if let Some(p) = guard.as_mut().filter(|p| p.login_id == thread_login) {
                            // State is verified inside the lock: only the
                            // live session's own state may complete it.
                            p.captured = Some(match p.state == get("state").unwrap_or_default() {
                                true => result,
                                false => Err("state mismatch — start the sign-in again".into()),
                            });
                        }
                    }
                    return;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(_) => return,
        }
    });

    Ok(LoginStart { login_id, auth_url })
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

/// Exchange the Firebase token for the API key + server, then store the
/// account.
async fn complete(firebase_id_token: &str) -> Result<String, String> {
    let register = post_seat(
        REGISTER_API_BASE_URL,
        "RegisterUser",
        json!({ "firebase_id_token": firebase_id_token }),
    )
    .await?;
    let api_key = register
        .get("apiKey")
        .or_else(|| register.get("api_key"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("RegisterUser returned no API key")?
        .to_string();
    let api_server_url = register
        .get("apiServerUrl")
        .or_else(|| register.get("api_server_url"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_API_SERVER_URL)
        .to_string();

    let claims = firebase_claims(firebase_id_token);
    let account_id = claims
        .as_ref()
        .and_then(|c| c.get("sub"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("Firebase token carries no subject")?
        .to_string();
    let email = claims
        .as_ref()
        .and_then(|c| c.get("email"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
        .to_string();
    let display_name = register
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
        .to_string();
    let label = if !email.is_empty() {
        email.clone()
    } else if !display_name.is_empty() {
        display_name.clone()
    } else {
        format!("Windsurf {}", &account_id[..account_id.len().min(12)])
    };

    crate::login_accounts::record_login(
        "windsurf",
        crate::login_accounts::LoginAccount {
            account_id,
            email,
            label: label.clone(),
            access_token: firebase_id_token.to_string(),
            refresh_token: String::new(),
            id_token: String::new(),
            expires_at: String::new(),
            added_at: chrono::Utc::now().timestamp(),
            extra: json!({ "apiKey": api_key, "apiServerUrl": api_server_url }),
        },
    )?;
    Ok(label)
}

pub async fn poll(login_id: &str) -> LoginPoll {
    enum Take {
        Waiting,
        Done,
        Failed(String),
        Ready(String),
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
                        Some(Ok(token)) => Take::Ready(token),
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
        Take::Ready(token) => {
            let mark_done = |done_ok: bool| {
                if let Ok(mut guard) = PENDING.lock() {
                    if let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id) {
                        p.done = done_ok;
                    }
                }
            };
            match complete(&token).await {
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
    fn seat_url_is_the_seat_management_path() {
        assert_eq!(
            seat_url("https://server.codeium.com/", "GetUserStatus"),
            "https://server.codeium.com/exa.seat_management_pb.SeatManagementService/GetUserStatus"
        );
    }

    #[test]
    fn firebase_claims_decode_the_middle_chunk() {
        use base64::Engine;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&serde_json::json!({ "sub": "u-1", "email": "a@b.c" })).unwrap());
        let claims = firebase_claims(&format!("header.{payload}.sig")).expect("claims");
        assert_eq!(claims.get("sub").and_then(Value::as_str), Some("u-1"));
        assert!(firebase_claims("not-a-jwt").is_none());
    }
}
