//! OpenDesign Cloud (open-design.ai, "Vela") coding-plan quota.
//!
//! Two credential lanes exist and they are NOT interchangeable:
//! - `vela_ctrl_…` **control key** — reads quota and wallet. Obtained through
//!   the CLI device-code flow below. This module's primary lane.
//! - `sk-amr-…` **runtime key** — inference only. Every control-plane route
//!   answers 401 with it (verified live 2026-11-23), so it is rejected up
//!   front instead of producing a confusing auth error.
//!
//! Live evidence 2026-11-24 against a real `go` workspace:
//! - `GET /workspaces/current` → 200, carries `workspaceId` + `planId`.
//! - `GET /workspaces/{id}/billing/preflight` → 200 with `codingPlan.windows`
//!   (5h + 7d for `go`), every credit as a **decimal string**.
//! - `GET /workspaces/{id}/wallet/balance` → 200, also a decimal string.
//! - `host`/`profile`/`cliVersion` are required on the device-code POST; an
//!   empty body answers 500.
//! - The activation page can sit on "登录中…" while the browser session
//!   settles; the token poll flips to `approved` on its own.
//!
//! Semantics that shape this parser (each one has bitten an implementation):
//! - Windows are mapped by `durationSeconds` (18000 = 5h, 604800 = 7d), never
//!   by array position.
//! - `go` carries both windows; higher tiers may carry only the 7-day one.
//!   Render what arrived — never fabricate a 0% row for a missing window.
//! - The windows are parallel limits, not a shared pool: never sum them.
//! - `eligible: false` means no active subscription, which must not render a
//!   healthy 0% card.
//! - preflight and wallet are independent reads; one failing must not blank
//!   the other.
//! - A real `"0.0000"` balance is a fact, not a missing value.

use super::{json_body, send_with_direct_fallback, Metric, Snapshot};
use serde_json::Value;

const ID: &str = "opendesign";
const NAME: &str = "OpenDesign";
const API_BASE: &str = "https://amr-api.open-design.ai/api/v1";
const MAX_BODY_BYTES: usize = 256 * 1024;

const WINDOW_5H_SECS: i64 = 18_000;
const WINDOW_7D_SECS: i64 = 604_800;

pub fn local_credential_hint() -> Option<String> {
    matches!(
        crate::secretstore::get(TOKEN_TARGET),
        crate::secretstore::Secret::Found(_)
    )
    .then(|| "Pane OpenDesign 登录会话".to_string())
}

pub async fn snapshot() -> Snapshot {
    let crate::secretstore::Secret::Found(key) = crate::secretstore::get(TOKEN_TARGET) else {
        return Snapshot::no_credentials(
            ID,
            NAME,
            "请先在 Pane 里登录 OpenDesign（浏览器批准一次即可）。",
        );
    };
    match fetch(&key).await {
        Ok(snap) => snap,
        Err(error) => Snapshot::error(ID, NAME, error),
    }
}

/// The control key is the only credential this provider accepts. The runtime
/// key shares the vendor's prefix family but cannot read quota at all.
fn reject_runtime_key(key: &str) -> Option<String> {
    (key.starts_with("sk-amr-") || key.starts_with("sk-amr_")).then(|| {
        "这是推理密钥（sk-amr-…），只能调用模型，无法查询额度。请在 Pane 里用 OpenDesign 登录获取控制面凭据。".to_string()
    })
}

async fn get_json(path: &str, key: &str, workspace_id: Option<&str>) -> Result<(u16, Value), String> {
    let url = format!("{API_BASE}{path}");
    let mut request = send_with_direct_fallback(|client| {
        let builder = client
            .get(&url)
            .bearer_auth(key)
            .header("accept", "application/json");
        match workspace_id {
            // The workspace must ride a header; as a query param it 401s.
            Some(id) => builder.header("x-vela-workspace-id", id),
            None => builder,
        }
    })
    .await
    .map_err(|e| format!("OpenDesign 请求失败：{e}"))?;
    let status = request.status().as_u16();
    if status == 401 || status == 403 {
        return Ok((status, Value::Null));
    }
    let doc = json_body(request, MAX_BODY_BYTES, "OpenDesign").await?;
    Ok((status, doc))
}

