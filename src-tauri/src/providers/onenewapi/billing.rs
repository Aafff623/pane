use super::super::Metric;
use super::fingerprint::DisplayUnit;
use serde_json::Value;

const USAGE_TO_USD: f64 = 0.01;
const UNLIMITED_SENTINEL: f64 = 100_000_000.0;
const MAX_JS_DATE_MS: i64 = 8_640_000_000_000_000;

pub fn metrics_from(
    sub: &Value,
    usage: Option<&Value>,
    unit: &DisplayUnit,
) -> Result<Vec<Metric>, String> {
    let limit = parse_limit(sub);
    let used = usage.and_then(parse_used);
    let mut metrics = Vec::new();
    match (limit, used) {
        (Some(limit), Some(used)) => {
            let pct = (used / limit * 100.0).clamp(0.0, 100.0);
            // Daily-reset bonus fields only apply when the subscription body
            // actually carries a next_reset_time. Pane's pace engine treats
            // (resets_at, period_ms) as a pair — emit a complete pair or none.
            let reset_pair = usage
                .and_then(|v| v.get("next_reset_time"))
                .and_then(|v| v.as_i64())
                .filter(|ms| *ms > 0)
                .map(|ms| (ms * 1000, 24 * 60 * 60 * 1000));
            let mut progress = Metric::progress(
                "Usage",
                pct,
                Some(format_pair(unit, used, limit)),
            );
            if let Some((resets_at, period_ms)) = reset_pair {
                progress.resets_at = Some(resets_at);
                progress.period_ms = Some(period_ms);
            }
            metrics.push(progress);
        }
        (Some(limit), None) => {
            metrics.push(Metric::text("Limit", format_amount(unit, limit)));
        }
        (None, Some(used)) => {
            metrics.push(Metric::text("Used", format_amount(unit, used)));
        }
        (None, None) if subscription_is_unlimited(sub) => {
            metrics.push(Metric::text("Used", format_amount(unit, 0.0)));
        }
        (None, None) => return Err("no billing data in response".into()),
    }
    if let Some(metric) = optional_expiry_metric(sub.get("access_until")) {
        metrics.push(metric);
    }
    Ok(metrics)
}

/// A NewAPI subscription plan's card view, daily-reset style: a Usage
/// progress bar (left = remaining of today's $2 000 allowance) with a
/// countdown to the next reset, plus the plan's end date.
pub(crate) struct SubscriptionView {
    pub plan_label: String,
    pub metrics: Vec<Metric>,
}

/// Parses a `/api/subscription/self` response. `None` when the body has no
/// usable active subscription — callers fall back to the wallet billing
/// display.
pub(crate) fn subscription_view(
    body: &Value,
    unit: &DisplayUnit,
    per_unit: f64,
    rate: f64,
) -> Option<SubscriptionView> {
    if body.get("success") != Some(&Value::Bool(true)) {
        return None;
    }
    let subs = body.get("data")?.get("subscriptions")?.as_array()?;
    let active = subs
        .iter()
        .filter_map(|entry| entry.get("subscription"))
        .find(|sub| {
            sub.get("status")
                .and_then(Value::as_str)
                .is_some_and(|status| status.eq_ignore_ascii_case("active"))
        })?;
    let id = active.get("id").and_then(Value::as_i64)?;
    let total = quota_units(active.get("amount_total"))?;
    let used = quota_units(active.get("amount_used")).unwrap_or(0.0).max(0.0);
    let per_unit = if per_unit.is_finite() && per_unit > 0.0 {
        per_unit
    } else {
        500_000.0
    };
    let display = |quota: f64| match unit {
        DisplayUnit::Tokens => quota,
        _ => quota / per_unit * rate,
    };

    let mut metrics = Vec::new();
    if total > 0.0 {
        // Pane's progress card reads used_percent as "used", so the bar fills
        // with spent quota and the headline reads "X% left" (100 - used%).
        let used_value = display(used);
        let total_value = display(total);
        let pct = (used / total * 100.0).clamp(0.0, 100.0);
        let mut usage =
            Metric::progress("Usage", pct, Some(format_pair(unit, used_value, total_value)));
        usage.resets_at = parse_access_until(active.get("next_reset_time"));
        // Daily reset → the bar's pace window is 24 h, the same way the
        // GLM plan delivers "Resets in 2h 14m" for a 5-hour session.
        usage.period_ms = Some(24 * 60 * 60 * 1000);
        metrics.push(usage);
    } else {
        // new-api plan semantics: total 0 = unlimited allowance; no "left",
        // so just show what went through the plan.
        metrics.push(Metric::text("Used", format_amount(unit, display(used))));
    }
    if let Some(metric) = optional_expiry_metric(active.get("end_time")) {
        metrics.push(metric);
    }
    Some(SubscriptionView {
        plan_label: format!("Subscription #{id}"),
        metrics,
    })
}

