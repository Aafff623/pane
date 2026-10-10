//! Command Code (GOAT / Pro / Max plans) usage windows. Bearer API key
//! against the undocumented /alpha billing endpoints the official CLI
//! itself uses — parsed tolerantly, since they carry no stability promise.
//! Plans meter three stacked windows: a 5-hour rolling window, a weekly
//! window, and monthly credits that refresh on the billing-cycle
//! anniversary. Watch the semantics: `credits.monthlyCredits` is what is
//! LEFT (not used), and an idle 5-hour window reports `resetAt: 0`.
//! Purchased/free credits sit outside every window — when a window is
//! exhausted, requests keep drawing from that extra pool.
//!
//! Key sources: our Settings pane, COMMAND_CODE_API_KEY, or the Command
//! Code CLI's own ~/.commandcode/auth.json.

use super::{http, stored_api_key, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "commandcode";
const NAME: &str = "Command Code";

const CREDITS_URL: &str = "https://api.commandcode.ai/alpha/billing/credits";
const SUBS_URL: &str = "https://api.commandcode.ai/alpha/billing/subscriptions";

const SESSION_MS: i64 = 5 * 3_600_000;
const WEEK_MS: i64 = 7 * 86_400_000;

fn find_key() -> Option<String> {
    if let Some(key) = stored_api_key("commandcode", &["COMMAND_CODE_API_KEY"]) {
        return Some(key);
    }
    // The Command Code CLI's own credential file. Field name is not part
    // of any documented contract, so any short string field that looks
    // like the key (user_… / token blob) is accepted.
    let path = dirs::home_dir()?.join(".commandcode").join("auth.json");
    let raw = super::read_small_text(&path, 64 * 1024, "auth.json").ok()?;
    let doc: Value = serde_json::from_str(&raw).ok()?;
    for field in ["apiKey", "api_key", "token", "key"] {
        if let Some(key) = doc.get(field).and_then(Value::as_str) {
            let key = key.trim();
            if !key.is_empty() {
                return Some(key.to_string());
            }
        }
    }
    None
}

/// Pure local probe for the Customize gear panel (no network): the
/// Command Code CLI's own credential file exists.
pub fn local_credential_hint() -> Option<String> {
    let path = dirs::home_dir()?.join(".commandcode").join("auth.json");
    std::fs::metadata(path)
        .ok()
        .map(|_| "Command Code CLI credentials (~/.commandcode/auth.json)".to_string())
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Live test of a user-pasted key, without saving it (Customize "Test").
pub async fn snapshot_with_key(key: &str) -> Snapshot {
    match fetch_with_key(key, ID, NAME).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Fetches a key while preserving the identity of the account card that
/// owns it (Goat 1 / Goat 2 … — several GOAT subscriptions side by side).
pub async fn snapshot_with_key_as(key: &str, card_id: &str, card_name: &str) -> Snapshot {
    match fetch_with_key(key, card_id, card_name).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(card_id, card_name, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(key) = find_key() else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Paste a Command Code API key in Settings (gear icon).",
        ));
    };
    fetch_with_key(&key, ID, NAME).await
}

async fn fetch_with_key(key: &str, card_id: &str, card_name: &str) -> Result<Snapshot, String> {
    let credits_req = http().get(CREDITS_URL).bearer_auth(key).send();
    let subs_req = http().get(SUBS_URL).bearer_auth(key).send();
    let (credits_resp, subs_resp) = tokio::join!(credits_req, subs_req);

    let credits_resp = credits_resp.map_err(|e| format!("credits request: {e}"))?;
    if credits_resp.status().as_u16() == 401 {
        return Err("API key was rejected — check it in Settings".into());
    }
    if !credits_resp.status().is_success() {
        return Err(format!("credits endpoint: HTTP {}", credits_resp.status()));
    }
    let credits: Value = super::json_body(credits_resp, 64 * 1024, "credits").await?;

    // Plan name and the billing-cycle reset come from subscriptions. Any
    // failure here degrades the Monthly row rather than the whole card.
    let mut subs: Option<Value> = None;
    if let Ok(resp) = subs_resp {
        if resp.status().is_success() {
            if let Ok(doc) = super::json_body(resp, 64 * 1024, "subscriptions").await {
                subs = Some(doc);
            }
        }
    }

    let (plan, metrics) = metrics_from_docs(&credits, subs.as_ref())?;
    Ok(Snapshot::ok(card_id, card_name, plan, metrics))
}

fn plan_display_name(plan_id: Option<&str>) -> Option<String> {
    let id = plan_id?.to_lowercase();
    if id.contains("goat") {
        Some("GOAT".into())
    } else if id.contains("team") {
        Some("Team Pro".into())
    } else if id.contains("pro") {
        Some("Pro".into())
    } else {
        plan_id.map(str::to_string)
    }
}

/// One window row (Session/Weekly): used/cap from the credits doc. An
/// absent window (plan without it) yields None.
fn window_metric(node: Option<&Value>, label: &str, period_ms: i64) -> Option<Metric> {
    let node = node?;
    let used = node.get("used").and_then(Value::as_f64)?;
    let cap = node.get("cap").and_then(Value::as_f64)?;
    if cap <= 0.0 {
        return None;
    }
    // resetAt == 0 on an idle window that hasn't opened yet — render as
    // "not started" rather than a countdown to the epoch.
    let resets_at = node
        .get("resetAt")
        .and_then(Value::as_i64)
        .filter(|ms| *ms > 0);
    Some(
        Metric::progress(
            label,
            (used / cap * 100.0).clamp(0.0, 100.0),
            Some(format!("${used:.2} of ${cap:.2} used")),
        )
        .with_reset(resets_at, Some(period_ms)),
    )
}

/// ISO-8601 (RFC 3339) instant → epoch ms, e.g. the subscription cycle's
/// currentPeriodEnd.
fn iso_to_epoch_ms(raw: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|t| t.timestamp_millis())
}

