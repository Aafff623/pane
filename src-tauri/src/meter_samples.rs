//! Window-level meter samples for the per-card burn curve (#55-04). Daily
//! credit history already exists; this keeps the last 45 days of 5-minute
//! `remaining %` points per (card, metric) so the UI can fit a straight line
//! and answer "when does this run out at the current pace?".

use crate::providers::Snapshot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const VERSION: u32 = 1;
const MAX_POINTS: usize = 6_000;
const KEEP_MS: i64 = 45 * 86_400_000;
const MIN_GAP_MS: i64 = 5 * 60_000;
const MIN_FIT_POINTS: usize = 3;
const MIN_FIT_SPAN_MS: i64 = 3_600_000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MeterPoint {
    pub at: i64,
    /// Remaining percent (100 - used), so a falling line means burning down.
    pub left: f64,
    pub start: i64,
    pub resets_at: i64,
}

#[derive(Serialize, Deserialize, Default)]
struct MeterFile {
    version: u32,
    entries: BTreeMap<String, Vec<MeterPoint>>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BurnFit {
    pub eta_ms: Option<i64>,
    pub slope_per_hour: f64,
    pub points: usize,
    pub span_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeterSeries {
    pub id: String,
    pub label: String,
    pub points: Vec<MeterPoint>,
    pub fit: Option<BurnFit>,
}

fn path() -> PathBuf {
    crate::providers::config_dir().join("meter_samples.json")
}

fn load() -> MeterFile {
    let raw = fs::read_to_string(path()).unwrap_or_default();
    let mut file: MeterFile = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .unwrap_or_else(|_| MeterFile {
            version: VERSION,
            entries: BTreeMap::new(),
        });
    if file.version > VERSION {
        file = MeterFile {
            version: VERSION,
            entries: BTreeMap::new(),
        };
    }
    file.version = VERSION;
    file
}

fn save(file: &MeterFile) {
    let Ok(raw) = serde_json::to_string(file) else { return };
    let temp = path().with_extension("json.tmp");
    if fs::write(&temp, raw).is_ok() {
        let _ = fs::rename(&temp, path());
    }
}

/// Same value keeps the first and last point (a flat meter still has a
/// span); a changed value inside the 5-minute bucket updates the last point
/// instead of adding a noisy one. A different reset/start is a new cycle.
fn push_point(points: &mut Vec<MeterPoint>, mut point: MeterPoint) {
    if let Some(last) = points.last() {
        if last.resets_at != point.resets_at || last.start != point.start {
            points.clear();
        }
    }
    let len = points.len();
    if let Some(last) = points.last_mut() {
        if last.left == point.left {
            if len == 1 {
                points.push(point);
            } else {
                last.at = point.at;
            }
            return;
        }
        if point.at <= last.at || point.at - last.at < MIN_GAP_MS {
            last.at = point.at.max(last.at);
            last.left = point.left;
            return;
        }
    }
    points.push(point);
    if points.len() > MAX_POINTS {
        let drop = points.len() - MAX_POINTS;
        points.drain(0..drop);
    }
}

fn prune(file: &mut MeterFile, now_ms: i64) {
    let cutoff = now_ms - KEEP_MS;
    file.entries.retain(|_, points| {
        points.retain(|point| point.at >= cutoff);
        points.len() >= 2
    });
}

/// Least squares over `(minutes, left%)`. Fewer than three points or less
/// than an hour of span is not a trend; a flat/rising line has no run-out.
pub fn fit(points: &[MeterPoint], now_ms: i64) -> Option<BurnFit> {
    let mut usable: Vec<&MeterPoint> = points.iter().filter(|point| point.at <= now_ms).collect();
    usable.sort_by_key(|point| point.at);
    if usable.len() < MIN_FIT_POINTS {
        return None;
    }
    let first = usable[0].at;
    let span_ms = usable[usable.len() - 1].at - first;
    if span_ms < MIN_FIT_SPAN_MS {
        return None;
    }
    let xs: Vec<f64> = usable.iter().map(|point| (point.at - first) as f64 / 60_000.0).collect();
    let ys: Vec<f64> = usable.iter().map(|point| point.left).collect();
    let x_mean = xs.iter().sum::<f64>() / xs.len() as f64;
    let y_mean = ys.iter().sum::<f64>() / ys.len() as f64;
    let denominator: f64 = xs.iter().map(|x| (x - x_mean).powi(2)).sum();
    if denominator <= f64::EPSILON {
        return None;
    }
    let slope = xs
        .iter()
        .zip(ys.iter())
        .map(|(x, y)| (x - x_mean) * (y - y_mean))
        .sum::<f64>()
        / denominator;
    if !slope.is_finite() {
        return None;
    }
    let last_left = *ys.last().unwrap_or(&0.0);
    let eta_ms = if slope < 0.0 && last_left > 0.0 {
        Some(now_ms + ((last_left / -slope) * 60_000.0).round() as i64)
    } else {
        None
    };
    Some(BurnFit {
        eta_ms,
        slope_per_hour: slope * 60.0,
        points: usable.len(),
        span_ms,
    })
}

/// Called after every successful refresh pass. Failed/stale snapshots and
/// metrics without a reset window never enter the store.
pub fn record(snapshots: &[Snapshot], now_ms: i64) {
    let mut file = load();
    for snapshot in snapshots.iter().filter(|snapshot| snapshot.status == "ok") {
        for metric in &snapshot.metrics {
            if metric.kind != "progress" {
                continue;
            }
            let (Some(used), Some(resets_at), Some(period_ms)) =
                (metric.used_percent, metric.resets_at, metric.period_ms)
            else {
                continue;
            };
            if resets_at <= now_ms || period_ms <= 0 {
                continue;
            }
            let key = format!("{}|{}", snapshot.id, metric.label);
            push_point(
                file.entries.entry(key).or_default(),
                MeterPoint {
                    at: now_ms,
                    left: 100.0 - used.clamp(0.0, 100.0),
                    start: resets_at - period_ms,
                    resets_at,
                },
            );
        }
    }
    prune(&mut file, now_ms);
    save(&file);
}

pub fn series() -> Vec<MeterSeries> {
    let now = chrono::Utc::now().timestamp_millis();
    load()
        .entries
        .into_iter()
        .filter_map(|(key, points)| {
            let (id, label) = key.split_once('|')?;
            let fit = fit(&points, now);
            Some(MeterSeries {
                id: id.to_string(),
                label: label.to_string(),
                points,
                fit,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(at: i64, left: f64, reset: i64) -> MeterPoint {
        MeterPoint {
            at,
            left,
            start: reset - 86_400_000,
            resets_at: reset,
        }
    }

    #[test]
    fn same_value_keeps_first_and_last_and_new_cycle_clears() {
        let mut points = Vec::new();
        push_point(&mut points, point(0, 80.0, 9_000_000));
        push_point(&mut points, point(60_000, 80.0, 9_000_000));
        push_point(&mut points, point(120_000, 80.0, 9_000_000));
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].at, 0);
        assert_eq!(points[1].at, 120_000);
        // 400 s later: past the 5-minute bucket, so this is a new point.
        push_point(&mut points, point(500_000, 70.0, 9_000_000));
        assert_eq!(points.len(), 3);
        push_point(&mut points, point(580_000, 90.0, 18_000_000));
        assert_eq!(points.len(), 1);
    }

    #[test]
    fn fit_uses_three_points_over_an_hour_and_extrapolates_zero() {
        let now = 7_200_000;
        let points = [
            point(0, 80.0, 9_000_000),
            point(3_600_000, 60.0, 9_000_000),
            point(7_200_000, 40.0, 9_000_000),
        ];
        let result = fit(&points, now).expect("fit");
        assert_eq!(result.points, 3);
        assert!((result.slope_per_hour - (-20.0)).abs() < 0.001);
        let eta = result.eta_ms.expect("eta");
        assert!((eta - (7_200_000 + 2 * 3_600_000)).abs() < 1_000);
        assert!(fit(&points[..2], now).is_none());
        let flat = [
            point(0, 50.0, 9_000_000),
            point(3_600_000, 50.0, 9_000_000),
            point(7_200_000, 50.0, 9_000_000),
        ];
        assert!(fit(&flat, now).unwrap().eta_ms.is_none());
    }
}
