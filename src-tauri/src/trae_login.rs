//! Trae (international) login — the IDE's browser authorization, ported
//! from cockpit's `trae_oauth.rs` for the `trae.ai` edition (the CN
//! edition has its own `traecn` card and its own hosts).
//!
//! The flow: GetLoginGuidance names the live auth host, the browser
//! authorizes with the IDE's full parameter set (device/machine ids,
//! PKCE S256, callback URL), the loopback `/authorize` callback carries
//! the auth code, and the exchange turns it into the session token the
//! quota endpoints accept. A per-device ECDSA P-256 public key rides in
//! the exchange's DeviceInfo — the vendor binds the session to it.

use base64::Engine;
use rand_core::{OsRng, RngCore};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const GUIDANCE_ENDPOINTS: [&str; 3] = [
    "https://api.trae.ai/cloudide/api/v3/trae/GetLoginGuidance",
    "https://www.trae.ai/cloudide/api/v3/trae/GetLoginGuidance",
    "https://api.marscode.com/cloudide/api/v3/trae/GetLoginGuidance",
];
const AUTHORIZATION_PATH: &str = "/authorization";
const CALLBACK_PATH: &str = "/authorize";
const EXCHANGE_TOKEN_PATH: &str = "/trae/api/v3/oauth/ExchangeToken";
const USER_INFO_PATH: &str = "/cloudide/api/v3/trae/GetUserInfo";
pub const DEFAULT_HOST: &str = "https://grow-normal.trae.ai";
const AUTH_CLIENT_ID: &str = "ono9krqynydwx5";
/// The IDE's own minimum accepted app version; older values are rejected
/// by the auth page (cockpit's floor).
const MIN_AUTH_APP_VERSION: &str = "3.5.54";
const APP_TYPE: &str = "stable";
const PLUGIN_VERSION: &str = "local";
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
    verifier: String,
    login_host: String,
    machine_id: String,
    device_id: String,
    captured: Option<Result<String, String>>,
    deadline: Instant,
    done: bool,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn random_token() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    // UUID-ish: cockpit hands the vendor a v4 in these fields.
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn code_challenge(verifier: &str) -> String {
    b64url(&Sha256::digest(verifier.as_bytes()))
}

fn pct(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The device public key the exchange expects: SPKI DER, PEM-wrapped.
fn device_public_key_pem() -> Result<String, String> {
    use p256::pkcs8::EncodePublicKey;
    let secret = p256::SecretKey::random(&mut OsRng);
    let public = secret.public_key();
    let der = public
        .to_public_key_der()
        .map_err(|e| format!("device key encode: {e}"))?;
    let body = b64_encode(&der.as_bytes());
    let mut pem = String::from("-----BEGIN PUBLIC KEY-----\n");
    for chunk in body.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        pem.push('\n');
    }
    pem.push_str("-----END PUBLIC KEY-----\n");
    Ok(pem)
}

fn b64_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn callback_html(ok: bool) -> String {
    let (fg, text) = if ok {
        ("#4ade80", "Sign-in complete — you can close this tab and return to Pane.")
    } else {
        ("#f87171", "Sign-in failed — return to Pane and try again.")
    };
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Pane</title></head>\
         <body style=\"font-family:sans-serif;background:#18181b;color:{fg};display:flex;\
         align-items:center;justify-content:center;height:100vh;margin:0\"><p>{text}</p></body></html>"
    )
}

/// Ask the vendor which host currently serves the login page; the first
/// endpoint that answers wins (cockpit's candidate order).
async fn login_host() -> Result<String, String> {
    let trace = random_token();
    let mut errors: Vec<String> = Vec::new();
    for endpoint in GUIDANCE_ENDPOINTS {
        let resp = match crate::providers::http()
            .post(endpoint)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("User-Agent", "Trae/1.0.0 pane")
            .json(&json!({ "loginTraceID": trace, "login_trace_id": trace }))
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                errors.push(format!("{endpoint} => {e}"));
                continue;
            }
        };
        if !resp.status().is_success() {
            errors.push(format!("{endpoint} => HTTP {}", resp.status()));
            continue;
        }
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        let host = ["Result", "result", "data", "Data"]
            .iter()
            .find_map(|k| body.pointer(&format!("/{k}")))
            .and_then(|v| {
                ["loginHost", "login_host", "host", "url", "LoginHost"]
                    .iter()
                    .find_map(|f| v.get(*f).and_then(Value::as_str))
            })
            .or_else(|| {
                ["loginHost", "login_host", "host", "url"]
                    .iter()
                    .find_map(|f| body.get(*f).and_then(Value::as_str))
            })
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        if let Some(host) = host {
            return Ok(if host.starts_with("http") {
                host
            } else {
                format!("https://{host}")
            });
        }
        errors.push(format!("{endpoint} => no login host in response"));
    }
    Err(format!("Trae login guidance failed: {}", errors.join(" | ")))
}

