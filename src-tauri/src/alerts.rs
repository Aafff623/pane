//! Pace-based notification rules, mirroring the Mac app:
//! - "Almost Out" — a metric drops under 10% remaining.
//! - "Cutting It Close" — projected to finish the period with <10% spare.
//! - "Will Run Out" — projected to hit the limit before the reset.
//!
//! Anti-spam: an alert fires only when a quota *worsens while the app is
//! running* (the first reading after launch is a silent baseline), fires
//! once per state, re-arms if the metric recovers, and the slate is wiped
//! when a new reset period begins. State is in-memory by design — matching
//! the Mac's "already-bad at launch won't alert" behavior.

use crate::providers::Snapshot;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Default, Clone)]
struct MetricState {
    resets_at: Option<i64>,
    seen: bool,
    almost_out: bool,
    close: bool,
    run_out: bool,
}

fn states() -> &'static Mutex<HashMap<String, MetricState>> {
    static STATES: OnceLock<Mutex<HashMap<String, MetricState>>> = OnceLock::new();
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct Alert {
    pub title: String,
    pub body: String,
}

#[derive(PartialEq, Clone, Copy)]
enum Verdict {
    Ok,
    Close,
    RunOut,
}

/// Same straight-line projection the UI uses for bar colors.
fn verdict(used: f64, resets_at: Option<i64>, period_ms: Option<i64>) -> (Verdict, f64) {
    let used = used.clamp(0.0, 100.0);
    let left = 100.0 - used;
    if left < 0.5 {
        return (Verdict::RunOut, 0.0);
    }
    let (Some(resets_at), Some(period_ms)) = (resets_at, period_ms) else {
        return (Verdict::Ok, left);
    };
    if period_ms <= 0 {
        return (Verdict::Ok, left);
    }
    let now = chrono::Utc::now().timestamp_millis();
    let remain = (resets_at - now).max(0);
    let elapsed = period_ms - remain;
    let frac = elapsed as f64 / period_ms as f64;
    if frac < 0.05 || elapsed < 5 * 60_000 {
        return (Verdict::Ok, left);
    }
    let projected = used / frac;
    let spare = (100.0 - projected).max(0.0);
    if projected >= 100.0 {
        (Verdict::RunOut, 0.0)
    } else if projected >= 90.0 {
        (Verdict::Close, spare.max(1.0))
    } else {
        (Verdict::Ok, spare)
    }
}

/// A reset time that moved by more than ten minutes means a new period
/// (small drifts happen because some providers report "seconds from now").
fn period_changed(old: Option<i64>, new: Option<i64>) -> bool {
    match (old, new) {
        (Some(a), Some(b)) => (a - b).abs() > 10 * 60_000,
        _ => false,
    }
}

pub fn evaluate(snapshots: &[Snapshot], cfg: &Value) -> Vec<Alert> {
    let want = |key: &str| cfg.get(key).and_then(Value::as_bool).unwrap_or(false);
    let want_almost = want("notifyAlmostOut");
    let want_close = want("notifyCuttingClose");
    let want_runout = want("notifyWillRunOut");
    if !(want_almost || want_close || want_runout) {
        return Vec::new();
    }

    let mut alerts = Vec::new();
    let Ok(mut map) = states().lock() else { return alerts };

    for snapshot in snapshots.iter().filter(|s| s.status == "ok") {
        for metric in snapshot.metrics.iter().filter(|m| m.kind == "progress") {
            // Restored Kimi API rows are last-known, not live — don't
            // fire Almost Out off a wallet timeout.
            if snapshot.id == "kimi"
                && snapshot.warning.is_some()
                && matches!(metric.label.as_str(), "API" | "Credits used")
            {
                continue;
            }
            let Some(used) = metric.used_percent else { continue };
            let key = format!("{}:{}", snapshot.id, metric.label);
            let entry = map.entry(key).or_default();

            if period_changed(entry.resets_at, metric.resets_at) {
                *entry = MetricState::default();
            }
            entry.resets_at = metric.resets_at;

            let left = (100.0 - used.clamp(0.0, 100.0)).max(0.0);
            let (v, spare) = verdict(used, metric.resets_at, metric.period_ms);
            let almost_now = left < 10.0;
            let close_now = v == Verdict::Close;
            let run_out_now = v == Verdict::RunOut;
            let baseline = !entry.seen;
            entry.seen = true;

            if !baseline {
                let name = format!("{} {}", snapshot.name, metric.label);
                if want_runout && run_out_now && !entry.run_out {
                    alerts.push(Alert {
                        title: "Will Run Out".into(),
                        body: format!("{name} is on pace to hit its limit before the reset."),
                    });
                } else if want_close && close_now && !entry.close {
                    alerts.push(Alert {
                        title: "Cutting It Close".into(),
                        body: format!("{name} is on pace to finish with only ~{spare:.0}% spare."),
                    });
                }
                if want_almost && almost_now && !entry.almost_out {
                    alerts.push(Alert {
                        title: "Almost Out".into(),
                        body: format!("{name} is under 10% remaining ({left:.0}% left)."),
                    });
                }
            }

            entry.almost_out = almost_now;
            entry.close = close_now;
            entry.run_out = run_out_now;
        }
    }
    alerts
}
