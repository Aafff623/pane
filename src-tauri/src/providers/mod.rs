pub mod aihubmix;
pub mod antigravity;
pub mod claude;
pub mod clawsgo;
pub mod codebuff;
pub mod codex;
pub mod commandcode;
pub mod copilot;
pub mod cursor;
pub mod deepseek;
pub mod devin;
pub mod doubao;
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
pub mod qodercn;
pub mod qwen;
pub mod shandianshuo;
pub mod searchquota;
pub mod clinepass;
pub mod sensenova;
pub mod apigoto;
pub mod relaybalance;
pub mod siliconflow;
pub mod stepfun;
pub mod stepfun_plan;
pub mod traecn;
pub mod zai;
pub mod amp;
pub mod bedrock;
pub mod chutes;
pub mod deepgram;
pub mod openai_api;
pub mod poe;
pub mod venice;
pub mod vertexai;
pub mod warp;
pub mod kiro;
pub mod mimo;
pub mod trae;
pub mod qoder;
pub mod zed;
pub mod factory;
pub mod jetbrains;
pub mod groq;
pub mod huggingface;
pub mod longcat;
pub mod sub2api;
pub mod mistral;
pub mod perplexity;
pub mod volcengine;

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

/// "127.0.0.1:7897" or "http=127.0.0.1:7897;https=127.0.0.1:7897" → one URL.
/// https wins when both are listed (every vendor we talk to is https).
fn parse_win_proxy(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let pick = |scheme: &str| {
        raw.split(';')
            .filter_map(|part| part.trim().split_once('='))
            .find(|(key, _)| key.trim().eq_ignore_ascii_case(scheme))
            .map(|(_, value)| value.trim().to_string())
            .filter(|value| !value.is_empty())
    };
    let host = pick("https")
        .or_else(|| pick("http"))
        .or_else(|| (!raw.contains('=')).then(|| raw.to_string()))?;
    if host.is_empty() {
        return None;
    }
    Some(if host.contains("://") { host } else { format!("http://{host}") })
}

/// The Windows system proxy (HKCU → Internet Settings), which Clash-style
/// tools flip whenever the user switches nodes or ports — following it keeps
/// Pane in step with the OS without a config edit. PAC scripts are ignored
/// (reqwest cannot execute them). Read once per app run, like the config
/// proxy: a port change needs an app restart, not a config change.
#[cfg(windows)]
fn windows_system_proxy() -> Option<String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
    };
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let subkey = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings");
    unsafe {
        let mut enabled: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let name = wide("ProxyEnable");
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(subkey.as_ptr()),
            windows::core::PCWSTR(name.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut enabled as *mut u32 as *mut std::ffi::c_void),
            Some(&mut size),
        );
        if status != ERROR_SUCCESS || enabled == 0 {
            return None;
        }
        let mut buf = [0u16; 512];
        let mut size = (buf.len() * 2) as u32;
        let name = wide("ProxyServer");
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(subkey.as_ptr()),
            windows::core::PCWSTR(name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut std::ffi::c_void),
            Some(&mut size),
        );
        if status != ERROR_SUCCESS {
            return None;
        }
        let chars = (size as usize / 2).saturating_sub(1); // drop the NUL
        let raw = String::from_utf16_lossy(&buf[..chars.min(buf.len())]);
        parse_win_proxy(&raw)
    }
}

#[cfg(not(windows))]
fn windows_system_proxy() -> Option<String> {
    None
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
    // Proxy order: the config's explicit choice first, then the Windows
    // system proxy the OS and browsers already honor (Clash flips its port
    // there when the user switches nodes). reqwest's own env-var handling
    // stays as the last resort when neither is set.
    if let Some(url) = proxy_url().map(str::to_string).or_else(windows_system_proxy) {
        if let Ok(proxy) = reqwest::Proxy::all(url) {
            let proxy = proxy.no_proxy(reqwest::NoProxy::from_string("localhost,127.0.0.1,::1"));
            builder = builder.proxy(proxy);
        }
    }
    builder
}

pub fn http() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| http_builder().build().expect("failed to build http client"))
        .clone()
}

/// Same timeouts as [`http`] but no proxy at all — neither the config's nor
/// the environment's. The fallback for vendor endpoints that a machine's
/// env proxy breaks at the TLS layer while direct access works (seen with a
/// Clash port resetting auth.openai.com's handshake).
pub fn http_direct() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| {
            http_builder()
                .no_proxy()
                .build()
                .expect("failed to build direct http client")
        })
        .clone()
}

/// One error plus its source chain, joined — reqwest's Display stops at
/// "error sending request" and hides the real cause (DNS, TLS, reset).
pub fn error_chain(err: &dyn std::error::Error) -> String {
    let mut parts = vec![err.to_string()];
    let mut cursor = err.source();
    while let Some(next) = cursor {
        let text = next.to_string();
        if !parts.contains(&text) {
            parts.push(text);
        }
        cursor = next.source();
    }
    parts.join(" ← ")
}

