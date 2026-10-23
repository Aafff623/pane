pub mod aihubmix;
pub mod amp;
pub mod antigravity;
pub mod apigoto;
pub mod apikey;
pub mod bedrock;
pub mod chromium;
pub mod chutes;
pub mod claude;
pub mod clawsgo;
pub mod clinepass;
pub mod codebuddy;
pub mod codebuff;
pub mod codex;
pub mod commandcode;
pub mod copilot;
pub mod cursor;
pub mod deepgram;
pub mod deepseek;
pub mod devin;
pub mod doubao;
pub mod elevenlabs;
pub mod factory;
pub mod giphy;
pub mod grok;
pub mod groq;
pub mod hermes;
pub mod huggingface;
pub mod jetbrains;
pub mod kilo;
pub mod kimi;
pub mod kiro;
pub mod longcat;
pub mod mimo;
pub mod minimax;
pub mod mistral;
pub mod moonshot;
pub mod novita;
pub mod ollama;
pub mod onenewapi;
pub mod openai_api;
pub mod opencode;
pub mod openrouter;
pub mod perplexity;
pub mod poe;
pub mod qoder;
pub mod qodercn;
pub mod qwen;
pub mod relaybalance;
pub mod searchquota;
pub mod sensenova;
pub mod shandianshuo;
pub mod siliconflow;
pub mod stepfun;
pub mod stepfun_plan;
pub mod sub2api;
pub mod tokenrhythm;
pub mod trae;
pub mod traecn;
pub mod venice;
pub mod vertexai;
pub mod volcengine;
pub mod warp;
pub mod windsurf;
pub mod workbuddy_local;
pub mod workbuddy_official_usage;
pub mod workbuddy_web;
pub mod xfyun;
pub mod zai;
pub mod zcode_weekend;
pub mod zed;

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
    /// Per-key availability for the search/MCP families, whose several
    /// api-keys flatten into rows on ONE snapshot: `Some(true)` when this
    /// round's read returned a value, `Some(false)` when it failed. `None`
    /// on headline/summary rows and on every other provider, so the UI can
    /// count keys without parsing row text (which carries both readings and
    /// failure messages).
    #[serde(default)]
    pub available: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<crate::usage_units::UsageReading>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_usage: Option<crate::usage_units::UsageDays>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pace: Option<crate::pace::Pace>,
}

impl Metric {
    pub fn progress(label: &str, used_percent: f64, detail: Option<String>) -> Self {
        // Unknown/invalid arithmetic must never masquerade as an empty quota
        // or reach JSON serialization as NaN/Infinity.
        if !used_percent.is_finite() {
            return Self::text(label, "Usage unavailable".into());
        }
        Self {
            label: label.into(),
            kind: "progress".into(),
            used_percent: Some(used_percent.clamp(0.0, 100.0)),
            detail,
            value: None,
            resets_at: None,
            period_ms: None,
            available: None,
            usage: None,
            daily_usage: None,
            pace: None,
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
            available: None,
            usage: None,
            daily_usage: None,
            pace: None,
        }
    }

    pub fn with_reset(mut self, resets_at: Option<i64>, period_ms: Option<i64>) -> Self {
        self.resets_at = resets_at;
        self.period_ms = period_ms;
        self
    }

    /// Marks a per-key row of a search/MCP family as available this round
    /// (see `available`). Only those families call this.
    pub fn with_available(mut self, available: bool) -> Self {
        self.available = Some(available);
        self
    }

    pub fn with_usage(mut self, unit: crate::usage_units::UsageUnit, used: f64, total: Option<f64>) -> Self {
        let reading = crate::usage_units::UsageReading { unit, used, total, scope: String::new() };
        if reading.valid() {
            self.usage = Some(reading);
        }
        self
    }

    pub fn with_usage_scope(mut self, scope: String) -> Self {
        if let Some(reading) = &mut self.usage {
            reading.scope = scope;
        }
        self
    }

