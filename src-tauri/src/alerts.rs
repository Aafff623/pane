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
    reset_soon: bool,
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
    let want_reset_soon = want("notifyResetSoon");
    if !(want_almost || want_close || want_runout || want_reset_soon) {
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

            // "Reset soon": a 5-hour rolling window with an hour or less to
            // go. Fires once per period and — unlike the pace alerts — also
            // on the first reading after launch: the countdown is
            // time-critical, and staying silent at launch would swallow
            // exactly the case the alert exists for.
            const FIVE_HOUR_MIN: i64 = 4 * 3_600_000;
            const FIVE_HOUR_MAX: i64 = 6 * 3_600_000;
            let reset_soon_now = match (metric.resets_at, metric.period_ms) {
                (Some(resets_at), Some(period)) if (FIVE_HOUR_MIN..=FIVE_HOUR_MAX).contains(&period) => {
                    let remain = resets_at - chrono::Utc::now().timestamp_millis();
                    remain > 0 && remain <= 60 * 60_000
                }
                _ => false,
            };
            let reset_soon_announce = want_reset_soon && reset_soon_now && !entry.reset_soon;

            let baseline = !entry.seen;
            entry.seen = true;

            if reset_soon_announce {
                let mins = metric
                    .resets_at
                    .map(|r| ((r - chrono::Utc::now().timestamp_millis()).max(0) / 60_000) as u64)
                    .unwrap_or(60);
                let name = snapshot.name.clone();
                let loc = crate::i18n::resolved_locale(cfg);
                alerts.push(Alert {
                    title: match loc {
                        "zh" => "5 小时窗口即将重置".into(),
                        "ru" => "5-часовое окно скоро обновится".into(),
                        _ => "5-hour window resetting soon".into(),
                    },
                    body: match loc {
                        "zh" => format!("{name} 的 5 小时窗口还剩 {mins} 分钟，到期后额度刷新。"),
                        "ru" => format!("5-часовое окно «{name}» обновится через {mins} мин."),
                        _ => format!("{name}'s 5-hour window resets in {mins} min — quota refreshes then."),
                    },
                });
            }

            if !baseline {
                let shown = crate::i18n::metric_label(cfg, &metric.label);
                let name = format!("{} {}", snapshot.name, shown);
                let loc = crate::i18n::resolved_locale(cfg);
                if want_runout && run_out_now && !entry.run_out {
                    alerts.push(Alert {
                        title: match loc {
                            "zh" => "将会用完".into(),
                            "ru" => "Кончится до сброса".into(),
                            _ => "Will Run Out".into(),
                        },
                        body: match loc {
                            "zh" => format!("{name} 按当前速度会在重置前用完。"),
                            "ru" => format!("{name} при текущем темпе исчерпается до сброса."),
                            _ => format!("{name} is on pace to hit its limit before the reset."),
                        },
                    });
                } else if want_close && close_now && !entry.close {
                    alerts.push(Alert {
                        title: match loc {
                            "zh" => "余量紧张".into(),
                            "ru" => "Запас на исходе".into(),
                            _ => "Cutting It Close".into(),
                        },
                        body: match loc {
                            "zh" => format!("{name} 按当前速度重置时大约只剩 {spare:.0}%。"),
                            "ru" => format!("{name} к сбросу останется примерно {spare:.0}%."),
                            _ => format!(
                                "{name} is on pace to finish with only ~{spare:.0}% spare."
                            ),
                        },
                    });
                }
                if want_almost && almost_now && !entry.almost_out {
                    alerts.push(Alert {
                        title: match loc {
                            "zh" => "即将用完".into(),
                            "ru" => "Почти кончилось".into(),
                            _ => "Almost Out".into(),
                        },
                        body: match loc {
                            "zh" => format!("{name} 剩余不足 10%（还剩 {left:.0}%）。"),
                            "ru" => format!("{name} осталось меньше 10% (ещё {left:.0}%)."),
                            _ => format!("{name} is under 10% remaining ({left:.0}% left)."),
                        },
                    });
                }
            }

            entry.almost_out = almost_now;
            entry.close = close_now;
            entry.run_out = run_out_now;
            entry.reset_soon = reset_soon_now;
        }
    }
    alerts
}

