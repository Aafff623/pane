//! Windsurf (Codeium) — quota through the editor's own seat-management
//! service.
//!
//! Login-only family: the card exists through Pane-managed logins
//! (windsurf_login), one account card each. The quota lives in the
//! plan status the service returns for a freshly minted one-time auth
//! token: daily/weekly remaining percents, their reset instants, and
//! the overage balance. Percent semantics match the vendor's UI —
//! `RemainingPercent` is converted to used%.

use super::{Metric, Snapshot};
use serde_json::{json, Value};

const ID: &str = "windsurf";
const NAME: &str = "Windsurf";
const DAY_MS: i64 = 24 * 3600 * 1000;
const WEEK_MS: i64 = 7 * DAY_MS;
/// Overage is reported in micros; dollars are friendlier and match the
/// vendor's own display.
const MICROS_PER_DOLLAR: f64 = 1_000_000.0;

pub async fn snapshot() -> Snapshot {
    Snapshot::no_credentials(
        ID,
        NAME,
        "Sign in with Windsurf from the auth center to see its quota.",
    )
}

pub fn local_credential_hint() -> Option<String> {
    None
}

pub async fn snapshot_with_login(login: crate::login_accounts::LoginAccount) -> Snapshot {
    let id = crate::login_accounts::card_id_for_account(ID, &login);
    let name = login_card_name(&login);
    match fetch_with_login(&login).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(&id, &name, e),
    }
}

pub(crate) fn login_card_name(login: &crate::login_accounts::LoginAccount) -> String {
    if !login.label.trim().is_empty() {
        return login.label.clone();
    }
    if !login.email.trim().is_empty() {
        return format!("Windsurf — {}", login.email);
    }
    format!("Windsurf @{}", &login.account_id[..login.account_id.len().min(12)])
}

async fn fetch_with_login(login: &crate::login_accounts::LoginAccount) -> Result<Snapshot, String> {
    let id = crate::login_accounts::card_id_for_account(ID, login);
    let name = login_card_name(login);
    let api_server_url = login
        .extra
        .get("apiServerUrl")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(crate::windsurf_login::DEFAULT_API_SERVER_URL);

    // A one-time auth token per fetch: it is the currency the plan
    // status is billed against, and it expires immediately after use.
    let auth_token = crate::windsurf_login::post_seat(
        api_server_url,
        "GetOneTimeAuthToken",
        json!({ "firebaseIdToken": login.access_token }),
    )
    .await?;
    let auth_token = auth_token
        .get("authToken")
        .or_else(|| auth_token.get("auth_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("GetOneTimeAuthToken returned no token")?
        .to_string();

    let plan = crate::windsurf_login::post_seat(
        api_server_url,
        "GetPlanStatus",
        json!({ "authToken": auth_token, "includeSubscription": true }),
    )
    .await?;
    let plan_status = plan
        .get("planStatus")
        .or_else(|| plan.get("plan_status"))
        .cloned()
        .unwrap_or(plan);
    let metrics = plan_metrics(&plan_status);
    if metrics.is_empty() {
        return Err("Windsurf reported no plan status".into());
    }
    Ok(Snapshot::ok(&id, &name, plan_name(&plan_status), metrics))
}

fn number_at(root: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|k| {
        root.get(*k)
            .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())))
            .filter(|v| v.is_finite())
    })
}

fn unix_seconds_at(root: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|k| {
        let raw = root.get(*k)?;
        let value = raw.as_i64().or_else(|| {
            raw.as_str()
                .and_then(|s| s.trim().parse::<f64>().ok())
                .map(|n| n as i64)
        })?;
        // The service reports seconds; tolerate millis.
        Some(if value > 1_000_000_000_000 { value / 1000 } else { value })
    })
}

/// Remaining percent → used percent, clamped to the meter range.
fn used_from_remaining(remaining: Option<f64>) -> Option<f64> {
    remaining.map(|r| (100.0 - r).clamp(0.0, 100.0))
}

