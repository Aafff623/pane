//! `/v1/limits` — a stable, machine-readable limits envelope
//! (`openusage.limits.v1`) kept separate from the UI-shaped `/v1/usage` rows.
//! Only structured values enter; free-text detail is never parsed, and a
//! metric without a known structured reading is omitted rather than zeroed.

use crate::providers::Snapshot;
use serde_json::{json, Map, Value};

pub const SCHEMA: &str = "openusage.limits.v1";

/// Exact card id first, otherwise every card in the same provider family.
/// `/v1/usage/:id` keeps its exact-only behavior for macOS parity.
pub fn select_snapshots(snapshots: &[Snapshot], id: &str) -> Vec<Snapshot> {
    let exact = snapshots.iter().any(|snapshot| snapshot.id == id);
    if exact {
        return snapshots
            .iter()
            .filter(|snapshot| snapshot.id == id)
            .cloned()
            .collect();
    }
    let family = crate::provider_catalog::family_of(id);
    snapshots
        .iter()
        .filter(|snapshot| crate::provider_catalog::family_of(&snapshot.id) == family)
        .cloned()
        .collect()
}

fn effective_resets_at(metric: &crate::providers::Metric) -> Option<i64> {
    if metric.pace.as_ref().is_some_and(|pace| pace.not_started) {
        return None;
    }
    metric.resets_at
}

