//! CodeBuddy (Tencent) login — the IDE's scan-login handshake, ported
//! from cockpit's `codebuddy_oauth.rs`: a state request mints the login
//! page, the user signs in there, and the token is collected by polling
//! `auth/token` until the scan completes. There is no loopback callback
//! and no user code to type — the page is the whole ceremony.
//!
//! The account lands in the codebuddy login store under its `uid`, with
//! the tenant/domain headers the billing endpoints need riding in
//! `extra` (see providers::codebuddy).

use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

const API_PREFIX: &str = "/v2/plugin";

/// One CodeBuddy-family product. The family shares the `copilot` Keycloak
/// realm and the `/v2/plugin` billing gateway; members differ in host, the
/// `platform` handshake parameter, and — for WorkBuddy AI — the billing
/// path prefix and the refresh-source header.
pub struct Family {
    pub family_id: &'static str,
    pub display_name: &'static str,
    pub api_endpoint: &'static str,
    pub platform: &'static str,
    /// Value of `X-Auth-Refresh-Source` on the token-refresh call. The
    /// WorkBuddy clients send `plugin`; the CodeBuddy IDE sends `ide-main`.
    /// The AI variant answers `invalid_grant` (code 12153) without it.
    pub refresh_source: &'static str,
    /// WorkBuddy AI serves billing under `/billing/...` first and only
    /// falls back to `/v2/billing/...` on a 404 — see `billing_paths`.
    pub bare_billing_prefix: bool,
    /// Product variant: AI-only billing/domain rules and desktop files.
    pub intl: bool,
    /// Official desktop auth store file name (both variants share the dir).
    pub local_auth_file: Option<&'static str>,
    /// Official desktop data root (`~/.workbuddy` / `~/.workbuddy-ai`).
    pub data_root: Option<&'static str>,
}

impl Family {
    /// Billing path candidates in try order. Only an HTTP 404 advances to
    /// the next candidate; 401/10085/transport errors are never path issues.
    pub fn billing_paths(&self, path: &str) -> Vec<String> {
        if !self.bare_billing_prefix {
            return vec![path.to_string()];
        }
        let primary = path.strip_prefix("/v2").unwrap_or(path).to_string();
        let fallback = format!("/v2{primary}");
        if fallback == primary {
            vec![primary]
        } else {
            vec![primary, fallback]
        }
    }

    /// The account's `domain` must belong to this variant. An empty domain
    /// cannot be judged and is accepted (the caller's compatibility rule);
    /// everything else is judged by suffix, never by substring.
    pub fn matches_domain(&self, domain: &str) -> bool {
        let domain = domain.trim().to_ascii_lowercase();
        if domain.is_empty() {
            return true;
        }
        let ai = domain == "workbuddy.ai" || domain.ends_with(".workbuddy.ai");
        if self.intl { ai } else { !ai }
    }
}

/// Adding a family member is adding one row here — the login flow, the
/// provider module and the catalog wiring all read from this table
/// (`.scratch/codebuddy-family/spec.md` D1/D2).
pub const FAMILIES: &[Family] = &[
    Family {
        family_id: "codebuddy",
        display_name: "CodeBuddy",
        api_endpoint: "https://www.codebuddy.ai",
        platform: "ide",
        refresh_source: "ide-main",
        bare_billing_prefix: false,
        intl: false,
        local_auth_file: None,
        data_root: None,
    },
    Family {
        family_id: "codebuddycn",
        display_name: "CodeBuddy CN",
        api_endpoint: "https://www.codebuddy.cn",
        platform: "ide",
        refresh_source: "ide-main",
        bare_billing_prefix: false,
        intl: false,
        local_auth_file: None,
        data_root: None,
    },
    Family {
        family_id: "workbuddy",
        display_name: "WorkBuddy",
        api_endpoint: "https://copilot.tencent.com",
        platform: "workbuddy",
        refresh_source: "plugin",
        bare_billing_prefix: false,
        intl: false,
        local_auth_file: Some("workbuddy-desktop.info"),
        data_root: Some(".workbuddy"),
    },
    Family {
        family_id: "workbuddyai",
        display_name: "WorkBuddy AI",
        api_endpoint: "https://www.workbuddy.ai",
        platform: "workbuddy-ai",
        refresh_source: "plugin",
        bare_billing_prefix: true,
        intl: true,
        local_auth_file: Some("workbuddy-desktop-ai.info"),
        data_root: Some(".workbuddy-ai"),
    },
];

