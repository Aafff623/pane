//! Pure parsing and pagination helpers for WorkBuddy's official
//! `get-user-request-usage` ledger. No network, no Tauri, no UI and no ledger
//! writes: the probe in `.scratch/workbuddy-family-v2/issues/09-...` must pass
//! before callers wire this to a card.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

/// Hard cap on continuation rounds. A server that never advances the cursor
/// must stop here instead of looping forever.
pub const MAX_SWEEP_ROUNDS: usize = 100;
/// Detail rows kept for display; aggregation always runs on the full set.
pub const MAX_DETAIL_ROWS: usize = 1000;
const MAX_TEXT_CHARS: usize = 120;

#[derive(Clone, Debug, PartialEq)]
pub struct OfficialUsageRow {
    pub request_id: String,
    pub request_ts_ms: i64,
    pub model: Option<String>,
    pub client: Option<String>,
    pub credit: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OfficialUsagePage {
    pub rows: Vec<OfficialUsageRow>,
    /// Rows the server actually returned, including malformed ones that were
    /// skipped. Pagination must use this, not `rows.len()`.
    pub raw_rows: usize,
    /// Server-reported total. It is known to be capped by the server, so it
    /// must never drive pagination or a completeness claim.
    pub total: Option<u64>,
    pub page_size: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SweepDecision {
    Complete,
    Continue { start_ms: i64 },
    Stalled,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OfficialUsageSummary {
    pub total_credit: f64,
    pub by_day: BTreeMap<String, f64>,
    pub by_model: BTreeMap<String, f64>,
    pub by_client: BTreeMap<String, f64>,
}

/// Parse one response page. Bad rows are skipped (never turned into zero
/// usage); unknown fields are ignored, including prompt/input payloads.
pub fn parse_page(value: &Value, page_size: usize) -> Result<OfficialUsagePage, String> {
    if page_size == 0 {
        return Err("page size must be positive".into());
    }
    let data = value.get("data").ok_or("missing data")?;
    let (total, rows_value) = if data.is_array() {
        (value.get("total").and_then(as_u64), data)
    } else {
        let total = data.get("total").and_then(as_u64);
        let rows = data.get("data").ok_or("missing data.data")?;
        (total, rows)
    };
    let rows = rows_value.as_array().ok_or("data.data is not an array")?;
    let raw_rows = rows.len();
    let mut parsed = Vec::with_capacity(raw_rows);
    for row in rows {
        if let Some(row) = parse_row(row) {
            parsed.push(row);
        }
    }
    Ok(OfficialUsagePage {
        rows: parsed,
        raw_rows,
        total,
        page_size,
    })
}

fn parse_row(value: &Value) -> Option<OfficialUsageRow> {
    let object = value.as_object()?;
    let request_id = optional_text(object.get("requestId").or_else(|| object.get("request_id")))?;
    let request_ts_ms = timestamp_ms(object.get("requestTime").or_else(|| object.get("requestTs"))?)?;
    let credit = credit_value(object.get("credit")?)?;
    if !credit.is_finite() || credit < 0.0 {
        return None;
    }
    Some(OfficialUsageRow {
        request_id,
        request_ts_ms,
        model: optional_text(object.get("model")),
        client: optional_text(object.get("client")),
        credit,
    })
}

fn as_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.trim().parse::<u64>().ok())
}

fn optional_text(value: Option<&Value>) -> Option<String> {
    let raw = value?.as_str()?.trim();
    if raw.is_empty() {
        return None;
    }
    Some(raw.chars().take(MAX_TEXT_CHARS).collect())
}

fn credit_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn timestamp_ms(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64().and_then(number_ms),
        Value::String(text) => {
            let text = text.trim();
            if let Ok(number) = text.parse::<i64>() {
                return number_ms(number);
            }
            if let Ok(parsed) = DateTime::parse_from_rfc3339(text) {
                return Some(parsed.timestamp_millis());
            }
            for format in [
                "%Y-%m-%d %H:%M:%S%.f",
                "%Y-%m-%dT%H:%M:%S%.f",
                "%Y-%m-%d %H:%M:%S",
            ] {
                if let Ok(naive) = NaiveDateTime::parse_from_str(text, format) {
                    return Some(Utc.from_utc_datetime(&naive).timestamp_millis());
                }
            }
            None
        }
        _ => None,
    }
}

