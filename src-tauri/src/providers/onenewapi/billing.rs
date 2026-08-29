use super::super::Metric;
use serde_json::Value;

const USAGE_TO_USD: f64 = 0.01;
const UNLIMITED_SENTINEL: f64 = 100_000_000.0;
const MAX_JS_DATE_MS: i64 = 8_640_000_000_000_000;

pub fn metrics_from(sub: &Value, usage: Option<&Value>) -> Result<Vec<Metric>, String> {
    let limit = parse_limit(sub);
    let used = usage.and_then(parse_used);
    let mut metrics = Vec::new();
    match (limit, used) {
        (Some(limit), Some(used)) => {
            let pct = (used / limit * 100.0).clamp(0.0, 100.0);
            metrics.push(Metric::progress(
                "Usage",
                pct,
                Some(format!("${used:.2} of ${limit:.2}")),
            ));
        }
        (Some(limit), None) => {
            metrics.push(Metric::text("Limit", format!("${limit:.2}")));
        }
        (None, Some(used)) => {
            metrics.push(Metric::text("Used", format!("${used:.2}")));
        }
        (None, None) if subscription_is_unlimited(sub) => {
            metrics.push(Metric::text("Used", "$0.00".into()));
        }
        (None, None) => return Err("no billing data in response".into()),
    }
    if let Some(ms) = parse_access_until(sub.get("access_until")) {
        metrics.push(expiry_metric(ms));
    }
    Ok(metrics)
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

#[cfg(test)]
mod tests {
    use super::super::super::Metric;
    use super::metrics_from;
    use serde_json::{json, Value};

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
        let metrics = metrics_from(&captured_sub(), Some(&captured_usage())).unwrap();
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
        let metrics = metrics_from(&sub, None).unwrap();
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
        let metrics = metrics_from(&sub, Some(&usage)).unwrap();
        let usage = by_label(&metrics, "Usage");
        assert_eq!(usage.detail.as_deref(), Some("$0.00 of $10.00"));
    }

    #[test]
    fn exact_sentinel_is_unlimited_and_falls_through() {
        let sub = json!({
            "hard_limit_usd": 100000000,
            "system_hard_limit_usd": 12.5
        });
        let metrics = metrics_from(&sub, None).unwrap();
        assert_eq!(by_label(&metrics, "Limit").value.as_deref(), Some("$12.50"));

        let both_sentinel = json!({
            "hard_limit_usd": 100000000.0,
            "system_hard_limit_usd": 100000000
        });
        let usage = json!({"total_usage": 250});
        let metrics = metrics_from(&both_sentinel, Some(&usage)).unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(by_label(&metrics, "Used").value.as_deref(), Some("$2.50"));

        let above_sentinel = json!({"hard_limit_usd": 500_000_000.0});
        let metrics = metrics_from(&above_sentinel, Some(&usage)).unwrap();
        assert_eq!(by_label(&metrics, "Used").value.as_deref(), Some("$2.50"));

        let unlimited_no_usage = metrics_from(&both_sentinel, None).unwrap();
        assert_eq!(
            by_label(&unlimited_no_usage, "Used").value.as_deref(),
            Some("$0.00")
        );
    }

    #[test]
    fn used_only_and_limit_only_and_neither() {
        let used_only = metrics_from(&json!({}), Some(&json!({"total_usage": 1234}))).unwrap();
        assert_eq!(
            by_label(&used_only, "Used").value.as_deref(),
            Some("$12.34")
        );

        let limit_only = metrics_from(&json!({"hard_limit_usd": 9}), None).unwrap();
        assert_eq!(
            by_label(&limit_only, "Limit").value.as_deref(),
            Some("$9.00")
        );

        assert_eq!(
            expect_err(metrics_from(&json!({}), None)),
            "no billing data in response"
        );
        assert_eq!(
            expect_err(metrics_from(
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
            expect_err(metrics_from(&sub, Some(&usage))),
            "no billing data in response"
        );

        let sub = json!({
            "hard_limit_usd": true,
            "system_hard_limit_usd": null
        });
        let usage = json!({"total_usage": "10"});
        assert_eq!(
            expect_err(metrics_from(&sub, Some(&usage))),
            "no billing data in response"
        );
    }

    #[test]
    fn percent_is_clamped() {
        let sub = json!({"hard_limit_usd": 1});
        let over = json!({"total_usage": 50_000});
        let pct = by_label(&metrics_from(&sub, Some(&over)).unwrap(), "Usage")
            .used_percent
            .unwrap();
        assert_eq!(pct, 100.0);

        let zero = json!({"total_usage": 0});
        let pct = by_label(&metrics_from(&sub, Some(&zero)).unwrap(), "Usage")
            .used_percent
            .unwrap();
        assert_eq!(pct, 0.0);
    }

    #[test]
    fn expiry_omitted_when_missing_invalid_or_overflowing() {
        let sub = json!({"hard_limit_usd": 1});
        let metrics = metrics_from(&sub, None).unwrap();
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
            let metrics = metrics_from(&sub, None).unwrap();
            assert!(
                metrics.iter().all(|m| m.label != "Expiry"),
                "expiry should be omitted for {sub}"
            );
        }
    }

    #[test]
    fn usage_never_inherits_access_until() {
        let metrics = metrics_from(&captured_sub(), Some(&captured_usage())).unwrap();
        let usage = by_label(&metrics, "Usage");
        assert_eq!(usage.resets_at, None);
        assert_eq!(usage.period_ms, None);
        assert_eq!(
            by_label(&metrics, "Expiry").resets_at,
            Some(1_790_479_748_000)
        );
    }
}