/// Send a request with a small transport ladder: the proxy-respecting client
/// twice (TLS through an unstable local proxy is the common flake), then a
/// direct connection. A 403 from the first two attempts also moves on — edge
/// blocks track the transport — but a DIRECT 403 is returned as-is: it is
/// usually a final answer (e.g. OpenAI's unsupported-country block, which
/// retrying can never fix).
pub async fn send_with_direct_fallback(
    build: impl Fn(&reqwest::Client) -> reqwest::RequestBuilder,
) -> Result<reqwest::Response, String> {
    let mut last = String::new();
    for attempt in 0..3 {
        let client = if attempt < 2 { http() } else { http_direct() };
        match build(&client).send().await {
            Ok(resp) => {
                if resp.status().as_u16() == 403 && attempt < 2 {
                    last = "HTTP 403 over the proxy path".into();
                    continue;
                }
                return Ok(resp);
            }
            Err(err) => last = error_chain(&err),
        }
    }
    Err(format!("request failed after retries ({last})"))
}

/// Same client as [`http`] but never follows redirects. One/New API status
/// and billing calls must not be bounced onto another origin.
pub fn http_no_redirect() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| {
            http_builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("failed to build http client")
        })
        .clone()
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
    read_small_bytes(path, max_bytes, what).and_then(|bytes| {
        String::from_utf8(bytes).map_err(|_| format!("read {what}: not valid UTF-8 text"))
    })
}

/// Binary twin of [`read_small_text`] for credential blobs that are not
/// text at all (Qoder CN's os_crypt auth.v1.dat) — same symlink and size
/// guards, raw bytes back.
pub(crate) fn read_small_bytes(
    path: &std::path::Path,
    max_bytes: u64,
    what: &str,
) -> Result<Vec<u8>, String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("read {what}: {e}"))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(format!("{what} is not a regular file"));
    }
    if meta.len() > max_bytes {
        return Err(format!("{what} is unexpectedly large — not reading it"));
    }
    std::fs::read(path).map_err(|e| format!("read {what}: {e}"))
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