fn quota_units(v: Option<&Value>) -> Option<f64> {
    v.and_then(as_finite_f64).filter(|n| *n >= 0.0)
}

fn format_amount(unit: &DisplayUnit, n: f64) -> String {
    match unit {
        DisplayUnit::Usd => format!("${n:.2}"),
        DisplayUnit::Cny => format!("¥{n:.2}"),
        DisplayUnit::Tokens => format!("{n:.0}"),
        DisplayUnit::Custom(sym) => format!("{sym}{n:.2}"),
    }
}

fn format_pair(unit: &DisplayUnit, used: f64, limit: f64) -> String {
    match unit {
        DisplayUnit::Tokens => format!("{:.0} of {:.0}", used, limit),
        _ => format!(
            "{} of {}",
            format_amount(unit, used),
            format_amount(unit, limit)
        ),
    }
}

fn parse_limit(sub: &Value) -> Option<f64> {
    usable_dollars(sub.get("hard_limit_usd"))
        .or_else(|| usable_dollars(sub.get("system_hard_limit_usd")))
}

fn subscription_is_unlimited(sub: &Value) -> bool {
    is_unlimited_field(sub.get("hard_limit_usd"))
        || is_unlimited_field(sub.get("system_hard_limit_usd"))
}

fn is_unlimited_field(v: Option<&Value>) -> bool {
    v.and_then(as_finite_f64)
        .is_some_and(|n| n >= UNLIMITED_SENTINEL)
}

fn usable_dollars(v: Option<&Value>) -> Option<f64> {
    let n = as_finite_f64(v?)?;
    if n > 0.0 && n < UNLIMITED_SENTINEL {
        Some(n)
    } else {
        None
    }
}

fn parse_used(usage: &Value) -> Option<f64> {
    let n = as_finite_f64(usage.get("total_usage")?)?;
    if n < 0.0 {
        return None;
    }
    let used = n * USAGE_TO_USD;
    used.is_finite().then_some(used)
}

fn parse_access_until(v: Option<&Value>) -> Option<i64> {
    let v = v?;
    let secs = if let Some(i) = v.as_i64() {
        i
    } else {
        let f = as_finite_f64(v)?;
        if f <= 0.0 || f != f.trunc() || f > i64::MAX as f64 {
            return None;
        }
        f as i64
    };
    if secs <= 0 {
        return None;
    }
    secs.checked_mul(1000).filter(|ms| *ms <= MAX_JS_DATE_MS)
}

fn as_finite_f64(v: &Value) -> Option<f64> {
    v.as_f64().filter(|n| n.is_finite())
}

fn expiry_metric(resets_at: i64) -> Metric {
    Metric {
        label: "Expiry".into(),
        kind: "action".into(),
        used_percent: None,
        detail: None,
        value: None,
        resets_at: Some(resets_at),
        period_ms: None,
    }
}

/// Epoch-seconds field → `Expiry` metric, skipped when missing/invalid.
fn optional_expiry_metric(v: Option<&Value>) -> Option<Metric> {
    parse_access_until(v).map(expiry_metric)
}