fn number_ms(value: i64) -> Option<i64> {
    if value <= 0 {
        None
    } else if value > 1_000_000_000_000 {
        Some(value)
    } else {
        Some(value * 1000)
    }
}

/// The server returns the earliest rows of the window and caps `total` at the
/// same size, so `pageNum > 1` is not a reliable way to get newer rows.
/// Advance `startTime` inclusively to the newest timestamp instead; a short
/// page means done, and no timestamp advance means stalled.
pub fn decide_next(page: &OfficialUsagePage, previous_start_ms: i64) -> SweepDecision {
    if page.raw_rows < page.page_size {
        return SweepDecision::Complete;
    }
    let Some(newest) = page.rows.iter().map(|row| row.request_ts_ms).max() else {
        return SweepDecision::Complete;
    };
    if newest <= previous_start_ms {
        SweepDecision::Stalled
    } else {
        SweepDecision::Continue { start_ms: newest }
    }
}

/// Merge pages in any order, deduping the boundary row on
/// `(requestId, requestTs)` and sorting oldest first.
pub fn merge_pages(pages: &[OfficialUsagePage]) -> Vec<OfficialUsageRow> {
    let mut rows = Vec::new();
    let mut seen = HashSet::new();
    for page in pages {
        for row in &page.rows {
            if seen.insert((row.request_id.clone(), row.request_ts_ms)) {
                rows.push(row.clone());
            }
        }
    }
    rows.sort_by(|left, right| {
        left.request_ts_ms
            .cmp(&right.request_ts_ms)
            .then_with(|| left.request_id.cmp(&right.request_id))
    });
    rows
}

/// The newest rows for a detail list; aggregation must still use the full set.
pub fn detail_rows(rows: &[OfficialUsageRow]) -> &[OfficialUsageRow] {
    let start = rows.len().saturating_sub(MAX_DETAIL_ROWS);
    &rows[start..]
}

pub fn sweep_round_limit_reached(rounds: usize) -> bool {
    rounds >= MAX_SWEEP_ROUNDS
}

