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
//! - The quota side is unavailable too: nothing exposes a window or a limit
//!   (every limit/quota path 404s, the app object has no such field, `/logs`
//!   is 403). So the per-app progress bar is measured against a ceiling
//!   **DERIVED** from the documented 100-calls/hour beta cap × 24, with the
//!   newest *completed* day as its numerator, only for apps whose payload
//!   explicitly identifies the beta tier. Both halves are labelled as
//!   such on the row — never present either as a live or measured figure.
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
/// The beta-key cap GIPHY documents per key: "All API Keys start as beta
/// keys, which are rate limited (100 searches/API calls per hour.)" —
/// https://developers.giphy.com/docs/api/ (same table in
/// docs/api/tenor-migration/). Both apps on this account are `unsubmitted` /
/// `exclusive_acl: "giphy_beta"`, i.e. beta.
const BETA_CALLS_PER_HOUR: u64 = 100;
/// The bar's denominator: the documented hourly cap scaled to a day.
///
/// DERIVED, never read. It is an upper bound implied by the cap, not a quota
/// GIPHY reports, and it is not tied to any calendar or billing period — see
/// the module docstring before "fixing" it into a measured value.
const DERIVED_DAILY_CEILING: u64 = BETA_CALLS_PER_HOUR * 24;

// The token is retained only in memory for identity comparison. Never log or
// derive Debug for this cache: account changes must not replay old stats.
fn cache() -> &'static Mutex<Option<(String, Instant, Snapshot)>> {
    static CACHE: Mutex<Option<(String, Instant, Snapshot)>> = Mutex::new(None);
    &CACHE
}

pub fn local_credential_hint() -> Option<String> {
    stored_api_key(ID, ENV_VARS).map(|_| "Pane GIPHY dashboard token".into())
}

pub async fn snapshot() -> Snapshot {
    let token = stored_api_key(ID, ENV_VARS);
    {
        let mut cached = cache().lock().unwrap_or_else(|e| e.into_inner());
        if let Some((cached_token, at, snap)) = cached.as_ref() {
            if token.as_deref() == Some(cached_token.as_str()) && at.elapsed() < TTL {
                return snap.clone();
            }
        }
        // Drop expired data and credentials from a removed/replaced account.
        *cached = None;
    }
    let snap = match fetch(token.as_deref()).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    };
    // Failures must not block explicit retry or a newly supplied credential
    // for the 30-minute stats TTL.
    if snap.status == "ok" {
        if let Some(token) = token {
            *cache().lock().unwrap_or_else(|e| e.into_inner()) =
                Some((token, Instant::now(), snap.clone()));
        }
    }
    snap
}

async fn fetch(token: Option<&str>) -> Result<Snapshot, String> {
    let Some(token) = token else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste your GIPHY dashboard access_token (developers.giphy.com → DevTools → Application → Cookies) in Settings.",
        ));
    };
    let apps = get_json(&format!("{API_BASE}/me/apps"), token).await?;
    let apps = app_list(&apps);
    if apps.is_empty() {
        return Ok(Snapshot::ok(
            ID,
            NAME,
            None,
            vec![Metric::text("Apps", "No apps on this GIPHY account".into())],
        ));
    }
    let today = chrono::Utc::now().date_naive();
    let (start, end) = window_dates(today);
    let mut stats: BTreeMap<i64, AppStats> = BTreeMap::new();
    for app in &apps {
        let id = app.id;
        let url =
            format!("{API_BASE}/apps/{id}/stats/api-calls-count?start_date={start}&end_date={end}");
        let doc = get_json(&url, token).await?;
        // The daily series already carries both numbers the card needs — the
        // 7-day sum and the newest completed day — so no second fetch.
        stats.insert(
            id,
            AppStats {
                week_total: series_total(&doc),
                last_day: last_day_bucket(&doc, today),
            },
        );
    }
    let mut snap = Snapshot::ok(ID, NAME, plan_label(&apps), rows_from(&apps, &stats));
    snap.warning = Some(
        "GIPHY aggregates stats per day — today's calls appear after the daily rollup.".into(),
    );
    snap.dashboard_url = Some(DASHBOARD_URL.into());
    Ok(snap)
}

fn plan_label(apps: &[App]) -> Option<String> {
    (!apps.is_empty() && apps.iter().all(|app| app.beta))
        .then(|| "Beta key · 100 calls/hour (GIPHY-documented)".into())
}

#[derive(Debug, PartialEq, Eq)]
struct App {
    id: i64,
    name: String,
    beta: bool,
}