    pub fn with_daily_usage(mut self, usage: crate::usage_units::UsageDays) -> Self {
        self.daily_usage = Some(usage);
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
    /// True when this snapshot came from the boot-time disk cache
    /// (cached_usage): the frontend renders a neutral "cached · refreshing"
    /// chip instead of the failure-driven ⚠ — a warm start is not a fault.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub boot_cache: bool,
    pub warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard_url: Option<String>,
    /// Earliest resource-pack expiry for the auth center's ordering. It is
    /// not rendered on the card; pack rows already carry their own text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_pack_expiry: Option<i64>,
}

impl Snapshot {
    pub fn ok(id: &str, name: &str, plan: Option<String>, mut metrics: Vec<Metric>) -> Self {
        crate::pace::attach(&mut metrics, chrono::Utc::now().timestamp_millis());
        Self {
            id: id.into(),
            name: name.into(),
            plan,
            status: "ok".into(),
            error: None,
            metrics,
            stale: false,
            boot_cache: false,
            warning: None,
            dashboard_url: None,
            auth_pack_expiry: None,
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
            boot_cache: false,
            warning: None,
            dashboard_url: None,
            auth_pack_expiry: None,
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
            boot_cache: false,
            warning: None,
            dashboard_url: None,
            auth_pack_expiry: None,
        }
    }

    /// Credentials ARE present and usable, but this provider's quota cannot
    /// be read without something more (a browser-session token, a password,
    /// an extra id). The card belongs on the board either way — the user
    /// asked for exactly this: never make a provider vanish because one
    /// additional step is missing; say which step it is.
    ///
    /// `hint` is the user-facing "what to provide, and where to get it";
    /// `plan` is whatever the provider can already state about the account
    /// (e.g. a wallet balance it CAN read).
    pub fn needs_setup(
        id: &str,
        name: &str,
        plan: Option<String>,
        metrics: Vec<Metric>,
        hint: &str,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            plan,
            status: "needs_setup".into(),
            error: Some(hint.into()),
            metrics,
            stale: false,
            boot_cache: false,
            warning: None,
            dashboard_url: None,
            auth_pack_expiry: None,
        }
    }
}

/// Optional outbound proxy from config.json `proxy: { enabled, url }`.
/// Re-read for each client lookup; loopback requests stay direct.
fn proxy_url() -> Option<String> {
    let cfg: serde_json::Value = std::fs::read_to_string(config_dir().join("config.json"))
        .ok().and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())?;
    let proxy = cfg.get("proxy")?;
    if !proxy.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(false) {
        return None;
    }
    let url = proxy.get("url")?.as_str()?.trim();
    ["http://", "https://", "socks5://"].iter()
        .any(|scheme| url.starts_with(scheme)).then(|| url.to_string())
}

/// The effective outbound proxy URL — the config's explicit choice first,
/// then the Windows system proxy the OS and browsers already honor (Clash
/// flips its port there when the user switches nodes) — or None for direct.
/// Shared by every provider client and the updater client so release
/// traffic follows the same chain; reqwest's own env-var handling stays as
/// the last resort when neither is set.
pub(crate) fn outbound_proxy_url() -> Option<String> {
    proxy_url().or_else(crate::platform::system_proxy)
}

fn http_builder(proxy_url: Option<&str>) -> reqwest::ClientBuilder {
    let mut builder = reqwest::Client::builder()
        .user_agent("Pane-Windows/0.3")
        .timeout(std::time::Duration::from_secs(20))
        // At boot the network is often still coming up; without a connect
        // cap every request rides the full 20 s, and the UI's first paint
        // waits on the slowest provider chain. Failing to connect in 5 s
        // is a dead network — fail fast, serve the cached snapshot.
        .connect_timeout(std::time::Duration::from_secs(5));
    if let Some(url) = proxy_url {
        if let Ok(proxy) = reqwest::Proxy::all(url) {
            let proxy = proxy.no_proxy(reqwest::NoProxy::from_string("localhost,127.0.0.1,::1"));
            builder = builder.proxy(proxy);
        }
    }
    builder
}

// Keep only the current pool per redirect policy. In-flight requests may
// finish on the previous client; subsequent requests see the new proxy.
#[derive(Default)]
struct HttpClientCache {
    current: Option<(Option<String>, reqwest::Client)>,
}

