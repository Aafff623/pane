//! Local read-only HTTP API, Mac parity: GET http://127.0.0.1:6736/v1/usage
//! returns the latest snapshots in the original app's documented wire format
//! (docs/local-http-api.md), so scripts written for the Mac app work here too.
//!
//! Since T-0032 four read-only diagnostic endpoints share the same posture
//! (loopback bind, loopback-only Host, no CORS, no auth, never credentials):
//! `/v1/status` (runtime facts), `/v1/providers` (full snapshots incl. the
//! status/error/stale/warning the Mac wire format drops), `/v1/doctor`
//! (checks with remediation) and `/v1/sources` (spend-source presence).
//! They exist so a user's own coding agent can triage Pane without waiting
//! for a release. Write actions stay out of scope on purpose: anything that
//! mutates state would need a bearer token first (handoff §4).

use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};

use crate::providers::Snapshot;

static LATEST: OnceLock<Mutex<Value>> = OnceLock::new();
/// Full snapshots for `/v1/providers` — the Mac-compatible wire format above
/// deliberately keeps only usage numbers, which leaves an agent unable to
/// tell "no data" from "disabled / no credentials / auth failed / network".
static LATEST_FULL: OnceLock<Mutex<Vec<Snapshot>>> = OnceLock::new();
/// When the refresh loop last published (epoch ms; 0 = never this session).
static LAST_PUBLISH_MS: OnceLock<Mutex<i64>> = OnceLock::new();
static STARTED_MS: OnceLock<i64> = OnceLock::new();

fn latest() -> &'static Mutex<Value> {
    LATEST.get_or_init(|| Mutex::new(Value::Array(vec![])))
}

fn latest_full() -> &'static Mutex<Vec<Snapshot>> {
    LATEST_FULL.get_or_init(|| Mutex::new(Vec::new()))
}

fn last_publish_ms() -> &'static Mutex<i64> {
    LAST_PUBLISH_MS.get_or_init(|| Mutex::new(0))
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn started_ms() -> i64 {
    *STARTED_MS.get_or_init(now_ms)
}

/// Called after each usage fetch with the enabled providers' snapshots.
pub fn publish(snapshots: &[Snapshot]) {
    let fetched_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let arr: Vec<Value> = snapshots
        .iter()
        .map(|s| provider_json(s, &fetched_at))
        .collect();
    if let Ok(mut v) = latest().lock() {
        *v = Value::Array(arr);
    }
    if let Ok(mut full) = latest_full().lock() {
        *full = snapshots.to_vec();
    }
    if let Ok(mut t) = last_publish_ms().lock() {
        *t = now_ms();
    }
}

pub(crate) fn provider_json(s: &Snapshot, fetched_at: &str) -> Value {
    json!({
        "providerId": s.id,
        "displayName": s.name,
        "plan": s.plan,
        "lines": metric_lines(&s.metrics),
        "fetchedAt": fetched_at,
    })
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn iso8601(epoch_ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(epoch_ms)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}

/// Only loopback spellings may appear in the Host header. A missing header
/// is allowed (HTTP/1.0 scripts); a rebound hostname is not.
fn host_ok(host: Option<&str>) -> bool {
    let Some(host) = host else { return true };
    let host = host.trim().to_ascii_lowercase();
    let bare = host.strip_suffix(":6736").unwrap_or(&host);
    matches!(bare, "127.0.0.1" | "localhost" | "[::1]")
}

// ── diagnostic endpoints (T-0032) ───────────────────────────────────────────

/// Strip URLs out of provider-supplied text. reqwest errors embed the whole
/// request URL (`... error sending request for url (https://host/path)`),
/// which can carry a private relay hostname or a tenant path — the same
/// reason the usage wire format omits dashboard/origin. Everything else in
/// the message survives, so the diagnosis stays useful.
pub(crate) fn sanitize_text(text: &str) -> String {
    const MARK: &str = "<endpoint>";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let at = ["http://", "https://"]
            .iter()
            .filter_map(|p| rest.find(p))
            .min();
        let Some(at) = at else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '"' | '\'' | ',' | ']' | '}'))
            .unwrap_or(tail.len());
        out.push_str(MARK);
        rest = &tail[end..];
    }
}

