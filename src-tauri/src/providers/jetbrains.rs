//! JetBrains AI — local-only card reading the IDE's own quota file
//! (mechanism per CodexBar's JetBrains provider; no network, no key).
//!
//! Every JetBrains IDE writes `<config>/JetBrains/<product>/options/
//! AIAssistantQuotaManager2.xml` (Android Studio under `Google/`); the
//! `quotaInfo` / `nextRefill` attributes are HTML-encoded JSON. The most
//! recently modified file wins — that is the IDE the user actually uses.

use super::{Metric, Snapshot};
use std::path::PathBuf;

const ID: &str = "jetbrains";
const NAME: &str = "JetBrains AI";

pub async fn snapshot() -> Snapshot {
    match fetch() {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Pure local probe for the Customize gear panel (no network).
pub fn local_credential_hint() -> Option<String> {
    (!quota_files().is_empty()).then(|| "JetBrains IDE quota file".to_string())
}

fn fetch() -> Result<Snapshot, String> {
    let Some(path) = quota_files().into_iter().max_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "JetBrains AI quota not found. Use a JetBrains IDE with AI Assistant once, then refresh.",
        ));
    };
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("read quota file: {e}"))?;
    parse_quota_file(&raw)
}

/// `<IDE config root>` → every product's quota file (JetBrains + Google
/// for Android Studio), newest-first candidates.
fn quota_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(cfg) = dirs::config_dir() {
        for vendor in ["JetBrains", "Google"] {
            let base = cfg.join(vendor);
            let Ok(entries) = std::fs::read_dir(&base) else { continue };
            for entry in entries.flatten() {
                let options = entry.path().join("options").join("AIAssistantQuotaManager2.xml");
                if options.is_file() {
                    out.push(options);
                }
            }
        }
    }
    out
}

/// XML attribute values are HTML-encoded JSON — decode, then parse.
fn decode_attr(raw: &str) -> String {
    raw.replace("&quot;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

fn attr_json(raw_xml: &str, attr: &str) -> Option<serde_json::Value> {
    // No XML crate: the attribute always sits on the single <component> or
    // <option> line; find `attr="…"` and decode. Values never contain a
    // raw quote (they are HTML-encoded), so the closing quote is safe.
    let marker = format!("{attr}=\"");
    let start = raw_xml.find(&marker)? + marker.len();
    let end = raw_xml[start..].find('"')? + start;
    serde_json::from_str(&decode_attr(&raw_xml[start..end])).ok()
}

fn parse_quota_file(raw: &str) -> Result<Snapshot, String> {
    let quota = attr_json(raw, "quotaInfo").ok_or("quota file has no quotaInfo")?;
    let refill = attr_json(raw, "nextRefill");

    let tariff_current = quota.pointer("/tariffQuota/current").and_then(v_f64);
    let tariff_maximum = quota.pointer("/tariffQuota/maximum").and_then(v_f64);
    let mut metrics = Vec::new();
    if let (Some(used), Some(total)) = (tariff_current, tariff_maximum) {
        if total > 0.0 {
            // The IDE's "monthly credits left" view: tariff only, top-ups
            // excluded from the percentage.
            let pct = (used / total * 100.0).clamp(0.0, 100.0);
            let reset = refill
                .as_ref()
                .and_then(|r| r.get("next"))
                .and_then(|n| n.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.timestamp_millis());
            metrics.push(
                Metric::progress("Monthly credits", pct, Some(format!("{used:.0} of {total:.0} credits used")))
                    .with_reset(reset, None),
            );
        }
    }
    // Top-up credits are a separate pool with its own availability.
    let topup = quota.pointer("/topUpQuota");
    if let Some(available) = topup.and_then(|t| t.get("available")).and_then(v_f64) {
        let maximum = topup.and_then(|t| t.get("maximum")).and_then(v_f64).unwrap_or(available);
        if maximum > 0.0 || available > 0.0 {
            metrics.push(Metric::text("Top-up credits", format!("{available:.0} of {maximum:.0} left")));
        }
    }
    if let Some(until) = quota.get("until").and_then(|u| u.as_str()) {
        metrics.push(Metric::text("Subscription", until.to_string()));
    }
    if metrics.is_empty() {
        return Err("quota file carried no usable numbers".into());
    }
    Ok(Snapshot::ok(ID, NAME, Some("AI Assistant".into()), metrics))
}

fn v_f64(v: &serde_json::Value) -> Option<f64> {
    v.as_f64().or_else(|| v.as_i64().map(|n| n as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quota_xml(quota: &str, refill: &str) -> String {
        let enc = |s: &str| s.replace('"', "&quot;").replace('<', "&lt;");
        format!(
            "<application><component name=\"AIAssistantQuotaManager2\" quotaInfo=\"{}\" nextRefill=\"{}\" /></application>",
            enc(quota),
            enc(refill)
        )
    }

    #[test]
    fn parses_the_ide_quota_xml() {
        let raw = quota_xml(
            r#"{"type":"Available","current":420,"maximum":1000,"tariffQuota":{"current":420,"maximum":1000,"available":580},"topUpQuota":{"current":50,"maximum":200,"available":150},"until":"2027-01-15"}"#,
            r#"{"type":"Known","next":"2026-11-01T00:00:00Z","tariff":{"amount":1000,"duration":"PT720H"}}"#,
        );
        let snap = parse_quota_file(&raw).unwrap();
        assert_eq!(snap.plan.as_deref(), Some("AI Assistant"));
        let monthly = &snap.metrics[0];
        assert_eq!(monthly.label, "Monthly credits");
        assert_eq!(monthly.used_percent, Some(42.0));
        assert_eq!(monthly.resets_at, Some(1_793_491_200_000));
        assert_eq!(snap.metrics[1].value.as_deref(), Some("150 of 200 left"));
        assert_eq!(snap.metrics[2].value.as_deref(), Some("2027-01-15"));
    }

    #[test]
    fn missing_quota_info_is_an_error() {
        assert!(parse_quota_file("<application></application>").is_err());
        // HTML entities must round-trip through the decoder.
        assert_eq!(decode_attr(&enc_for_test()), r#"{"a":"b"c}"#);
    }

    fn enc_for_test() -> String {
        r#"{"a":"b"c}"#.replace('"', "&quot;")
    }
}
