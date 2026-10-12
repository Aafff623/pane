//! CodeBuddy family (Tencent) — one module for every family member
//! (CodeBuddy / CodeBuddy CN / WorkBuddy). They share the `copilot`
//! account realm and the `/v2/plugin` billing gateway and differ only in
//! host and the `platform` handshake parameter, so the family table in
//! `codebuddy_login` parameterizes everything here: adding a member is a
//! row there, not a copy of this file.
//!
//! There is no local credential source in Pane: cards exist only through
//! Pane-managed logins (codebuddy_login), one account card each. The card
//! face is a credits meter (total/remaining, reset at the earliest future
//! cycle end, frozen capacity as a Reserved row) plus the vendor's dosage
//! notice — same three fields the web dashboard shows.

use super::{http, Metric, Snapshot};
use crate::codebuddy_login::Family;
use serde_json::{json, Value};
use std::time::Duration;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Login-only family: the bare card reports how to add an account, and
/// every stored login renders its own `<family>@<fingerprint>` card.
///
/// WorkBuddy gets one more source ahead of that hint: the desktop app's own
/// auth store, read straight off disk (`workbuddy_local`) — the route magpie
/// takes, so no scan is involved. It is read-only, and when the store is
/// encrypted at rest or absent the card falls back to the sign-in hint and
/// the other routes stay the user's choice.
pub async fn snapshot(f: &'static Family) -> Snapshot {
    if f.family_id == "workbuddy" {
        // Tier 1: the desktop app's own auth store, read straight off disk
        // (`workbuddy_local`) — no ceremony, the route magpie takes.
        let local_failure = match crate::providers::workbuddy_local::read() {
            crate::providers::workbuddy_local::LocalRead::Session(session) => {
                return snapshot_from_desktop(f, *session).await;
            }
            crate::providers::workbuddy_local::LocalRead::Encrypted => Some(
                "WorkBuddy's desktop sign-in is stored encrypted on this machine and Pane does not decrypt it. Sign in from the auth center instead.".to_string(),
            ),
            crate::providers::workbuddy_local::LocalRead::Unreadable(reason) => {
                Some(format!("WorkBuddy desktop sign-in could not be read: {reason}"))
            }
            crate::providers::workbuddy_local::LocalRead::Missing => None,
        };
        // Tier 3: the explicitly-imported web session, when enabled. Off
        // means not a single request leaves this machine.
        if let Some(snap) =
            crate::providers::workbuddy_web::snapshot(f.family_id, f.display_name).await
        {
            return snap;
        }
        // Tier 1 failed and no web session: the card states the specific
        // reason and offers the tier-2 jump to the auth center.
        if let Some(reason) = local_failure {
            return Snapshot::no_credentials(f.family_id, f.display_name, &reason);
        }
    }
    let hint = format!("Sign in with {} from the auth center to see its quota.", f.display_name);
    Snapshot::no_credentials(f.family_id, f.display_name, &hint)
}

/// The desktop app's session, shaped like a Pane login so it takes the same
/// read path.
///
/// Follows magpie's route for this source: the access token is used as it is
/// until it is within a minute of expiring, then refreshed once and the
/// rotated pair kept **here** — WorkBuddy's own file is never written, and
/// nothing is minted into Pane's login store either, so the desktop app stays
/// the sole owner of that file.
async fn snapshot_from_desktop(
    f: &'static Family,
    session: crate::providers::workbuddy_local::LocalSession,
) -> Snapshot {
    let fresh = desktop_fresh_tokens(f, &session).await;
    let (access_token, refresh_token) = match fresh {
        Some(pair) => pair,
        None => (session.access_token.clone(), session.refresh_token.clone()),
    };
    let login = crate::login_accounts::LoginAccount {
        account_id: session.uid.clone(),
        email: String::new(),
        label: format!("{} — desktop app", f.display_name),
        access_token,
        refresh_token,
        id_token: String::new(),
        expires_at: session
            .expires_at_ms
            .and_then(chrono::DateTime::from_timestamp_millis)
            .map(|t| t.to_rfc3339())
            .unwrap_or_default(),
        added_at: 0,
        extra: json!({ "uid": session.uid, "domain": session.domain }),
    };
    let id = crate::login_accounts::card_id_for_account(f.family_id, &login);
    let name = login_card_name(f, &login);

    match fetch_with_login(f, &login, false).await {
        Ok(snapshot) => snapshot,
        Err(error) => Snapshot::error(&id, &name, error),
    }
}

