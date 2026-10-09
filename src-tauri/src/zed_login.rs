//! Zed login — the editor's native app sign-in, ported from cockpit's
//! `zed_oauth.rs`: Pane opens an ephemeral loopback port, hands Zed its
//! public key, and the browser posts back a `user_id` plus an access
//! token sealed to that key (RSA-OAEP, with the vendor's Pkcs1v15
//! fallback).
//!
//! Zed publishes no usage API to desktop credentials — the billing
//! endpoints answer 401 for them, which is why the reference only ever
//! reads the profile (cockpit's own note says the same). The account
//! therefore carries identity + plan; the editor's usage stays local
//! (see providers::zed, which reads the editor's own thread database).

use base64::Engine;
use rand_core::{OsRng, RngCore};
use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey, EncodeRsaPublicKey};
use rsa::{Oaep, Pkcs1v15Encrypt, RsaPrivateKey};
use sha2::Sha256;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const SIGNIN_URL: &str = "https://zed.dev/native_app_signin";
pub const CLOUD_BASE_URL: &str = "https://cloud.zed.dev";
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
    private_key_der: Vec<u8>,
    captured: Option<Result<(String, String), String>>,
    deadline: Instant,
    done: bool,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
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

fn decode_b64(raw: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(raw.as_bytes())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(raw.as_bytes()))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(raw.as_bytes()))
        .map_err(|e| format!("decode: {e}"))
}

/// A fresh 2048-bit key pair; only the public half goes to Zed.
fn key_pair() -> Result<(Vec<u8>, String), String> {
    let private = RsaPrivateKey::new(&mut OsRng, 2048).map_err(|e| format!("keygen: {e}"))?;
    let private_der = private
        .to_pkcs1_der()
        .map_err(|e| format!("encode private key: {e}"))?;
    let public = private.to_public_key();
    let public_der = public
        .to_pkcs1_der()
        .map_err(|e| format!("encode public key: {e}"))?;
    Ok((private_der.as_bytes().to_vec(), b64(public_der.as_bytes())))
}

/// Unseal the callback's token with the login's private key.
fn decrypt_token(private_der: &[u8], sealed: &str) -> Result<String, String> {
    let private = RsaPrivateKey::from_pkcs1_der(private_der).map_err(|e| format!("parse key: {e}"))?;
    let cipher = decode_b64(sealed)?;
    let plain = private
        .decrypt(Oaep::new::<Sha256>(), &cipher)
        .or_else(|_| private.decrypt(Pkcs1v15Encrypt, &cipher))
        .map_err(|_| "callback token could not be decrypted — wrong session key".to_string())?;
    String::from_utf8(plain).map_err(|e| format!("token is not UTF-8: {e}"))
}

pub fn start() -> Result<LoginStart, String> {
    let (private_der, public_key) = key_pair()?;
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| format!("bind callback port: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("callback port lookup: {e}"))?
        .port();
    listener.set_nonblocking(true).map_err(|e| format!("listener setup: {e}"))?;

    let login_id = b64(&{
        let mut bytes = [0u8; 12];
        OsRng.fill_bytes(&mut bytes);
        bytes
    });
    let auth_url = format!(
        "{SIGNIN_URL}?native_app_port={port}&native_app_public_key={}",
        pct(&public_key)
    );
    {
        let mut guard = PENDING.lock().map_err(|_| "login state poisoned".to_string())?;
        *guard = Some(Pending {
            login_id: login_id.clone(),
            private_key_der: private_der.clone(),
            captured: None,
            deadline: Instant::now() + LOGIN_TIMEOUT,
            done: false,
        });
    }

    let thread_login = login_id.clone();
    // The listener thread owns this session's private key (a clone — the
    // pending state keeps its own copy so a poll can never lose it).
    let thread_key = private_der.clone();
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
                let target = line.split_whitespace().nth(1).unwrap_or_default();
                let query = target.split_once('?').map(|(_, q)| q).unwrap_or_default();
                let mut params: Vec<(String, String)> = Vec::new();
                for pair in query.split('&') {
                    if let Some((k, v)) = pair.split_once('=') {
                        params.push((k.to_string(), v.to_string()));
                    }
                }
                let get = |name: &str| {
                    params
                        .iter()
                        .find(|(k, _)| k == name)
                        .map(|(_, v)| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                };
                let outcome = match get("error") {
                    Some(err) => Err(format!(
                        "authorization failed: {err}{}",
                        get("error_description").map(|d| format!(" ({d})")).unwrap_or_default()
                    )),
                    None => match (get("user_id"), get("access_token")) {
                        (Some(user_id), Some(sealed)) => {
                            decrypt_token(&thread_key, &sealed).map(|token| (user_id, token))
                        }
                        _ => Err("callback carried no user_id / access_token".into()),
                    },
                };
                let ok = outcome.is_ok();
                let html = "<!doctype html><html><head><meta charset=\"utf-8\"><title>Pane</title></head>\
                     <body style=\"font-family:sans-serif;background:#18181b;display:flex;\
                     align-items:center;justify-content:center;height:100vh;margin:0\">\
                     <p style=\"font-family:sans-serif;color:4ade80\">Sign-in complete — you can close this tab and return to Pane.</p></body></html>";
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
                        Some(Ok(pair)) => Take::Ready(pair.0, pair.1),
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
        Take::Ready(user_id, access_token) => {
            let stored = crate::login_accounts::record_login(
                "zed",
                crate::login_accounts::LoginAccount {
                    account_id: user_id.clone(),
                    email: String::new(),
                    label: format!("Zed — {user_id}"),
                    access_token,
                    refresh_token: String::new(),
                    id_token: String::new(),
                    expires_at: String::new(),
                    added_at: chrono::Utc::now().timestamp(),
                    extra: serde_json::Value::Null,
                },
            );
            finish(login_id);
            match stored {
                Ok(()) => LoginPoll { done: true, label: Some(format!("Zed — {user_id}")), error: None },
                Err(e) => LoginPoll { done: false, label: None, error: Some(e) },
            }
        }
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
    fn a_sealed_token_unseals_with_its_own_key() {
        // Round-trip: RSA-OAEP encrypt with the public half, decrypt with
        // the private half, exactly like the browser → callback path.
        use rsa::traits::PublicKeyParts;
        let (private_der, public_b64) = key_pair().expect("keypair");
        let private = RsaPrivateKey::from_pkcs1_der(&private_der).unwrap();
        let public = RsaPublicKey::new(
            private.n().clone(),
            private.e().clone(),
        )
        .expect("public");
        let cipher = public
            .encrypt(&mut OsRng, Oaep::new::<Sha256>(), b"zed-access-token")
            .expect("encrypt");
        let sealed = b64(&cipher);
        assert_eq!(
            decrypt_token(&private_der, &sealed).unwrap(),
            "zed-access-token"
        );
        assert!(public_b64.len() > 100);
    }

    #[test]
    fn a_foreign_key_cannot_unseal_the_token() {
        use rsa::traits::PublicKeyParts;
        let (mine, _) = key_pair().expect("mine");
        let (theirs, _) = key_pair().expect("theirs");
        let their_private = RsaPrivateKey::from_pkcs1_der(&theirs).unwrap();
        let their_public =
            RsaPublicKey::new(their_private.n().clone(), their_private.e().clone()).unwrap();
        let cipher = their_public
            .encrypt(&mut OsRng, Oaep::new::<Sha256>(), b"token")
            .expect("encrypt");
        assert!(decrypt_token(&mine, &b64(&cipher)).is_err());
    }
}
