//! One pace calculation shared by alerts and the card/API surfaces.
//!
//! Pace is a projection, not a measurement. A young window stays `unknown`
//! instead of turning noise into a scary Close/RunOut state.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaceState {
    Ahead,
    Close,
    Over,
    Spent,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pace {
    pub state: PaceState,
    pub projected_used: Option<f64>,
    pub run_out_at: Option<i64>,
    pub settled: bool,
    pub elapsed_percent: Option<f64>,
}

const MIN_SETTLED_MS: i64 = 15 * 60_000;
const MIN_SETTLED_FRACTION: f64 = 0.05;

pub fn compute(used_percent: f64, resets_at: Option<i64>, period_ms: Option<i64>, now_ms: i64) -> Pace {
    let used = used_percent.clamp(0.0, 100.0);
    if used >= 99.5 {
        return Pace {
            state: PaceState::Spent,
            projected_used: Some(100.0),
            run_out_at: Some(now_ms),
            settled: true,
            elapsed_percent: Some(100.0),
        };
    }
    let (Some(resets_at), Some(period_ms)) = (resets_at, period_ms) else {
        return unknown();
    };
    if period_ms <= 0 {
        return unknown();
    }
    let remain = (resets_at - now_ms).max(0);
    let elapsed = period_ms - remain;
    if elapsed <= 0 || elapsed < MIN_SETTLED_MS {
        return unknown();
    }
    let frac = elapsed as f64 / period_ms as f64;
    if !(MIN_SETTLED_FRACTION..=1.0).contains(&frac) {
        return unknown();
    }
    let projected = used / frac;
    if !projected.is_finite() {
        return unknown();
    }
    let state = if projected >= 100.0 {
        PaceState::Over
    } else if projected >= 90.0 {
        PaceState::Close
    } else {
        PaceState::Ahead
    };
    let run_out_at = if used > 0.0 && projected > 100.0 {
        let rate_per_ms = used / elapsed as f64;
        let ms_to_limit = (100.0 - used) / rate_per_ms;
        (ms_to_limit <= remain as f64).then(|| now_ms + ms_to_limit.round() as i64)
    } else {
        None
    };
    Pace {
        state,
        projected_used: Some(projected.min(100.0)),
        run_out_at,
        settled: true,
        elapsed_percent: Some(frac * 100.0),
    }
}

fn unknown() -> Pace {
    Pace {
        state: PaceState::Unknown,
        projected_used: None,
        run_out_at: None,
        settled: false,
        elapsed_percent: None,
    }
}

pub fn attach(metrics: &mut [crate::providers::Metric], now_ms: i64) {
    for metric in metrics.iter_mut().filter(|metric| metric.kind == "progress") {
        let Some(used) = metric.used_percent else { continue };
        metric.pace = Some(compute(used, metric.resets_at, metric.period_ms, now_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3_600_000;

    #[test]
    fn young_windows_stay_unknown() {
        let now = 1_000_000;
        let pace = compute(20.0, Some(now + 4 * HOUR + 55 * 60_000), Some(5 * HOUR), now);
        assert_eq!(pace.state, PaceState::Unknown);
        assert!(!pace.settled);
    }

    #[test]
    fn ahead_close_and_over_are_distinct() {
        let now = 10_000_000;
        let period = 5 * HOUR;
        let ahead = compute(10.0, Some(now + period - 2 * HOUR), Some(period), now);
        assert_eq!(ahead.state, PaceState::Ahead);
        let close = compute(36.0, Some(now + 3 * HOUR), Some(period), now);
        assert_eq!(close.state, PaceState::Close);
        let over = compute(95.0, Some(now + 3 * HOUR), Some(period), now);
        assert_eq!(over.state, PaceState::Over);
        assert!(over.run_out_at.is_some());
    }

    #[test]
    fn spent_is_immediate() {
        let now = 1_000;
        let pace = compute(100.0, Some(now + HOUR), Some(5 * HOUR), now);
        assert_eq!(pace.state, PaceState::Spent);
        assert!(pace.settled);
    }

    #[test]
    fn missing_reset_metadata_is_unknown() {
        assert_eq!(compute(50.0, None, Some(HOUR), 1).state, PaceState::Unknown);
        assert_eq!(compute(50.0, Some(2), None, 1).state, PaceState::Unknown);
    }
}