fn iso(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|date| date.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_default()
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

pub fn limits_payload(snapshots: &[Snapshot], generated_at: &str) -> Value {
    let mut providers = Map::new();
    let mut errors = Vec::new();
    for snapshot in snapshots {
        if snapshot.status != "ok" {
            errors.push(json!({
                "providerId": snapshot.id,
                "code": snapshot.status,
                "message": snapshot.error.as_deref().map(crate::httpapi::sanitize_text),
            }));
            continue;
        }
        let mut resources = Map::new();
        for metric in &snapshot.metrics {
            if let Some(quota) = metric.quota.as_ref().filter(|quota| quota.valid()) {
                let mut row = Map::new();
                row.insert("kind".into(), json!(if quota.limit.is_some() { "consumption" } else { "balance" }));
                row.insert("unit".into(), json!(quota.unit));
                if quota.limit.is_some() {
                    let limit = quota.limit.unwrap_or(0.0);
                    let remaining = (limit - quota.used).max(0.0);
                    row.insert("used".into(), json!(quota.used));
                    row.insert("limit".into(), json!(limit));
                    row.insert("remaining".into(), json!(remaining));
                    if limit > 0.0 {
                        row.insert("utilization".into(), json!((quota.used / limit).clamp(0.0, 1.0)));
                    }
                } else {
                    row.insert("remaining".into(), json!(quota.used));
                    row.insert("available".into(), json!(quota.used > 0.0));
                }
                resources.insert(metric.label.clone(), Value::Object(row));
                continue;
            }
            if let Some(reading) = metric.usage.as_ref().filter(|reading| reading.valid()) {
                let mut row = Map::new();
                row.insert("kind".into(), json!("consumption"));
                row.insert("unit".into(), json!(reading.unit));
                row.insert("used".into(), json!(reading.used));
                if let Some(total) = reading.total.filter(|total| *total > 0.0) {
                    row.insert("limit".into(), json!(total));
                    row.insert("remaining".into(), json!((total - reading.used).max(0.0)));
                    row.insert("utilization".into(), json!((reading.used / total).clamp(0.0, 1.0)));
                }
                if let Some(resets_at) = effective_resets_at(metric) {
                    row.insert("resetsAt".into(), json!(iso(resets_at)));
                }
                if let Some(period_ms) = metric.period_ms.filter(|period| *period > 0) {
                    row.insert("windowSeconds".into(), json!(period_ms / 1000));
                }
                resources.insert(metric.label.clone(), Value::Object(row));
                continue;
            }
            let Some(used) = metric.used_percent.and_then(finite) else { continue };
            let used = used.clamp(0.0, 100.0);
            let mut row = Map::new();
            row.insert("kind".into(), json!("consumption"));
            row.insert("unit".into(), json!("percent"));
            row.insert("used".into(), json!(used));
            row.insert("limit".into(), json!(100.0));
            row.insert("remaining".into(), json!((100.0 - used).max(0.0)));
            row.insert("utilization".into(), json!(used / 100.0));
            if let Some(resets_at) = effective_resets_at(metric) {
                row.insert("resetsAt".into(), json!(iso(resets_at)));
            }
            if let Some(period_ms) = metric.period_ms.filter(|period| *period > 0) {
                row.insert("windowSeconds".into(), json!(period_ms / 1000));
            }
            resources.insert(metric.label.clone(), Value::Object(row));
        }
        if resources.is_empty() {
            continue;
        }
        providers.insert(
            snapshot.id.clone(),
            json!({
                "displayName": snapshot.name,
                "plan": snapshot.plan,
                "fetchedAt": generated_at,
                "stale": snapshot.stale,
                "resources": resources,
            }),
        );
    }
    json!({
        "schema": SCHEMA,
        "generatedAt": generated_at,
        "providers": providers,
        "errors": errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{Metric, Snapshot};

    #[test]
    fn untouched_window_has_no_reset_countdown() {
        // Snapshot::ok attaches pace itself; make the window genuinely full
        // (0% with a reset exactly one period ahead) so the production rule
        // marks it not_started.
        let now = chrono::Utc::now().timestamp_millis();
        let mut metric = Metric::progress("Session", 0.0, None);
        metric.resets_at = Some(now + 604_800_000);
        metric.period_ms = Some(604_800_000);
        let snapshot = Snapshot::ok("antigravity", "Antigravity", None, vec![metric]);
        assert!(snapshot.metrics[0]
            .pace
            .as_ref()
            .is_some_and(|pace| pace.not_started));
        let payload = limits_payload(&[snapshot], "2026-10-26T00:00:00Z");
        let session = &payload["providers"]["antigravity"]["resources"]["Session"];
        assert!(session.get("resetsAt").is_none(), "{session}");
    }

    #[test]
    fn balance_quota_projects_as_a_balance_resource() {
        let mut metric = Metric::text("Extra credits", "$7.50 left".into());
        metric.quota = Some(crate::usage_units::QuotaReading {
            unit: "usd".into(),
            used: 7.5,
            limit: None,
        });
        let snapshot = Snapshot::ok("codex", "Codex", None, vec![metric]);
        let payload = limits_payload(&[snapshot], "2026-10-26T00:00:00Z");
        let credits = &payload["providers"]["codex"]["resources"]["Extra credits"];
        assert_eq!(credits["kind"], "balance");
        assert_eq!(credits["unit"], "usd");
        assert_eq!(credits["remaining"], 7.5);
        assert_eq!(credits["available"], true);
        assert!(credits.get("used").is_none());
        assert!(credits.get("limit").is_none());
    }

    #[test]
    fn id_selection_prefers_exact_then_falls_back_to_family() {
        let a = Snapshot::ok(
            "onenewapi@abc",
            "Site A",
            None,
            vec![Metric::progress("Usage", 10.0, None)],
        );
        let b = Snapshot::ok(
            "onenewapi@def",
            "Site B",
            None,
            vec![Metric::progress("Usage", 20.0, None)],
        );
        let claude = Snapshot::ok("claude", "Claude", None, vec![Metric::progress("Session", 1.0, None)]);
        let all = vec![a, b, claude];
        let exact = select_snapshots(&all, "onenewapi@abc");
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].id, "onenewapi@abc");
        let family = select_snapshots(&all, "onenewapi");
        assert_eq!(family.len(), 2);
        assert!(select_snapshots(&all, "missing").is_empty());
    }

    #[test]
    fn envelope_projects_progress_usage_and_omits_unknown_rows() {
        let mut metric = Metric::progress("Weekly", 42.0, None);
        metric.resets_at = Some(1_790_000_000_000);
        metric.period_ms = Some(604_800_000);
        let usage = Metric {
            usage: Some(crate::usage_units::UsageReading {
                unit: crate::usage_units::UsageUnit::Tokens,
                used: 120.0,
                total: Some(1000.0),
                scope: String::new(),
            }),
            ..Metric::text("Tokens", "120".into())
        };
        let snapshot = Snapshot::ok("claude", "Claude", None, vec![metric, usage]);
        let payload = limits_payload(&[snapshot], "2026-10-26T00:00:00Z");
        assert_eq!(payload["schema"], SCHEMA);
        let weekly = &payload["providers"]["claude"]["resources"]["Weekly"];
        assert_eq!(weekly["used"], 42.0);
        assert_eq!(weekly["limit"], 100.0);
        assert_eq!(weekly["utilization"], 0.42);
        assert_eq!(weekly["windowSeconds"], 604_800);
        let tokens = &payload["providers"]["claude"]["resources"]["Tokens"];
        assert_eq!(tokens["remaining"], 880.0);
        // A text-only row never invents a quota.
        let text_only = Snapshot::ok("x", "X", None, vec![Metric::text("Note", "hi".into())]);
        let payload = limits_payload(&[text_only], "2026-10-26T00:00:00Z");
        assert!(payload["providers"].as_object().unwrap().is_empty());
    }
}
