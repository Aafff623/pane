//! 闪电说 desktop membership usage.
use super::{http, json_body, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "shandianshuo";
const NAME: &str = "闪电说";
const TOKEN_TARGET: &str = "LegacyGeneric:target=auth_token.Shandianshuo Safe Storage";
const URL: &str = "https://api.shandianshuo.cn/v1/member/usage";
const SPEECH_LIMIT: f64 = 600.0;
const AGENT_LIMIT: f64 = 1000.0;

pub fn local_credential_hint() -> Option<String> {
    let cfg = dirs::config_dir().map(|p| p.join("Shandianshuo").join("config.json"));
    if crate::platform::secret(TOKEN_TARGET).is_some() || cfg.is_some_and(|p| p.is_file()) {
        Some("闪电说本地登录会话".into())
    } else { None }
}

pub async fn snapshot() -> Snapshot {
    let Some(token) = crate::platform::secret(TOKEN_TARGET) else {
        return Snapshot::no_credentials(ID, NAME, "请先登录闪电说桌面端。");
    };
    match fetch(&token).await { Ok(snapshot) => snapshot, Err(error) => Snapshot::error(ID, NAME, error) }
}

fn number(value: &Value) -> Option<f64> { value.as_f64().or_else(|| value.as_str()?.parse().ok()) }

/// Reads a numeric field by exact object path, e.g. `["speech", "usedMinutes"]`.
fn number_at<'a>(root: &'a Value, path: &[&str]) -> Option<f64> {
    let mut current = root;
    for key in path { current = current.get(key)?; }
    number(current)
}

fn find_text(root: &Value, keys: &[&str]) -> Option<String> {
    match root {
        Value::Object(map) => keys.iter().find_map(|key| map.get(*key).and_then(Value::as_str).filter(|s| !s.trim().is_empty()).map(str::to_owned)).or_else(|| map.values().find_map(|v| find_text(v, keys))),
        Value::Array(items) => items.iter().find_map(|v| find_text(v, keys)), _ => None,
    }
}

/// GET /v1/member/usage responds with nested objects, e.g.
/// `{"speech":{"usedMinutes":380,"quotaMinutes":600,"isUnlimited":false},
///   "assistant":{"usedCredits":7.53,"creditQuota":1000,"isUnlimited":false},
///   "booster":{...},"resetDate":"2026-10-03T05:54:42.000Z"}`
async fn fetch(token: &str) -> Result<Snapshot, String> {
    let response = http().get(URL).bearer_auth(token.trim()).send().await.map_err(|e| format!("闪电说请求失败: {e}"))?;
    if matches!(response.status().as_u16(), 401 | 403) { return Err("闪电说登录已过期，请重新登录桌面端".into()); }
    if !response.status().is_success() { return Err(format!("闪电说接口返回 HTTP {}", response.status())); }
    let body = json_body(response, 128 * 1024, "闪电说").await?;
    let speech_used = number_at(&body, &["speech", "usedMinutes"]).map(|v| v.max(0.0)).ok_or("闪电说返回中没有语音分钟用量")?;
    let speech_quota = number_at(&body, &["speech", "quotaMinutes"]).filter(|v| *v > 0.0).unwrap_or(SPEECH_LIMIT);
    let agent_used = number_at(&body, &["assistant", "usedCredits"]).map(|v| v.max(0.0)).ok_or("闪电说返回中没有 Agent 积分用量")?;
    let agent_quota = number_at(&body, &["assistant", "creditQuota"]).filter(|v| *v > 0.0).unwrap_or(AGENT_LIMIT);
    let reset = find_text(&body, &["resetDate", "currentPeriodEnd", "current_period_end", "expiresAt", "expires_at"]).and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.timestamp_millis());
    let metrics = vec![
        Metric::progress("直接说", (speech_used / speech_quota * 100.0).clamp(0.0, 100.0), Some(format!("{speech_used:.0} / {speech_quota:.0} 分钟"))).with_reset(reset, Some(31 * 86_400_000)),
        Metric::progress("Agent 执行积分", (agent_used / agent_quota * 100.0).clamp(0.0, 100.0), Some(format!("{agent_used:.2} / {agent_quota:.0} 积分"))).with_reset(reset, Some(31 * 86_400_000)),
    ];
    let plan = find_text(&body, &["tierDisplayName", "tier_display_name", "plan", "tier"]).or_else(|| Some("Pro".into()));
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_usage_shape() {
        let body: Value = serde_json::json!({
            "speech": { "usedMinutes": 380, "quotaMinutes": 600, "isUnlimited": false },
            "assistant": { "usedCredits": 7.53, "creditQuota": 1000, "isUnlimited": false },
            "booster": { "voice": { "totalPurchased": 0, "totalConsumed": 0, "remaining": 0 } },
            "resetDate": "2026-10-03T05:54:42.000Z"
        });
        assert_eq!(number_at(&body, &["speech", "usedMinutes"]), Some(380.0));
        assert_eq!(number_at(&body, &["speech", "quotaMinutes"]), Some(600.0));
        assert_eq!(number_at(&body, &["assistant", "usedCredits"]), Some(7.53));
        assert_eq!(number_at(&body, &["assistant", "creditQuota"]), Some(1000.0));
        assert_eq!(number_at(&body, &["speech", "missing"]), None);
        let reset = find_text(&body, &["resetDate", "currentPeriodEnd"])
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.timestamp_millis());
        assert!(reset.is_some());
    }
}
