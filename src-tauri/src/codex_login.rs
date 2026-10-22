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
    exchanging: bool,
    result: Option<Result<Option<String>, String>>,
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

pub fn build_auth_url(state: &str, challenge: &str) -> String {
    format!(
        "{AUTH_ENDPOINT}?response_type=code&client_id={CLIENT_ID}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256&id_token_add_organizations=true&codex_cli_simplified_flow=true&originator={}",
        crate::oauth_util::pct_encode(REDIRECT_URI),
        crate::oauth_util::pct_encode(SCOPES),
        state,
        challenge,
        crate::oauth_util::pct_encode(ORIGINATOR),
    )
}

/// Bind the loopback port and wait for one callback. Errors with
/// [`PORT_IN_USE`] when the Codex CLI already owns the port.
pub fn start() -> Result<LoginStart, String> {
    let listener =
        TcpListener::bind(("127.0.0.1", CALLBACK_PORT)).map_err(|_| PORT_IN_USE.to_string())?;
    listener.set_nonblocking(true).map_err(|e| format!("listener setup: {e}"))?;

    let login_id = crate::oauth_util::random_b64url(16);
    let state = crate::oauth_util::random_b64url(16);
    let verifier = crate::oauth_util::random_b64url(48);
    let auth_url = build_auth_url(&state, &crate::oauth_util::code_challenge(&verifier));
    {
        let mut guard = PENDING.lock().map_err(|_| "login state poisoned".to_string())?;
        *guard = Some(Pending {
            login_id: login_id.clone(),
            state,
            verifier,
            captured: None,
            deadline: Instant::now() + LOGIN_TIMEOUT,
            done: false,
            exchanging: false,
            result: None,
        });
    }

    let thread_login = login_id.clone();
    std::thread::spawn(move || serve_callback(listener, thread_login, &PENDING));

    Ok(LoginStart { login_id, auth_url })
}

fn serve_callback(listener: TcpListener, login_id: String, pending: &Mutex<Option<Pending>>) {
    use std::io::BufRead;
    loop {
        {
            let guard = pending.lock().unwrap_or_else(|e| e.into_inner());
            if !guard.as_ref().is_some_and(|p| p.login_id == login_id && !p.done && Instant::now() <= p.deadline) { return; }
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                // Browsers may preconnect without sending a request. Bound both
                // reads and writes so an idle connection cannot consume the login.
                let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
                let mut line = String::new();
                let read = std::io::BufReader::new((&mut stream).take(8192)).read_line(&mut line);
                if read.is_err() || line.len() >= 8192 || !line.ends_with('\n') { continue; }
                let mut request = line.split_whitespace();
                let method = request.next().unwrap_or_default();
                let target = request.next().unwrap_or_default();
                let url = reqwest::Url::parse(&format!("http://localhost{target}"));
                let mut status = "404 Not Found";
                let mut text = "Not found.";
                let mut outcome = None;
                if method == "GET" && target.starts_with('/') {
                    if let Some(url) = url.ok().filter(|url| url.path() == "/auth/callback") {
                        let params: Vec<_> = url.query_pairs().collect();
                        let one = |key: &str| {
                            let mut values = params.iter().filter(|(k, _)| k == key).map(|(_, v)| v.as_ref());
                            let value = values.next().filter(|v| !v.is_empty())?;
                            values.next().is_none().then_some(value)
                        };
                        let expected = pending.lock().unwrap_or_else(|e| e.into_inner()).as_ref()
                            .filter(|p| p.login_id == login_id && !p.done).map(|p| p.state.clone());
                        status = "400 Bad Request";
                        text = "Invalid callback. Return to Pane and finish the current sign-in.";
                        if one("state").is_some_and(|state| expected.as_deref() == Some(state)) {
                            if one("error").is_some() {
                                outcome = Some(Err("sign-in was canceled or denied — start again".into()));
                                text = "Sign-in was canceled or denied. Return to Pane to try again.";
                            } else if let Some(code) = one("code") {
                                status = "200 OK";
                                text = "Authorization response received. Return to Pane to confirm sign-in finished.";
                                outcome = Some(Ok(code.to_string()));
                            }
                        }
                    }
                }
                if let Some(result) = outcome {
                    let mut guard = pending.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id && !p.done && Instant::now() <= p.deadline) {
                        p.captured = Some(result);
                    } else { return; }
                }
                let terminal = status == "200 OK" || text.starts_with("Sign-in was canceled");
                let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
                if terminal { return; }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(25)),
            Err(_) => return,
        }
    }
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
fn outcome_poll(result: Result<Option<String>, String>) -> LoginPoll {
    match result {
        Ok(label) => LoginPoll { done: true, label, error: None },
        Err(error) => LoginPoll { done: false, label: None, error: Some(error) },
    }
}

/// Commit only the still-current flow. Cancellation and completion are
/// serialized by the pending-state mutex; persistence never crosses an await.
fn finish_login(pending: &Mutex<Option<Pending>>, login_id: &str,
    result: Result<crate::oauth::StoredTokens, String>,
    save: impl FnOnce(&crate::oauth::StoredTokens) -> Result<(), String>) -> LoginPoll {
    let mut guard = pending.lock().unwrap_or_else(|e| e.into_inner());
    let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id && !p.done && Instant::now() <= p.deadline) else {
        return outcome_poll(Err("sign-in canceled or superseded — start again".into()));
    };
    let result = result.and_then(|tokens| { save(&tokens)?; Ok(tokens.label) });
    p.done = true;
    p.exchanging = false;
    p.result = Some(result.clone());
    outcome_poll(result)
}

