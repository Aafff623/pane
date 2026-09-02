pub mod aihubmix;
pub mod antigravity;
pub mod claude;
pub mod codebuff;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod deepseek;
pub mod devin;
pub mod elevenlabs;
pub mod grok;
pub mod hermes;
pub mod kilo;
pub mod kimi;
pub mod minimax;
pub mod moonshot;
pub mod novita;
pub mod ollama;
pub mod onenewapi;
pub mod opencode;
pub mod openrouter;
pub mod qwen;
pub mod relaybalance;
pub mod siliconflow;
pub mod stepfun;
pub mod zai;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// One row inside a provider card, e.g. "Session ▓▓▓░░ 43% left · Resets in 2h".
/// `resets_at` (epoch ms) + `period_ms` are the structured facts the pace
/// engine needs; the UI formats countdowns and projections from them.
#[derive(Serialize, Deserialize, Clone)]
pub struct Metric {
    pub label: String,
    pub kind: String, // "progress" | "text"
    pub used_percent: Option<f64>,
    pub detail: Option<String>,
    pub value: Option<String>,
    pub resets_at: Option<i64>,
    pub period_ms: Option<i64>,
}

impl Metric {
    pub fn progress(label: &str, used_percent: f64, detail: Option<String>) -> Self {
        Self {
            label: label.into(),
            kind: "progress".into(),
            used_percent: Some(used_percent),
            detail,
            value: None,
            resets_at: None,
            period_ms: None,
        }
    }

    #[allow(dead_code)]
    pub fn text(label: &str, value: String) -> Self {
        Self {
            label: label.into(),
            kind: "text".into(),
            used_percent: None,
            detail: None,
            value: Some(value),
            resets_at: None,
            period_ms: None,
        }
    }

    pub fn with_reset(mut self, resets_at: Option<i64>, period_ms: Option<i64>) -> Self {
        self.resets_at = resets_at;
        self.period_ms = period_ms;
        self
    }
}

/// Everything one provider reports back after a refresh. `stale` marks a
/// snapshot that is actually the last good fetch, shown because the newest
/// attempt failed transiently (`warning` carries that error).
#[derive(Serialize, Deserialize, Clone)]
pub struct Snapshot {
    pub id: String,
    pub name: String,
    pub plan: Option<String>,
    pub status: String, // "ok" | "no_credentials" | "error"
    pub error: Option<String>,
    pub metrics: Vec<Metric>,
    pub stale: bool,
    pub warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard_url: Option<String>,
}

impl Snapshot {
    pub fn ok(id: &str, name: &str, plan: Option<String>, metrics: Vec<Metric>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            plan,
            status: "ok".into(),
            error: None,
            metrics,
            stale: false,
            warning: None,
            dashboard_url: None,
        }
    }

    pub fn no_credentials(id: &str, name: &str, hint: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            plan: None,
            status: "no_credentials".into(),
            error: Some(hint.into()),
            metrics: vec![],
            stale: false,
            warning: None,
            dashboard_url: None,
        }
    }

    pub fn error(id: &str, name: &str, message: String) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            plan: None,
            status: "error".into(),
            error: Some(message),
            metrics: vec![],
            stale: false,
            warning: None,
            dashboard_url: None,
        }
    }
}

/// Optional outbound proxy from config.json `proxy: { enabled, url }`.
/// Loaded once per app run (Mac parity — a change needs a restart) and never
/// applied to loopback, so the local Antigravity/HTTP-API traffic stays direct.
fn proxy_url() -> Option<&'static str> {
    static PROXY: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    PROXY
        .get_or_init(|| {
            let cfg: serde_json::Value = std::fs::read_to_string(config_dir().join("config.json"))
                .ok()
                .and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())?;
            let proxy = cfg.get("proxy")?;
            if !proxy
                .get("enabled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                return None;
            }
            let url = proxy.get("url")?.as_str()?.trim().to_string();
            let valid = ["http://", "https://", "socks5://"]
                .iter()
                .any(|s| url.starts_with(s));
            if url.is_empty() || !valid {
                return None;
            }
            Some(url)
        })
        .as_deref()
}