fn read_config() -> Value {
    std::fs::read_to_string(crate::providers::config_dir().join("config.json"))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn config_id_list(cfg: &Value, key: &str) -> Vec<String> {
    cfg.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn refresh_minutes(cfg: &Value) -> u64 {
    cfg.get("refreshMinutes")
        .and_then(Value::as_u64)
        .filter(|m| *m > 0)
        .unwrap_or(5)
}

fn metric_lines(metrics: &[crate::providers::Metric]) -> Vec<Value> {
    metrics
        .iter()
        .map(|m| {
            if m.kind == "progress" {
                json!({
                    "type": "progress",
                    "label": m.label,
                    "used": m.used_percent,
                    "limit": 100,
                    "format": { "kind": "percent" },
                    "resetsAt": m.resets_at.map(iso8601),
                    "periodDurationMs": m.period_ms,
                    "color": Value::Null,
                })
            } else {
                json!({
                    "type": "text",
                    "label": m.label,
                    "value": m.value,
                    "subtitle": m.detail,
                    "resetsAt": m.resets_at.map(iso8601),
                    "color": Value::Null,
                })
            }
        })
        .collect()
}

/// One provider row for `/v1/providers` — the usage wire format plus the
/// diagnostic fields, with every provider-supplied string URL-stripped.
pub(crate) fn provider_full_json(s: &Snapshot, fetched_at: &str) -> Value {
    json!({
        "providerId": s.id,
        "displayName": s.name,
        "plan": s.plan,
        "status": s.status,
        "stale": s.stale,
        "error": s.error.as_deref().map(sanitize_text),
        "warning": s.warning.as_deref().map(sanitize_text),
        "lines": metric_lines(&s.metrics),
        "fetchedAt": fetched_at,
    })
}

fn providers_payload(cfg: &Value, fetched_at: &str) -> Value {
    let (snaps, published_at) = match latest_full().lock() {
        Ok(v) => (v.clone(), *last_publish_ms().lock().unwrap_or_else(|e| e.into_inner())),
        Err(_) => (Vec::new(), 0),
    };
    providers_payload_from(&snaps, published_at, cfg, fetched_at)
}

/// Pure core of [`providers_payload`] — tests drive this directly so the
/// process-wide snapshot state cannot race between parallel cases.
pub(crate) fn providers_payload_from(
    snaps: &[Snapshot],
    published_at: i64,
    cfg: &Value,
    fetched_at: &str,
) -> Value {
    let providers: Vec<Value> = snaps.iter().map(|s| provider_full_json(s, fetched_at)).collect();
    let disabled = config_id_list(cfg, "disabled");
    let removed = config_id_list(cfg, "removedProviders");
    let by_status = |want: &str| snaps.iter().filter(|s| s.status == want).count();
    json!({
        "providers": providers,
        "disabled": disabled,
        "removed": removed,
        "counts": {
            "active": snaps.len(),
            "disabled": disabled.len(),
            "removed": removed.len(),
            "ok": by_status("ok"),
            "error": by_status("error"),
            "no_credentials": by_status("no_credentials"),
            "needs_setup": by_status("needs_setup"),
            "stale": snaps.iter().filter(|s| s.stale).count(),
        },
        "lastRefreshAt": (published_at > 0).then(|| iso8601(published_at)),
    })
}

/// `/v1/status` — runtime facts about this instance. No credentials, no
/// provider endpoints; the proxy is reported as a source, never an address.
fn status_payload(cfg: &Value, fetched_at: &str) -> Value {
    let (snaps, published_at) = match latest_full().lock() {
        Ok(v) => (v.clone(), *last_publish_ms().lock().unwrap_or_else(|e| e.into_inner())),
        Err(_) => (Vec::new(), 0),
    };
    let minutes = refresh_minutes(cfg);
    let age_secs = (published_at > 0).then(|| (now_ms() - published_at).max(0) / 1000);
    let proxy_source = if cfg.get("proxy").and_then(|p| p.get("url")).and_then(Value::as_str).is_some_and(|u| !u.trim().is_empty()) {
        "config"
    } else if crate::providers::outbound_proxy_url().is_some() {
        "system"
    } else {
        "none"
    };
    json!({
        "app": "pane",
        "version": env!("CARGO_PKG_VERSION"),
        "api": { "port": 6736, "bound": true },
        "startedAt": iso8601(started_ms()),
        "uptimeSecs": (now_ms() - started_ms()).max(0) / 1000,
        "refresh": {
            "minutes": minutes,
            "lastRefreshAt": (published_at > 0).then(|| iso8601(published_at)),
            "ageSecs": age_secs,
        },
        "providers": {
            "active": snaps.len(),
            "disabled": config_id_list(cfg, "disabled").len(),
            "removed": config_id_list(cfg, "removedProviders").len(),
            "ok": snaps.iter().filter(|s| s.status == "ok").count(),
            "error": snaps.iter().filter(|s| s.status == "error").count(),
            "no_credentials": snaps.iter().filter(|s| s.status == "no_credentials").count(),
            "needs_setup": snaps.iter().filter(|s| s.status == "needs_setup").count(),
            "stale": snaps.iter().filter(|s| s.stale).count(),
        },
        "config": {
            "autostart": cfg.get("autostart").and_then(Value::as_bool),
            "telemetry": cfg.get("telemetry").and_then(Value::as_bool),
            "proxy": { "configured": proxy_source != "none", "source": proxy_source },
        },
        "generatedAt": fetched_at,
    })
}

fn check(name: &str, ok: bool, classification: &str, detail: String, remediation: Option<&str>) -> Value {
    json!({
        "name": name,
        "ok": ok,
        "classification": classification,
        "detail": detail,
        "remediation": remediation,
    })
}

/// `/v1/doctor` — checks modelled on REA's DoctorCheck: every failure ships
/// a remediation. Only facts this process can actually establish become
/// ok/fail; anything not decidable in-process is `info` with the honest
/// reason, never a guessed verdict.
fn doctor_payload(cfg: &Value, fetched_at: &str) -> Value {
    let (snaps, published_at) = match latest_full().lock() {
        Ok(v) => (v.clone(), *last_publish_ms().lock().unwrap_or_else(|e| e.into_inner())),
        Err(_) => (Vec::new(), 0),
    };
    let minutes = refresh_minutes(cfg);
    let mut checks: Vec<Value> = Vec::new();

    checks.push(check(
        "api_bound",
        true,
        "healthy",
        format!("read-only API answering on 127.0.0.1:6736 (v{})", env!("CARGO_PKG_VERSION")),
        None,
    ));

    // Refresh loop: published at least once and recent relative to the
    // configured interval (with slack for a slow network round).
    if published_at == 0 {
        checks.push(check(
            "refresh_loop_alive",
            false,
            "failed",
            "no usage snapshot has been published this session".into(),
            Some("open the Pane panel once; if the card list stays empty, restart Pane and check the tray icon is present"),
        ));
    } else {
        let age = (now_ms() - published_at).max(0) / 1000;
        let budget = minutes as i64 * 90 + 120;
        let ok = age <= budget;
        checks.push(check(
            "refresh_loop_alive",
            ok,
            if ok { "healthy" } else { "failed" },
            format!("last refresh {age}s ago (interval {minutes} min)"),
            (!ok).then_some("the refresh loop looks stuck — restart Pane; if it recurs, report with this output"),
        ));
    }

    // Snapshot presence + per-status rollups.
    let active = snaps.len();
    checks.push(check(
        "providers_enabled",
        active > 0,
        if active > 0 { "healthy" } else { "failed" },
        format!("{active} provider card(s) in the last refresh"),
        (active == 0).then_some("enable at least one provider in Pane → Customize, or check config.json's `disabled` list"),
    ));

    let errored: Vec<&str> = snaps.iter().filter(|s| s.status == "error").map(|s| s.id.as_str()).collect();
    checks.push(check(
        "provider_errors",
        errored.is_empty(),
        if errored.is_empty() { "healthy" } else { "failed" },
        if errored.is_empty() {
            "no provider is reporting an error".into()
        } else {
            format!("error on: {}", errored.join(", "))
        },
        (!errored.is_empty()).then_some("see /v1/providers for each card's error text and route it by provider (auth expiry vs endpoint vs network)"),
    ));

    let stale: Vec<&str> = snaps.iter().filter(|s| s.stale).map(|s| s.id.as_str()).collect();
    checks.push(check(
        "provider_stale",
        stale.is_empty(),
        if stale.is_empty() { "healthy" } else { "failed" },
        if stale.is_empty() {
            "no card is serving a stale fallback".into()
        } else {
            format!("stale (last good fetch shown): {}", stale.join(", "))
        },
        (!stale.is_empty()).then_some("the newest fetch failed transiently — check the card's warning in /v1/providers"),
    ));

    let no_creds: Vec<&str> = snaps
        .iter()
        .filter(|s| s.status == "no_credentials")
        .map(|s| s.id.as_str())
        .collect();
    checks.push(check(
        "providers_without_credentials",
        no_creds.is_empty(),
        "info",
        if no_creds.is_empty() {
            "every enabled card has credentials".into()
        } else {
            format!("not configured yet: {}", no_creds.join(", "))
        },
        (!no_creds.is_empty()).then_some("add the key/account for these providers if you expect data (this is not a fault)"),
    ));

    // Proxy config validity — malformed values are silently skipped by the
    // HTTP clients, which is exactly the kind of quiet failure worth naming.
    let proxy_raw = cfg
        .get("proxy")
        .and_then(|p| p.get("url"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let proxy_ok = proxy_raw.trim().is_empty() || reqwest::Url::parse(proxy_raw.trim()).is_ok();
    checks.push(check(
        "proxy_config_valid",
        proxy_ok,
        if proxy_ok { "healthy" } else { "failed" },
        if proxy_raw.trim().is_empty() {
            "no explicit proxy configured (system proxy / env may still apply)".into()
        } else {
            format!("configured proxy parses: {proxy_ok}")
        },
        (!proxy_ok).then_some("fix proxy.url in Pane → Network (a malformed URL is silently ignored, so requests go direct)"),
    ));

    let sources = crate::spend::source_statuses();
    let detected = sources.iter().filter(|s| s.state == "detected").count();
    checks.push(check(
        "spend_sources_detected",
        detected > 0,
        if detected > 0 { "healthy" } else { "failed" },
        format!("{detected}/{} local spend source(s) present", sources.len()),
        (detected == 0).then_some("no supported CLI logs found on this machine — the spend panel will read 0; see /v1/sources for the full list"),
    ));

    // Not decidable from inside this process — report the fact, not a guess.
    checks.push(check(
        "webview_cache",
        true,
        "info",
        "WebView2 cache is per-user under %LOCALAPPDATA%\\com.jazii.pane\\EBWebView; staleness cannot be judged from here".into(),
        Some("if the UI shows stale content after an update, delete that folder and restart Pane"),
    ));
    checks.push(check(
        "single_instance_guard",
        true,
        "info",
        "second launches are intercepted by the single-instance plugin; this process cannot verify its own exclusivity".into(),
        None,
    ));

    let failed = checks
        .iter()
        .filter(|c| c.get("classification").and_then(Value::as_str) == Some("failed"))
        .count();
    json!({
        "app": "pane",
        "version": env!("CARGO_PKG_VERSION"),
        "healthy": failed == 0,
        "failed": failed,
        "checks": checks,
        "generatedAt": fetched_at,
    })
}

fn sources_payload() -> Value {
    let sources = crate::spend::source_statuses();
    let detected = sources.iter().filter(|s| s.state == "detected").count();
    json!({
        "sources": sources,
        "detected": detected,
        "total": sources.len(),
    })
}

fn route(method: &tiny_http::Method, url: &str) -> (u16, String) {
    let path = url.split('?').next().unwrap_or(url);
    match method {
        tiny_http::Method::Get => {
            if path == "/v1/usage" {
                let body = latest()
                    .lock()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| "[]".into());
                (200, body)
            } else if path == "/v1/providers" {
                (200, providers_payload(&read_config(), &now_iso()).to_string())
            } else if path == "/v1/status" {
                (200, status_payload(&read_config(), &now_iso()).to_string())
            } else if path == "/v1/doctor" {
                (200, doctor_payload(&read_config(), &now_iso()).to_string())
            } else if path == "/v1/sources" {
                (200, sources_payload().to_string())
            } else if let Some(id) = path.strip_prefix("/v1/usage/") {
                match latest().lock() {
                    Ok(v) => v
                        .as_array()
                        .and_then(|a| {
                            a.iter()
                                .find(|p| p.get("providerId").and_then(Value::as_str) == Some(id))
                        })
                        .map(|p| (200, p.to_string()))
                        .unwrap_or((404, json!({"error": "provider_not_found"}).to_string())),
                    Err(_) => (503, json!({"error": "server_busy"}).to_string()),
                }
            } else {
                (404, json!({"error": "not_found"}).to_string())
            }
        }
        _ => (405, json!({"error": "method_not_allowed"}).to_string()),
    }
}

/// Binds 127.0.0.1:6736 and serves until the app exits. If the port is
/// taken the API is silently unavailable this session (Mac parity).
pub fn start() {
    std::thread::spawn(|| {
        let server = match tiny_http::Server::http("127.0.0.1:6736") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[pane] local API: port 6736 unavailable ({e}) — API off");
                return;
            }
        };
        eprintln!("[pane] local API: http://127.0.0.1:6736/v1/usage");
        for request in server.incoming_requests() {
            // DNS-rebinding guard: a page can point its own hostname at
            // 127.0.0.1, making this server "same-origin" in the victim's
            // browser — CORS never applies then. Legitimate local clients
            // address us by loopback names only; anything else is refused.
            let host = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Host"))
                .map(|h| h.value.as_str().to_string());
            let (status, body) = if host_ok(host.as_deref()) {
                route(request.method(), request.url())
            } else {
                (403, json!({"error": "forbidden_host"}).to_string())
            };
            let mut response = tiny_http::Response::from_string(body).with_status_code(status);
            // Deliberately NO Access-Control-Allow-Origin header: with
            // permissive CORS, any website the user visits could silently
            // read their usage data from this port. Browsers now block
            // cross-origin reads; scripts, widgets, and curl are unaffected
            // (CORS only constrains browsers). The Mac app allows "*" and
            // discloses it — we chose the stricter default.
            for (k, v) in [("Content-Type", "application/json")] {
                if let Ok(h) = tiny_http::Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                    response.add_header(h);
                }
            }
            let _ = request.respond(response);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{host_ok, provider_json, publish, route};
    use crate::providers::{Metric, Snapshot};
    use serde_json::json;

    fn onenewapi_snap() -> Snapshot {
        let mut snap = Snapshot::ok(
            "onenewapi@abc",
            "Site · Key 1",
            None,
            vec![Metric::progress(
                "Usage",
                48.63,
                Some("$592.18 of $1217.82".into()),
            )],
        );
        snap.dashboard_url = Some("https://panel.example.com".into());
        snap
    }

    #[test]
    fn onenewapi_json_omits_origin_dashboard_and_secrets() {
        let snap = onenewapi_snap();
        let json = provider_json(&snap, "2026-07-26T00:00:00Z");
        assert_eq!(json["providerId"], "onenewapi@abc");
        assert_eq!(json["displayName"], "Site · Key 1");
        assert!(json.get("dashboardUrl").is_none());
        assert!(json.get("dashboard_url").is_none());
        assert!(json.get("baseUrl").is_none());
        assert!(json.get("origin").is_none());
        let raw = json.to_string();
        for leak in [
            "https://panel.example.com",
            "panel.example.com",
            "dashboard",
            "sk-",
            "apiKey",
            "api_key",
        ] {
            assert!(
                !raw.to_ascii_lowercase()
                    .contains(&leak.to_ascii_lowercase()),
                "local HTTP leaked {leak}: {raw}"
            );
        }
    }

    #[test]
    fn get_by_id_uses_full_snapshot_id() {
        publish(&[onenewapi_snap()]);
        let (status, body) = route(&tiny_http::Method::Get, "/v1/usage/onenewapi@abc");
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["providerId"], "onenewapi@abc");
        assert_eq!(v["displayName"], "Site · Key 1");
        assert!(!body.contains("https://panel.example.com"));
        assert!(!body.to_ascii_lowercase().contains("dashboard"));

        let (missing, _) = route(&tiny_http::Method::Get, "/v1/usage/onenewapi");
        assert_eq!(missing, 404);
    }

    #[test]
    fn host_header_must_be_loopback() {
        // Loopback spellings, with and without the port.
        for good in [
            "127.0.0.1:6736",
            "127.0.0.1",
            "localhost:6736",
            "LOCALHOST",
            "[::1]:6736",
        ] {
            assert!(host_ok(Some(good)), "{good} should be allowed");
        }
        // Absent header (HTTP/1.0 scripts) stays allowed.
        assert!(host_ok(None));
        // A rebound hostname resolving to 127.0.0.1 is refused — this is
        // the DNS-rebinding case CORS can't catch.
        for bad in [
            "evil.example:6736",
            "evil.example",
            "127.0.0.1.evil.example:6736",
            "localhost.evil.example",
        ] {
            assert!(!host_ok(Some(bad)), "{bad} should be refused");
        }
    }

    // ── T-0032 diagnostic endpoints ─────────────────────────────────────────

    #[test]
    fn sanitize_text_strips_urls_keeps_the_rest() {
        // The reqwest shape that motivated this: the URL carries the relay
        // host/query, everything else is the diagnosis.
        let raw = "error sending request for url (https://relay.example.com/v1/usage?key=abc): connection refused";
        let out = super::sanitize_text(raw);
        assert!(!out.contains("relay.example.com"));
        assert!(!out.contains("key=abc"));
        assert!(out.contains("<endpoint>"));
        assert!(out.contains("connection refused"));
        // Plain text passes through untouched.
        assert_eq!(
            super::sanitize_text("token expired, run claude again"),
            "token expired, run claude again"
        );
        // Multiple URLs; surrounding punctuation survives.
        assert_eq!(
            super::sanitize_text("try http://a.test/x and https://b.test/y, then stop"),
            "try <endpoint> and <endpoint>, then stop"
        );
    }

    #[test]
    fn providers_payload_keeps_diagnostics_and_drops_endpoints() {
        let mut snap = Snapshot::error(
            "claude",
            "Claude",
            "token refresh failed: error sending request for url (https://private.relay.example/token)".into(),
        );
        snap.stale = true;
        snap.warning = Some("last good fetch shown; https://private.relay.example/v1 failed".into());
        snap.dashboard_url = Some("https://panel.example.com".into());
        let cfg = json!({ "disabled": ["kimi"], "removedProviders": ["linkso"] });
        // Pure core, no global state: parallel cases cannot race here.
        let payload =
            super::providers_payload_from(&[snap], 1_700_000_000_000, &cfg, "2026-07-26T00:00:00Z");
        let card = &payload["providers"][0];
        assert_eq!(card["providerId"], "claude");
        assert_eq!(card["status"], "error");
        assert_eq!(card["stale"], true);
        assert!(card["error"].as_str().unwrap().contains("<endpoint>"));
        assert!(card["warning"].as_str().unwrap().contains("<endpoint>"));
        assert_eq!(payload["disabled"][0], "kimi");
        assert_eq!(payload["removed"][0], "linkso");
        assert_eq!(payload["counts"]["error"], 1);
        assert_eq!(payload["counts"]["stale"], 1);
        let raw = payload.to_string().to_ascii_lowercase();
        for leak in [
            "private.relay.example",
            "panel.example.com",
            "dashboard",
            "apikey",
            "api_key",
            "sk-",
        ] {
            assert!(!raw.contains(leak), "leaked {leak}: {raw}");
        }
    }

    #[test]
    fn doctor_payload_is_wellformed_and_failures_carry_remediation() {
        super::publish(&[onenewapi_snap()]);
        let payload = super::doctor_payload(&json!({}), "2026-07-26T00:00:00Z");
        let checks = payload["checks"].as_array().unwrap();
        assert!(checks.len() >= 8, "expected the full check list");
        let mut seen = std::collections::HashSet::new();
        let mut failed = 0usize;
        for c in checks {
            for field in ["name", "ok", "classification", "detail", "remediation"] {
                assert!(c.get(field).is_some(), "check missing {field}: {c}");
            }
            let name = c["name"].as_str().unwrap().to_string();
            assert!(seen.insert(name.clone()), "duplicate check {name}");
            let cls = c["classification"].as_str().unwrap();
            assert!(
                matches!(cls, "healthy" | "failed" | "info" | "unknown"),
                "bad classification {cls}"
            );
            if cls == "failed" {
                failed += 1;
                assert!(
                    c["remediation"].is_string(),
                    "failed check {name} must carry a remediation"
                );
            }
        }
        for required in [
            "api_bound",
            "refresh_loop_alive",
            "providers_enabled",
            "provider_errors",
            "proxy_config_valid",
            "spend_sources_detected",
        ] {
            assert!(seen.contains(required), "missing check {required}");
        }
        // healthy must agree with the failure roll-up (no guessed verdicts).
        assert_eq!(payload["failed"].as_u64().unwrap() as usize, failed);
        assert_eq!(payload["healthy"], failed == 0);
        assert!(!payload.to_string().to_ascii_lowercase().contains("sk-"));
    }

    #[test]
    fn status_payload_shape_and_no_secret_echo() {
        super::publish(&[onenewapi_snap()]);
        let cfg = json!({ "refreshMinutes": 7, "proxy": { "url": "http://127.0.0.1:7890" } });
        let payload = super::status_payload(&cfg, "2026-07-26T00:00:00Z");
        assert_eq!(payload["app"], "pane");
        assert_eq!(payload["api"]["port"], 6736);
        assert_eq!(payload["refresh"]["minutes"], 7);
        assert_eq!(payload["config"]["proxy"]["source"], "config");
        assert!(payload["version"].as_str().is_some_and(|v| !v.is_empty()));
        assert!(payload["uptimeSecs"].as_i64().is_some());
        // The proxy ADDRESS must not be echoed — only its source.
        let raw = payload.to_string();
        assert!(!raw.contains("7890"), "proxy address leaked: {raw}");
        assert!(!raw.to_ascii_lowercase().contains("dashboard"));
    }

    #[test]
    fn sources_payload_shape() {
        let payload = super::sources_payload();
        let list = payload["sources"].as_array().unwrap();
        assert!(!list.is_empty());
        assert_eq!(payload["total"].as_u64().unwrap() as usize, list.len());
        assert!(payload["detected"].as_u64().is_some());
        for s in list {
            for field in ["id", "tool", "form", "collector", "state", "feeds"] {
                assert!(s.get(field).is_some(), "source missing {field}");
            }
        }
    }

    #[test]
    fn diagnostic_routes_answer_and_reject_bad_methods() {
        for path in ["/v1/status", "/v1/providers", "/v1/doctor", "/v1/sources"] {
            let (status, body) = route(&tiny_http::Method::Get, path);
            assert_eq!(status, 200, "{path}");
            let v: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert!(v.is_object(), "{path} must return an object");
        }
        let (missing, _) = route(&tiny_http::Method::Get, "/v1/nope");
        assert_eq!(missing, 404);
        let (method, _) = route(&tiny_http::Method::Post, "/v1/doctor");
        assert_eq!(method, 405);
    }
}