pub fn family(family_id: &str) -> Option<&'static Family> {
    FAMILIES.iter().find(|f| f.family_id == family_id)
}

/// The descriptor for a registered member. Panics only on a caller naming
/// an id the table does not hold — a programming error, never user input.
pub fn known_family(family_id: &str) -> &'static Family {
    family(family_id).expect("codebuddy family member is registered")
}
const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
const REMOTE_POLL_INTERVAL: Duration = Duration::from_millis(1500);
/// Browser identity headers the IDE sends on every plugin call.
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

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
    /// The family this sign-in belongs to — its endpoint serves the polls.
    family: &'static Family,
    /// The server-issued state the token poll keys on.
    state: String,
    next_remote_poll: Instant,
    deadline: Instant,
    done: bool,
}

/// One pending sign-in per family. A second scan for the SAME family
/// replaces its predecessor (the user restarted the flow); scans for two
/// different families must not invalidate each other — the earlier single
/// slot made a CN scan silently kill an AI scan.
static PENDING: LazyLock<Mutex<HashMap<&'static str, Pending>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Per-account refresh locks (`family:account_id`), so two concurrent
/// refreshes for one account run once and the later caller reuses the
/// rotated token instead of clobbering it with its own read-modify-write.
static REFRESH_LOCKS: LazyLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn refresh_lock(key: String) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = REFRESH_LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    locks.entry(key).or_default().clone()
}

fn http() -> reqwest::Client {
    crate::providers::http()
}

fn plugin_url(f: &Family, path: &str) -> String {
    format!("{}{API_PREFIX}{path}", f.api_endpoint)
}

/// codebuddy.ai answers every plugin call with `{code, data, message}`;
/// 0 and 200 are its two success codes.
fn success_code(body: &Value) -> bool {
    matches!(body.get("code").and_then(Value::as_i64), Some(0) | Some(200))
}

fn data_of(body: &Value) -> Option<&Value> {
    body.get("data").filter(|d| d.is_object())
}