/// A secret out of the OS credential store — Windows Credential Manager,
/// the macOS keychain, or the freedesktop Secret Service. `target` is the
/// service name Go's keyring library writes under, which is what the CLIs
/// Pane reads (gh, Antigravity) use on every platform.
pub fn credential_string(target: &str) -> Option<String> {
    crate::platform::secret(target)
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

/// API key lookup: the OS credential vault first (lazily migrating the
/// legacy plaintext file on first sight), then environment variables.
pub fn stored_api_key(provider: &str, env_vars: &[&str]) -> Option<String> {
    let path = config_dir().join(format!("{provider}.json"));
    if let Some(key) = crate::secretstore::resolve_provider_key(provider, &path) {
        return Some(key);
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

/// Pure "is a key saved" probe (no env fallback): the OS credential vault
/// first, then the plaintext fallback file. Used by get_credential_status
/// to report "stored key" separately from the env-var fallback.
pub fn stored_key_file(provider: &str) -> Option<String> {
    let path = config_dir().join(format!("{provider}.json"));
    crate::secretstore::resolve_provider_key(provider, &path)
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

/// Drops %APPDATA%\Pane\<provider>.json — used by the default-account
/// migration after the key is imported into accounts/<provider>.json, so
/// deleting the imported account actually deletes the key (no zombie file
/// that would re-import it on the next fetch).
pub fn remove_stored_key_file(provider: &str) {
    let path = config_dir().join(format!("{provider}.json"));
    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// Long-window wall mirroring
// ---------------------------------------------------------------------------

/// Families where a long-window (weekly/monthly) cap is a hard wall: once it
/// is exhausted, requests are rejected no matter how much short-window
/// budget remains. Families whose long cap has escape hatches (Codex's
/// banked reset credits, pools that fail over) must NOT be listed here —
/// the mirror would show "dead" while the account can still answer.
/// Extend one family at a time, after its wall behavior is confirmed live.
pub const WEEK_WALL_MIRROR_FAMILIES: &[&str] = &["kimi"];

/// Shortest window that counts as a "long wall" (6 days: weekly and monthly
/// qualify, 5-hour sessions never do).
const WALL_MIN_PERIOD_MS: i64 = 6 * 24 * 3600_000;

/// "{label} limit reached · resets in {human}" — plain-words explanation for
/// an exhausted window, shown under the meter instead of a bare red ring.
pub fn exhaustion_detail(label: &str, resets_at: Option<i64>) -> String {
    let Some(ms) = resets_at else {
        return format!("{label} limit reached");
    };
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mins = (ms - now_ms).max(0) / 60_000;
    let human = if mins >= 1440 {
        format!("{}d {}h", mins / 1440, (mins % 1440) / 60)
    } else if mins >= 60 {
        format!("{}h {}m", mins / 60, mins % 60)
    } else {
        format!("{mins}m")
    };
    format!("{label} limit reached · resets in {human}")
}

/// While a long window's wall stands, a still-green session meter is a
/// mirage — the card would read "32% used, all fine" for an account that
/// cannot answer anything until the wall lifts. Mirror the wall onto the
/// session meter: 100%, the wall's own reset instant and period, and a
/// detail naming it. Applies after parsing, on every fetch, so the card,
/// the overview rings and the local API all agree.
pub fn mirror_week_wall(card_id: &str, snap: &mut Snapshot) {
    let family = card_id.split('@').next().unwrap_or(card_id);
    if !WEEK_WALL_MIRROR_FAMILIES.contains(&family) || snap.status != "ok" {
        return;
    }
    // The hardest standing wall wins: the longest-period exhausted window.
    let wall = snap
        .metrics
        .iter()
        .filter(|m| {
            m.kind == "progress"
                && m.used_percent.is_some_and(|u| u >= 99.5)
                && m.period_ms.is_some_and(|p| p >= WALL_MIN_PERIOD_MS)
        })
        .max_by_key(|m| m.period_ms.unwrap_or(0));
    let Some(wall) = wall else { return };
    let (used, resets_at, period_ms, label) =
        (wall.used_percent, wall.resets_at, wall.period_ms, wall.label.clone());
    for m in snap.metrics.iter_mut() {
        if m.kind == "progress" && m.period_ms.is_some_and(|p| p < WALL_MIN_PERIOD_MS) {
            m.used_percent = Some(used.unwrap_or(100.0).max(100.0).min(100.0));
            m.resets_at = resets_at;
            m.period_ms = period_ms;
            m.detail = Some(exhaustion_detail(&label, resets_at));
        }
    }
}

#[cfg(test)]
mod wall_tests {
    use super::*;

    const HOUR_MS: i64 = 3600_000;
    const DAY_MS: i64 = 24 * HOUR_MS;

    fn snap_with(session_used: f64, weekly_used: f64) -> Snapshot {
        let resets = chrono::Utc::now().timestamp_millis() + 4 * DAY_MS;
        Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![
                Metric::progress("Session", session_used, None)
                    .with_reset(Some(resets - 3 * DAY_MS), Some(5 * HOUR_MS)),
                Metric::progress("Weekly", weekly_used, None)
                    .with_reset(Some(resets), Some(7 * DAY_MS)),
            ],
        )
    }

    #[test]
    fn standing_weekly_wall_mirrors_onto_short_windows() {
        let mut snap = snap_with(32.0, 100.0);
        let resets = snap.metrics[1].resets_at;
        mirror_week_wall("kimi", &mut snap);
        let session = &snap.metrics[0];
        assert_eq!(session.used_percent, Some(100.0));
        assert_eq!(session.resets_at, resets);
        assert_eq!(session.period_ms, Some(7 * DAY_MS));
        let detail = session.detail.as_deref().expect("mirror detail");
        assert!(detail.starts_with("Weekly limit reached"), "{detail}");
    }

    #[test]
    fn healthy_weekly_leaves_short_windows_alone() {
        let mut snap = snap_with(32.0, 10.4);
        mirror_week_wall("kimi", &mut snap);
        assert!((snap.metrics[0].used_percent.unwrap() - 32.0).abs() < 0.01);
        assert!(snap.metrics[0].detail.is_none());
    }

    #[test]
    fn families_outside_the_list_are_untouched() {
        // Codex is deliberately excluded: banked reset credits keep the
        // account usable past the weekly line, so the mirror would lie.
        let mut snap = snap_with(32.0, 100.0);
        snap.id = "codex".into();
        mirror_week_wall("codex", &mut snap);
        assert!((snap.metrics[0].used_percent.unwrap() - 32.0).abs() < 0.01);
    }

    #[test]
    fn parallel_account_cards_mirror_too() {
        let mut snap = snap_with(32.0, 100.0);
        mirror_week_wall("kimi@deadbeef", &mut snap);
        assert_eq!(snap.metrics[0].used_percent, Some(100.0));
    }

    #[test]
    fn error_chain_walks_the_source_chain() {
        use std::error::Error;
        #[derive(Debug)]
        struct Outer(std::io::Error);
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "request failed")
            }
        }
        impl Error for Outer {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                Some(&self.0)
            }
        }
        let err = Outer(std::io::Error::other("root cause: connection reset"));
        let text = error_chain(&err);
        assert!(text.starts_with("request failed"), "{text}");
        assert!(text.contains("root cause: connection reset"), "{text}");
    }

    #[test]
    fn windows_proxy_string_parses_all_the_shapes() {
        assert_eq!(parse_win_proxy("127.0.0.1:7897").as_deref(), Some("http://127.0.0.1:7897"));
        assert_eq!(
            parse_win_proxy("http=127.0.0.1:7892;https=127.0.0.1:7897").as_deref(),
            Some("http://127.0.0.1:7897")
        );
        assert_eq!(parse_win_proxy("http=1.2.3.4:8080").as_deref(), Some("http://1.2.3.4:8080"));
        assert_eq!(
            parse_win_proxy("https://1.2.3.4:8080").as_deref(),
            Some("https://1.2.3.4:8080")
        );
        assert_eq!(parse_win_proxy(""), None);
        assert_eq!(parse_win_proxy("http=;https="), None);
    }
}