/// Drop every metric keyed as `{snapshot_id}:…`. Prefix is `id + ':'` so
/// `onenewapi@abc` does not also wipe `onenewapi@abcd`.
pub fn forget_snapshot(id: &str) {
    let prefix = format!("{id}:");
    let Ok(mut map) = states().lock() else {
        return;
    };
    map.retain(|k, _| !k.starts_with(&prefix));
}

#[cfg(test)]
pub fn insert_state_for_test(key: &str) {
    let Ok(mut map) = states().lock() else {
        return;
    };
    map.insert(key.to_string(), MetricState::default());
}

#[cfg(test)]
pub fn has_state_for_test(key: &str) -> bool {
    states()
        .lock()
        .map(|map| map.contains_key(key))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::Metric;

    const FIVE_H: i64 = 18_000_000;

    /// A 5-hour-window snapshot whose reset lands `remain_ms` from now.
    fn snap_5h(id: &str, remain_ms: i64) -> Snapshot {
        let now = chrono::Utc::now().timestamp_millis();
        Snapshot::ok(
            id,
            "Test",
            None,
            vec![Metric::progress("Session", 10.0, None)
                .with_reset(Some(now + remain_ms), Some(FIVE_H))],
        )
    }

    fn reset_soon_cfg() -> Value {
        serde_json::json!({ "locale": "en", "notifyResetSoon": true })
    }

    #[test]
    fn reset_soon_fires_on_first_reading() {
        // Unlike the pace alerts this one is not baseline-silent: the last
        // hour of a window is time-critical whatever moment it is noticed.
        let alerts = evaluate(&[snap_5h("t-rs-first", 30 * 60_000)], &reset_soon_cfg());
        assert_eq!(alerts.len(), 1, "{alerts:?}", alerts = alerts.len());
        assert_eq!(alerts[0].title, "5-hour window resetting soon");
        assert!(alerts[0].body.contains("Test"), "{}", alerts[0].body);
        assert!(alerts[0].body.contains("resets in"), "{}", alerts[0].body);
    }

    #[test]
    fn reset_soon_silent_far_from_reset() {
        assert!(evaluate(&[snap_5h("t-rs-far", 2 * 3_600_000)], &reset_soon_cfg()).is_empty());
    }

    #[test]
    fn reset_soon_only_for_five_hour_windows() {
        let now = chrono::Utc::now().timestamp_millis();
        let daily = Snapshot::ok(
            "t-rs-daily",
            "Test",
            None,
            vec![Metric::progress("Daily", 10.0, None)
                .with_reset(Some(now + 30 * 60_000), Some(24 * 3_600_000))],
        );
        assert!(evaluate(&[daily], &reset_soon_cfg()).is_empty());
    }

    #[test]
    fn reset_soon_fires_once_and_rearms_next_period() {
        let cfg = reset_soon_cfg();
        // Last hour of the window: announces.
        assert_eq!(evaluate(&[snap_5h("t-rs-cycle", 30 * 60_000)], &cfg).len(), 1);
        // Same window a few minutes later: silent.
        assert!(evaluate(&[snap_5h("t-rs-cycle", 25 * 60_000)], &cfg).is_empty());
        // New period, far from its reset: silent, slate cleared.
        assert!(evaluate(&[snap_5h("t-rs-cycle", 5 * 3_600_000)], &cfg).is_empty());
        // That period's own last hour announces again.
        assert_eq!(evaluate(&[snap_5h("t-rs-cycle", 50 * 60_000)], &cfg).len(), 1);
    }

    #[test]
    fn reset_soon_respects_toggle() {
        let cfg = serde_json::json!({ "locale": "en" });
        assert!(evaluate(&[snap_5h("t-rs-off", 30 * 60_000)], &cfg).is_empty());
    }

    #[test]
    fn forget_snapshot_drops_that_id_only() {
        insert_state_for_test("onenewapi@ticket07-abc:Usage");
        insert_state_for_test("onenewapi@ticket07-abc:Expiry");
        insert_state_for_test("onenewapi@ticket07-abcd:Usage");
        forget_snapshot("onenewapi@ticket07-abc");
        assert!(!has_state_for_test("onenewapi@ticket07-abc:Usage"));
        assert!(!has_state_for_test("onenewapi@ticket07-abc:Expiry"));
        assert!(has_state_for_test("onenewapi@ticket07-abcd:Usage"));
        forget_snapshot("onenewapi@ticket07-abcd");
    }
}