/// The rotated pair for a desktop-file account, cached in memory for the
/// session (magpie's `wbTokens`): without it every refresh cycle would read
/// the file's now-stale stamp and rotate the token again.
static DESKTOP_TOKENS: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<String, (String, String)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// magpie's rule for a desktop-store token: refresh only when the access
/// token is within a minute of expiring, the store actually said when it
/// expires, and the refresh token is present and not itself lapsed. Anything
/// else leaves the token alone and lets the request try what there is.
fn desktop_needs_refresh(
    session: &crate::providers::workbuddy_local::LocalSession,
    now_ms: i64,
) -> bool {
    let Some(expires_at) = session.expires_at_ms else {
        return false; // the store did not say — trust it until the server refuses
    };
    if now_ms < expires_at - 60_000 {
        return false; // still has life
    }
    if session.refresh_token.trim().is_empty() {
        return false;
    }
    !session.refresh_expires_at_ms.is_some_and(|at| now_ms >= at)
}

/// Refresh the desktop app's token only when it is about to lapse (magpie
/// refreshes a minute ahead), and only when the refresh token itself is still
/// inside its own lifetime. A refresh failure is not fatal: the current token
/// is tried as it is, exactly as magpie does.
async fn desktop_fresh_tokens(
    f: &'static Family,
    session: &crate::providers::workbuddy_local::LocalSession,
) -> Option<(String, String)> {
    let cached = DESKTOP_TOKENS.lock().ok()?.get(&session.uid).cloned();
    if let Some(pair) = &cached {
        return Some(pair.clone());
    }
    if !desktop_needs_refresh(session, chrono::Utc::now().timestamp_millis()) {
        return None;
    }
    let rotated = crate::codebuddy_login::refresh(
        f,
        &session.access_token,
        &session.refresh_token,
        Some(&session.domain),
    )
    .await
    .ok()?;
    let access = rotated
        .get("accessToken")
        .or_else(|| rotated.get("access_token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    let refresh = rotated
        .get("refreshToken")
        .or_else(|| rotated.get("refresh_token"))
        .and_then(Value::as_str)
        .unwrap_or(&session.refresh_token)
        .to_string();
    DESKTOP_TOKENS
        .lock()
        .ok()?
        .insert(session.uid.clone(), (access.clone(), refresh.clone()));
    Some((access, refresh))
}

/// Pure local probe for the Customize gear panel (no network): the desktop
/// app's sign-in store, when it holds a readable session.
pub fn local_credential_hint(f: &Family) -> Option<String> {
    if f.family_id != "workbuddy" {
        return None;
    }
    match crate::providers::workbuddy_local::read() {
        crate::providers::workbuddy_local::LocalRead::Session(_) => {
            Some("WorkBuddy desktop app sign-in (read-only)".to_string())
        }
        crate::providers::workbuddy_local::LocalRead::Encrypted => {
            Some("WorkBuddy desktop sign-in is encrypted at rest".to_string())
        }
        _ => None,
    }
}

/// Snapshot for one Pane-managed family login. A rejected token
/// refreshes once through the IDE's refresh call (rotated pair written
/// back) before the error stands.
pub async fn snapshot_with_login(
    f: &'static Family,
    login: crate::login_accounts::LoginAccount,
) -> Snapshot {
    let id = crate::login_accounts::card_id_for_account(f.family_id, &login);
    let name = login_card_name(f, &login);
    match fetch_with_login(f, &login, true).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(&id, &name, e),
    }
}

pub(crate) fn login_card_name(f: &Family, login: &crate::login_accounts::LoginAccount) -> String {
    if !login.label.trim().is_empty() {
        return login.label.clone();
    }
    if !login.email.trim().is_empty() {
        return format!("{} — {}", f.display_name, login.email);
    }
    format!("{} @{}", f.display_name, &login.account_id[..login.account_id.len().min(12)])
}

struct Access {
    token: String,
    refresh_token: String,
    tenant: crate::codebuddy_login::Tenant,
}

async fn fetch_with_login(
    f: &'static Family,
    login: &crate::login_accounts::LoginAccount,
    allow_refresh: bool,
) -> Result<Snapshot, String> {
    let id = crate::login_accounts::card_id_for_account(f.family_id, login);
    let name = login_card_name(f, login);
    let access = Access {
        token: login.access_token.clone(),
        refresh_token: login.refresh_token.clone(),
        tenant: crate::codebuddy_login::Tenant::from_extra(&login.extra),
    };

    // The vendor signals a dead session with 401/403 on the first call,
    // so the token is probed rather than pre-judged by its expiry stamp.
    let first = billing_post(f, &access, "/v2/billing/meter/get-dosage-notify", None).await?;
    if first.rejected && allow_refresh && !access.refresh_token.trim().is_empty() {
        let rotated = crate::codebuddy_login::refresh(
            f,
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
        write_back(f.family_id, login, &new_token, &new_refresh, &rotated);
        let updated = Access {
            token: new_token,
            refresh_token: new_refresh,
            tenant: access.tenant,
        };
        return Box::pin(finish(f, &updated, &id, &name)).await;
    }
    if first.rejected {
        return Err(format!(
            "{} session expired — sign in again from the auth center",
            f.display_name
        ));
    }
    finish(f, &access, &id, &name).await
}

fn write_back(
    family_id: &str,
    login: &crate::login_accounts::LoginAccount,
    token: &str,
    refresh_token: &str,
    rotated: &Value,
) {
    let mut accounts = crate::login_accounts::load(family_id);
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
    let _ = crate::login_accounts::save(family_id, &accounts);
}

struct Billing {
    body: Value,
    rejected: bool,
}

async fn billing_post(
    f: &Family,
    access: &Access,
    path: &str,
    body: Option<Value>,
) -> Result<Billing, String> {
    // The billing gateway rejects requests without a browser User-Agent
    // (403 / code 10085), so UA is mandatory on every call.
    let mut req = http()
        .post(format!("{}{path}", f.api_endpoint))
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

async fn finish(f: &Family, access: &Access, id: &str, name: &str) -> Result<Snapshot, String> {
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
        billing_post(f, access, "/v2/billing/meter/get-dosage-notify", None),
        billing_post(f, access, "/v2/billing/meter/get-payment-type", None),
        billing_post(f, access, "/v2/billing/meter/get-user-resource", Some(resource_body)),
    );
    let dosage = dosage?;
    let payment = payment?;
    let resources = resources?;

    let mut metrics = Vec::new();
    let dosage_data = dosage.body.get("data").cloned().unwrap_or(Value::Null);
    let notice = ["dosageNotifyEn", "dosageNotifyZh", "dosageNotifyCode"]
        .iter()
        .find_map(|k| dosage_data.get(*k).and_then(Value::as_str).filter(|s| !s.is_empty()));
    if let Some(notice) = notice {
        metrics.push(Metric::text("Quota", notice.to_string()));
    }
    let payment_type = payment
        .body
        .get("data")
        .and_then(|d| d.as_str().or_else(|| d.get("paymentType").and_then(Value::as_str)))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let now_ms = chrono::Utc::now().timestamp_millis();
    let packs = resource_items(&resources.body);
    let mut face = credits_face_from_items(&packs, now_ms);
    if face.is_none() {
        // Enterprise members get an empty resource list from the personal
        // endpoint; their usage lives behind the enterprise endpoint the
        // web dashboard's own /profile/usage page calls.
        if access.tenant.enterprise_id.is_some() {
            if let Ok(body) = fetch_enterprise_usage(f, access).await {
                face = enterprise_face_from_body(&body, now_ms);
            }
        }
    }
    let mut plan = None;
    if let Some(face) = &face {
        metrics.extend(credits_face_metrics(face));
        plan = face.plan.clone();
    }
    for pack in &packs {
        // A pack the meter already aggregates would only repeat it.
        if face.is_some() && is_credits_pack(pack) {
            continue;
        }
        let (label, detail) = pack_row(pack);
        metrics.push(Metric::text(&label, detail));
    }
    if metrics.is_empty() {
        return Err(format!("{} reported no quota data", f.display_name));
    }
    metrics.truncate(6);
    Ok(Snapshot::ok(id, name, plan.or(payment_type), metrics))
}

/// The packs array, wherever the gateway decided to nest it this quarter.
fn resource_items(body: &Value) -> Vec<&Value> {
    [
        "/data/resources",
        "/data/data/resources",
        "/data/Response/Data/Accounts",
        "/data/data/Response/Data/Accounts",
        "/Response/Data/Accounts",
    ]
    .into_iter()
    .find_map(|path| body.pointer(path).and_then(Value::as_array))
    .map(|items| items.iter().collect())
    .unwrap_or_default()
}

/// The card's credits face, aggregated over the packs whose unit is
/// `credits` (spec D6): one meter from total/remaining, the frozen
/// capacity as a Reserved row, the earliest future cycle end as the
/// reset, and the package name as the plan label.
pub(crate) struct CreditsFace {
    pub(crate) total: f64,
    pub(crate) remain: f64,
    pub(crate) frozen: f64,
    /// Nothing finite to meter against, and the vendor said unlimited —
    /// a family with any finite pack still meters (this stays false).
    pub(crate) unlimited: bool,
    pub(crate) reset_ms: Option<i64>,
    pub(crate) plan: Option<String>,
}

impl CreditsFace {
    /// Rollover above the grant reads as zero used, never negative.
    pub(crate) fn used(&self) -> f64 {
        (self.total - self.remain).max(0.0)
    }
}

/// Numbers arrive as JSON numbers or decimal strings ("450.0") — both parse.
fn num(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn pack_num(pack: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|k| num(pack.get(*k)))
}

/// `yyyy-MM-dd HH:mm:ss` with no zone — the vendor means China Standard
/// Time (UTC+8), matching the web dashboard's own reading.
fn cycle_end_ms(raw: &str) -> Option<i64> {
    use chrono::TimeZone;
    let naive = chrono::NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%d %H:%M:%S").ok()?;
    let utc8 = chrono::FixedOffset::east_opt(8 * 3600)?;
    Some(utc8.from_local_datetime(&naive).single()?.timestamp_millis())
}

/// Enterprise members: the personal resource endpoint answers with an
/// empty list, so the web dashboard's own `/profile/usage` page reads this
/// one instead (empty body, the enterprise id rides the tenant headers).
async fn fetch_enterprise_usage(f: &Family, access: &Access) -> Result<Value, String> {
    let billing = billing_post(
        f,
        access,
        "/v2/billing/meter/get-enterprise-user-usage",
        Some(json!({})),
    )
    .await?;
    if billing.rejected {
        return Err(format!("{} enterprise usage: session rejected", f.display_name));
    }
    Ok(billing.body)
}

/// Cycle stamps arrive as `yyyy-MM-dd HH:mm:ss` (UTC+8) or as epoch
/// numbers in seconds or millis — accept all three.
fn timestamp_ms(value: &Value) -> Option<i64> {
    let from_epoch = |raw: i64| {
        Some(if raw > 1_000_000_000_000 { raw } else { raw.saturating_mul(1000) })
    };
    match value {
        Value::Number(n) => n.as_i64().and_then(from_epoch),
        Value::String(s) => {
            let s = s.trim();
            cycle_end_ms(s).or_else(|| s.parse::<i64>().ok().and_then(from_epoch))
        }
        _ => None,
    }
}

/// The enterprise usage shape: `{credit, limitNum, cycleStartTime,
/// cycleEndTime, cycleResetTime}`. Used = `credit`, remaining = `limitNum −
/// credit`, and a limit of -1 means unlimited.
fn enterprise_face_from_body(body: &Value, now_ms: i64) -> Option<CreditsFace> {
    let data = ["/data/data", "/data"]
        .iter()
        .find_map(|path| body.pointer(path))
        .unwrap_or(body);
    let limit = pack_num(data, &["limitNum", "limit_num"])?;
    let used = pack_num(data, &["usedNum", "used_num", "credit"]).unwrap_or(0.0);
    let unlimited = limit == -1.0;
    let reset_ms = ["cycleEndTime", "cycle_end_time", "cycleResetTime", "cycle_reset_time"]
        .iter()
        .filter_map(|key| data.get(*key))
        .filter_map(timestamp_ms)
        .filter(|ms| *ms > now_ms)
        .min();
    Some(CreditsFace {
        total: if unlimited { 0.0 } else { limit },
        remain: if unlimited { 0.0 } else { (limit - used).max(0.0) },
        frozen: 0.0,
        unlimited,
        reset_ms,
        plan: None,
    })
}

/// A pack counts as credits unless it says otherwise — the older shapes
/// carried no `CapacityUnit` at all.
fn is_credits_pack(pack: &Value) -> bool {
    pack.get("CapacityUnit")
        .and_then(Value::as_str)
        .map_or(true, |unit| unit.eq_ignore_ascii_case("credits"))
}

/// Test-only convenience: the whole body in, the face out.
#[cfg(test)]
fn credits_face(body: &Value, now_ms: i64) -> Option<CreditsFace> {
    credits_face_from_items(&resource_items(body), now_ms)
}

/// One credits face → card rows. Shared by the token route (`finish`) and
/// the WorkBuddy web-session route, so both stay on one meter/unlimited/
/// left/reserved basis.
pub(crate) fn credits_face_metrics(face: &CreditsFace) -> Vec<Metric> {
    let mut metrics = Vec::new();
    if face.total > 0.0 {
        // Rollover above the grant reads as 0% used, never negative.
        let used = face.used();
        let pct = (used / face.total * 100.0).clamp(0.0, 100.0);
        let meter = Metric::progress(
            "Credits",
            pct,
            Some(format!("{used:.0} of {:.0} credits used", face.total)),
        )
        .with_reset(face.reset_ms, None);
        metrics.push(meter);
    } else if face.unlimited {
        metrics.push(Metric::text("Credits", "Unlimited".to_string()));
    } else {
        // No total to meter against — the honest degrade is plain numbers.
        metrics.push(Metric::text("Credits", format!("{:.0} credits left", face.remain)));
    }
    if face.frozen > 0.0 {
        metrics.push(Metric::text("Reserved", format!("{:.0} credits reserved", face.frozen)));
    }
    metrics
}

pub(crate) fn credits_face_from_items(packs: &[&Value], now_ms: i64) -> Option<CreditsFace> {
    let mut face = CreditsFace {
        total: 0.0,
        remain: 0.0,
        frozen: 0.0,
        unlimited: false,
        reset_ms: None,
        plan: None,
    };
    let mut seen = false;
    let mut saw_unlimited = false;
    for pack in packs {
        if !is_credits_pack(pack) {
            continue;
        }
        seen = true;
        let total = pack_num(
            pack,
            &["CycleCapacitySizePrecise", "CycleCapacitySize", "CycleTotalCapacity", "totalAmount", "total"],
        );
        let remain = pack_num(
            pack,
            &["CycleCapacityRemainPrecise", "CycleCapacityRemain", "CycleRemainCapacity", "remainAmount", "remain"],
        );
        // Unlimited packs (limit -1) carry no meter — never summed in.
        let unlimited = pack.get("Unlimited").and_then(Value::as_bool).unwrap_or(false)
            || total.is_some_and(|t| t == -1.0);
        if unlimited {
            saw_unlimited = true;
        } else {
            if let Some(t) = total {
                face.total += t;
            }
            if let Some(r) = remain {
                face.remain += r;
            }
        }
        if let Some(frozen) = pack_num(pack, &["CycleFrozenCapacity", "frozenCapacity"]) {
            face.frozen += frozen;
        }
        if face.plan.is_none() {
            face.plan = ["PackageName", "SubscriptionPackageName", "resourceName", "name", "productName", "packageName"]
                .iter()
                .find_map(|k| {
                    pack.get(*k)
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                });
        }
        for key in ["CycleEndTime", "cycleEndTime", "endTime", "packageEndTime"] {
            if let Some(raw) = pack
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                if let Some(ms) = cycle_end_ms(raw).filter(|ms| *ms > now_ms) {
                    face.reset_ms = Some(face.reset_ms.map_or(ms, |cur| cur.min(ms)));
                }
            }
        }
    }
    // "Unlimited" is a property of the whole face, not of one pack: a
    // family that also holds finite credits still gets a meter.
    face.unlimited = seen && saw_unlimited && face.total == 0.0;
    seen.then_some(face)
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

    fn desktop_session(expires_at_ms: Option<i64>, refresh_expires_at_ms: Option<i64>, refresh: &str) -> crate::providers::workbuddy_local::LocalSession {
        crate::providers::workbuddy_local::LocalSession {
            uid: "u-1".into(),
            access_token: "at".into(),
            refresh_token: refresh.into(),
            domain: "www.workbuddy.cn".into(),
            expires_at_ms,
            refresh_expires_at_ms,
        }
    }

    #[test]
    fn a_desktop_token_with_life_left_is_left_alone() {
        let now = 1_000_000_000_000i64;
        let s = desktop_session(Some(now + 86_400_000), Some(now + 172_800_000), "rt");
        assert!(!desktop_needs_refresh(&s, now));
    }

    #[test]
    fn a_desktop_token_inside_the_last_minute_is_refreshed() {
        let now = 1_000_000_000_000i64;
        let s = desktop_session(Some(now + 30_000), Some(now + 86_400_000), "rt");
        assert!(desktop_needs_refresh(&s, now));
        // exactly a minute ahead is still "about to lapse"
        let edge = desktop_session(Some(now + 60_000), Some(now + 86_400_000), "rt");
        assert!(desktop_needs_refresh(&edge, now));
    }

    #[test]
    fn an_undated_desktop_token_is_trusted_until_refused() {
        let now = 1_000_000_000_000i64;
        assert!(!desktop_needs_refresh(&desktop_session(None, None, "rt"), now));
    }

    #[test]
    fn a_lapsed_refresh_token_stops_the_refresh() {
        let now = 1_000_000_000_000i64;
        // refresh token already past its own expiry
        let lapsed = desktop_session(Some(now - 1), Some(now - 1), "rt");
        assert!(!desktop_needs_refresh(&lapsed, now));
        // no refresh token at all
        let none = desktop_session(Some(now - 1), None, "  ");
        assert!(!desktop_needs_refresh(&none, now));
    }

    #[test]
    fn resource_items_accepts_the_known_shapes() {
        let body = json!({ "data": { "resources": [ { "resourceName": "Pro pack" } ] } });
        assert_eq!(resource_items(&body).len(), 1);
        let nested = json!({ "data": { "data": { "resources": [ { "a": 1 } ] } } });
        assert_eq!(resource_items(&nested).len(), 1);
        let accounts = json!({ "data": { "Response": { "Data": { "Accounts": [ { "a": 1 } ] } } } });
        assert_eq!(resource_items(&accounts).len(), 1);
        let double_nested = json!({ "data": { "data": { "Response": { "Data": { "Accounts": [ { "a": 1 }, { "b": 2 } ] } } } } });
        assert_eq!(resource_items(&double_nested).len(), 2);
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

    fn resource_body(packs: Value) -> Value {
        json!({ "code": 0, "data": { "Response": { "Data": { "Accounts": packs } } } })
    }

    const NOW_MS: i64 = 1_760_000_000_000; // 2025-10-09 ~09:46 UTC

    #[test]
    fn credits_face_meters_from_decimal_strings() {
        // The gateway's own wrap: precise fields are decimal strings.
        let face = credits_face(
            &resource_body(json!([{
                "PackageCode": "TCACA_code_free",
                "PackageName": "体验版",
                "CycleCapacitySizePrecise": "500",
                "CycleCapacityRemainPrecise": "450.5",
                "CapacityUnit": "credits",
                "CycleEndTime": "2026-01-31 00:00:00"
            }])),
            NOW_MS,
        )
        .expect("face");
        assert_eq!(face.total, 500.0);
        assert_eq!(face.remain, 450.5);
        assert_eq!(face.used(), 49.5);
        assert_eq!(face.plan.as_deref(), Some("体验版"));
        // 2026-01-31 00:00:00 UTC+8 = 1769788800000.
        assert_eq!(face.reset_ms, Some(1_769_788_800_000));
    }

    #[test]
    fn only_credits_packs_are_summed() {
        let face = credits_face(
            &resource_body(json!([
                { "PackageName": "credits pack", "CycleCapacitySize": 200.0, "CycleCapacityRemain": 100.0, "CapacityUnit": "credits" },
                { "PackageName": "gpu pack", "CycleCapacitySize": 999.0, "CycleCapacityRemain": 1.0, "CapacityUnit": "gpu_minutes" }
            ])),
            NOW_MS,
        )
        .expect("face");
        assert_eq!(face.total, 200.0);
        assert_eq!(face.remain, 100.0);
    }

    #[test]
    fn rollover_above_the_grant_reads_zero_used() {
        let face = credits_face(
            &resource_body(json!([{
                "CycleCapacitySize": 100.0, "CycleCapacityRemain": 180.0, "CapacityUnit": "credits"
            }])),
            NOW_MS,
        )
        .expect("face");
        assert_eq!(face.used(), 0.0);
    }

    #[test]
    fn unlimited_packs_carry_no_meter() {
        let face = credits_face(
            &resource_body(json!([
                { "CycleCapacitySize": 500.0, "CycleCapacityRemain": 100.0, "CapacityUnit": "credits" },
                { "CycleCapacitySize": -1.0, "CycleCapacityRemain": -1.0, "CapacityUnit": "credits", "Unlimited": false }
            ])),
            NOW_MS,
        )
        .expect("face");
        assert_eq!(face.total, 500.0);
        assert_eq!(face.remain, 100.0);
        assert!(!face.unlimited);
    }

    #[test]
    fn an_all_unlimited_family_reads_unlimited() {
        let face = credits_face(
            &resource_body(json!([
                { "CycleCapacitySize": -1.0, "CycleCapacityRemain": -1.0, "CapacityUnit": "credits" }
            ])),
            NOW_MS,
        )
        .expect("face");
        assert!(face.unlimited);
        assert_eq!(face.total, 0.0);
    }

    #[test]
    fn zero_total_degrades_but_frozen_stays_visible() {
        let face = credits_face(
            &resource_body(json!([{
                "CycleCapacityRemain": 0.0, "CycleFrozenCapacity": 12.0, "CapacityUnit": "credits"
            }])),
            NOW_MS,
        )
        .expect("face");
        assert_eq!(face.total, 0.0);
        assert_eq!(face.remain, 0.0);
        assert_eq!(face.frozen, 12.0);
    }

    #[test]
    fn reset_takes_the_earliest_future_cycle_end_in_utc8() {
        // 2025-01-01 is in the past at NOW_MS — only the 2026 end counts.
        let face = credits_face(
            &resource_body(json!([
                { "CycleCapacitySize": 10.0, "CapacityUnit": "credits", "CycleEndTime": "2025-01-01 00:00:00" },
                { "CycleCapacitySize": 20.0, "CapacityUnit": "credits", "CycleEndTime": "2026-02-01 00:00:00" },
                { "CycleCapacitySize": 30.0, "CapacityUnit": "credits", "CycleEndTime": "2026-03-01 00:00:00" }
            ])),
            NOW_MS,
        )
        .expect("face");
        // 2026-02-01 00:00:00 UTC+8 = 1769875200000.
        assert_eq!(face.reset_ms, Some(1_769_875_200_000));
    }

    #[test]
    fn cycle_end_ms_reads_bare_timestamps_as_utc8() {
        // 2026-01-01 00:00 +08:00 == 2025-12-31T16:00:00Z = 1767196800000.
        assert_eq!(cycle_end_ms("2026-01-01 00:00:00"), Some(1_767_196_800_000));
        assert_eq!(cycle_end_ms("not a date"), None);
    }

    #[test]
    fn missing_unit_defaults_to_credits_for_legacy_shapes() {
        // The older codebuddy.ai shape carried no CapacityUnit at all.
        let face = credits_face(
            &resource_body(json!([{
                "resourceName": "Pro pack", "totalAmount": 100.0, "remainAmount": 40.0
            }])),
            NOW_MS,
        )
        .expect("face");
        assert_eq!(face.total, 100.0);
        assert_eq!(face.remain, 40.0);
        assert_eq!(face.plan.as_deref(), Some("Pro pack"));
    }

    #[test]
    fn enterprise_face_reads_limit_and_credit() {
        // The shape cockpit's #1911 fix reads: total 2000 / used 378.
        let face = enterprise_face_from_body(
            &json!({
                "data": { "data": {
                    "limitNum": 2000, "credit": 378,
                    "cycleStartTime": "2026-01-01 00:00:00",
                    "cycleEndTime": "2026-02-01 00:00:00"
                } }
            }),
            NOW_MS,
        )
        .expect("face");
        assert_eq!(face.total, 2000.0);
        assert_eq!(face.used(), 378.0);
        assert_eq!(face.remain, 1622.0);
        assert_eq!(face.reset_ms, Some(1_769_875_200_000));
    }

    #[test]
    fn enterprise_limit_minus_one_is_unlimited() {
        let face = enterprise_face_from_body(
            &json!({ "data": { "limitNum": -1, "credit": 5 } }),
            NOW_MS,
        )
        .expect("face");
        assert!(face.unlimited);
        assert_eq!(face.total, 0.0);
    }

    #[test]
    fn enterprise_accepts_epoch_and_underscored_keys() {
        let face = enterprise_face_from_body(
            &json!({ "limit_num": 100, "used_num": 10, "cycle_reset_time": 1_769_875_200_000i64 }),
            NOW_MS,
        )
        .expect("face");
        assert_eq!(face.total, 100.0);
        assert_eq!(face.used(), 10.0);
        assert_eq!(face.reset_ms, Some(1_769_875_200_000));
        // No limit at all is not a face — the caller keeps its own reading.
        assert!(enterprise_face_from_body(&json!({ "credit": 5 }), NOW_MS).is_none());
    }
}