pub fn aggregate(rows: &[OfficialUsageRow]) -> OfficialUsageSummary {
    let mut summary = OfficialUsageSummary::default();
    for row in rows {
        summary.total_credit += row.credit;
        let day = Utc
            .timestamp_millis_opt(row.request_ts_ms)
            .single()
            .map(|value| value.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".into());
        *summary.by_day.entry(day).or_default() += row.credit;
        *summary
            .by_model
            .entry(row.model.clone().unwrap_or_else(|| "unknown".into()))
            .or_default() += row.credit;
        *summary
            .by_client
            .entry(row.client.clone().unwrap_or_else(|| "unknown".into()))
            .or_default() += row.credit;
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(id: &str, timestamp: Value, credit: Value) -> Value {
        json!({
            "requestId": id,
            "requestTime": timestamp,
            "credit": credit,
            "model": "model-a",
            "client": "web",
            "input": "must not survive parsing"
        })
    }

    fn page(rows: Value, total: Option<u64>) -> Value {
        let mut data = json!({ "data": rows });
        if let Some(total) = total {
            data["total"] = json!(total);
        }
        json!({ "code": 0, "data": data })
    }

    fn official(id: &str, timestamp: i64, credit: f64) -> OfficialUsageRow {
        OfficialUsageRow {
            request_id: id.into(),
            request_ts_ms: timestamp,
            model: Some("model-a".into()),
            client: Some("web".into()),
            credit,
        }
    }

    #[test]
    fn parses_string_and_numeric_fields_and_skips_bad_rows() {
        let value = page(
            json!([
                row("r1", json!(1_700_000_000), json!("2.5")),
                row("r2", json!("2026-10-01T12:00:00Z"), json!(1)),
                row("bad", json!(1_700_000_001), json!(-1)),
                row("bad-ts", json!("not-a-time"), json!(1))
            ]),
            Some(4),
        );
        let parsed = parse_page(&value, 3000).expect("valid page");
        assert_eq!(parsed.rows.len(), 2);
        assert_eq!(parsed.rows[0].credit, 2.5);
        assert_eq!(parsed.rows[0].request_ts_ms, 1_700_000_000_000);
        assert_eq!(parsed.rows[1].request_ts_ms, 1_790_856_000_000);
        assert_eq!(parsed.total, Some(4));
    }

    #[test]
    fn accepts_the_bare_data_array_shape() {
        let parsed = parse_page(
            &json!({ "data": [row("r1", json!(1_700_000_000), json!(1))] }),
            3000,
        )
        .expect("array page");
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.total, None);
    }

    #[test]
    fn full_page_advances_short_page_completes_and_no_advance_stalls() {
        let full = OfficialUsagePage {
            rows: vec![official("r1", 10, 1.0), official("r2", 20, 1.0)],
            raw_rows: 2,
            total: Some(2),
            page_size: 2,
        };
        assert_eq!(decide_next(&full, 0), SweepDecision::Continue { start_ms: 20 });
        assert_eq!(decide_next(&full, 20), SweepDecision::Stalled);

        let short = OfficialUsagePage {
            rows: vec![official("r1", 20, 1.0)],
            raw_rows: 1,
            total: Some(2),
            page_size: 2,
        };
        assert_eq!(decide_next(&short, 0), SweepDecision::Complete);
    }

    #[test]
    fn a_dropped_row_does_not_turn_a_full_page_into_complete() {
        let value = page(
            json!([
                row("r1", json!(1_700_000_000), json!(1)),
                json!({ "requestId": "bad", "requestTime": "nope", "credit": 1 })
            ]),
            Some(2),
        );
        let parsed = parse_page(&value, 2).unwrap();
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.raw_rows, 2);
        assert_eq!(decide_next(&parsed, 0), SweepDecision::Continue { start_ms: 1_700_000_000_000 });
        assert!(sweep_round_limit_reached(MAX_SWEEP_ROUNDS));
        assert!(!sweep_round_limit_reached(MAX_SWEEP_ROUNDS - 1));
    }

    #[test]
    fn boundary_rows_are_deduped_and_detail_keeps_the_newest() {
        let first = OfficialUsagePage {
            rows: vec![official("r1", 10, 1.0), official("r2", 20, 1.0)],
            raw_rows: 2,
            total: None,
            page_size: 2,
        };
        let second = OfficialUsagePage {
            rows: vec![official("r2", 20, 1.0), official("r3", 30, 1.0)],
            raw_rows: 2,
            total: None,
            page_size: 2,
        };
        let merged = merge_pages(&[first, second]);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged.iter().map(|row| row.request_id.as_str()).collect::<Vec<_>>(), ["r1", "r2", "r3"]);
        assert_eq!(detail_rows(&merged).len(), 3);
    }

    #[test]
    fn aggregate_sums_by_day_model_and_client() {
        let rows = vec![
            official("r1", 1_790_000_000_000, 2.0),
            official("r2", 1_790_000_000_000, 3.0),
            OfficialUsageRow {
                request_id: "r3".into(),
                request_ts_ms: 1_790_100_000_000,
                model: None,
                client: None,
                credit: 4.0,
            },
        ];
        let summary = aggregate(&rows);
        assert_eq!(summary.total_credit, 9.0);
        assert_eq!(summary.by_day.get("2026-09-21"), Some(&5.0));
        assert_eq!(summary.by_model.get("model-a"), Some(&5.0));
        assert_eq!(summary.by_model.get("unknown"), Some(&4.0));
        assert_eq!(summary.by_client.get("web"), Some(&5.0));
    }
}
