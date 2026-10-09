//! Browser PKCE login for Codex — the loopback flow the Codex CLI itself
//! uses (127.0.0.1:1455 + S256), ported from cockpit's `codex_oauth.rs`.
//!
//! Preferred over the device-code fallback; a completed sign-in lands in the
//! codex account store via the same `record_login` path, so every login
//! becomes its own card. The port belongs to the Codex CLI while it runs —
//! `start` then answers [`PORT_IN_USE`] and the caller falls back to the
//! device-code flow, which is exactly cockpit's fallback ladder too.
//!
//! ## Field notes (sign-in failures seen in the wild)
//!
//! - **Browser lands on "Just a moment…" / 403**: Cloudflare's managed
//!   challenge on the consent page. Ad-blocking extensions (AdGuard and
//!   friends inject scripts into the page) break the challenge script; an
//!   incognito window or disabling the blocker for `auth.openai.com` passes.
//!   A flagged proxy exit IP repeats the wall — switching nodes helps.
//! - **`token exchange: error sending request`**: the shell's HTTP proxy
//!   (`HTTP_PROXY` / `https_proxy` / `all_proxy`) reset TLS for
//!   `auth.openai.com`. [`crate::providers::send_with_direct_fallback`]
//!   retries the exchange over a direct connection.
//! - **`token exchange: HTTP 403`**: two distinct causes, told apart by the
//!   response body — `unsupported_country_region_territory` means the exit IP
//!   is in a region OpenAI rejects (a direct connection from the mainland
//!   can NEVER pass; the sign-in must ride a working proxy), otherwise the
//!   edge blocked it and the browser-side guidance below applies.
//! - Local proxy TLS through Clash-style ports can flake intermittently —
//!   the transport ladder retries the proxy path once before going direct.
//! - The authorize URL carries the official client's `codex_cli_simplified_flow`,
//!   `id_token_add_organizations` and `originator` parameters, so the browser
//!   takes the simple consent path instead of the dashboard one.
//! - Auth codes are single-use and a pending login expires after 5 minutes —
//!   every failure means starting a fresh browser sign-in.

use base64::Engine;
use rand_core::{OsRng, RngCore};
use serde::Serialize;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const AUTH_ENDPOINT: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_ENDPOINT: &str = "https://auth.openai.com/oauth/token";
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const CALLBACK_PORT: u16 = 1455;
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const SCOPES: &str = "openid profile email offline_access";
/// The official desktop client's originator; the authorize URL carries it so
/// the browser takes the simplified consent path.
const ORIGINATOR: &str = "Codex Desktop";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);
/// Sentinel the frontend matches on to fall back to the device-code flow.
pub const PORT_IN_USE: &str = "CODEX_OAUTH_PORT_IN_USE";

struct Pending {
    login_id: String,
    state: String,
    verifier: String,
    /// The callback's outcome, waiting for the next poll to pick it up.
    captured: Option<Result<String, String>>,
    deadline: Instant,
    done: bool,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStart {
    pub login_id: String,
    pub auth_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginPoll {
    pub done: bool,
    pub label: Option<String>,
    pub error: Option<String>,
}

fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn random_verifier() -> String {
    let mut bytes = [0u8; 48];
    OsRng.fill_bytes(&mut bytes);
    b64url(&bytes)
}

fn random_state() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    b64url(&bytes)
}

/// RFC 7636 S256: base64url(SHA-256(verifier)).
pub fn code_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
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
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(v) = u8::from_str_radix(hex, 16) {
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

pub fn build_auth_url(state: &str, challenge: &str) -> String {
    format!(
        "{AUTH_ENDPOINT}?response_type=code&client_id={CLIENT_ID}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256&id_token_add_organizations=true&codex_cli_simplified_flow=true&originator={}",
        pct_encode(REDIRECT_URI),
        pct_encode(SCOPES),
        state,
        challenge,
        pct_encode(ORIGINATOR),
    )
}

/// First request line of the loopback callback → (code, state).
fn parse_callback_request(request_line: &str) -> Option<(String, String)> {
    let path = request_line.split_whitespace().nth(1)?;
    let query = path.split_once('?')?.1;
    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else { continue };
        match key {
            "code" => code = Some(pct_decode(value)),
            "state" => state = Some(pct_decode(value)),
            _ => {}
        }
    }
    Some((code?, state?))
}

fn callback_html() -> &'static str {
    "<!doctype html><html><head><meta charset=\"utf-8\"><title>Pane</title></head>\
     <body style=\"font-family:sans-serif;background:#18181b;color:#f4f4f5;display:flex;\
     align-items:center;justify-content:center;height:100vh;margin:0\">\
     <p>Sign-in complete — you can close this tab and return to Pane.</p></body></html>"
}