fn build_auth_url(
    login_host: &str,
    trace: &str,
    callback_url: &str,
    machine_id: &str,
    device_id: &str,
    challenge: &str,
) -> String {
    let host = login_host.trim_end_matches('/');
    format!(
        "{host}{AUTHORIZATION_PATH}?login_version=1&auth_from=trae&login_channel=native_ide&\
         plugin_version={plugin}&auth_type=local&client_id={client}&redirect=0&\
         login_trace_id={trace}&auth_callback_url={cb}&machine_id={machine}&device_id={device}&\
         x_device_id={device}&x_machine_id={machine}&x_device_brand=Generic&x_device_type=PC&\
         x_os_version=10.0.0&x_env=prod&x_app_version={app}&x_app_type={app_type}&\
         code_challenge={challenge}&code_challenge_method=S256",
        plugin = pct(PLUGIN_VERSION),
        client = AUTH_CLIENT_ID,
        trace = pct(trace),
        cb = pct(callback_url),
        machine = pct(machine_id),
        device = pct(device_id),
        app = pct(MIN_AUTH_APP_VERSION),
        app_type = APP_TYPE,
        challenge = pct(challenge),
    )
}

pub async fn start() -> Result<LoginStart, String> {
    let host = login_host().await?;
    let probe = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| format!("bind callback port: {e}"))?;
    let port = probe
        .local_addr()
        .map_err(|e| format!("callback port lookup: {e}"))?
        .port();
    drop(probe);
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| format!("bind callback port: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| format!("listener setup: {e}"))?;

    let login_id = random_token();
    let trace = random_token();
    let state = random_token();
    let verifier = b64url(&{
        let mut bytes = [0u8; 48];
        OsRng.fill_bytes(&mut bytes);
        bytes
    });
    let machine_id = random_token();
    let device_id = random_token();
    let callback_url = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");
    let auth_url = build_auth_url(
        &host,
        &trace,
        &callback_url,
        &machine_id,
        &device_id,
        &code_challenge(&verifier),
    );
    {
        let mut guard = PENDING.lock().map_err(|_| "login state poisoned".to_string())?;
        *guard = Some(Pending {
            login_id: login_id.clone(),
            state,
            verifier,
            login_host: host,
            machine_id,
            device_id,
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
                if !path.starts_with(CALLBACK_PATH) {
                    let _ = stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                    continue;
                }
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
                        .filter(|v| !v.is_empty())
                };
                let code = ["authCode", "auth_code", "AuthCode", "authorization_code", "code"]
                    .iter()
                    .find_map(|k| get(k))
                    .or_else(|| {
                        ["authCodeInfo", "auth_code_info", "AuthCodeInfo"]
                            .iter()
                            .find_map(|k| get(k))
                            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                            .and_then(|v| {
                                ["AuthCode", "authCode", "auth_code", "code"]
                                    .iter()
                                    .find_map(|f| v.get(*f).and_then(Value::as_str))
                                    .map(str::to_string)
                            })
                    });
                let error = get("error").or_else(|| get("error_description"));
                let outcome = match (code, error) {
                    (_, Some(err)) => Err(format!("authorization failed: {err}")),
                    (Some(code), None) => Ok(code),
                    _ => Err("callback carried no auth code".into()),
                };
                let ok = outcome.is_ok();
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
                if let Ok(mut guard) = PENDING.lock() {
                    if let Some(p) = guard.as_mut().filter(|p| p.login_id == thread_login) {
                        p.captured = Some(outcome);
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

fn pick_str(value: &Value, paths: &[&[&str]]) -> Option<String> {
    for path in paths {
        let mut current = value;
        let mut ok = true;
        for key in *path {
            match current.get(*key) {
                Some(next) => current = next,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            if let Some(text) = current.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                return Some(text.to_string());
            }
        }
    }
    None
}

async fn post_json(url: &str, body: Value) -> Result<Value, String> {
    let resp = crate::providers::http()
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("User-Agent", "Trae/1.0.0 pane")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("{url} request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("{url}: HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| format!("{url} parse: {e}"))
}

/// Exchange the auth code, fetch the profile, store the account.
async fn complete(code: &str, verifier: &str, machine_id: &str, device_id: &str) -> Result<String, String> {
    let device_info = json!({
        "DeviceID": device_id,
        "MachineID": machine_id,
        "PlatformCode": "IDE_PC",
        "DeviceType": "PC",
        "DeviceName": "PC",
        "DeviceModel": "Generic",
        "ClientVersion": MIN_AUTH_APP_VERSION,
        "DevicePublicKey": device_public_key_pem()?,
        "DeviceBrand": "Generic",
        "DeviceCPU": "",
        "OSInfo": "PC",
        "OSVersion": "10.0.0",
    });
    let exchanged = post_json(
        &format!("{DEFAULT_HOST}{EXCHANGE_TOKEN_PATH}"),
        json!({
            "ClientID": AUTH_CLIENT_ID,
            "AuthCode": code,
            "CodeVerifier": verifier,
            "DeviceInfo": device_info,
            "IDEVersion": MIN_AUTH_APP_VERSION,
        }),
    )
    .await?;
    let access_token = pick_str(
        &exchanged,
        &[
            &["Result", "AccessToken"],
            &["Result", "accessToken"],
            &["result", "accessToken"],
            &["data", "accessToken"],
            &["data", "access_token"],
        ],
    )
    .ok_or("ExchangeToken returned no access token")?;
    let refresh_token = pick_str(
        &exchanged,
        &[
            &["Result", "RefreshToken"],
            &["Result", "refreshToken"],
            &["result", "refreshToken"],
            &["data", "refreshToken"],
        ],
    )
    .unwrap_or_default();

    // Profile: the user id is the account identity, the email the label.
    let profile = post_json(
        &format!("{DEFAULT_HOST}{USER_INFO_PATH}"),
        json!({}),
    )
    .await
    .ok()
    .and_then(|v| pick_str(&v, &[&["Result"], &["result"], &["data"]]).map(|_| v.clone()).or(Some(v)))
    .unwrap_or(Value::Null);
    let user_id = pick_str(
        &profile,
        &[
            &["Result", "UserID"],
            &["Result", "userId"],
            &["Result", "user_id"],
            &["result", "userId"],
            &["data", "userId"],
        ],
    );
    let email = pick_str(
        &profile,
        &[
            &["Result", "Email"],
            &["Result", "email"],
            &["result", "email"],
            &["data", "email"],
        ],
    )
    .unwrap_or_default();
    let nickname = pick_str(
        &profile,
        &[
            &["Result", "Nickname"],
            &["Result", "nickname"],
            &["Result", "Name"],
            &["data", "nickname"],
        ],
    )
    .unwrap_or_default();
    let account_id = match user_id.or_else(|| (!email.is_empty()).then(|| email.clone())) {
        Some(id) => id,
        None => return Err("Trae profile returned no account identity".into()),
    };
    let label = if !email.is_empty() {
        email.clone()
    } else if !nickname.is_empty() {
        nickname.clone()
    } else {
        account_id.clone()
    };

    crate::login_accounts::record_login(
        "trae",
        crate::login_accounts::LoginAccount {
            account_id,
            email,
            label: label.clone(),
            access_token,
            refresh_token,
            id_token: String::new(),
            expires_at: String::new(),
            added_at: chrono::Utc::now().timestamp(),
            extra: json!({ "host": DEFAULT_HOST, "deviceId": device_id, "machineId": machine_id }),
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
                        Some(Ok(code)) => Take::Ready(code),
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
    let (verifier, machine_id, device_id) = {
        let Ok(guard) = PENDING.lock() else {
            return LoginPoll { done: false, label: None, error: Some("login state poisoned".into()) };
        };
        match guard.as_ref() {
            Some(p) => (p.verifier.clone(), p.machine_id.clone(), p.device_id.clone()),
            None => (String::new(), String::new(), String::new()),
        }
    };
    match take {
        Take::Waiting => LoginPoll { done: false, label: None, error: None },
        Take::Done => LoginPoll { done: true, label: None, error: None },
        Take::Failed(error) => LoginPoll { done: false, label: None, error: Some(error) },
        Take::Ready(code) => {
            let mark_done = |done_ok: bool| {
                if let Ok(mut guard) = PENDING.lock() {
                    if let Some(p) = guard.as_mut().filter(|p| p.login_id == login_id) {
                        p.done = done_ok;
                    }
                }
            };
            match complete(&code, &verifier, &machine_id, &device_id).await {
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
    fn auth_url_carries_the_ide_parameter_set() {
        let url = build_auth_url(
            "https://www.trae.ai",
            "trace-1",
            "http://127.0.0.1:5000/authorize",
            "machine-1",
            "device-1",
            "challenge-1",
        );
        assert!(url.starts_with("https://www.trae.ai/authorization?"));
        assert!(url.contains("client_id=ono9krqynydwx5"));
        assert!(url.contains("auth_from=trae"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&pct("http://127.0.0.1:5000/authorize")));
        assert!(url.contains(&pct("challenge-1")));
        assert!(url.contains("x_app_version=3.5.54"));
    }

    #[test]
    fn device_public_key_is_pem_spki() {
        let pem = device_public_key_pem().expect("key");
        assert!(pem.starts_with("-----BEGIN PUBLIC KEY-----"));
        assert!(pem.trim_end().ends_with("-----END PUBLIC KEY-----"));
    }

    #[test]
    fn pick_str_walks_nested_paths() {
        let value = json!({ "Result": { "AccessToken": "at", "UserID": "u-1" } });
        assert_eq!(
            pick_str(&value, &[&["Result", "AccessToken"], &["data", "accessToken"]]).as_deref(),
            Some("at")
        );
        assert_eq!(pick_str(&value, &[&["Result", "UserID"]]).as_deref(), Some("u-1"));
        assert!(pick_str(&value, &[&["data", "token"]]).is_none());
    }
}
