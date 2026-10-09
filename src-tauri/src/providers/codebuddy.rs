//! CodeBuddy (Tencent, codebuddy.ai) — quota through the IDE's own
//! billing endpoints.
//!
//! There is no local credential source in Pane: the card exists only
//! through Pane-managed logins (codebuddy_login), one account card each.
//! The quota surface is text-shaped — the vendor's dosage notice plus
//! the payment type and the resource packs — so the rows are text, not
//! meters, exactly like cockpit renders the same three fields.

use super::{http, Metric, Snapshot};
use serde_json::{json, Value};
use std::time::Duration;

const ID: &str = "codebuddy";
const NAME: &str = "CodeBuddy";
const API_ENDPOINT: &str = "https://www.codebuddy.ai";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Login-only family: the bare card reports how to add an account, and
/// every stored login renders its own `codebuddy@<fingerprint>` card.
pub async fn snapshot() -> Snapshot {
    Snapshot::no_credentials(
        ID,
        NAME,
        "Sign in with CodeBuddy from the auth center to see its quota.",
    )
}

pub fn local_credential_hint() -> Option<String> {
    None
}

/// Snapshot for one Pane-managed CodeBuddy login. A rejected token
/// refreshes once through the IDE's refresh call (rotated pair written
/// back) before the error stands.
pub async fn snapshot_with_login(login: crate::login_accounts::LoginAccount) -> Snapshot {
    let id = crate::login_accounts::card_id_for_account(ID, &login);
    let name = login_card_name(&login);
    match fetch_with_login(&login, true).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(&id, &name, e),
    }
}

pub(crate) fn login_card_name(login: &crate::login_accounts::LoginAccount) -> String {
    if !login.label.trim().is_empty() {
        return login.label.clone();
    }
    if !login.email.trim().is_empty() {
        return format!("CodeBuddy — {}", login.email);
    }
    format!("CodeBuddy @{}", &login.account_id[..login.account_id.len().min(12)])
}

struct Access {
    token: String,
    refresh_token: String,
    tenant: crate::codebuddy_login::Tenant,
}