fn http_builder() -> reqwest::ClientBuilder {
    let mut builder = reqwest::Client::builder()
        .user_agent("Pane-Windows/0.3")
        .timeout(std::time::Duration::from_secs(20))
        // At boot the network is often still coming up; without a connect
        // cap every request rides the full 20 s, and the UI's first paint
        // waits on the slowest provider chain. Failing to connect in 5 s
        // is a dead network — fail fast, serve the cached snapshot.
        .connect_timeout(std::time::Duration::from_secs(5));
    if let Some(url) = proxy_url() {
        if let Ok(proxy) = reqwest::Proxy::all(url) {
            let proxy = proxy.no_proxy(reqwest::NoProxy::from_string("localhost,127.0.0.1,::1"));
            builder = builder.proxy(proxy);
        }
    }
    builder
}

pub fn http() -> reqwest::Client {
    http_builder().build().expect("failed to build http client")
}

/// Same client as [`http`] but never follows redirects. One/New API status
/// and billing calls must not be bounced onto another origin.
pub fn http_no_redirect() -> reqwest::Client {
    http_builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("failed to build http client")
}

/// JSON bodies from vendor APIs are tiny (quota + token responses). Cap
/// before parse so a huge payload can't stall a refresh or blow RAM —
/// same idea as the share-card decode bound.
pub(crate) async fn json_body(
    resp: reqwest::Response,
    max_bytes: usize,
    what: &str,
) -> Result<serde_json::Value, String> {
    if resp.content_length().is_some_and(|n| n > max_bytes as u64) {
        return Err(format!("{what}: response too large"));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("{what}: {e}"))?;
    if bytes.len() > max_bytes {
        return Err(format!("{what}: response too large"));
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("{what} parse: {e}"))
}

pub(crate) fn read_small_text(
    path: &std::path::Path,
    max_bytes: u64,
    what: &str,
) -> Result<String, String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("read {what}: {e}"))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(format!("{what} is not a regular file"));
    }
    if meta.len() > max_bytes {
        return Err(format!("{what} is unexpectedly large — not reading it"));
    }
    std::fs::read_to_string(path).map_err(|e| format!("read {what}: {e}"))
}

/// Where Pane keeps its own settings, e.g. saved API keys:
/// C:\Users\you\AppData\Roaming\Pane
///
/// The app shipped as "OpenUsage" before the rename — on first call, an
/// existing %APPDATA%\OpenUsage is moved over so nobody loses their config,
/// keys, or caches. If the move fails but the old dir is usable, keep using
/// the old dir rather than silently starting fresh.
pub fn config_dir() -> PathBuf {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let base = dirs::config_dir().unwrap_or_default();
        let new = base.join("Pane");
        let old = base.join("OpenUsage");
        if !new.exists() && old.exists() {
            let _ = std::fs::rename(&old, &new);
            if !new.exists() {
                return old;
            }
        }
        new
    })
    .clone()
}

/// Reads a generic credential's blob from Windows Credential Manager.
pub fn read_windows_credential(target: &str) -> Option<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };
    let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    let mut pcred: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        if CredReadW(PCWSTR(wide.as_ptr()), CRED_TYPE_GENERIC, None, &mut pcred).is_err() {
            return None;
        }
        let cred = &*pcred;
        let blob =
            std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize)
                .to_vec();
        CredFree(pcred as *mut std::ffi::c_void);
        Some(blob)
    }
}

/// Credential blob → text: UTF-8 or UTF-16 LE, unwrapping go-keyring's
/// `go-keyring-base64:` prefix (used by Go CLIs like gh and Antigravity).
pub fn credential_string(target: &str) -> Option<String> {
    let blob = read_windows_credential(target)?;
    let text = String::from_utf8(blob.clone()).ok().or_else(|| {
        if blob.len() % 2 == 0 {
            let utf16: Vec<u16> = blob
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16(&utf16).ok()
        } else {
            None
        }
    })?;
    let text = text.trim().trim_matches('\0').to_string();
    if let Some(b64) = text.strip_prefix("go-keyring-base64:") {
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .ok()?;
        return String::from_utf8(decoded).ok();
    }
    Some(text)
}

/// Percent-used meter for pay-as-you-go balances. These APIs report only
/// what's left — never "of how much" — so Pane remembers the highest
/// balance it has ever seen per provider (a top-up raises it automatically)
/// and meters usage against that high-water mark. Persisted so restarts
/// keep the story. As a progress row it also feeds the notification rules
/// ("Almost Out" fires under 10% remaining) like every other meter.
pub fn credit_meter(provider: &str, sign: &str, balance: f64) -> Option<Metric> {
    credit_meter_labeled(provider, sign, balance, "Credits used", "")
}