pub(crate) fn plan_metrics(plan_status: &Value) -> Vec<Metric> {
    let mut metrics = Vec::new();
    let daily_used = used_from_remaining(number_at(
        plan_status,
        &["dailyQuotaRemainingPercent", "daily_quota_remaining_percent"],
    ));
    let weekly_used = used_from_remaining(number_at(
        plan_status,
        &["weeklyQuotaRemainingPercent", "weekly_quota_remaining_percent"],
    ));
    if let Some(used) = daily_used {
        let reset = unix_seconds_at(
            plan_status,
            &["dailyQuotaResetAtUnix", "daily_quota_reset_at_unix"],
        )
        .map(|s| s * 1000);
        metrics.push(Metric::progress("Daily", used, None).with_reset(reset, Some(DAY_MS)));
    }
    if let Some(used) = weekly_used {
        let reset = unix_seconds_at(
            plan_status,
            &["weeklyQuotaResetAtUnix", "weekly_quota_reset_at_unix"],
        )
        .map(|s| s * 1000);
        metrics.push(Metric::progress("Weekly", used, None).with_reset(reset, Some(WEEK_MS)));
    }
    if let Some(micros) = number_at(plan_status, &["overageBalanceMicros", "overage_balance_micros"]) {
        if micros > 0.0 {
            metrics.push(Metric::text("Overage", format!("${:.2} left", micros / MICROS_PER_DOLLAR)));
        }
    }
    metrics
}

fn plan_name(plan_status: &Value) -> Option<String> {
    ["planName", "plan_name", "tier", "teamsTier", "subscriptionTier"]
        .iter()
        .find_map(|key| {
            let text = match plan_status.get(*key)? {
                Value::String(s) => s.trim().to_string(),
                Value::Number(n) => n.to_string(),
                _ => return None,
            };
            (!text.is_empty()).then_some(text)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn remaining_percent_becomes_used_percent() {
        assert_eq!(used_from_remaining(Some(0.0)), Some(100.0));
        assert_eq!(used_from_remaining(Some(37.5)), Some(62.5));
        assert_eq!(used_from_remaining(Some(120.0)), Some(0.0)); // clamped
        assert_eq!(used_from_remaining(None), None);
    }

    #[test]
    fn plan_metrics_render_daily_weekly_and_overage() {
        let metrics = plan_metrics(&json!({
            "dailyQuotaRemainingPercent": 25.0,
            "dailyQuotaResetAtUnix": 1_772_323_200_i64,
            "weeklyQuotaRemainingPercent": 80.0,
            "weeklyQuotaResetAtUnix": 1_772_323_200_i64,
            "overageBalanceMicros": 2_500_000_i64
        }));
        assert_eq!(metrics.len(), 3);
        assert_eq!(metrics[0].label, "Daily");
        assert_eq!(metrics[0].used_percent, Some(75.0));
        assert_eq!(metrics[0].resets_at, Some(1_772_323_200_000));
        assert_eq!(metrics[1].label, "Weekly");
        assert_eq!(metrics[1].used_percent, Some(20.0));
        assert_eq!(metrics[2].label, "Overage");
        assert_eq!(metrics[2].value.as_deref(), Some("$2.50 left"));
    }

    #[test]
    fn empty_plan_status_yields_no_rows() {
        assert!(plan_metrics(&json!({})).is_empty());
        // A quota-billing plan with the fields absent stays empty rather
        // than inventing a 100% row.
        assert!(plan_metrics(&json!({ "planName": "Pro" })).is_empty());
    }

    #[test]
    fn millisecond_resets_are_normalized() {
        let metrics = plan_metrics(&json!({
            "dailyQuotaRemainingPercent": 50.0,
            "dailyQuotaResetAtUnix": 1_772_323_200_000_i64
        }));
        assert_eq!(metrics[0].resets_at, Some(1_772_323_200_000));
    }
}