#[cfg(test)]
mod tests {
    use super::super::super::Metric;
    use super::super::fingerprint::DisplayUnit;
    use super::metrics_from;
    use serde_json::{json, Value};

    fn usd_metrics(sub: &Value, usage: Option<&Value>) -> Result<Vec<Metric>, String> {
        metrics_from(sub, usage, &DisplayUnit::Usd)
    }

    fn captured_sub() -> Value {
        json!({
            "object": "billing_subscription",
            "has_payment_method": true,
            "soft_limit_usd": 1217.81744,
            "hard_limit_usd": 1217.81744,
            "system_hard_limit_usd": 1217.81744,
            "access_until": 1790479748
        })
    }

    fn captured_usage() -> Value {
        json!({
            "object": "list",
            "total_usage": 59218.42860000001
        })
    }

    fn by_label<'a>(metrics: &'a [Metric], label: &str) -> &'a Metric {
        metrics
            .iter()
            .find(|m| m.label == label)
            .unwrap_or_else(|| panic!("missing metric {label}"))
    }

    fn expect_err(result: Result<Vec<Metric>, String>) -> String {
        match result {
            Err(e) => e,
            Ok(_) => panic!("expected billing error"),
        }
    }

    #[test]
    fn captured_example_usage_and_expiry() {
        let metrics = usd_metrics(&captured_sub(), Some(&captured_usage())).unwrap();
        assert_eq!(metrics.len(), 2);

        let usage = by_label(&metrics, "Usage");
        assert_eq!(usage.kind, "progress");
        let pct = usage.used_percent.unwrap();
        assert!((pct - 48.63).abs() < 0.01, "percent {pct} should be ~48.63");
        assert_eq!(format!("{pct:.2}"), "48.63");
        assert_eq!(usage.detail.as_deref(), Some("$592.18 of $1217.82"));
        assert_eq!(usage.resets_at, None);
        assert_eq!(usage.period_ms, None);
        assert_eq!(usage.value, None);

        let expiry = by_label(&metrics, "Expiry");
        assert_eq!(expiry.kind, "action");
        assert_eq!(expiry.resets_at, Some(1_790_479_748_000));
        assert_eq!(expiry.detail, None);
        assert_eq!(expiry.period_ms, None);
        assert_eq!(expiry.used_percent, None);
    }

    #[test]
    fn limit_falls_back_to_system_hard_limit() {
        let sub = json!({
            "hard_limit_usd": 0,
            "system_hard_limit_usd": 50.5
        });
        let metrics = usd_metrics(&sub, None).unwrap();
        let limit = by_label(&metrics, "Limit");
        assert_eq!(limit.kind, "text");
        assert_eq!(limit.value.as_deref(), Some("$50.50"));
        assert!(metrics.iter().all(|m| m.label != "Expiry"));
    }

    #[test]
    fn hard_limit_wins_over_system() {
        let sub = json!({
            "hard_limit_usd": 10,
            "system_hard_limit_usd": 99
        });
        let usage = json!({"total_usage": 0});
        let metrics = usd_metrics(&sub, Some(&usage)).unwrap();
        let usage = by_label(&metrics, "Usage");
        assert_eq!(usage.detail.as_deref(), Some("$0.00 of $10.00"));
    }

    #[test]
    fn exact_sentinel_is_unlimited_and_falls_through() {
        let sub = json!({
            "hard_limit_usd": 100000000,
            "system_hard_limit_usd": 12.5
        });
        let metrics = usd_metrics(&sub, None).unwrap();
        assert_eq!(by_label(&metrics, "Limit").value.as_deref(), Some("$12.50"));

        let both_sentinel = json!({
            "hard_limit_usd": 100000000.0,
            "system_hard_limit_usd": 100000000
        });
        let usage = json!({"total_usage": 250});
        let metrics = usd_metrics(&both_sentinel, Some(&usage)).unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(by_label(&metrics, "Used").value.as_deref(), Some("$2.50"));

        let above_sentinel = json!({"hard_limit_usd": 500_000_000.0});
        let metrics = usd_metrics(&above_sentinel, Some(&usage)).unwrap();
        assert_eq!(by_label(&metrics, "Used").value.as_deref(), Some("$2.50"));

        let unlimited_no_usage = usd_metrics(&both_sentinel, None).unwrap();
        assert_eq!(
            by_label(&unlimited_no_usage, "Used").value.as_deref(),
            Some("$0.00")
        );
    }

    #[test]
    fn used_only_and_limit_only_and_neither() {
        let used_only = usd_metrics(&json!({}), Some(&json!({"total_usage": 1234}))).unwrap();
        assert_eq!(
            by_label(&used_only, "Used").value.as_deref(),
            Some("$12.34")
        );

        let limit_only = usd_metrics(&json!({"hard_limit_usd": 9}), None).unwrap();
        assert_eq!(
            by_label(&limit_only, "Limit").value.as_deref(),
            Some("$9.00")
        );

        assert_eq!(
            expect_err(usd_metrics(&json!({}), None)),
            "no billing data in response"
        );
        assert_eq!(
            expect_err(usd_metrics(
                &json!({"soft_limit_usd": 100}),
                Some(&json!({}))
            )),
            "no billing data in response"
        );
    }

    #[test]
    fn invalid_and_non_positive_amounts_are_missing() {
        let sub = json!({
            "hard_limit_usd": -5,
            "system_hard_limit_usd": "12",
            "access_until": 0
        });
        let usage = json!({"total_usage": -1});
        assert_eq!(
            expect_err(usd_metrics(&sub, Some(&usage))),
            "no billing data in response"
        );

        let sub = json!({
            "hard_limit_usd": true,
            "system_hard_limit_usd": null
        });
        let usage = json!({"total_usage": "10"});
        assert_eq!(
            expect_err(usd_metrics(&sub, Some(&usage))),
            "no billing data in response"
        );
    }

    #[test]
    fn percent_is_clamped() {
        let sub = json!({"hard_limit_usd": 1});
        let over = json!({"total_usage": 50_000});
        let pct = by_label(&usd_metrics(&sub, Some(&over)).unwrap(), "Usage")
            .used_percent
            .unwrap();
        assert_eq!(pct, 100.0);

        let zero = json!({"total_usage": 0});
        let pct = by_label(&usd_metrics(&sub, Some(&zero)).unwrap(), "Usage")
            .used_percent
            .unwrap();
        assert_eq!(pct, 0.0);
    }

    #[test]
    fn expiry_omitted_when_missing_invalid_or_overflowing() {
        let sub = json!({"hard_limit_usd": 1});
        let metrics = usd_metrics(&sub, None).unwrap();
        assert!(metrics.iter().all(|m| m.label != "Expiry"));

        for until in [
            json!(0),
            json!(-1),
            json!("1790479748"),
            json!(null),
            json!(1.5),
            json!(9_000_000_000_000_i64),
            json!(i64::MAX),
        ] {
            let sub = json!({"hard_limit_usd": 1, "access_until": until});
            let metrics = usd_metrics(&sub, None).unwrap();
            assert!(
                metrics.iter().all(|m| m.label != "Expiry"),
                "expiry should be omitted for {sub}"
            );
        }
    }

    #[test]
    fn usage_never_inherits_access_until() {
        let metrics = usd_metrics(&captured_sub(), Some(&captured_usage())).unwrap();
        let usage = by_label(&metrics, "Usage");
        assert_eq!(usage.resets_at, None);
        assert_eq!(usage.period_ms, None);
        assert_eq!(
            by_label(&metrics, "Expiry").resets_at,
            Some(1_790_479_748_000)
        );
    }

    #[test]
    fn cny_keeps_cents_math_and_yen_symbol() {
        let metrics =
            metrics_from(&captured_sub(), Some(&captured_usage()), &DisplayUnit::Cny).unwrap();
        let usage = by_label(&metrics, "Usage");
        let pct = usage.used_percent.unwrap();
        assert!((pct - 48.63).abs() < 0.01, "percent {pct} should be ~48.63");
        assert_eq!(usage.detail.as_deref(), Some("¥592.18 of ¥1217.82"));
    }

    #[test]
    fn tokens_keep_cents_math_without_dollar_label() {
        let sub = json!({"hard_limit_usd": 1_000_000});
        let usage = json!({"total_usage": 25_000_000});
        let metrics = metrics_from(&sub, Some(&usage), &DisplayUnit::Tokens).unwrap();
        let usage = by_label(&metrics, "Usage");
        assert_eq!(usage.detail.as_deref(), Some("250000 of 1000000"));
        assert_ne!(usage.detail.as_deref(), Some("$250000.00 of $1000000.00"));
        let pct = usage.used_percent.unwrap();
        assert!((pct - 25.0).abs() < 0.01, "percent {pct}");
    }

    #[test]
    fn tokens_used_only_and_unlimited_zero_are_integers() {
        let used_only =
            metrics_from(&json!({}), Some(&json!({"total_usage": 1234})), &DisplayUnit::Tokens)
                .unwrap();
        assert_eq!(by_label(&used_only, "Used").value.as_deref(), Some("12"));

        let unlimited = json!({
            "hard_limit_usd": 100000000,
            "system_hard_limit_usd": 100000000
        });
        let zero = metrics_from(&unlimited, None, &DisplayUnit::Tokens).unwrap();
        assert_eq!(by_label(&zero, "Used").value.as_deref(), Some("0"));
    }

    #[test]
    fn custom_symbol_formats_currency_amounts() {
        let metrics = metrics_from(
            &captured_sub(),
            Some(&captured_usage()),
            &DisplayUnit::Custom("€".into()),
        )
        .unwrap();
        assert_eq!(
            by_label(&metrics, "Usage").detail.as_deref(),
            Some("€592.18 of €1217.82")
        );
    }

    // --- subscription_view -------------------------------------------------

    use super::subscription_view;

    /// Mirrors the live `/api/subscription/self` shape: quota units at
    /// 500 000 per USD, daily reset, one month term. $558 of $2 000 used.
    fn subscription_body() -> Value {
        json!({
            "success": true,
            "message": "",
            "data": {
                "billing_preference": "subscription_first",
                "subscriptions": [{
                    "subscription": {
                        "id": 239,
                        "plan_id": 3,
                        "amount_total": 1_000_000_000i64,
                        "amount_used": 279_000_000i64,
                        "start_time": 1_787_829_306,
                        "end_time": 1_791_164_506,
                        "status": "active",
                        "last_reset_time": 1_791_069_600,
                        "next_reset_time": 1_791_155_200
                    }
                }],
                "all_subscriptions": []
            }
        })
    }

    #[test]
    fn subscription_view_shows_daily_reset_progress_bar() {
        let view = subscription_view(&subscription_body(), &DisplayUnit::Usd, 500_000.0, 1.0)
            .expect("active subscription");
        assert_eq!(view.plan_label, "Subscription #239");
        assert_eq!(view.metrics.len(), 2);

        // $2 000 allowance, $558 used → ~27.9% used, the card reads "72.1%
        // left" with the bar filled to the used side.
        let usage = by_label(&view.metrics, "Usage");
        assert_eq!(usage.kind, "progress");
        assert!((usage.used_percent.unwrap() - 27.9).abs() < 1e-9);
        assert_eq!(usage.detail.as_deref(), Some("$558.00 of $2000.00"));
        assert_eq!(usage.resets_at, Some(1_791_155_200_000));
        assert_eq!(usage.period_ms, Some(24 * 60 * 60 * 1000));

        let expiry = by_label(&view.metrics, "Expiry");
        assert_eq!(expiry.kind, "action");
        assert_eq!(expiry.resets_at, Some(1_791_164_506_000));
    }

    #[test]
    fn subscription_view_skips_inactive_entries_and_rejects_empty_bodies() {
        let mut body = subscription_body();
        body["data"]["subscriptions"][0]["subscription"]["status"] = json!("expired");
        assert!(subscription_view(&body, &DisplayUnit::Usd, 500_000.0, 1.0).is_none());

        assert!(subscription_view(
            &json!({"success": true, "data": {"subscriptions": []}}),
            &DisplayUnit::Usd,
            500_000.0,
            1.0
        )
        .is_none());
        assert!(subscription_view(
            &json!({"success": false, "message": "nope"}),
            &DisplayUnit::Usd,
            500_000.0,
            1.0
        )
        .is_none());
        assert!(subscription_view(&json!({}), &DisplayUnit::Usd, 500_000.0, 1.0).is_none());
    }

    #[test]
    fn subscription_view_unlimited_allowance_shows_used_only() {
        let mut body = subscription_body();
        let sub = &mut body["data"]["subscriptions"][0]["subscription"];
        sub["amount_total"] = json!(0);
        sub["amount_used"] = json!(125_000_000i64);
        sub["next_reset_time"] = json!(0);
        sub["end_time"] = json!(0);
        let view = subscription_view(&body, &DisplayUnit::Usd, 500_000.0, 1.0).unwrap();
        assert_eq!(view.metrics.len(), 1);
        assert_eq!(by_label(&view.metrics, "Used").value.as_deref(), Some("$250.00"));
    }

    #[test]
    fn subscription_view_overuse_fills_bar_and_missing_used_means_empty() {
        let mut body = subscription_body();
        let sub = &mut body["data"]["subscriptions"][0]["subscription"];
        sub["amount_used"] = json!(1_500_000_000i64);
        let view = subscription_view(&body, &DisplayUnit::Usd, 500_000.0, 1.0).unwrap();
        let usage = by_label(&view.metrics, "Usage");
        assert_eq!(usage.used_percent, Some(100.0));
        assert_eq!(usage.detail.as_deref(), Some("$3000.00 of $2000.00"));

        // A missing amount_used reads as nothing spent → bar empty, full
        // allowance still left.
        body["data"]["subscriptions"][0]["subscription"]
            .as_object_mut()
            .unwrap()
            .remove("amount_used");
        let view = subscription_view(&body, &DisplayUnit::Usd, 500_000.0, 1.0).unwrap();
        let usage = by_label(&view.metrics, "Usage");
        assert_eq!(usage.used_percent, Some(0.0));
        assert_eq!(usage.detail.as_deref(), Some("$0.00 of $2000.00"));
    }

    #[test]
    fn subscription_view_applies_rate_and_token_units_and_defaults() {
        // used_percent is always the raw quota ratio (27.9% here), currency
        // conversion only affects the human-readable detail.
        // CNY site: 279 000 000 / 600 000 * 7.25 = ¥3371.25,
        // 1 000 000 000 / 600 000 * 7.25 = ¥12083.33.
        let view = subscription_view(&subscription_body(), &DisplayUnit::Cny, 600_000.0, 7.25)
            .unwrap();
        let usage = by_label(&view.metrics, "Usage");
        assert!((usage.used_percent.unwrap() - 27.9).abs() < 1e-9);
        assert_eq!(usage.detail.as_deref(), Some("¥3371.25 of ¥12083.33"));

        // Tokens sites keep raw quota counts without a currency symbol.
        let view = subscription_view(&subscription_body(), &DisplayUnit::Tokens, 500_000.0, 1.0)
            .unwrap();
        let usage = by_label(&view.metrics, "Usage");
        assert!((usage.used_percent.unwrap() - 27.9).abs() < 1e-9);
        assert_eq!(usage.detail.as_deref(), Some("279000000 of 1000000000"));

        // A broken per_unit falls back to the new-api default instead of
        // dividing by zero; still shows progress bar with correct USD amounts.
        let view = subscription_view(&subscription_body(), &DisplayUnit::Usd, 0.0, 1.0).unwrap();
        let usage = by_label(&view.metrics, "Usage");
        assert!((usage.used_percent.unwrap() - 27.9).abs() < 1e-9);
        assert_eq!(usage.detail.as_deref(), Some("$558.00 of $2000.00"));
    }
}