/// credit_meter with a caller-chosen row label and caption suffix —
/// purchased-credit pools (Codex Extra credits, Devin's extra balance)
/// meter identically but shouldn't all be called "Credits used", and some
/// carry an extra unit in the caption ("· N credits").
pub fn credit_meter_labeled(
    provider: &str,
    sign: &str,
    balance: f64,
    label: &str,
    caption_suffix: &str,
) -> Option<Metric> {
    if !balance.is_finite() || balance < 0.0 {
        return None;
    }
    // Providers refresh concurrently and this is a read-modify-write on a
    // shared file — serialize it, or one card's just-raised high-water
    // mark can be overwritten by another's stale copy.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock();
    let path = config_dir().join("credit_baselines.json");
    let mut doc: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    let high = doc
        .get(provider)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    if balance > high {
        doc[provider] = serde_json::Value::from(balance);
        let _ = std::fs::write(
            &path,
            serde_json::to_string_pretty(&doc).unwrap_or_default(),
        );
    }
    let high = high.max(balance);
    if high <= 0.0 {
        return None;
    }
    let used = ((1.0 - balance / high) * 100.0).clamp(0.0, 100.0);
    Some(Metric::progress(
        label,
        used,
        Some(format!(
            "{sign}{balance:.2} of {sign}{high:.2} left{caption_suffix}"
        )),
    ))
}

/// Candidate roots where a second account's CLI config dir may live:
/// dot-folders in the home directory plus dirs under ~/.config — the
/// places CLAUDE_CONFIG_DIR / CODEX_HOME setups conventionally point.
/// Shared by every provider family that supports multi-account discovery.
pub(crate) fn account_scan_roots() -> Vec<std::path::PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let mut roots = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&home) {
        for e in entries.flatten() {
            let p = e.path();
            let dotted = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'));
            if dotted && p.is_dir() {
                roots.push(p);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(home.join(".config")) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                roots.push(p);
            }
        }
    }
    roots
}

/// True when Customize has this provider switched off. Disabled providers
/// must not make network calls — including a folded-in wallet fetch that
/// lives on another card (Kimi Code's Moonshot API bar).
pub fn provider_disabled(id: &str) -> bool {
    let Ok(raw) = std::fs::read_to_string(config_dir().join("config.json")) else {
        return false;
    };
    let Ok(cfg) = serde_json::from_str::<serde_json::Value>(raw.trim_start_matches('\u{feff}'))
    else {
        return false;
    };
    cfg.get("disabled")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(id)))
}

/// API key lookup: our saved config file first, then environment variables.
pub fn stored_api_key(provider: &str, env_vars: &[&str]) -> Option<String> {
    let path = config_dir().join(format!("{provider}.json"));
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(key) = doc.get("apiKey").and_then(serde_json::Value::as_str) {
                let key = key.trim();
                if !key.is_empty() {
                    return Some(key.to_string());
                }
            }
        }
    }
    for var in env_vars {
        if let Ok(key) = std::env::var(var) {
            let key = key.trim().to_string();
            if !key.is_empty() {
                return Some(key);
            }
        }
    }
    None
}

/// Pure file probe (no env fallback): the apiKey saved in
/// %APPDATA%\Pane\<provider>.json, if any. Used by get_credential_status
/// to report "stored key" separately from the env-var fallback.
pub fn stored_key_file(provider: &str) -> Option<String> {
    let path = config_dir().join(format!("{provider}.json"));
    let raw = std::fs::read_to_string(&path).ok()?;
    let doc = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let key = doc.get("apiKey")?.as_str()?.trim().to_string();
    if key.is_empty() { None } else { Some(key) }
}

/// Companion to `stored_api_key` for providers whose saved blob also carries
/// a custom endpoint: reads the `baseUrl` field of the same
/// %APPDATA%\Pane\<provider>.json. No env fallback — a URL typed into
/// Settings is the only source.
pub fn stored_base_url(provider: &str) -> Option<String> {
    let path = config_dir().join(format!("{provider}.json"));
    let raw = std::fs::read_to_string(&path).ok()?;
    let doc = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    let url = doc.get("baseUrl")?.as_str()?.trim();
    if url.is_empty() {
        return None;
    }
    Some(url.to_string())
}