/// Keep app identity and the explicitly reported tier, in response order.
fn app_list(doc: &Value) -> Vec<App> {
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
                    let beta = a.get("exclusive_acl").and_then(Value::as_str) == Some("giphy_beta");
                    Some(App { id, name, beta })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// One app's numbers, both read from the SAME `api-calls-count` response.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct AppStats {
    /// Sum of the window's daily buckets — the 7-day row.
    week_total: u64,
    /// Newest completed day as `(YYYY-MM-DD, count)` — the bar's numerator.
    last_day: Option<(String, u64)>,
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

/// The newest day bucket in an `api-calls-count` response.
///
/// Only valid dates strictly before today can represent completed days.
/// Do not rely on the server continuing to omit today's partial bucket.
fn last_day_bucket(doc: &Value, today: chrono::NaiveDate) -> Option<(String, u64)> {
    doc.get("stats")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(|r| {
            let date = r.get("date").and_then(Value::as_str)?;
            let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
            if parsed >= today || parsed.format("%Y-%m-%d").to_string() != date {
                return None;
            }
            let count = r.get("count").and_then(Value::as_u64)?;
            Some((date.to_string(), count))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
}

/// Used-percent against the DERIVED daily ceiling. Own function so the
/// arithmetic and its denominator stay testable.
fn ceiling_percent(day_calls: u64) -> f64 {
    day_calls as f64 / DERIVED_DAILY_CEILING as f64 * 100.0
}

/// Two rows per app, both from the same daily series: a progress row (newest
/// completed day vs the derived ceiling) and the 7-day total the card has
/// always shown. Pure so the harness can drive it.
///
/// Labels must stay distinct: the frontend pins rows by label and renders only
/// the first match, so a collision would silently drop a row.
fn rows_from(apps: &[App], stats: &BTreeMap<i64, AppStats>) -> Vec<Metric> {
    let mut rows = Vec::with_capacity(apps.len() * 2);
    for app in apps {
        // Names are not unique. Including the stable app id also prevents a
        // user-supplied name from colliding with another app's weekly label.
        let name = format!("{} [app {}]", app.name, app.id);
        let s = stats.get(&app.id).cloned().unwrap_or_default();
        rows.push(match &s.last_day {
            Some((date, calls)) if app.beta => Metric::progress(
                &name,
                ceiling_percent(*calls),
                Some(format!(
                    "{calls} of {DERIVED_DAILY_CEILING} calls · last completed day {date} · \
                     ceiling derived from {BETA_CALLS_PER_HOUR} calls/hour × 24"
                )),
            ),
            // No completed day yet: a 0% bar would read as "no calls used",
            // which is a claim this data cannot support. Say it is missing.
            None => Metric::text(&name, "No completed day yet".into()),
            Some((date, calls)) => {
                Metric::text(&name, format!("{calls} calls · last completed day {date}"))
            }
        });
        rows.push(Metric::text(
            &format!("{name} · last {WINDOW_DAYS} days"),
            format!("{} calls", s.week_total),
        ));
    }
    rows
}

/// `(start, end)` as `YYYY-MM-DD`, inclusive, spanning WINDOW_DAYS.
fn window_dates(today: chrono::NaiveDate) -> (String, String) {
    let start = today - chrono::Duration::days(WINDOW_DAYS - 1);
    (
        start.format("%Y-%m-%d").to_string(),
        today.format("%Y-%m-%d").to_string(),
    )
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

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 16).unwrap()
    }

    fn beta_app(id: i64, name: &str) -> App {
        App {
            id,
            name: name.into(),
            beta: true,
        }
    }

    #[test]
    fn app_list_reads_id_and_name() {
        let doc: Value = serde_json::from_str(
            r#"{"apps":[{"id":673068,"name":"localdev"},{"id":657549,"name":"hermes-search"},{"name":"no-id"}]}"#,
        )
        .unwrap();
        let apps = app_list(&doc);
        assert_eq!(
            apps,
            vec![
                App {
                    id: 673068,
                    name: "localdev".into(),
                    beta: false
                },
                App {
                    id: 657549,
                    name: "hermes-search".into(),
                    beta: false
                },
            ]
        );
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
    fn last_day_bucket_picks_the_newest_date_not_the_last_row() {
        // Out-of-order response: the newest *date* wins, not the tail element.
        let doc: Value = serde_json::from_str(
            r#"{"stats":[{"date":"2026-10-14","count":17},{"date":"2026-10-09","count":3},{"date":"2026-10-11","count":5}]}"#,
        )
        .unwrap();
        assert_eq!(
            last_day_bucket(&doc, today()),
            Some(("2026-10-14".into(), 17))
        );
    }

    #[test]
    fn last_day_bucket_skips_incomplete_rows_and_absent_series() {
        let doc: Value = serde_json::from_str(
            r#"{"stats":[{"date":"2026-10-14","count":17},{"date":"2026-10-15"},{"count":9},{"date":"2026-10-16","count":"x"}]}"#,
        )
        .unwrap();
        // A row missing either half is skipped, not read as a zero-count day.
        assert_eq!(
            last_day_bucket(&doc, today()),
            Some(("2026-10-14".into(), 17))
        );
        assert_eq!(
            last_day_bucket(&serde_json::json!({"stats": []}), today()),
            None
        );
        assert_eq!(last_day_bucket(&serde_json::json!({}), today()), None);
    }

    #[test]
    fn ceiling_is_the_documented_hourly_rate_scaled_to_a_day() {
        // Locks the published number: the row text tells the user where the
        // ceiling comes from, so changing the derivation must be deliberate.
        assert_eq!(BETA_CALLS_PER_HOUR, 100);
        assert_eq!(DERIVED_DAILY_CEILING, 2400);
        assert_eq!(ceiling_percent(2400), 100.0);
        assert_eq!(ceiling_percent(1200), 50.0);
        assert_eq!(ceiling_percent(0), 0.0);
    }

    #[test]
    fn rows_pair_a_ceiling_bar_with_the_week_total() {
        let apps = vec![beta_app(1, "a"), beta_app(2, "b")];
        let mut stats = BTreeMap::new();
        stats.insert(
            1,
            AppStats {
                week_total: 21,
                last_day: Some(("2026-10-14".into(), 1200)),
            },
        );
        let rows = rows_from(&apps, &stats);

        assert_eq!(rows.len(), 4);
        // Bar row: numerator and denominator both visible, derivation spelled
        // out — the frontend's detail translation keys on this exact shape.
        assert_eq!(rows[0].label, "a [app 1]");
        assert_eq!(rows[0].kind, "progress");
        assert_eq!(rows[0].used_percent, Some(50.0));
        assert_eq!(
            rows[0].detail.as_deref(),
            Some("1200 of 2400 calls · last completed day 2026-10-14 · ceiling derived from 100 calls/hour × 24")
        );
        assert_eq!(rows[1].label, "a [app 1] · last 7 days");
        assert_eq!(rows[1].kind, "text");
        assert_eq!(rows[1].value.as_deref(), Some("21 calls"));
        // "b" has no series at all: no bar (a 0% one would be a claim this
        // data cannot support), and its week row still reports a real zero.
        assert_eq!(rows[2].label, "b [app 2]");
        assert_eq!(rows[2].kind, "text");
        assert_eq!(rows[2].value.as_deref(), Some("No completed day yet"));
        assert_eq!(rows[3].label, "b [app 2] · last 7 days");
        assert_eq!(rows[3].value.as_deref(), Some("0 calls"));
    }

    #[test]
    fn row_labels_stay_unique_per_app() {
        // The frontend renders only the FIRST metric matching a pinned label,
        // so a duplicate label would silently hide a row.
        let apps = vec![
            beta_app(1, "a"),
            beta_app(2, "a"),
            beta_app(3, "a [app 1] · last 7 days"),
        ];
        let mut stats = BTreeMap::new();
        stats.insert(
            1,
            AppStats {
                week_total: 1,
                last_day: Some(("2026-10-14".into(), 1)),
            },
        );
        let rows = rows_from(&apps, &stats);
        let mut labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), rows.len());
    }

    #[test]
    fn completed_day_rejects_today_future_and_invalid_dates() {
        let doc = serde_json::json!({"stats": [
            {"date": "2026-10-15", "count": 17},
            {"date": "2026-10-16", "count": 200},
            {"date": "2026-10-17", "count": 300},
            {"date": "2026-02-30", "count": 400},
            {"date": "garbage", "count": 500},
            {"date": "2026-9-30", "count": 600}
        ]});
        assert_eq!(
            last_day_bucket(&doc, today()),
            Some(("2026-10-15".into(), 17))
        );
        let partial = serde_json::json!({"stats": [{"date": "2026-10-16", "count": 1}]});
        assert_eq!(last_day_bucket(&partial, today()), None);
    }

    #[test]
    fn ceiling_requires_an_explicit_beta_tier_per_app() {
        let apps = app_list(&serde_json::json!({"apps": [
            {"id": 1, "name": "beta", "exclusive_acl": "giphy_beta"},
            {"id": 2, "name": "approved", "exclusive_acl": "giphy_production", "status": "approved"},
            {"id": 3, "name": "unknown"}
        ]}));
        let stats = apps
            .iter()
            .map(|app| {
                (
                    app.id,
                    AppStats {
                        week_total: 1200,
                        last_day: Some(("2026-10-15".into(), 1200)),
                    },
                )
            })
            .collect();
        let rows = rows_from(&apps, &stats);
        assert_eq!(rows[0].kind, "progress");
        assert_eq!(rows[2].kind, "text");
        assert_eq!(rows[4].kind, "text");
        assert_eq!(
            rows[2].value.as_deref(),
            Some("1200 calls · last completed day 2026-10-15")
        );
        assert!(plan_label(&apps).is_none());
        assert!(plan_label(&apps[..1]).is_some());
        assert!(plan_label(&[]).is_none());
    }

    #[test]
    fn duplicate_names_keep_independent_app_statistics() {
        let apps = vec![beta_app(1, "same"), beta_app(2, "same")];
        let stats = BTreeMap::from([
            (
                1,
                AppStats {
                    week_total: 17,
                    last_day: Some(("2026-10-15".into(), 17)),
                },
            ),
            (
                2,
                AppStats {
                    week_total: 99,
                    last_day: Some(("2026-10-15".into(), 99)),
                },
            ),
        ]);
        let rows = rows_from(&apps, &stats);
        assert_eq!(rows[1].value.as_deref(), Some("17 calls"));
        assert_eq!(rows[3].value.as_deref(), Some("99 calls"));
        assert_ne!(rows[0].label, rows[2].label);
        assert_ne!(rows[0].used_percent, rows[2].used_percent);
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