/// Bind the loopback port and wait for one callback. Errors with
/// [`PORT_IN_USE`] when the Codex CLI already owns the port.
pub fn start() -> Result<LoginStart, String> {
    let listener =
        TcpListener::bind(("127.0.0.1", CALLBACK_PORT)).map_err(|_| PORT_IN_USE.to_string())?;
    listener.set_nonblocking(true).map_err(|e| format!("listener setup: {e}"))?;

    let login_id = random_state();
    let state = random_state();
    let verifier = random_verifier();
    let auth_url = build_auth_url(&state, &code_challenge(&verifier));
    {
        let mut guard = PENDING.lock().map_err(|_| "login state poisoned".to_string())?;
        *guard = Some(Pending {
            login_id: login_id.clone(),
            state,
            verifier,
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
                let mut buf = [0u8; 4096];
                let read = stream.read(&mut buf).unwrap_or(0);
                let line = String::from_utf8_lossy(&buf[..read])
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                let html = callback_html();
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html.len(),
                        html
                    )
                    .as_bytes(),
                );
                if let Some((code, got_state)) = parse_callback_request(&line) {
                    if let Ok(mut guard) = PENDING.lock() {
                        if let Some(p) = guard.as_mut() {
                            if p.login_id == thread_login {
                                p.captured = Some(if got_state == p.state {
                                    Ok(code)
                                } else {
                                    Err("sign-in state mismatch — start again".into())
                                });
                            }
                        }
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

/// The token exchange for one captured code.
async fn exchange_code(code: &str, verifier: &str) -> Result<crate::oauth::StoredTokens, String> {
    // Direct-retry on transport failure: a machine's environment proxy can
    // reset this endpoint's TLS while direct access works (Settings → Network
    // owns the proxy Pane actually intends to use).
    let resp = crate::providers::send_with_direct_fallback(|client| {
        client
            .post(TOKEN_ENDPOINT)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", REDIRECT_URI),
                ("client_id", CLIENT_ID),
                ("code_verifier", verifier),
            ])
    })
    .await
    .map_err(|e| format!("token exchange: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.as_u16() == 403 {
            if body.contains("unsupported_country_region_territory") {
                // A direct connection from an unsupported region — retrying
                // will never fix it; the sign-in must ride a working proxy.
                return Err(
                    "token exchange: HTTP 403 — OpenAI rejects this network exit's \
                     country/region (unsupported_country_region_territory). Sign-in has to go \
                     through a working proxy: set one in Settings → Network (or fix the system \
                     proxy), then retry."
                        .into(),
                );
            }
            return Err(
                "token exchange: HTTP 403 — the edge blocked this sign-in. Common causes: \
                 an ad-blocking extension interfering with the browser challenge, or a flagged \
                 network exit IP. Disable the blocker (or use an incognito window) / switch \
                 network node, then retry."
                    .into(),
            );
        }
        return Err(format!("token exchange: HTTP {status}"));
    }
    let tok: Value = resp.json().await.map_err(|e| format!("token exchange parse: {e}"))?;
    let access_token = tok
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or("token exchange returned no access_token")?
        .to_string();
    let refresh_token = tok.get("refresh_token").and_then(Value::as_str).unwrap_or_default().to_string();
    let id_token = tok.get("id_token").and_then(Value::as_str).unwrap_or_default().to_string();
    let expires_in = tok.get("expires_in").and_then(Value::as_i64).unwrap_or(3600);
    let expires_at = (chrono::Utc::now() + chrono::Duration::seconds(expires_in)).to_rfc3339();
    let claims = crate::providers::codex::jwt_claims(&id_token);
    let account_id = claims
        .as_ref()
        .and_then(|c| c.pointer("/https:~1~1api.openai.com~1auth/chatgpt_account_id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let label = claims
        .as_ref()
        .and_then(|c| c.get("email"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(crate::oauth::StoredTokens {
        access_token,
        refresh_token,
        expires_at,
        label,
        account_id,
        id_token: (!id_token.is_empty()).then_some(id_token),
    })
}

/// One poll tick: nothing yet, the account's label when done, or the error.
pub async fn poll(login_id: &str) -> LoginPoll {
    enum Take {
        Waiting,
        Done,
        Failed(String),
        Ready(String, String),
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
                        Some(Ok(code)) => Take::Ready(code, p.verifier.clone()),
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
        Take::Ready(code, verifier) => match exchange_code(&code, &verifier).await {
            Ok(tokens) => {
                let label = tokens.label.clone();
                if let Err(e) = crate::codex_accounts::record_login(&tokens) {
                    if let Ok(mut guard) = PENDING.lock() {
                        if let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id) {
                            p.done = true;
                        }
                    }
                    return LoginPoll { done: false, label: None, error: Some(e) };
                }
                if let Ok(mut guard) = PENDING.lock() {
                    if let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id) {
                        p.done = true;
                    }
                }
                LoginPoll { done: true, label, error: None }
            }
            Err(e) => {
                if let Ok(mut guard) = PENDING.lock() {
                    if let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id) {
                        p.done = true;
                    }
                }
                LoginPoll { done: false, label: None, error: Some(e) }
            }
        },
    }
}

/// Drops the pending login (the listener thread notices and exits).
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
    fn s256_challenge_matches_the_rfc_vector() {
        // RFC 7636 appendix B.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(code_challenge(verifier), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn callback_parsing_takes_code_and_state() {
        let line = "GET /auth/callback?code=ac%2F1&state=st-42 HTTP/1.1";
        let (code, state) = parse_callback_request(line).unwrap();
        assert_eq!(code, "ac/1");
        assert_eq!(state, "st-42");
        assert!(parse_callback_request("GET /auth/callback HTTP/1.1").is_none());
        assert!(parse_callback_request("GET /auth/callback?code=only HTTP/1.1").is_none());
    }

    #[test]
    fn auth_url_carries_pkce_parameters() {
        let url = build_auth_url("st", "ch");
        assert!(url.starts_with(AUTH_ENDPOINT));
        assert!(url.contains("code_challenge=ch&code_challenge_method=S256"));
        assert!(url.contains(&format!("redirect_uri={}", pct_encode(REDIRECT_URI))));
        assert!(url.contains("scope=openid%20profile%20email%20offline_access"));
        // The official client's simplified-flow parameters (cockpit parity).
        assert!(url.contains("id_token_add_organizations=true"));
        assert!(url.contains("codex_cli_simplified_flow=true"));
        assert!(url.contains("originator=Codex%20Desktop"));
    }
}