/// The parse pipeline fetch() runs, kept pure for the unit tests below.
fn metrics_from_docs(
    credits: &Value,
    subs: Option<&Value>,
) -> Result<(Option<String>, Vec<Metric>), String> {
    let windows = credits.get("windowLimits").ok_or_else(|| {
        "unexpected credits response shape (endpoint is undocumented)".to_string()
    })?;
    let credit_pool = credits.get("credits");

    let mut metrics = Vec::new();
    if let Some(m) = window_metric(windows.get("fiveHour"), "Session", SESSION_MS) {
        metrics.push(m);
    }
    if let Some(m) = window_metric(windows.get("weekly"), "Weekly", WEEK_MS) {
        metrics.push(m);
    }
    if metrics.is_empty() {
        // Windows absent: a positive credit pool still renders (the card
        // degrades to a Monthly row). Windows absent AND every pool at
        // zero means the subscription itself is inactive — say so plainly
        // instead of blaming the response shape.
        let pool_nonzero = credit_pool
            .map(|c| {
                ["monthlyCredits", "purchasedCredits", "freeCredits"]
                    .iter()
                    .any(|k| c.get(*k).and_then(Value::as_f64).is_some_and(|v| v > 0.0))
            })
            .unwrap_or(false);
        if !pool_nonzero {
            return Err(
                "subscription inactive or expired — no window limits and no credits on this account"
                    .into(),
            );
        }
    }

    // monthlyCredits is remaining balance. Pricing tables do not establish
    // this account's actual cap (custom/team/promotional plans can differ).
    let monthly_left = credit_pool
        .and_then(|c| c.get("monthlyCredits"))
        .and_then(Value::as_f64);
    let sub = subs.and_then(|s| s.get("data")).or(subs);
    let plan_id = sub
        .and_then(|s| s.get("planId"))
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(left) = monthly_left {
        let period_end = sub
            .and_then(|s| s.get("currentPeriodEnd"))
            .and_then(Value::as_str)
            .and_then(iso_to_epoch_ms);
        let mut value = format!("${left:.2} credits left");
        if period_end.is_some() {
            value.push_str(" · resets with billing cycle");
        }
        metrics.push(Metric::text("Monthly", value).with_reset(period_end, None));
    }

    // Extra credits (purchased + free) bypass the window limits — the
    // number that matters once a window is exhausted.
    let extra = credit_pool
        .map(|c| {
            c.get("purchasedCredits")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                + c.get("freeCredits").and_then(Value::as_f64).unwrap_or(0.0)
        })
        .unwrap_or(0.0);
    if extra > 0.0 {
        metrics.push(Metric::text(
            "Extra credits",
            format!("${extra:.2} · not window-limited"),
        ));
    }

    let plan = plan_display_name(plan_id.as_deref());
    Ok((plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Shape captured live from a GOAT account on 2026-09-12 (numbers
    /// only — nothing identifying).
    fn goat_credits() -> Value {
        json!({
            "credits": {
                "belowThreshold": false,
                "creditThreshold": 0,
                "monthlyCredits": 28.7129895114,
                "purchasedCredits": 0.0,
                "freeCredits": 0.0
            },
            "windowLimits": {
                "limited": true,
                "exceeded": null,
                "fiveHour": {"used": 0.06364232, "cap": 14, "exceeded": false, "resetAt": 1789218784243i64},
                "weekly": {"used": 6.236695549, "cap": 35, "exceeded": false, "resetAt": 1789645200864i64}
            }
        })
    }

    fn goat_subs() -> Value {
        json!({
            "success": true,
            "data": {
                "status": "active",
                "currentPeriodStart": "2026-09-02T18:03:24.000Z",
                "currentPeriodEnd": "2026-10-02T18:03:24.000Z",
                "planId": "individual-goat"
            }
        })
    }

    fn labels(metrics: &[Metric]) -> Vec<&str> {
        metrics.iter().map(|m| m.label.as_str()).collect()
    }

    #[test]
    fn goat_account_renders_three_windows() {
        let (plan, metrics) = metrics_from_docs(&goat_credits(), Some(&goat_subs())).unwrap();
        assert_eq!(plan.as_deref(), Some("GOAT"));
        assert_eq!(labels(&metrics), ["Session", "Weekly", "Monthly"]);

        assert_eq!(metrics[0].used_percent, Some(0.06364232 / 14.0 * 100.0));
        assert_eq!(metrics[0].resets_at, Some(1789218784243));
        assert_eq!(metrics[0].period_ms, Some(SESSION_MS));

        // Remaining is observed; a public price table is not an account cap.
        assert_eq!(metrics[2].used_percent, None);
        assert_eq!(
            metrics[2].resets_at,
            iso_to_epoch_ms("2026-10-02T18:03:24.000Z")
        );
        assert!(metrics[2]
            .value
            .as_deref()
            .unwrap()
            .contains("$28.71 credits left"));
    }

    #[test]
    fn idle_five_hour_window_has_no_reset() {
        // Second live account: unused 5-hour window reports resetAt: 0.
        let credits = json!({
            "credits": {"monthlyCredits": 68.8, "purchasedCredits": 0, "freeCredits": 0},
            "windowLimits": {
                "fiveHour": {"used": 0, "cap": 14, "resetAt": 0},
                "weekly": {"used": 1.19, "cap": 35, "resetAt": 1789733684393i64}
            }
        });
        let (_, metrics) = metrics_from_docs(&credits, Some(&goat_subs())).unwrap();
        assert_eq!(metrics[0].resets_at, None); // not "not started" → no epoch-0 countdown
        assert_eq!(metrics[0].period_ms, Some(SESSION_MS));
        assert_eq!(metrics[1].resets_at, Some(1789733684393));
    }

    #[test]
    fn missing_subscriptions_degrades_monthly_to_dollars() {
        let (plan, metrics) = metrics_from_docs(&goat_credits(), None).unwrap();
        assert_eq!(plan, None);
        assert_eq!(metrics[2].kind, "text");
        assert!(metrics[2].value.as_deref().unwrap().contains("$28.71"));
        assert_eq!(metrics[2].resets_at, None);
    }

    #[test]
    fn unknown_plan_degrades_monthly_to_dollars_not_wrong_percent() {
        let mut subs = goat_subs();
        subs["data"]["planId"] = json!("some-future-plan");
        let (plan, metrics) = metrics_from_docs(&goat_credits(), Some(&subs)).unwrap();
        assert_eq!(plan.as_deref(), Some("some-future-plan"));
        assert_eq!(metrics[2].kind, "text");
    }

    #[test]
    fn extra_credits_row_appears_only_when_present() {
        let (_, plain) = metrics_from_docs(&goat_credits(), None).unwrap();
        assert!(!labels(&plain).contains(&"Extra credits"));

        let mut credits = goat_credits();
        credits["credits"]["purchasedCredits"] = json!(3.2);
        credits["credits"]["freeCredits"] = json!(0.5);
        let (_, extra) = metrics_from_docs(&credits, None).unwrap();
        assert_eq!(
            labels(&extra),
            ["Session", "Weekly", "Monthly", "Extra credits"]
        );
        assert_eq!(
            extra[3].value.as_deref(),
            Some("$3.70 · not window-limited")
        );
    }

    #[test]
    fn paid_plan_name_never_implies_static_monthly_cap() {
        for id in [
            "individual-goat",
            "individual-pro",
            "team-pro",
            "enterprise-pro",
        ] {
            let mut subs = goat_subs();
            subs["data"]["planId"] = json!(id);
            let (_, rows) = metrics_from_docs(&goat_credits(), Some(&subs)).unwrap();
            assert_eq!(rows[2].kind, "text");
            assert_eq!(rows[2].used_percent, None);
        }
    }

    #[test]
    fn window_without_cap_is_skipped() {
        let credits = json!({
            "credits": {"monthlyCredits": 10.0},
            "windowLimits": {
                "fiveHour": {"used": 1.0, "cap": 0, "resetAt": 0},
                "weekly": {"used": 2.0, "cap": 35.0, "resetAt": 1789733684393i64}
            }
        });
        let (_, metrics) = metrics_from_docs(&credits, None).unwrap();
        assert_eq!(labels(&metrics), ["Weekly", "Monthly"]);
    }

    #[test]
    fn totally_unexpected_shape_is_an_error() {
        assert!(metrics_from_docs(&json!({"foo": 1}), None).is_err());
        let no_windows = json!({"credits": {"monthlyCredits": 5.0}});
        assert!(metrics_from_docs(&no_windows, None).is_err());
    }

    #[test]
    fn dead_subscription_says_so_plainly() {
        // Live Goat 1 shape (2026-10-05): subscription inactive — no live
        // window limits, every credit pool at zero, subscriptions returns
        // data:null. The old "no usage windows in credits response"
        // message sent the user hunting for a parser bug.
        let credits = json!({
            "credits": {"monthlyCredits": 0.0, "purchasedCredits": 0.0, "freeCredits": 0.0},
            "windowLimits": {"limited": false, "fiveHour": null, "weekly": null}
        });
        let subs = json!({"success": true, "data": null});
        let err = metrics_from_docs(&credits, Some(&subs))
            .err()
            .expect("dead shape must be an error");
        assert!(
            err.contains("subscription inactive or expired"),
            "got: {err}"
        );
    }

    #[test]
    fn zero_windows_with_positive_pool_degrades_to_monthly() {
        // A windowless account that still has credits is a legal shape —
        // it must render a Monthly row rather than an error.
        let credits = json!({
            "credits": {"monthlyCredits": 12.5, "purchasedCredits": 0.0, "freeCredits": 0.0},
            "windowLimits": {"limited": false, "fiveHour": null, "weekly": null}
        });
        let (plan, metrics) = metrics_from_docs(&credits, None).unwrap();
        assert_eq!(plan, None);
        assert_eq!(labels(&metrics), ["Monthly"]);
        assert_eq!(metrics[0].kind, "text");
        assert!(metrics[0].value.as_deref().unwrap().contains("$12.50"));
    }
}
