//! GIPHY developer-dashboard call stats — the Partner Hub endpoint that
//! backs the dashboard's own usage view.
//!
//! Credential: the dashboard **access_token** (the session cookie the
//! browser holds at developers.giphy.com), NOT the API key. Verified
//! 2026-10-15: sending the API key as a Bearer token answers
//! `403 Not a Partner Hub User` — the key resolves to an *app* principal
//! while these stats are gated to *dashboard users*.
//!
//! What the endpoints give (all verified live 2026-10-15):
//! - `GET /me/apps` → every app on the account (`apps[].id / name`).
//! - `GET /apps/{id}/stats/api-calls-count?start_date=&end_date=` → a
//!   **per-day** series `{"stats":[{"date":"2026-10-14","count":17}]}`.
//! - `GET /apps/{id}/stats/total-counters?…` → lifetime `total_api_calls`.
//!
//! Limits that shape this card:
//! - The series is DAILY only (`interval=hour` is ignored) and **today's
//!   bucket is empty** — GIPHY aggregates offline, so "calls this hour"
//!   cannot be derived. The card therefore shows per-app 7-day counts and
//!   says plainly that today's number arrives later.
//! - The key→app mapping is not exposed anywhere, so we list every app the
//!   account owns instead of guessing which one the pasted key belongs to
//!   (owner's account has two: an empty `localdev` and `hermes-search`).
//! - Refresh runs every 5 minutes; the stats change daily at best, so a
//!   30-minute TTL keeps the partner-hub traffic negligible.

use super::{http, json_body, stored_api_key, Metric, Snapshot};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const ID: &str = "giphy";
const NAME: &str = "GIPHY";
const API_BASE: &str = "https://partner-hub-api.giphy.com/api/v1/partner";
const DASHBOARD_URL: &str = "https://developers.giphy.com/dashboard/";
const MAX_BODY_BYTES: usize = 256 * 1024;
/// Only dashboard-token names — never GIPHY_API_KEY, which is the *app* key
/// and is rejected by these endpoints (403 Not a Partner Hub User).
const ENV_VARS: &[&str] = &["GIPHY_PARTNER_TOKEN", "GIPHY_ACCESS_TOKEN"];
/// Stats are daily; probing more often than this buys nothing.
const TTL: Duration = Duration::from_secs(30 * 60);
/// Days shown per app (today-6 .. today, the same range the dashboard uses).
const WINDOW_DAYS: i64 = 7;

fn cache() -> &'static Mutex<Option<(Instant, Snapshot)>> {
    static CACHE: Mutex<Option<(Instant, Snapshot)>> = Mutex::new(None);
    &CACHE
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, ENV_VARS).map(|_| "Pane GIPHY dashboard token".into())
}

pub async fn snapshot() -> Snapshot {
    if let Some((at, snap)) = cache().lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        if at.elapsed() < TTL {
            return snap.clone();
        }
    }
    let snap = match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    };
    // A missing credential must not be cached as the current answer — the
    // user may paste one a second later.
    if snap.status != "no_credentials" {
        *cache().lock().unwrap_or_else(|e| e.into_inner()) = Some((Instant::now(), snap.clone()));
    }
    snap
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(token) = stored_api_key(ID, ENV_VARS) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste your GIPHY dashboard access_token (developers.giphy.com → DevTools → Application → Cookies) in Settings.",
        ));
    };
    let apps = get_json(&format!("{API_BASE}/me/apps"), &token).await?;
    let apps = app_list(&apps);
    if apps.is_empty() {
        return Ok(Snapshot::ok(
            ID,
            NAME,
            Some(plan_label()),
            vec![Metric::text("Apps", "No apps on this GIPHY account".into())],
        ));
    }
    let (start, end) = window_dates(chrono::Utc::now().date_naive());
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for (id, name) in &apps {
        let url = format!(
            "{API_BASE}/apps/{id}/stats/api-calls-count?start_date={start}&end_date={end}"
        );
        let doc = get_json(&url, &token).await?;
        counts.insert(name.clone(), series_total(&doc));
    }
    let mut snap = Snapshot::ok(ID, NAME, Some(plan_label()), rows_from(&apps, &counts));
    snap.warning = Some(
        "GIPHY aggregates stats per day — today's calls appear after the daily rollup.".into(),
    );
    snap.dashboard_url = Some(DASHBOARD_URL.into());
    Ok(snap)
}