impl HttpClientCache {
    fn get(&mut self, proxy: Option<String>, no_redirect: bool) -> reqwest::Client {
        if let Some((key, client)) = &self.current {
            if key == &proxy { return client.clone(); }
        }
        let mut builder = http_builder(proxy.as_deref());
        if no_redirect { builder = builder.redirect(reqwest::redirect::Policy::none()); }
        let client = builder.build().expect("failed to build http client");
        self.current = Some((proxy, client.clone()));
        client
    }
}

pub fn http() -> reqwest::Client {
    static CACHE: std::sync::Mutex<HttpClientCache> = std::sync::Mutex::new(HttpClientCache { current: None });
    let proxy = outbound_proxy_url();
    CACHE.lock().unwrap_or_else(|e| e.into_inner()).get(proxy, false)
}

/// Same timeouts as [`http`] but no proxy at all — neither the config's nor
/// the environment's. The fallback for vendor endpoints that a machine's
/// env proxy breaks at the TLS layer while direct access works (seen with a
/// Clash port resetting auth.openai.com's handshake).
pub fn http_direct() -> reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT
        .get_or_init(|| {
            http_builder(None)
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
    static CACHE: std::sync::Mutex<HttpClientCache> = std::sync::Mutex::new(HttpClientCache { current: None });
    let proxy = outbound_proxy_url();
    CACHE.lock().unwrap_or_else(|e| e.into_inner()).get(proxy, true)
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

/// Balance-only APIs cannot establish consumed credits or a quota ceiling.
/// Preserve the actual balance without deriving a meter from local history.
pub fn balance_metric(
    sign: &str,
    balance: f64,
    label: &str,
    caption_suffix: &str,
) -> Option<Metric> {
    if !balance.is_finite() || balance < 0.0 {
        return None;
    }
    Some(Metric::text(
        label,
        format!("{sign}{balance:.2} left{caption_suffix}"),
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
    let disabled = cfg
        .get("disabled")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(id)));
    if disabled {
        return true;
    }
    cfg.get("removedProviders")
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
pub(crate) fn remove_stored_key_file_from(base: &std::path::Path, provider: &str) -> Result<(), String> {
    remove_stored_key_file_with(base, provider, || crate::secretstore::remove_checked(&crate::secretstore::provider_key(provider)))
}

fn remove_stored_key_file_with(base: &std::path::Path, provider: &str, remove_vault: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    // Relay identity includes baseUrl: keep that file if vault retirement fails.
    remove_vault()?;
    let path = base.join(format!("{provider}.json"));
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("retire legacy credential file: {error}")),
    }
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
    let (used, resets_at, period_ms, label) = (
        wall.used_percent,
        wall.resets_at,
        wall.period_ms,
        wall.label.clone(),
    );
    for m in snap.metrics.iter_mut() {
        if m.kind == "progress" && m.period_ms.is_some_and(|p| p < WALL_MIN_PERIOD_MS) {
            m.used_percent = Some(used.unwrap_or(100.0).max(100.0).min(100.0));
            m.resets_at = resets_at;
            m.period_ms = period_ms;
            m.detail = Some(exhaustion_detail(&label, resets_at));
        }
    }
}

/// The ⚠ threshold: the silent grace must have passed AND this is at least
/// the second consecutive failure — a single blip never warns (the cache
/// serves silently instead). Lives here so the parse-tests harness (which
/// does not compile lib.rs) can hold the decision logic.
pub fn should_mark_stale(age_past_grace: bool, consecutive_failures: u32) -> bool {
    age_past_grace && consecutive_failures >= 2
}

#[cfg(test)]
mod wall_tests {
    use super::*;

    #[tokio::test]
    async fn cached_clients_follow_proxy_changes_for_both_redirect_policies() {
        use std::io::{Read, Write};
        fn proxy(body: &'static str) -> (String, std::thread::JoinHandle<()>) {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            listener.set_nonblocking(true).unwrap();
            let worker = std::thread::spawn(move || {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(std::time::Instant::now() < deadline, "client never used the proxy");
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        Err(error) => panic!("accept: {error}"),
                    }
                };
                stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
                let mut request = [0; 4096];
                let n = stream.read(&mut request).unwrap();
                assert!(String::from_utf8_lossy(&request[..n]).contains("http://pane-proxy-test.invalid/"));
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            (format!("http://{addr}"), worker)
        }
        for no_redirect in [false, true] {
            let (first, worker1) = proxy("first");
            let (second, worker2) = proxy("second");
            let mut cache = HttpClientCache::default();
            for (url, expected) in [(first, "first"), (second, "second")] {
                let client = cache.get(Some(url), no_redirect);
                let response = client.get("http://pane-proxy-test.invalid/").send().await.unwrap();
                assert_eq!(response.text().await.unwrap(), expected);
            }
            worker1.join().unwrap(); worker2.join().unwrap();
        }
    }

    #[test]
    fn balance_only_sources_never_fabricate_a_quota_from_local_history() {
        let row = balance_metric("$", 7.5, "Extra credits", "").unwrap();
        assert_eq!(row.kind, "text");
        assert_eq!(row.used_percent, None);
        assert_eq!(row.value.as_deref(), Some("$7.50 left"));
        assert!(balance_metric("$", f64::NAN, "Extra credits", "").is_none());
    }

    #[test]
    fn invalid_percent_cannot_serialize_as_a_valid_empty_quota() {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let row = Metric::progress("Quota", invalid, None);
            assert_eq!(row.kind, "text");
            assert_eq!(row.used_percent, None);
            assert!(serde_json::to_value(row).is_ok());
        }
        assert_eq!(
            Metric::progress("Quota", 120.0, None).used_percent,
            Some(100.0)
        );
    }

    const HOUR_MS: i64 = 3600_000;
    const DAY_MS: i64 = 24 * HOUR_MS;

    #[test]
    fn stale_warning_needs_grace_passed_and_two_consecutive_failures() {
        assert!(!should_mark_stale(false, 0));
        assert!(!should_mark_stale(false, 5));
        // One blip past the grace stays silent; the second consecutive
        // failure is the warning line.
        assert!(!should_mark_stale(true, 0));
        assert!(!should_mark_stale(true, 1));
        assert!(should_mark_stale(true, 2));
    }

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
        assert_eq!(
            crate::platform::parse_win_proxy("127.0.0.1:7897").as_deref(),
            Some("http://127.0.0.1:7897")
        );
        assert_eq!(
            crate::platform::parse_win_proxy("http=127.0.0.1:7892;https=127.0.0.1:7897").as_deref(),
            Some("http://127.0.0.1:7897")
        );
        assert_eq!(
            crate::platform::parse_win_proxy("http=1.2.3.4:8080").as_deref(),
            Some("http://1.2.3.4:8080")
        );
        assert_eq!(
            crate::platform::parse_win_proxy("https://1.2.3.4:8080").as_deref(),
            Some("https://1.2.3.4:8080")
        );
        assert_eq!(crate::platform::parse_win_proxy(""), None);
        assert_eq!(crate::platform::parse_win_proxy("http=;https="), None);
    }
}