async fn fetch_with_login(
    login: &crate::login_accounts::LoginAccount,
    allow_refresh: bool,
) -> Result<Snapshot, String> {
    let id = crate::login_accounts::card_id_for_account(ID, login);
    let name = login_card_name(login);
    let access = Access {
        token: login.access_token.clone(),
        refresh_token: login.refresh_token.clone(),
        tenant: crate::codebuddy_login::Tenant::from_extra(&login.extra),
    };

    // The vendor signals a dead session with 401/403 on the first call,
    // so the token is probed rather than pre-judged by its expiry stamp.
    let first = billing_post(&access, "/v2/billing/meter/get-dosage-notify", None).await?;
    if first.rejected && allow_refresh && !access.refresh_token.trim().is_empty() {
        let rotated = crate::codebuddy_login::refresh(
            &access.token,
            &access.refresh_token,
            access.tenant.domain.as_deref(),
        )
        .await?;
        let new_token = rotated
            .get("accessToken")
            .or_else(|| rotated.get("access_token"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or("token refresh returned no accessToken")?
            .to_string();
        let new_refresh = rotated
            .get("refreshToken")
            .or_else(|| rotated.get("refresh_token"))
            .and_then(Value::as_str)
            .unwrap_or(&access.refresh_token)
            .to_string();
        write_back(login, &new_token, &new_refresh, &rotated);
        let updated = Access {
            token: new_token,
            refresh_token: new_refresh,
            tenant: access.tenant,
        };
        return Box::pin(finish(&updated, &id, &name)).await;
    }
    if first.rejected {
        return Err("CodeBuddy session expired — sign in again from the auth center".into());
    }
    finish(&access, &id, &name).await
}

fn write_back(
    login: &crate::login_accounts::LoginAccount,
    token: &str,
    refresh_token: &str,
    rotated: &Value,
) {
    let mut accounts = crate::login_accounts::load(ID);
    if let Some(entry) = accounts.iter_mut().find(|a| a.account_id == login.account_id) {
        entry.access_token = token.to_string();
        entry.refresh_token = refresh_token.to_string();
        if let Some(expires) = rotated
            .get("expiresAt")
            .or_else(|| rotated.get("expireTime"))
            .and_then(Value::as_i64)
        {
            let millis = if expires > 1_000_000_000_000 { expires } else { expires.saturating_mul(1000) };
            entry.expires_at = chrono::DateTime::from_timestamp_millis(millis)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339();
        }
        if let Some(domain) = rotated.get("domain").and_then(Value::as_str) {
            entry.extra["domain"] = Value::String(domain.to_string());
        }
    }
    let _ = crate::login_accounts::save(ID, &accounts);
}

struct Billing {
    body: Value,
    rejected: bool,
}

async fn billing_post(access: &Access, path: &str, body: Option<Value>) -> Result<Billing, String> {
    let mut req = http()
        .post(format!("{API_ENDPOINT}{path}"))
        .timeout(Duration::from_secs(30))
        .header("User-Agent", UA)
        .header("Accept", "application/json, text/plain, */*")
        .header("Authorization", format!("Bearer {}", access.token));
    if let Some(uid) = &access.tenant.uid {
        req = req.header("X-User-Id", uid);
    }
    if let Some(eid) = &access.tenant.enterprise_id {
        req = req.header("X-Enterprise-Id", eid).header("X-Tenant-Id", eid);
    }
    if let Some(d) = &access.tenant.domain {
        req = req.header("X-Domain", d);
    }
    let resp = match body {
        Some(body) => req.json(&body).send().await,
        None => req.send().await,
    }
    .map_err(|e| format!("{path} request: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Ok(Billing { body: Value::Null, rejected: true });
    }
    let text = resp.text().await.map_err(|e| format!("{path} read: {e}"))?;
    let body: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    // The vendor's envelope: code 0/200 means success, anything else is
    // an application error (not a transport one).
    let ok = matches!(body.get("code").and_then(Value::as_i64), Some(0) | Some(200));
    if !ok && !body.get("data").is_some() {
        let message = body
            .get("message")
            .or_else(|| body.get("msg"))
            .and_then(Value::as_str)
            .unwrap_or("no quota data");
        return Err(format!("{path}: {message}"));
    }
    Ok(Billing { body, rejected: false })
}

async fn finish(access: &Access, id: &str, name: &str) -> Result<Snapshot, String> {
    let now = chrono::Local::now();
    let resource_body = json!({
        "PageNumber": 1,
        "PageSize": 100,
        "ProductCode": "p_tcaca",
        "Status": [0, 3],
        "PackageEndTimeRangeBegin": now.format("%Y-%m-%d %H:%M:%S").to_string(),
        "PackageEndTimeRangeEnd": (now + chrono::Duration::days(365 * 101))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    });
    let (dosage, payment, resources) = tokio::join!(
        billing_post(access, "/v2/billing/meter/get-dosage-notify", None),
        billing_post(access, "/v2/billing/meter/get-payment-type", None),
        billing_post(access, "/v2/billing/meter/get-user-resource", Some(resource_body)),
    );
    let dosage = dosage?;
    let payment = payment?;
    let resources = resources?;

    let mut metrics = Vec::new();
    let dosage_data = dosage.body.get("data").cloned().unwrap_or(Value::Null);
    let notice = ["dosageNotifyEn", "dosageNotifyZh", "dosageNotifyCode"]
        .iter()
        .find_map(|k| dosage_data.get(*k).and_then(value_as_display).filter(|s| !s.is_empty()));
    if let Some(notice) = notice {
        metrics.push(Metric::text("Quota", notice.to_string()));
    }
    let payment_type = payment
        .body
        .get("data")
        .and_then(|d| d.as_str().or_else(|| d.get("paymentType").and_then(Value::as_str)))
        .map(str::trim)
        .filter(|s| !s.is_empty());
    for pack in resource_items(&resources.body) {
        let (label, detail) = pack_row(pack);
        metrics.push(Metric::text(&label, detail));
    }
    if metrics.is_empty() {
        return Err("CodeBuddy reported no quota data".into());
    }
    metrics.truncate(6);
    Ok(Snapshot::ok(id, name, payment_type.map(str::to_string), metrics))
}

fn value_as_display(value: &Value) -> Option<&str> {
    value.as_str()
}

fn resource_items(body: &Value) -> Vec<&Value> {
    ["/data/resources", "/data/data/resources", "/data/Response/Data/Accounts", "/Response/Data/Accounts"]
        .into_iter()
        .find_map(|path| body.pointer(path).and_then(Value::as_array))
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

/// One resource pack row: its name plus whatever remaining/expiry
/// numbers the entry carries (keys vary per pack type).
fn pack_row(pack: &Value) -> (String, String) {
    let label = ["resourceName", "name", "productName", "packageName"]
        .iter()
        .find_map(|k| pack.get(*k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()))
        .unwrap_or("Package")
        .to_string();
    let mut parts: Vec<String> = Vec::new();
    for key in ["remainAmount", "remain", "totalAmount", "usedAmount", "used", "total"] {
        if let Some(v) = pack.get(key).and_then(Value::as_f64) {
            if v.is_finite() {
                parts.push(format!("{key} {v:.0}"));
            }
        }
    }
    if let Some(exp) = pack
        .get("packageEndTime")
        .or_else(|| pack.get("endTime"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        parts.push(format!("ends {exp}"));
    }
    let detail = if parts.is_empty() {
        "active".to_string()
    } else {
        parts.join(" · ")
    };
    (label, detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resource_items_accepts_the_known_shapes() {
        let body = json!({ "data": { "resources": [ { "resourceName": "Pro pack" } ] } });
        assert_eq!(resource_items(&body).len(), 1);
        let nested = json!({ "data": { "data": { "resources": [ { "a": 1 } ] } } });
        assert_eq!(resource_items(&nested).len(), 1);
        assert!(resource_items(&json!({ "data": {} })).is_empty());
    }

    #[test]
    fn pack_rows_collect_numbers_and_expiry() {
        let (label, detail) = pack_row(&json!({
            "resourceName": "TCACA Pro",
            "remainAmount": 120.0,
            "usedAmount": 30.0,
            "packageEndTime": "2026-12-31 00:00:00"
        }));
        assert_eq!(label, "TCACA Pro");
        assert!(detail.contains("remainAmount 120"));
        assert!(detail.contains("ends 2026-12-31"));
        // An empty pack still renders one honest row.
        let (label, detail) = pack_row(&json!({}));
        assert_eq!(label, "Package");
        assert_eq!(detail, "active");
    }
}