fn as_str(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// The tenant headers the billing endpoints need when a CodeBuddy
/// account belongs to an enterprise tenant.
pub struct Tenant {
    pub uid: Option<String>,
    pub enterprise_id: Option<String>,
    pub domain: Option<String>,
}

impl Tenant {
    /// Read the tenant fields out of a stored account's `extra` blob.
    pub fn from_extra(extra: &Value) -> Tenant {
        Tenant {
            uid: as_str(extra, "uid"),
            enterprise_id: as_str(extra, "enterpriseId").or_else(|| as_str(extra, "enterprise_id")),
            domain: as_str(extra, "domain"),
        }
    }
}

pub async fn start(family: &'static Family) -> Result<LoginStart, String> {
    let resp = http()
        .post(plugin_url(family, &format!("/auth/state?platform={}", family.platform)))
        .header("User-Agent", UA)
        .header("X-No-Authorization", "true")
        .header("X-No-User-Id", "true")
        .header("X-No-Enterprise-Id", "true")
        .header("X-No-Department-Info", "true")
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| format!("auth/state request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("auth/state: HTTP {}", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| format!("auth/state parse: {e}"))?;
    let data = data_of(&body).ok_or("auth/state response has no data")?;
    let state = as_str(data, "state").ok_or("auth/state response has no state")?;
    let auth_url = as_str(data, "authUrl")
        .or_else(|| as_str(data, "auth_url"))
        .or_else(|| as_str(data, "url"))
        .unwrap_or_else(|| format!("{}/login?state={state}", family.api_endpoint));

    let login_id = crate::oauth_util::random_b64url(16);
    let mut guard = PENDING.lock().map_err(|_| "login state poisoned".to_string())?;
    guard.insert(
        family.family_id,
        Pending {
            login_id: login_id.clone(),
            family,
            state,
            next_remote_poll: Instant::now(),
            deadline: Instant::now() + LOGIN_TIMEOUT,
            done: false,
        },
    );
    Ok(LoginStart { login_id, auth_url })
}

pub async fn poll(login_id: &str) -> LoginPoll {
    let (family, state, expired, done, too_soon) = {
        let Ok(mut guard) = PENDING.lock() else {
            return LoginPoll { done: false, label: None, error: Some("login state poisoned".into()) };
        };
        let Some(p) = guard.values_mut().find(|p| p.login_id == login_id) else {
            return LoginPoll { done: false, label: None, error: Some("no pending sign-in".into()) };
        };
        let too_soon = Instant::now() < p.next_remote_poll;
        if !too_soon {
            p.next_remote_poll = Instant::now() + REMOTE_POLL_INTERVAL;
        }
        (p.family, p.state.clone(), Instant::now() > p.deadline, p.done, too_soon)
    };
    if done {
        return LoginPoll { done: true, label: None, error: None };
    }
    if expired {
        finish(login_id);
        return LoginPoll { done: false, label: None, error: Some("sign-in timed out — start again".into()) };
    }
    if too_soon {
        return LoginPoll { done: false, label: None, error: None };
    }

    let resp = match http()
        .get(plugin_url(family, &format!("/auth/token?state={state}")))
        .header("User-Agent", UA)
        .header("X-No-Authorization", "true")
        .header("X-No-User-Id", "true")
        .header("X-No-Enterprise-Id", "true")
        .header("X-No-Department-Info", "true")
        .send()
        .await
    {
        Ok(resp) => resp,
        // A hiccup mid-scan is not a failure — the next tick retries.
        Err(_) => return LoginPoll { done: false, label: None, error: None },
    };
    let Ok(body) = resp.json::<Value>().await else {
        return LoginPoll { done: false, label: None, error: None };
    };
    if !success_code(&body) {
        return LoginPoll { done: false, label: None, error: None }; // scan not finished
    }
    let Some(data) = data_of(&body) else {
        return LoginPoll { done: false, label: None, error: Some("token response has no data".into()) };
    };
    let access_token = match as_str(data, "accessToken").or_else(|| as_str(data, "access_token")) {
        Some(t) => t,
        None => return LoginPoll { done: false, label: None, error: None }, // code ok, no token yet
    };
    let refresh_token =
        as_str(data, "refreshToken").or_else(|| as_str(data, "refresh_token")).unwrap_or_default();
    let domain = as_str(data, "domain");
    // The IDE reports an absolute expiry in epoch seconds or millis.
    let expires_at = data
        .get("expiresAt")
        .or_else(|| data.get("expireTime"))
        .and_then(Value::as_i64)
        .map(|raw| {
            let millis = if raw > 1_000_000_000_000 { raw } else { raw.saturating_mul(1000) };
            chrono::DateTime::from_timestamp_millis(millis)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339()
        })
        .unwrap_or_default();

    // Account profile: uid / nickname / email / tenant.
    let profile = fetch_account(family, &access_token, &state, domain.as_deref()).await;
    let uid = profile
        .as_ref()
        .and_then(|p| as_str(p, "uid"))
        .unwrap_or_default();
    let nickname = profile.as_ref().and_then(|p| as_str(p, "nickname")).unwrap_or_default();
    let email = profile.as_ref().and_then(|p| as_str(p, "email")).unwrap_or_default();
    let enterprise_id = profile
        .as_ref()
        .and_then(|p| as_str(p, "enterpriseId").or_else(|| as_str(p, "enterprise_id")))
        .unwrap_or_default();

    let account_id = if !uid.is_empty() {
        uid.clone()
    } else if !email.is_empty() {
        email.clone()
    } else {
        finish(login_id);
        return LoginPoll { done: false, label: None, error: Some("sign-in returned no account identity".into()) };
    };
    let label = if !email.is_empty() {
        email.clone()
    } else if !nickname.is_empty() {
        nickname.clone()
    } else {
        account_id.clone()
    };
    let enterprise_name = profile
        .as_ref()
        .and_then(|p| as_str(p, "enterpriseName").or_else(|| as_str(p, "enterprise_name")))
        .unwrap_or_default();

    let mut extra = serde_json::Map::new();
    extra.insert("uid".into(), Value::String(uid));
    extra.insert("sourceKind".into(), Value::String("qr".into()));
    if !enterprise_id.is_empty() {
        extra.insert("enterpriseId".into(), Value::String(enterprise_id));
    }
    if !enterprise_name.is_empty() {
        extra.insert("enterpriseName".into(), Value::String(enterprise_name));
    }
    if let Some(d) = &domain {
        extra.insert("domain".into(), Value::String(d.clone()));
    }

    // Cross-variant isolation: an AI scan must never store a CN credential
    // (or the reverse). An empty domain cannot be judged and is accepted.
    if let Some(domain) = domain.as_deref() {
        if !family.matches_domain(domain) {
            finish(login_id);
            return LoginPoll {
                done: false,
                label: None,
                error: Some(format!(
                    "{} sign-in returned a credential for a different product ({domain}) — not saved",
                    family.display_name
                )),
            };
        }
    }

    let poll_label = label.clone();
    let stored = crate::login_accounts::record_login(
        family.family_id,
        crate::login_accounts::LoginAccount {
            account_id,
            email,
            label,
            access_token,
            refresh_token,
            id_token: String::new(),
            expires_at,
            added_at: chrono::Utc::now().timestamp(),
            extra: Value::Object(extra),
        },
    );
    finish(login_id);
    match stored {
        Ok(()) => LoginPoll { done: true, label: Some(poll_label), error: None },
        Err(e) => LoginPoll { done: false, label: None, error: Some(e) },
    }
}

async fn fetch_account(
    family: &Family,
    access_token: &str,
    state: &str,
    domain: Option<&str>,
) -> Option<Value> {
    let mut req = http()
        .get(plugin_url(family, &format!("/login/account?state={state}")))
        .header("User-Agent", UA)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("X-No-User-Id", "true")
        .header("X-No-Enterprise-Id", "true")
        .header("X-No-Department-Info", "true");
    if let Some(d) = domain {
        req = req.header("X-Domain", d);
    }
    let resp = req.send().await.ok()?;
    let body: Value = resp.json().await.ok()?;
    data_of(&body).cloned()
}

/// One refresh round-trip: the IDE carries the refresh token in its own
/// header (not the body) — returns the rotated token data.
pub async fn refresh(
    family: &Family,
    access_token: &str,
    refresh_token: &str,
    domain: Option<&str>,
) -> Result<Value, String> {
    let mut req = http()
        .post(plugin_url(family, "/auth/token/refresh"))
        .header("User-Agent", UA)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("X-Refresh-Token", refresh_token)
        .header("X-Auth-Refresh-Source", family.refresh_source);
    if let Some(d) = domain {
        req = req.header("X-Domain", d);
    }
    let resp = req.send().await.map_err(|e| format!("token refresh: {e}"))?;
    let body: Value = resp.json().await.map_err(|e| format!("token refresh parse: {e}"))?;
    if !success_code(&body) {
        let msg = body
            .get("message")
            .or_else(|| body.get("msg"))
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(format!("token refresh rejected (code): {msg}"));
    }
    data_of(&body)
        .cloned()
        .ok_or_else(|| "token refresh response has no data".to_string())
}

fn finish(login_id: &str) {
    if let Ok(mut guard) = PENDING.lock() {
        guard.retain(|_, p| p.login_id != login_id);
    }
}

pub fn cancel(login_id: &str) {
    finish(login_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn success_codes_cover_zero_and_200() {
        assert!(success_code(&json!({ "code": 0 })));
        assert!(success_code(&json!({ "code": 200 })));
        assert!(!success_code(&json!({ "code": 401 })));
        assert!(!success_code(&json!({})));
    }

    #[test]
    fn tenant_reads_the_stored_extra_blob() {
        let extra = json!({ "uid": "u-1", "enterpriseId": "e-9", "domain": "d.example" });
        let tenant = Tenant::from_extra(&extra);
        assert_eq!(tenant.uid.as_deref(), Some("u-1"));
        assert_eq!(tenant.enterprise_id.as_deref(), Some("e-9"));
        assert_eq!(tenant.domain.as_deref(), Some("d.example"));
        // A blob with nothing in it yields an all-empty tenant.
        let empty = Tenant::from_extra(&json!({}));
        assert!(empty.uid.is_none() && empty.enterprise_id.is_none() && empty.domain.is_none());
    }

    #[test]
    fn data_gate_rejects_non_objects() {
        assert!(data_of(&json!({ "data": { "a": 1 } })).is_some());
        assert!(data_of(&json!({ "data": "nope" })).is_none());
        assert!(data_of(&json!({})).is_none());
    }

    #[test]
    fn family_rows_stay_listed_everywhere() {
        // Drift guard (2026-10-12 bug class): a new family member must join
        // the login store list (its accounts live there) and the provider
        // catalog, or its scanned logins become invisible — exactly the
        // workbuddy/codebuddycn holes fixed on 2026-10-12.
        for family in FAMILIES {
            assert!(
                crate::login_accounts::LOGIN_FAMILIES.contains(&family.family_id),
                "{} missing from LOGIN_FAMILIES", family.family_id
            );
            assert!(
                crate::provider_catalog::provider_definition(family.family_id).is_some(),
                "{} missing from the provider catalog", family.family_id
            );
        }
    }

    #[test]
    fn billing_paths_only_fall_back_for_the_intl_variant() {
        let cn = known_family("workbuddy");
        assert_eq!(
            cn.billing_paths("/v2/billing/meter/get-user-resource"),
            vec!["/v2/billing/meter/get-user-resource"]
        );
        let ide = known_family("codebuddy");
        assert_eq!(
            ide.billing_paths("/v2/billing/meter/get-user-resource"),
            vec!["/v2/billing/meter/get-user-resource"]
        );
        let ai = known_family("workbuddyai");
        assert_eq!(
            ai.billing_paths("/v2/billing/meter/get-user-resource"),
            vec![
                "/billing/meter/get-user-resource",
                "/v2/billing/meter/get-user-resource"
            ]
        );
        // The web-summary path already sits at the root — one candidate.
        assert_eq!(
            ai.billing_paths("/billing/meter/get-user-resource-summary"),
            vec![
                "/billing/meter/get-user-resource-summary",
                "/v2/billing/meter/get-user-resource-summary"
            ]
        );
    }

    #[test]
    fn domain_matching_is_suffix_only_and_per_variant() {
        let cn = known_family("workbuddy");
        let ai = known_family("workbuddyai");
        // Empty domains cannot be judged.
        assert!(cn.matches_domain(""));
        assert!(ai.matches_domain("  "));
        // The AI variant accepts only the .workbuddy.ai product domain.
        assert!(ai.matches_domain("www.workbuddy.ai"));
        assert!(ai.matches_domain(" WWW.WORKBUDDY.AI "));
        assert!(!ai.matches_domain("www.codebuddy.cn"));
        assert!(!ai.matches_domain("www.workbuddy.ai.evil.com"));
        // The CN variant rejects the AI suffix and accepts its own domains.
        assert!(cn.matches_domain("www.workbuddy.cn"));
        assert!(cn.matches_domain("www.codebuddy.cn"));
        assert!(!cn.matches_domain("www.workbuddy.ai"));
    }

    #[test]
    fn desktop_files_are_split_per_variant() {
        assert_eq!(known_family("workbuddy").local_auth_file, Some("workbuddy-desktop.info"));
        assert_eq!(known_family("workbuddyai").local_auth_file, Some("workbuddy-desktop-ai.info"));
        assert_eq!(known_family("workbuddy").data_root, Some(".workbuddy"));
        assert_eq!(known_family("workbuddyai").data_root, Some(".workbuddy-ai"));
        assert_eq!(known_family("codebuddy").local_auth_file, None);
    }
}