pub async fn poll(login_id: &str) -> LoginPoll {
    let ready = {
        let mut guard = PENDING.lock().unwrap_or_else(|e| e.into_inner());
        let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id) else {
            return outcome_poll(Err("no pending sign-in".into()));
        };
        if let Some(result) = &p.result { return outcome_poll(result.clone()); }
        if Instant::now() > p.deadline {
            p.done = true;
            let result = Err("sign-in timed out — start again".into());
            p.result = Some(result.clone());
            return outcome_poll(result);
        }
        if p.exchanging { return LoginPoll { done: false, label: None, error: None }; }
        match p.captured.take() {
            Some(Ok(code)) => { p.exchanging = true; Some((code, p.verifier.clone())) }
            Some(Err(error)) => {
                p.done = true; p.result = Some(Err(error.clone()));
                return outcome_poll(Err(error));
            }
            None => None,
        }
    };
    let Some((code, verifier)) = ready else { return LoginPoll { done: false, label: None, error: None }; };
    let result = exchange_code(&code, &verifier).await;
    finish_login(&PENDING, login_id, result, crate::codex_accounts::record_login)
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
    fn loopback_ignores_unrelated_requests_before_the_real_callback() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let pending = std::sync::Arc::new(Mutex::new(Some(Pending {
            login_id: "synthetic-login".into(), state: "expected-state".into(),
            verifier: "synthetic-verifier".into(), captured: None,
            deadline: Instant::now() + Duration::from_secs(3), done: false, exchanging: false, result: None,
        })));
        let state = pending.clone();
        let worker = std::thread::spawn(move || serve_callback(listener, "synthetic-login".into(), &state));
        let request = |path: &str| {
            let mut stream = std::net::TcpStream::connect(addr).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
            write!(stream, "GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
            let mut response = String::new(); stream.read_to_string(&mut response).unwrap();
            response
        };
        let unrelated = request("/favicon.ico");
        assert!(unrelated.starts_with("HTTP/1.1 404"), "unrelated request must not complete sign-in");
        assert!(request("/auth/callback?code=bad&state=wrong").starts_with("HTTP/1.1 400"));
        assert!(request("/auth/callback?code=a&code=b&state=expected-state").starts_with("HTTP/1.1 400"));
        assert!(request("/other?code=bad&state=expected-state").starts_with("HTTP/1.1 404"));
        let callback = request("/auth/callback?code=synthetic-code&state=expected-state");
        assert!(callback.starts_with("HTTP/1.1 200"));
        worker.join().unwrap();
        let guard = pending.lock().unwrap();
        assert_eq!(guard.as_ref().unwrap().captured.as_ref().unwrap().as_ref().unwrap(), "synthetic-code");
    }

    #[test]
    fn canceled_or_superseded_exchange_cannot_save_accounts() {
        let mut current = Pending {
            login_id: "new-flow".into(), state: "state".into(), verifier: "v".into(),
            captured: None, deadline: Instant::now() + Duration::from_secs(3),
            done: false, exchanging: true, result: None,
        };
        let tokens = || crate::oauth::StoredTokens {
            access_token: "synthetic-access".into(), refresh_token: "synthetic-refresh".into(),
            expires_at: String::new(), label: Some("Synthetic".into()),
            account_id: Some("synthetic-account".into()), id_token: None,
        };
        let pending = Mutex::new(None);
        let result = finish_login(&pending, "old-flow", Ok(tokens()), |_| panic!("canceled flow saved"));
        assert!(result.error.is_some());
        current.exchanging = false;
        *pending.lock().unwrap() = Some(current);
        let result = finish_login(&pending, "old-flow", Ok(tokens()), |_| panic!("superseded flow saved"));
        assert!(result.error.is_some());
        assert!(!pending.lock().unwrap().as_ref().unwrap().done);
        let result = finish_login(&pending, "new-flow", Err("exchange failed".into()), |_| panic!("failed exchange saved"));
        assert_eq!(result.error.as_deref(), Some("exchange failed"));
        assert!(!result.done);
        assert!(pending.lock().unwrap().as_ref().unwrap().result.as_ref().unwrap().is_err());
    }

    #[test]
    fn s256_challenge_matches_the_rfc_vector() {
        // RFC 7636 appendix B.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(crate::oauth_util::code_challenge(verifier), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn callback_parsing_takes_code_and_state() {
        let line = "GET /auth/callback?code=ac%2F1&state=st-42 HTTP/1.1";
        let (code, state) = crate::oauth_util::parse_callback_request(line).unwrap();
        assert_eq!(code, "ac/1");
        assert_eq!(state, "st-42");
        assert!(crate::oauth_util::parse_callback_request("GET /auth/callback HTTP/1.1").is_none());
        assert!(crate::oauth_util::parse_callback_request("GET /auth/callback?code=only HTTP/1.1").is_none());
    }

    #[test]
    fn auth_url_carries_pkce_parameters() {
        let url = build_auth_url("st", "ch");
        assert!(url.starts_with(AUTH_ENDPOINT));
        assert!(url.contains("code_challenge=ch&code_challenge_method=S256"));
        assert!(url.contains(&format!("redirect_uri={}", crate::oauth_util::pct_encode(REDIRECT_URI))));
        assert!(url.contains("scope=openid%20profile%20email%20offline_access"));
        // The official client's simplified-flow parameters (cockpit parity).
        assert!(url.contains("id_token_add_organizations=true"));
        assert!(url.contains("codex_cli_simplified_flow=true"));
        assert!(url.contains("originator=Codex%20Desktop"));
    }
}