fn plan_label() -> String {
    "Beta key · 100 calls/hour (GIPHY-documented)".into()
}

/// `(id, name)` for every app on the account, in response order.
fn app_list(doc: &Value) -> Vec<(i64, String)> {
    doc.get("apps")
        .and_then(Value::as_array)
        .map(|apps| {
            apps.iter()
                .filter_map(|a| {
                    let id = a.get("id").and_then(Value::as_i64)?;
                    let name = a
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("(unnamed)")
                        .to_string();
                    Some((id, name))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Sum of the daily counts in an `api-calls-count` response. Missing or
/// malformed entries count as zero rather than failing the whole card.
fn series_total(doc: &Value) -> u64 {
    doc.get("stats")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r.get("count").and_then(Value::as_u64))
                .sum()
        })
        .unwrap_or(0)
}

/// One row per app: its 7-day call count. Pure so the harness can drive it.
fn rows_from(apps: &[(i64, String)], counts: &BTreeMap<String, u64>) -> Vec<Metric> {
    apps.iter()
        .map(|(_, name)| {
            let n = counts.get(name).copied().unwrap_or(0);
            Metric::text(
                name,
                format!("{n} calls · last {WINDOW_DAYS} days"),
            )
        })
        .collect()
}

/// `(start, end)` as `YYYY-MM-DD`, inclusive, spanning WINDOW_DAYS.
fn window_dates(today: chrono::NaiveDate) -> (String, String) {
    let start = today - chrono::Duration::days(WINDOW_DAYS - 1);
    (start.format("%Y-%m-%d").to_string(), today.format("%Y-%m-%d").to_string())
}

async fn get_json(url: &str, token: &str) -> Result<Value, String> {
    let resp = http()
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| format!("GIPHY stats: {e}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(
            "GIPHY dashboard token was rejected — copy a fresh access_token from the dashboard cookies".into(),
        );
    }
    if !status.is_success() {
        return Err(format!("GIPHY stats: HTTP {status}"));
    }
    json_body(resp, MAX_BODY_BYTES, "GIPHY stats").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_list_reads_id_and_name() {
        let doc: Value = serde_json::from_str(
            r#"{"apps":[{"id":673068,"name":"localdev"},{"id":657549,"name":"hermes-search"},{"name":"no-id"}]}"#,
        )
        .unwrap();
        let apps = app_list(&doc);
        assert_eq!(apps, vec![(673068, "localdev".into()), (657549, "hermes-search".into())]);
    }

    #[test]
    fn series_total_sums_days_and_tolerates_junk() {
        let doc: Value = serde_json::from_str(
            r#"{"stats":[{"date":"2026-10-10","count":3},{"date":"2026-10-14","count":17},{"count":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(series_total(&doc), 20);
        assert_eq!(series_total(&serde_json::json!({"stats": []})), 0);
        assert_eq!(series_total(&serde_json::json!({})), 0);
    }

    #[test]
    fn rows_carry_the_window_label_per_app() {
        let apps = vec![(1, "a".to_string()), (2, "b".to_string())];
        let mut counts = BTreeMap::new();
        counts.insert("a".to_string(), 21);
        let rows = rows_from(&apps, &counts);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "a");
        assert_eq!(rows[0].value.as_deref(), Some("21 calls · last 7 days"));
        assert_eq!(rows[1].value.as_deref(), Some("0 calls · last 7 days"));
    }

    #[test]
    fn window_is_seven_inclusive_days() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
        assert_eq!(
            window_dates(today),
            ("2026-10-09".to_string(), "2026-10-15".to_string())
        );
    }
}
