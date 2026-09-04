//! Cursor OAuth sign-in — a faithful port of cockpit-tools' cursor_oauth.rs
//! (D:\code\temp\cockpit-tools, crates/cockpit-core/src/modules/cursor_oauth.rs).
//! Flow: PKCE code challenge + uuid → browser opens cursor.com/loginDeepControl
//! → the user completes login in the browser → backend polls
//! api2.cursor.sh/auth/poll until the token pair arrives. No local callback
//! server — the poll endpoint is Cursor's own.

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const CURSOR_LOGIN_URL: &str = "https://cursor.com/loginDeepControl";
const CURSOR_POLL_ENDPOINT: &str = "https://api2.cursor.sh/auth/poll";
const OAUTH_POLL_INTERVAL_SECS: u64 = 2;
const OAUTH_EXPIRES_SECS: i64 = 300;

/// What cursor_oauth_start hands the frontend.
#[derive(Serialize)]
pub struct CursorOAuthStart {
    pub login_id: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval_seconds: u64,
}

/// What cursor_oauth_poll hands the frontend: `done` with the account
/// payload on success, `error` on terminal failure, neither while pending.
#[derive(Serialize)]
pub struct CursorOAuthPoll {
    pub done: bool,
    pub account: Option<CursorAccount>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CursorAccount {
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub email: String,
    #[serde(rename = "authId", default)]
    pub auth_id: Option<String>,
    #[serde(rename = "accessToken", default)]
    pub access_token: String,
    #[serde(rename = "refreshToken", default)]
    pub refresh_token: Option<String>,
    /// Membership badge (free/pro/pro_plus/ultra/enterprise), normalized
    /// the same way cockpit does.
    #[serde(default)]
    pub membership: Option<String>,
    #[serde(default)]
    pub captured_at: i64,
}

struct PendingOAuthState {
    login_id: String,
    uuid: String,
    code_verifier: String,
    expires_at: i64,
}

static PENDING: OnceLock<Mutex<Option<PendingOAuthState>>> = OnceLock::new();

fn pending() -> &'static Mutex<Option<PendingOAuthState>> {
    PENDING.get_or_init(|| Mutex::new(None))
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 32 bytes of entropy: RandomState is OS-seeded (ASLR/heap randomization
/// per process) and a monotonic counter keeps successive calls distinct —
/// verifier and uuid must not derive from each other. No new dependency.
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
    let digest = hasher.finalize();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

fn generate_uuid() -> String {
    let b = random_bytes();
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

/// Starts a Cursor login: builds the PKCE challenge URL and registers the
/// pending state for the poll phase.
pub fn start_login() -> Result<CursorOAuthStart, String> {
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    let login_uuid = generate_uuid();
    let login_id = login_uuid.clone();
    let verification_uri = format!(
        "{}?challenge={}&uuid={}&mode=login",
        CURSOR_LOGIN_URL, code_challenge, login_uuid
    );
    *pending().lock().unwrap() = Some(PendingOAuthState {
        login_id: login_id.clone(),
        uuid: login_uuid,
        code_verifier,
        expires_at: now_secs() + OAUTH_EXPIRES_SECS,
    });
    Ok(CursorOAuthStart {
        login_id,
        verification_uri,
        expires_in: OAUTH_EXPIRES_SECS as u64,
        interval_seconds: OAUTH_POLL_INTERVAL_SECS,
    })
}

/// Cancels a pending login: the next poll tick (or an in-flight one that
/// reads the state after this) sees no pending session and reports it.
pub fn cancel_login(login_id: Option<&str>) {
    if let Ok(mut guard) = pending().lock() {
        match (login_id, guard.as_ref()) {
            (Some(id), Some(state)) if state.login_id != id => return,
            _ => *guard = None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    auth_id: Option<String>,
}

/// ONE poll tick: a single HTTP request against Cursor's auth endpoint.
/// The frontend's 2s interval drives the rhythm — an internal loop here
/// would stack concurrent 5-minute pollers on every interval fire.
/// Ok-variants: done with the account, an error (terminal), or neither
/// (still waiting).
pub async fn poll_login(login_id: &str) -> CursorOAuthPoll {
    let (uuid, code_verifier) = {
        let guard = pending().lock().unwrap();
        let Some(state) = guard.as_ref() else {
            return CursorOAuthPoll {
                done: false,
                account: None,
                error: Some("没有进行中的 Cursor 登录会话".into()),
            };
        };
        if state.login_id != login_id {
            return CursorOAuthPoll {
                done: false,
                account: None,
                error: Some("login_id 不匹配".into()),
            };
        }
        if now_secs() > state.expires_at {
            *pending().lock().unwrap() = None;
            return CursorOAuthPoll {
                done: false,
                account: None,
                error: Some("登录会话已过期".into()),
            };
        }
        (state.uuid.clone(), state.code_verifier.clone())
    };

    let poll_url = format!(
        "{}?uuid={}&verifier={}",
        CURSOR_POLL_ENDPOINT, uuid, code_verifier
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| crate::providers::http());

    let resp = match client
        .get(&poll_url)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return CursorOAuthPoll { done: false, account: None, error: None },
    };
    let status = resp.status().as_u16();
    // 404/202 = not authorized yet; other non-200s are transient — the
    // next tick retries either way.
    if status != 200 {
        return CursorOAuthPoll { done: false, account: None, error: None };
    }
    let body = match resp.text().await {
        Ok(b) => b,
        Err(_) => return CursorOAuthPoll { done: false, account: None, error: None },
    };
    let poll_data: PollResponse = match serde_json::from_str(&body) {
        Ok(d) => d,
        Err(_) => return CursorOAuthPoll { done: false, account: None, error: None },
    };
    let Some(access_token) = poll_data.access_token else {
        return CursorOAuthPoll { done: false, account: None, error: None };
    };
    *pending().lock().unwrap() = None;
    let email = poll_data
        .auth_id
        .as_deref()
        .filter(|v| v.contains('@'))
        .unwrap_or("")
        .to_string();
    CursorOAuthPoll {
        done: true,
        account: Some(CursorAccount {
            label: String::new(),
            email,
            auth_id: poll_data.auth_id,
            access_token,
            refresh_token: poll_data.refresh_token,
            membership: None,
            captured_at: now_secs(),
        }),
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_and_challenge_are_url_safe_and_consistent() {
        let verifier = generate_code_verifier();
        assert!(!verifier.contains('+') && !verifier.contains('/') && !verifier.contains('='));
        let challenge = generate_code_challenge(&verifier);
        assert!(!challenge.contains('+') && !challenge.contains('/'));
        // Same input → same challenge (S256 is deterministic).
        assert_eq!(generate_code_challenge(&verifier), challenge);
    }

    #[test]
    fn uuid_shape_is_v4_like() {
        let uuid = generate_uuid();
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
    }
}