async fn fetch(key: &str) -> Result<Snapshot, String> {
    let key = key.trim();
    if let Some(message) = reject_runtime_key(key) {
        return Err(message);
    }

    let (status, workspace) = get_json("/workspaces/current", key, None).await?;
    if status == 401 || status == 403 {
        return Err("OpenDesign 拒绝了当前登录态，请在 Pane 里重新登录。".into());
    }
    if !(200..300).contains(&status) {
        return Err(format!("OpenDesign 工作区：HTTP {status}"));
    }
    let workspace_id = workspace
        .get("workspaceId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("OpenDesign 工作区响应缺少 workspaceId")?
        .to_string();

    // Independent reads: one may fail while the other still renders.
    let preflight = get_json(
        &format!("/workspaces/{workspace_id}/billing/preflight"),
        key,
        Some(&workspace_id),
    )
    .await;
    let wallet = get_json(
        &format!("/workspaces/{workspace_id}/wallet/balance"),
        key,
        Some(&workspace_id),
    )
    .await;

    build_snapshot(&workspace, preflight, wallet)
}

/// Decimal string or number → f64. The vendor sends credits and balances as
/// strings, but tolerate numbers so a contract change cannot silently break.
fn num(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    value
        .as_f64()
        .or_else(|| value.as_str()?.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn rfc3339_ms(value: Option<&Value>) -> Option<i64> {
    value
        .and_then(Value::as_str)
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp_millis())
}

fn window_label(duration_secs: i64) -> Option<&'static str> {
    match duration_secs {
        WINDOW_5H_SECS => Some("5 小时额度"),
        WINDOW_7D_SECS => Some("7 天额度"),
        _ => None,
    }
}

/// One progress row for a quota window. `None` when the window is unusable —
/// a missing `used` or a non-positive `limit` must never become `0 / limit`.
fn window_metric(window: &Value) -> Option<Metric> {
    let duration = num(window.get("durationSeconds"))? as i64;
    let label = window_label(duration)?;
    let used = num(window.get("usedCredits")).filter(|v| *v >= 0.0)?;
    let limit = num(window.get("limitCredits")).filter(|v| *v > 0.0)?;
    let remaining = num(window.get("remainingCredits"));
    let percent = (used / limit * 100.0).clamp(0.0, 100.0);

    let detail = match remaining {
        Some(left) => format!("{used:.0} / {limit:.0} · 剩 {left:.0}"),
        None => format!("{used:.0} / {limit:.0} credits"),
    };

    Some(
        Metric::progress(label, percent, Some(detail))
            .with_reset(rfc3339_ms(window.get("resetsAt")), Some(duration * 1000)),
    )
}

fn tier_label(tier: Option<&str>) -> Option<String> {
    let tier = tier?.trim();
    if tier.is_empty() {
        return None;
    }
    let mut chars = tier.chars();
    let first = chars.next()?.to_ascii_uppercase();
    Some(format!("{first}{}", chars.as_str()))
}

fn wallet_metric(value: Option<f64>, from_preflight: bool) -> Metric {
    match value {
        Some(balance) => {
            let label = if from_preflight { "钱包（preflight）" } else { "钱包" };
            Metric::text(label, format!("${balance:.2}"))
        }
        None => Metric::text("钱包", "不可用".into()),
    }
}

fn build_snapshot(
    workspace: &Value,
    preflight: Result<(u16, Value), String>,
    wallet: Result<(u16, Value), String>,
) -> Result<Snapshot, String> {
    let workspace_id = workspace
        .get("workspaceId")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let (preflight_status, preflight_doc, preflight_error) = match preflight {
        Ok((status, doc)) if (200..300).contains(&status) => (status, doc, None),
        Ok((status, _)) => (status, Value::Null, Some(format!("preflight HTTP {status}"))),
        Err(error) => (0, Value::Null, Some(error)),
    };
    let preflight_ok = preflight_error.is_none();

    let (wallet_status, wallet_doc) = match wallet {
        Ok(read) => read,
        Err(error) => (0, Value::Null),
    };
    let wallet_ok = (200..300).contains(&wallet_status);

    if !preflight_ok && !wallet_ok {
        return Err(match preflight_error {
            Some(error) => format!("OpenDesign 额度读取失败：{error}"),
            None => "OpenDesign 额度与钱包都读取失败".into(),
        });
    }

    // The billing payload names its own workspace; a mismatch means we are
    // about to render another account's numbers.
    if preflight_ok {
        if let Some(reported) = preflight_doc.get("workspaceId").and_then(Value::as_str) {
            if !reported.is_empty() && reported != workspace_id {
                return Err("OpenDesign 返回了其他工作区的账单，已拒绝显示。".into());
            }
        }
    }

    let coding_plan = preflight_doc.get("codingPlan");
    let eligible = coding_plan
        .and_then(|plan| plan.get("eligible"))
        .and_then(Value::as_bool);

    // Wallet: the dedicated endpoint wins; preflight's `balanceUsd` is only a
    // labelled fallback so the reader knows the source is the billing read.
    let wallet_balance = if wallet_ok {
        num(wallet_doc.get("balanceUsd"))
    } else {
        None
    };
    let preflight_balance = if preflight_ok {
        num(preflight_doc.get("balanceUsd"))
    } else {
        None
    };

    let mut metrics = Vec::new();
    let mut plan_name = None;

    if let Some(plan) = coding_plan.filter(|plan| plan.get("eligible").and_then(Value::as_bool) != Some(false)) {
        let tier = plan.get("tier").and_then(Value::as_str);
        plan_name = tier_label(tier);
        let windows = plan
            .get("windows")
            .and_then(Value::as_array)
            .map(|windows| {
                let mut rows: Vec<(i64, Metric)> = windows
                    .iter()
                    .filter_map(|window| {
                        let duration = num(window.get("durationSeconds"))? as i64;
                        window_metric(window).map(|metric| (duration, metric))
                    })
                    .collect();
                // Shortest window first, so the card reads the same no matter
                // what order the server happens to serialize.
                rows.sort_by_key(|(duration, _)| *duration);
                rows.into_iter().map(|(_, metric)| metric).collect::<Vec<_>>()
            })
            .unwrap_or_default();
        metrics.extend(windows);
    }

    // No usable windows: either no subscription, or the read failed. Both are
    // stated explicitly so neither can be mistaken for a healthy zero card.
    if metrics.is_empty() {
        if eligible == Some(false) {
            metrics.push(Metric::text(
                "订阅",
                "无生效的 Coding Plan".into(),
            ));
        } else if let Some(error) = preflight_error {
            metrics.push(Metric::text("额度", format!("不可用 · {error}")));
        } else {
            metrics.push(Metric::text("额度", "不可用".into()));
        }
    }

    if let Some(plan) = plan_name.clone() {
        metrics.push(Metric::text("套餐", plan));
    }

    let wallet_row = match (wallet_balance, preflight_balance) {
        (Some(balance), _) => wallet_metric(Some(balance), false),
        (None, Some(balance)) => wallet_metric(Some(balance), true),
        (None, None) => wallet_metric(None, false),
    };
    metrics.push(wallet_row);

    Ok(Snapshot::ok(ID, NAME, plan_name, metrics))
}

const TOKEN_TARGET: &str = "Pane:opendesign:control-key";

// ---------------------------------------------------------------- login

/// Device-code start. The body shape is load-bearing: an empty body answers
/// 500, so `cliVersion`/`host`/`profile` always go out.
pub async fn start_login() -> Result<crate::providers::opendesign_login::DeviceCode, String> {
    crate::providers::opendesign_login::start().await
}

pub async fn login(app: &tauri::AppHandle) -> Result<(), String> {
    let device = start_login().await?;
    crate::providers::opendesign_login::run_approval_window(app, device).await
}

pub fn status() -> crate::providers::tokenrhythm::LoginStatus {
    crate::providers::tokenrhythm::LoginStatus {
        signed_in: matches!(
            crate::secretstore::get(TOKEN_TARGET),
            crate::secretstore::Secret::Found(_)
        ),
        saved_at: None,
    }
}

pub fn clear() -> Result<(), String> {
    crate::secretstore::remove_checked(TOKEN_TARGET)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Workspace read exactly as captured live 2026-11-24.
    fn workspace() -> Value {
        serde_json::json!({
            "userId": "u", "appUserId": "a",
            "workspaceId": "ws_1", "workspaceName": "Personal",
            "personalMembershipTier": "go", "planId": "go",
            "billingState": "active", "role": "owner",
            "activeWorkspaceId": "ws_1"
        })
    }

    fn preflight(eligible: bool, tier: &str, windows: Value) -> Value {
        serde_json::json!({
            "schemaVersion": 1, "workspaceId": "ws_1", "workspaceMemberId": "m_1",
            "balanceUsd": "1.0000",
            "codingPlan": {
                "workspaceId": "ws_1", "eligible": eligible, "tier": tier,
                "windows": windows
            },
            "funding": "coding_plan", "billingMode": "shared"
        })
    }

    fn window(duration: i64, used: &str, limit: &str, remaining: &str, resets: &str) -> Value {
        serde_json::json!({
            "policyId": "p", "durationSeconds": duration,
            "resetMode": "anchored_recurring",
            "usedCredits": used, "limitCredits": limit, "remainingCredits": remaining,
            "windowStart": "2026-11-24T03:54:27.000Z", "resetsAt": resets
        })
    }

    fn wallet(balance: &str) -> Value {
        serde_json::json!({
            "balanceUsd": balance, "expiresAt": "2026-12-23T12:54:27.000Z",
            "billingScopeVersion": 2, "workspaceId": "ws_1", "workspaceMemberId": "m_1"
        })
    }

    #[test]
    fn go_plan_renders_both_windows_by_duration_not_order() {
        // Deliberately out of order: the 7-day window arrives first.
        let pf = preflight(true, "go", serde_json::json!([
            window(604_800, "1999", "150000", "148001", "2026-11-30T12:54:27.000Z"),
            window(18_000, "0", "50000", "50000", "2026-11-24T08:54:27.000Z"),
        ]));
        let snap = build_snapshot(&workspace(), Ok((200, pf)), Ok((200, wallet("1.0000")))).unwrap();
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["5 小时额度", "7 天额度", "套餐", "钱包"]);
        assert_eq!(snap.metrics[0].used_percent, Some(0.0));
        assert_eq!(snap.metrics[1].used_percent, Some(1.3326666666666667));
        assert!(snap.metrics[1].detail.as_deref().unwrap().contains("剩 148001"));
        assert_eq!(snap.metrics[2].value.as_deref(), Some("Go"));
        assert_eq!(snap.metrics[3].value.as_deref(), Some("$1.00"));
        assert_eq!(snap.plan.as_deref(), Some("Go"));
    }

    #[test]
    fn a_seven_day_only_plan_does_not_invent_a_five_hour_row() {
        let pf = preflight(true, "plus", serde_json::json!([
            window(604_800, "1000", "200000", "199000", "2026-11-30T12:54:27.000Z"),
        ]));
        let snap = build_snapshot(&workspace(), Ok((200, pf)), Ok((200, wallet("0.0000")))).unwrap();
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, vec!["7 天额度", "套餐", "钱包"]);
        assert!(!labels.contains(&"5 小时额度"));
    }

    #[test]
    fn ineligible_plan_is_not_a_zero_percent_card() {
        let pf = preflight(false, "", serde_json::json!([]));
        let snap = build_snapshot(&workspace(), Ok((200, pf)), Ok((200, wallet("0.0000")))).unwrap();
        assert!(snap.metrics.iter().all(|m| m.kind != "progress"));
        assert!(snap
            .metrics
            .iter()
            .any(|m| m.value.as_deref() == Some("无生效的 Coding Plan")));
    }

    #[test]
    fn a_window_without_used_credits_is_dropped_not_zeroed() {
        let pf = preflight(true, "go", serde_json::json!([
            { "durationSeconds": 18_000, "limitCredits": "50000", "resetsAt": "2026-11-24T08:54:27.000Z" },
            window(604_800, "10", "150000", "149990", "2026-11-30T12:54:27.000Z"),
        ]));
        let snap = build_snapshot(&workspace(), Ok((200, pf)), Ok((200, wallet("1.0000")))).unwrap();
        let labels: Vec<&str> = snap.metrics.iter().map(|m| m.label.as_str()).collect();
        assert!(!labels.contains(&"5 小时额度"));
        assert!(labels.contains(&"7 天额度"));
    }

    #[test]
    fn a_zero_limit_window_is_dropped() {
        let pf = preflight(true, "go", serde_json::json!([
            window(18_000, "0", "0", "0", "2026-11-24T08:54:27.000Z"),
        ]));
        let snap = build_snapshot(&workspace(), Ok((200, pf)), Ok((200, wallet("1.0000")))).unwrap();
        assert!(snap.metrics.iter().all(|m| m.kind != "progress"));
    }

    #[test]
    fn wallet_failure_keeps_preflight_rows_and_falls_back_to_preflight_balance() {
        let pf = preflight(true, "go", serde_json::json!([
            window(18_000, "0", "50000", "50000", "2026-11-24T08:54:27.000Z"),
        ]));
        let snap = build_snapshot(&workspace(), Ok((200, pf)), Ok((401, Value::Null))).unwrap();
        assert_eq!(snap.metrics[0].label, "5 小时额度");
        let wallet_row = snap.metrics.last().unwrap();
        // A 401 wallet carries no number, so preflight's balance is the
        // labelled fallback rather than "unavailable".
        assert_eq!(wallet_row.label, "钱包（preflight）");
        assert_eq!(wallet_row.value.as_deref(), Some("$1.00"));
    }

    #[test]
    fn wallet_failure_without_a_fallback_marks_the_row_unavailable() {
        let mut pf = preflight(true, "go", serde_json::json!([
            window(18_000, "0", "50000", "50000", "2026-11-24T08:54:27.000Z"),
        ]));
        pf.as_object_mut().unwrap().remove("balanceUsd");
        let snap = build_snapshot(&workspace(), Ok((200, pf)), Ok((401, Value::Null))).unwrap();
        let wallet_row = snap.metrics.last().unwrap();
        assert_eq!(wallet_row.label, "钱包");
        assert_eq!(wallet_row.value.as_deref(), Some("不可用"));
    }

    #[test]
    fn both_reads_failing_is_an_error_not_an_empty_card() {
        let result = build_snapshot(&workspace(), Ok((401, Value::Null)), Ok((401, Value::Null)));
        assert!(result.is_err());
    }

    #[test]
    fn a_mismatched_workspace_is_rejected() {
        let mut pf = preflight(true, "go", serde_json::json!([
            window(18_000, "0", "50000", "50000", "2026-11-24T08:54:27.000Z"),
        ]));
        pf["workspaceId"] = Value::String("ws_other".into());
        let result = build_snapshot(&workspace(), Ok((200, pf)), Ok((200, wallet("1.0000"))));
        assert!(result.is_err());
    }

    #[test]
    fn a_zero_balance_is_reported_as_zero() {
        let pf = preflight(true, "go", serde_json::json!([
            window(18_000, "0", "50000", "50000", "2026-11-24T08:54:27.000Z"),
        ]));
        let snap = build_snapshot(&workspace(), Ok((200, pf)), Ok((200, wallet("0.0000")))).unwrap();
        assert_eq!(snap.metrics.last().unwrap().value.as_deref(), Some("$0.00"));
    }

    #[test]
    fn a_runtime_key_is_rejected_with_an_explanation() {
        let message = reject_runtime_key("sk-amr-abc123").expect("runtime key rejected");
        assert!(message.contains("推理密钥"));
        assert!(reject_runtime_key("sk-amr_abc123").is_some());
        assert!(reject_runtime_key("vela_ctrl_K_abc").is_none());
    }

    #[test]
    fn preflight_balance_is_labelled_when_used_as_fallback() {
        let pf = preflight(true, "go", serde_json::json!([
            window(18_000, "0", "50000", "50000", "2026-11-24T08:54:27.000Z"),
        ]));
        let snap = build_snapshot(&workspace(), Ok((200, pf)), Ok((500, Value::Null))).unwrap();
        assert_eq!(snap.metrics.last().unwrap().label, "钱包（preflight）");
    }
}