#[cfg(test)]
mod legacy_key_retirement_tests {
    #[test]
    fn retirement_failure_keeps_the_relay_identity_file() {
        let base=std::env::temp_dir().join(format!("pane-retire-relay-{}",std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let path=base.join("relay-failure-regression.json");
        let metadata=r#"{"baseUrl":"https://relay.example.test"}"#;
        std::fs::write(&path,metadata).unwrap();
        assert!(super::remove_stored_key_file_with(&base,"relay-failure-regression",||Err("vault unavailable".into())).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(),metadata);
        let _=std::fs::remove_dir_all(base);
    }

    #[test]
    fn retired_main_key_cannot_resolve_from_the_vault_on_the_next_refresh() {
        let family = "retired-main-key-regression-unique";
        let base = std::env::temp_dir().join(format!("pane-retire-main-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        crate::secretstore::put(&crate::secretstore::provider_key(family), "synthetic-old-key").unwrap();
        super::remove_stored_key_file_from(&base, family).unwrap();
        let after = crate::secretstore::resolve_provider_key(family, &base.join(format!("{family}.json")));
        crate::secretstore::remove(&crate::secretstore::provider_key(family));
        assert!(after.is_none(), "retired legacy key was re-imported from its surviving vault slot");
        let _ = std::fs::remove_dir_all(base);
    }
}
