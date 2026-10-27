//! CodeBuddy family (Tencent) — one module for every family member
//! (CodeBuddy / CodeBuddy CN / WorkBuddy / WorkBuddy AI). They share the
//! `copilot` account realm and the plugin billing gateway and differ in
//! host, the `platform` handshake parameter, the billing path prefix and
//! (for WorkBuddy) whether a desktop auth store exists, so the family table
//! in `codebuddy_login` parameterizes everything here: adding a member is a
//! row there, not a copy of this file.
//!
//! Three quota sources exist, in this order:
//!   1. Pane-managed login cards (`snapshot_with_login`) — one per account.
//!   2. WorkBuddy only: the desktop app's own auth store, read-only.
//!   3. WorkBuddy CN only: an explicitly imported web session.
//! The card face is a credits meter (total/remaining, reset at the earliest
//! future cycle end, frozen capacity as a Reserved row) plus the vendor's
//! dosage notice — same three fields the web dashboard shows. A non-default
//! source appends one `Source` row, so the user always knows where the
//! numbers came from.

use super::{send_with_direct_fallback, Metric, Snapshot};
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
    if f.local_auth_file.is_some() {
        // Tier 1: the desktop app's own auth store for THIS variant, read
        // straight off disk (`workbuddy_local`) — no ceremony, the route
        // magpie takes. Skipped when the account is already managed by a
        // Pane login, so the same uid never yields two cards with one id.
        let local_failure = match crate::providers::workbuddy_local::read_for(f) {
            crate::providers::workbuddy_local::LocalRead::Session(session) => {
                if !desktop_shadowed_by_login(f, &session.uid) {
                    return snapshot_from_desktop(f, *session).await;
                }
                None
            }
            crate::providers::workbuddy_local::LocalRead::Encrypted => Some(format!(
                "{}'s desktop sign-in is stored encrypted on this machine and Pane does not decrypt it. Sign in from the auth center instead.",
                f.display_name
            )),
            crate::providers::workbuddy_local::LocalRead::Unreadable(reason) => Some(format!(
                "{} desktop sign-in could not be read: {reason}",
                f.display_name
            )),
            crate::providers::workbuddy_local::LocalRead::Missing => None,
        };
        // Tier 3 (CN only): the explicitly-imported web session, when
        // enabled. Off means not a single request leaves this machine. The
        // AI variant never borrows the CN web session — no cross-variant
        // fallback.
        if !f.intl {
            if let Some(mut snap) =
                crate::providers::workbuddy_web::snapshot(f.family_id, f.display_name).await
            {
                snap.metrics.push(source_row("web session (explicit import)"));
                return snap;
            }
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

/// A non-default quota source gets one visible row, so a desk-top or cookie
/// reading is never mistaken for a Pane-managed login.
fn source_row(detail: &str) -> Metric {
    Metric::text("Source", detail.to_string())
}

/// True when a Pane login already owns this uid — the login card renders it,
/// and a second desktop-derived snapshot with the same card id would be a
/// duplicate in the same refresh pass.
fn desktop_shadowed_by_login(f: &Family, uid: &str) -> bool {
    crate::login_accounts::load(f.family_id)
        .iter()
        .any(|account| account.account_id == uid)
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
        email: session.email.clone(),
        label: if !session.email.trim().is_empty() {
            session.email.clone()
        } else if !session.nickname.trim().is_empty() {
            session.nickname.clone()
        } else {
            format!("{} — desktop app", f.display_name)
        },
        access_token,
        refresh_token,
        id_token: String::new(),
        expires_at: session
            .expires_at_ms
            .and_then(chrono::DateTime::from_timestamp_millis)
            .map(|t| t.to_rfc3339())
            .unwrap_or_default(),
        added_at: 0,
        extra: json!({
            "uid": session.uid,
            "domain": session.domain,
            "sourceKind": "desktop",
            "enterpriseId": session.enterprise_id,
        }),
    };
    let id = crate::login_accounts::card_id_for_account(f.family_id, &login);
    let name = login_card_name(f, &login);

    match fetch_with_login(f, &login, false).await {
        Ok(mut snapshot) => {
            snapshot.metrics.push(source_row("desktop app (read-only)"));
            snapshot
        }
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
    if f.local_auth_file.is_none() {
        return None;
    }
    match crate::providers::workbuddy_local::read_for(f) {
        crate::providers::workbuddy_local::LocalRead::Session(_) => {
            Some(format!("{} desktop app sign-in (read-only)", f.display_name))
        }
        crate::providers::workbuddy_local::LocalRead::Encrypted => {
            Some(format!("{} desktop sign-in is encrypted at rest", f.display_name))
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
    if first.rejected() {
        if !allow_refresh || access.refresh_token.trim().is_empty() {
            mark_relogin(f.family_id, &login.account_id, "unauthorized");
            return Err(expired_message(f, &first));
        }
        let (new_token, new_refresh) = refresh_login(f, login, &access).await?;
        let updated = Access {
            token: new_token,
            refresh_token: new_refresh,
            tenant: access.tenant,
        };
        return Box::pin(finish(f, &updated, &id, &name)).await;
    }
    if !first.ok() {
        return Err(billing_error("/v2/billing/meter/get-dosage-notify", &first));
    }
    clear_relogin(f.family_id, &login.account_id);
    finish(f, &access, &id, &name).await
}

/// Refresh is single-flight per account: a second caller that queued behind
/// the first reuses the rotated pair already written back instead of issuing
/// its own refresh (which would spend the now-stale refresh token and could
/// clobber the newer one in the store).
async fn refresh_login(
    f: &'static Family,
    login: &crate::login_accounts::LoginAccount,
    access: &Access,
) -> Result<(String, String), String> {
    let lock = crate::codebuddy_login::refresh_lock(format!(
        "{}:{}",
        f.family_id, login.account_id
    ));
    let _guard = lock.lock().await;

    // Another task may have rotated the pair while this one waited.
    if let Some(current) = crate::login_accounts::load(f.family_id)
        .into_iter()
        .find(|account| account.account_id == login.account_id)
    {
        if !current.access_token.trim().is_empty() && current.access_token != login.access_token {
            clear_relogin(f.family_id, &login.account_id);
            return Ok((current.access_token, current.refresh_token));
        }
    }

    match crate::codebuddy_login::refresh(
        f,
        &access.token,
        &access.refresh_token,
        access.tenant.domain.as_deref(),
    )
    .await
    {
        Ok(rotated) => {
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
            clear_relogin(f.family_id, &login.account_id);
            Ok((new_token, new_refresh))
        }
        Err(error) => {
            mark_relogin(f.family_id, &login.account_id, refresh_error_kind(&error));
            Err(error)
        }
    }
}

/// `needsRelogin` is a non-secret flag the auth center and card can show;
/// it is cleared as soon as a range request succeeds again.
fn mark_relogin(family_id: &str, account_id: &str, kind: &str) {
    let mut accounts = crate::login_accounts::load(family_id);
    let Some(entry) = accounts.iter_mut().find(|account| account.account_id == account_id) else {
        return;
    };
    if !entry.extra.is_object() {
        entry.extra = json!({});
    }
    let already = entry.extra.get("needsRelogin").and_then(Value::as_bool) == Some(true)
        && entry.extra.get("lastErrorKind").and_then(Value::as_str) == Some(kind);
    if already {
        return;
    }
    entry.extra["needsRelogin"] = Value::Bool(true);
    entry.extra["lastErrorKind"] = Value::String(kind.to_string());
    let _ = crate::login_accounts::save(family_id, &accounts);
}

fn clear_relogin(family_id: &str, account_id: &str) {
    let mut accounts = crate::login_accounts::load(family_id);
    let Some(entry) = accounts.iter_mut().find(|account| account.account_id == account_id) else {
        return;
    };
    let had = entry.extra.get("needsRelogin").and_then(Value::as_bool) == Some(true);
    if !had {
        return;
    }
    if let Some(map) = entry.extra.as_object_mut() {
        map.remove("needsRelogin");
        map.remove("lastErrorKind");
    }
    let _ = crate::login_accounts::save(family_id, &accounts);
}

/// Coarse, non-secret classification for the persisted relogin flag.
fn refresh_error_kind(error: &str) -> &'static str {
    let lower = error.to_ascii_lowercase();
    if lower.contains("invalid_grant") || lower.contains("12153") {
        "refresh_rejected"
    } else if lower.contains("network") || lower.contains("request") || lower.contains("timeout") {
        "network"
    } else if lower.contains("code") || lower.contains("no data") {
        "vendor"
    } else {
        "unknown"
    }
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
            if !entry.extra.is_object() {
                entry.extra = json!({});
            }
            entry.extra["domain"] = Value::String(domain.to_string());
        }
    }
    if let Err(error) = crate::login_accounts::save(family_id, &accounts) {
        // The refreshed token still serves this request; a failed write-back
        // means the next refresh starts from the older pair, which the
        // single-flight lock and the store re-read above will notice.
        eprintln!("[pane] codebuddy token write-back failed: {error}");
    }
}

struct Billing {
    body: Value,
    status: u16,
    code: Option<i64>,
    message: Option<String>,
}

impl Billing {
    /// A dead session: the refresh ladder may run.
    fn rejected(&self) -> bool {
        matches!(self.status, 401 | 403)
    }

    /// The vendor's envelope: code 0/200 means success; a body without a
    /// code counts only when the transport was 2xx and data is present.
    fn ok(&self) -> bool {
        match self.code {
            Some(0) | Some(200) => true,
            None => self.status < 400 && self.body.get("data").is_some(),
            Some(_) => false,
        }
    }
}

/// One logical billing call, with the variant's path candidates. Only a 404
/// advances to the next candidate — 401, the WAF's 10085, and transport
/// failures are call outcomes, not path problems.
async fn billing_post(
    f: &Family,
    access: &Access,
    path: &str,
    body: Option<Value>,
) -> Result<Billing, String> {
    let candidates = f.billing_paths(path);
    let mut last = None;
    for candidate in &candidates {
        let billing = billing_post_one(f, access, candidate, body.clone()).await?;
        if billing.status == 404 && candidates.len() > 1 {
            last = Some(billing);
            continue;
        }
        return Ok(billing);
    }
    Ok(last.expect("billing_paths always yields a candidate"))
}

async fn billing_post_one(
    f: &Family,
    access: &Access,
    path: &str,
    body: Option<Value>,
) -> Result<Billing, String> {
    // The billing gateway rejects requests without a browser User-Agent
    // (403 / code 10085), so UA is mandatory on every call.
    let resp = send_with_direct_fallback(|client| {
        let mut req = client
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
        match &body {
            Some(body) => req.json(body),
            None => req,
        }
    })
    .await
    .map_err(|e| format!("{path} network: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(|e| format!("{path} read: {e}"))?;
    let body: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let code = body.get("code").and_then(Value::as_i64);
    let message = body
        .get("message")
        .or_else(|| body.get("msg"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(Billing { body, status, code, message })
}

fn billing_error(path: &str, billing: &Billing) -> String {
    match billing.status {
        401 => format!("{path}: session expired — sign in again"),
        403 if billing.code == Some(10085) => {
            format!("{path}: vendor rejected the request (browser UA required)")
        }
        403 => format!("{path}: access denied (HTTP 403)"),
        404 => format!("{path}: endpoint not found"),
        429 => format!("{path}: rate limited — try again later"),
        status if status >= 500 => format!("{path}: vendor error (HTTP {status})"),
        _ => billing
            .message
            .as_deref()
            .map(|message| format!("{path}: {message}"))
            .unwrap_or_else(|| format!("{path}: no quota data")),
    }
}

fn expired_message(f: &Family, billing: &Billing) -> String {
    match billing.status {
        403 if billing.code == Some(10085) => format!(
            "{} rejected the request (browser UA required) — sign in again from the auth center",
            f.display_name
        ),
        _ => format!(
            "{} session expired — sign in again from the auth center",
            f.display_name
        ),
    }
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
    let (dosage, payment, resources, official) = tokio::join!(
        billing_post(f, access, "/v2/billing/meter/get-dosage-notify", None),
        billing_post(f, access, "/v2/billing/meter/get-payment-type", None),
        billing_post(f, access, "/v2/billing/meter/get-user-resource", Some(resource_body)),
        official_usage(f, access),
    );
    let dosage = dosage?;
    if !dosage.ok() {
        return Err(billing_error("get-dosage-notify", &dosage));
    }
    // The payment type and the resource list are optional enrichment: a
    // successful session must not fail because one of them is unavailable.
    let payment = payment.ok().filter(|billing| billing.ok());
    let resources = resources?;
    if !resources.ok()
        && resource_items(&resources.body).is_empty()
        && access.tenant.enterprise_id.is_none()
    {
        return Err(billing_error("get-user-resource", &resources));
    }

    let mut metrics = Vec::new();
    let dosage_data = dosage.body.get("data").cloned().unwrap_or(Value::Null);
    let notice = ["dosageNotifyEn", "dosageNotifyZh", "dosageNotifyCode"]
        .iter()
        .find_map(|k| dosage_data.get(*k).and_then(Value::as_str).filter(|s| !s.is_empty()));
    if let Some(notice) = notice {
        metrics.push(Metric::text("Quota", notice.to_string()));
    }
    let payment_type = payment.as_ref().and_then(|payment| {
        payment
            .body
            .get("data")
            .and_then(|d| d.as_str().or_else(|| d.get("paymentType").and_then(Value::as_str)))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    });

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
    // Per-package detail (expiry + remaining) only when it would say
    // something the aggregate meter cannot: several live packs, or at
    // least one expiring inside the 7-day window.
    metrics.extend(package_detail_rows(&packs, now_ms));
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
    // Official request ledger: a separate, text-only reading. It is never
    // recorded as a meter sample and never summed with the credits face, so
    // the 30-day request count cannot corrupt the balance history. Added
    // after the empty guard so it can never make an empty quota look "ok".
    if let Some(official) = official.filter(|row| row.requests > 0) {
        metrics.push(super::Metric::text("Official requests (30d)", official.detail()));
    }
    metrics.truncate(12);
    let mut snapshot = Snapshot::ok(id, name, plan.or(payment_type), metrics);
    snapshot.auth_pack_expiry = earliest_pack_expiry(&packs, now_ms);
    Ok(snapshot)
}

/// One 30-day official request ledger. `requests`/`credits` are read from the
/// vendor's `get-user-request-usage` rows; they are deliberately kept out of
/// `Metric::usage` so the credits meter history never adds them.
#[derive(Default, Debug, PartialEq)]
struct OfficialUsage {
    requests: usize,
    credits: f64,
    /// The sweep stopped on an error, a stalled cursor or the round cap, so
    /// the count is a lower bound rather than the whole 30-day window.
    partial: bool,
    /// Credits per model and per client over the swept window (#31). The card
    /// names the biggest line of each so a 26-request month is not a single
    /// opaque number; both stay empty when the ledger named no model/client.
    by_model: std::collections::BTreeMap<String, f64>,
    by_client: std::collections::BTreeMap<String, f64>,
}

impl OfficialUsage {
    /// Biggest spender in a per-key map. Ties keep the first key in map order,
    /// so the same data always names the same line.
    fn top(map: &std::collections::BTreeMap<String, f64>) -> Option<&str> {
        map.iter()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(key, _)| key.as_str())
    }

    fn detail(&self) -> String {
        let credits = if (self.credits.fract()).abs() < 0.005 {
            format!("{:.0}", self.credits)
        } else {
            format!("{:.2}", self.credits)
        };
        let mut out = format!("{} requests · {} credits", self.requests, credits);
        if let Some(model) = Self::top(&self.by_model) {
            out.push_str(&format!(" · {model}"));
        }
        if let Some(client) = Self::top(&self.by_client) {
            out.push_str(&format!(" · via {client}"));
        }
        if self.partial {
            out.push_str(" · partial");
        }
        out
    }
}

/// WorkBuddy CN's official request ledger. Verified live 2026-10-24 on the
/// CN host (200 / code 0, 26 rows in the 30-day window). Other variants were
/// not probed and return `None` rather than guessing their host behaviour.
/// Parsing, pagination and aggregation are the pure helpers in
/// `workbuddy_official_usage`, so the card only owns the request loop.
async fn official_usage(f: &Family, access: &Access) -> Option<OfficialUsage> {
    use super::workbuddy_official_usage as official;
    if f.family_id != "workbuddy" {
        return None;
    }
    let now = chrono::Local::now();
    let mut start = (now - chrono::Duration::days(30))
        .format("%Y-%m-%d 00:00:00")
        .to_string();
    let end = now.format("%Y-%m-%d 23:59:59").to_string();
    let mut pages = Vec::new();
    let mut partial = false;
    let mut complete = false;
    for round in 0..official::MAX_SWEEP_ROUNDS {
        if official::sweep_round_limit_reached(round) {
            partial = true;
            break;
        }
        let previous_start_ms = chrono::NaiveDateTime::parse_from_str(
            &start,
            "%Y-%m-%d %H:%M:%S",
        )
        .ok()
        .map(|value| value.and_utc().timestamp_millis())
        .unwrap_or(0);
        let body = serde_json::json!({
            "startTime": start,
            "endTime": end,
            "pageNum": 1,
            "pageSize": 3000,
        });
        let billing = match billing_post(
            f,
            access,
            "/billing/meter/get-user-request-usage",
            Some(body),
        )
        .await
        {
            Ok(billing) => billing,
            Err(_) => {
                partial = true;
                break;
            }
        };
        if !billing.ok() {
            partial = true;
            break;
        }
        let page = match official::parse_page(&billing.body, 3000) {
            Ok(page) => page,
            Err(_) => {
                partial = true;
                break;
            }
        };
        let decision = official::decide_next(&page, previous_start_ms);
        pages.push(page);
        match decision {
            official::SweepDecision::Complete => {
                complete = true;
                break;
            }
            official::SweepDecision::Stalled => {
                partial = true;
                break;
            }
            official::SweepDecision::Continue { start_ms } => {
                let Some(next) = chrono::DateTime::from_timestamp_millis(start_ms) else {
                    partial = true;
                    break;
                };
                start = next.format("%Y-%m-%d %H:%M:%S").to_string();
            }
        }
    }
    if !complete && !pages.is_empty() && !partial {
        partial = true;
    }
    let rows = official::merge_pages(&pages);
    if rows.is_empty() {
        return None;
    }
    let summary = official::aggregate(&rows);
    Some(OfficialUsage {
        requests: rows.len(),
        credits: summary.total_credit,
        partial,
        by_model: summary.by_model,
        by_client: summary.by_client,
    })
}

/// The packs array, wherever the gateway decided to nest it this quarter.
fn resource_items(body: &Value) -> Vec<&Value> {    [
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
    if billing.rejected() {
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
        .with_usage(crate::usage_units::UsageUnit::Credits, used, Some(face.total))
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
            let raw = ["PackageName", "SubscriptionPackageName", "resourceName", "name", "productName", "packageName"]
                .iter()
                .find_map(|k| {
                    pack.get(*k)
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                });
            if let Some(raw) = raw {
                // "CodeBuddy个人版国内运营裂变包" is a product + campaign.
                // Keep the tier stable and put the campaign in parentheses;
                // a new promotion must not rename the plan.
                let (base, campaign) = super::split_plan_campaign(&raw);
                face.plan = Some(super::plan_with_campaign(&base, campaign.as_deref()));
            }
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

/// A resource pack expiring inside this window is called out on the card.
const EXPIRING_SOON_DAYS: i64 = 7;

struct PackageDetail {
    name: String,
    remain: f64,
    total: Option<f64>,
    expire_ms: Option<i64>,
}

fn package_name(pack: &Value) -> String {
    ["PackageName", "packageName", "resourceName", "name", "productName"]
        .iter()
        .find_map(|key| {
            pack.get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Credits package".to_string())
}

/// The pack's own end instant. The vendor uses `DeductionEndTime` /
/// `ExpiredTime` for the real deadline and `CycleEndTime` for the billing
/// cycle; a deduction stamp more than a year past the cycle is a placeholder
/// and the cycle wins. Anything further out than two years is treated as
/// "no deadline" rather than a fake countdown.
fn pack_expire_ms(pack: &Value, now_ms: i64) -> Option<i64> {
    const YEAR_MS: i64 = 365 * 86_400_000;
    let cycle = ["CycleEndTime", "cycleEndTime"]
        .iter()
        .filter_map(|key| pack.get(*key))
        .filter_map(timestamp_ms)
        .min();
    let deduction = ["DeductionEndTime", "deductionEndTime", "ExpiredTime", "expiredTime"]
        .iter()
        .filter_map(|key| pack.get(*key))
        .filter_map(timestamp_ms)
        .min();
    let chosen = match (cycle, deduction) {
        (Some(cycle), Some(deduction)) if deduction > cycle + YEAR_MS => Some(cycle),
        (_, Some(deduction)) => Some(deduction),
        (Some(cycle), None) => Some(cycle),
        (None, None) => None,
    }?;
    (chosen - now_ms <= 2 * YEAR_MS).then_some(chosen)
}

fn earliest_pack_expiry(packs: &[&Value], now_ms: i64) -> Option<i64> {
    package_details(packs, now_ms)
        .into_iter()
        .filter_map(|detail| detail.expire_ms)
        .min()
}

fn package_details(packs: &[&Value], now_ms: i64) -> Vec<PackageDetail> {
    packs
        .iter()
        .filter(|pack| is_credits_pack(pack))
        .filter_map(|pack| {
            // Unlimited packs carry -1 placeholders: no meter, no deadline.
            let unlimited = pack.get("Unlimited").and_then(Value::as_bool).unwrap_or(false)
                || pack_num(pack, &["CycleCapacitySizePrecise", "CycleCapacitySize", "CycleTotalCapacity", "totalAmount", "total"]) == Some(-1.0);
            if unlimited {
                return None;
            }
            let remain = pack_num(
                pack,
                &["CycleCapacityRemainPrecise", "CycleCapacityRemain", "CycleRemainCapacity", "remainAmount", "remain"],
            )?;
            if !(remain > 0.0) {
                return None;
            }
            let total = pack_num(
                pack,
                &["CycleCapacitySizePrecise", "CycleCapacitySize", "CycleTotalCapacity", "totalAmount", "total"],
            )
            .filter(|total| *total > 0.0);
            Some(PackageDetail {
                name: package_name(pack),
                remain,
                total,
                expire_ms: pack_expire_ms(pack, now_ms),
            })
        })
        .collect()
}

/// Package rows. Individual rows are reserved for packs that need
/// attention (expiring within a week, at most three); everything else is
/// folded into ONE summary row. A real WorkBuddy account can hold a dozen
/// near-identical packs, and eleven rows would bury the meter they belong
/// to.
fn package_detail_rows(packs: &[&Value], now_ms: i64) -> Vec<Metric> {
    let mut details = package_details(packs, now_ms);
    if details.is_empty() {
        return Vec::new();
    }
    details.sort_by_key(|detail| detail.expire_ms.unwrap_or(i64::MAX));
    let soon_count = details
        .iter()
        .take_while(|detail| {
            detail
                .expire_ms
                .is_some_and(|expire| expire - now_ms <= EXPIRING_SOON_DAYS * 86_400_000)
        })
        .count()
        .min(3);
    if soon_count == 0 && details.len() < 2 {
        return Vec::new();
    }

    let mut rows: Vec<Metric> = details
        .iter()
        .take(soon_count)
        .map(|detail| {
            let mut text = match detail.total {
                Some(total) => format!("{:.0}/{:.0} left", detail.remain, total),
                None => format!("{:.0} left", detail.remain),
            };
            if let Some(expire) = detail.expire_ms {
                text.push_str(&format!(
                    " · expires {}",
                    chrono::DateTime::from_timestamp_millis(expire)
                        .map(|at| at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ));
                let days = ((expire - now_ms).max(0) as f64 / 86_400_000.0).ceil() as i64;
                text.push_str(&format!(" · expires in {days}d"));
            } else {
                text.push_str(" · expiry unknown");
            }
            Metric::text(&detail.name, text)
        })
        .collect();

    let rest = &details[soon_count..];
    if !rest.is_empty() {
        let remain: f64 = rest.iter().map(|detail| detail.remain).sum();
        let mut text = format!("{} packages · {:.0} credits left", rest.len(), remain);
        match rest.iter().filter_map(|detail| detail.expire_ms).min() {
            Some(expire) => {
                if let Some(at) = chrono::DateTime::from_timestamp_millis(expire) {
                    text.push_str(&format!(
                        " · next expiry {}",
                        at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M")
                    ));
                }
            }
            None => text.push_str(" · expiry unknown"),
        }
        rows.push(Metric::text("Packages", text));
    }
    rows
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
            email: String::new(),
            nickname: String::new(),
            enterprise_id: String::new(),
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
    fn a_promotional_package_keeps_its_product_name() {
        let face = credits_face(
            &resource_body(json!([{
                "PackageName": "CodeBuddy个人版国内运营裂变包",
                "CycleCapacitySize": 2500.0,
                "CycleCapacityRemain": 2495.0,
                "CapacityUnit": "credits"
            }])),
            NOW_MS,
        )
        .expect("face");
        assert_eq!(
            face.plan.as_deref(),
            Some("CodeBuddy个人版 (国内运营裂变包)"),
            "a campaign must not rename the stable product"
        );
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

    fn billing(status: u16, code: Option<i64>) -> Billing {
        Billing { body: Value::Null, status, code, message: None }
    }

    #[test]
    fn billing_errors_are_distinct_and_readable() {
        assert!(billing_error("/x", &billing(401, None)).contains("session expired"));
        assert!(billing_error("/x", &billing(403, Some(10085))).contains("browser UA"));
        assert!(billing_error("/x", &billing(403, None)).contains("access denied"));
        assert!(billing_error("/x", &billing(429, None)).contains("rate limited"));
        assert!(billing_error("/x", &billing(404, None)).contains("endpoint not found"));
        assert!(billing_error("/x", &billing(503, None)).contains("HTTP 503"));
        let with_message = Billing {
            body: Value::Null,
            status: 200,
            code: Some(500),
            message: Some("no entitlement".into()),
        };
        assert!(billing_error("/x", &with_message).contains("no entitlement"));
    }

    #[test]
    fn billing_success_needs_a_success_code_or_data() {
        assert!(billing(200, Some(0)).ok());
        assert!(billing(200, Some(200)).ok());
        assert!(!billing(200, Some(401)).ok());
        assert!(!billing(401, None).ok());
        assert!(!billing(401, None).ok());
        let with_data = Billing { body: json!({ "data": {} }), status: 200, code: None, message: None };
        assert!(with_data.ok());
        let no_data = Billing { body: json!({}), status: 200, code: None, message: None };
        assert!(!no_data.ok());
        assert!(billing(403, None).rejected());
        assert!(!billing(429, None).rejected());
    }

    #[test]
    fn refresh_errors_are_classified_without_secrets() {
        assert_eq!(refresh_error_kind("token refresh rejected (code): invalid_grant"), "refresh_rejected");
        assert_eq!(refresh_error_kind("token refresh: network timeout"), "network");
        assert_eq!(refresh_error_kind("token refresh rejected (code): expired"), "vendor");
        assert_eq!(refresh_error_kind("something odd"), "unknown");
    }

    #[test]
    fn pack_expiry_prefers_the_real_deadline_and_ignores_placeholders() {
        // Deduction end within a year of the cycle wins.
        let pack = json!({
            "DeductionEndTime": "2026-02-01 00:00:00",
            "CycleEndTime": "2026-03-01 00:00:00"
        });
        assert_eq!(pack_expire_ms(&pack, NOW_MS), Some(1_769_875_200_000));
        // A deduction stamp more than a year past the cycle is a placeholder.
        let placeholder = json!({
            "DeductionEndTime": "2099-01-01 00:00:00",
            "CycleEndTime": "2026-03-01 00:00:00"
        });
        assert_eq!(pack_expire_ms(&placeholder, NOW_MS), Some(1_772_294_400_000));
        // Beyond two years from now reads as unknown, not a countdown.
        assert_eq!(pack_expire_ms(&json!({ "CycleEndTime": "2099-01-01 00:00:00" }), NOW_MS), None);
        assert_eq!(pack_expire_ms(&json!({}), NOW_MS), None);
    }

    #[test]
    fn package_rows_appear_only_when_they_add_information() {
        // One live pack, no deadline: the aggregate meter says it all.
        let single_pack = json!({
            "PackageName": "Pro", "CycleCapacitySize": 500.0, "CycleCapacityRemain": 450.0
        });
        let single = vec![&single_pack];
        assert!(package_detail_rows(&single, NOW_MS).is_empty());

        // Two live packs collapse into ONE summary row, never two.
        let later = json!({
            "PackageName": "Later", "CycleCapacitySize": 100.0, "CycleCapacityRemain": 90.0,
            "CycleEndTime": "2026-03-01 00:00:00"
        });
        let sooner = json!({
            "PackageName": "Sooner", "CycleCapacitySize": 200.0, "CycleCapacityRemain": 20.0,
            "CycleEndTime": "2026-02-01 00:00:00"
        });
        let rows = package_detail_rows(&[&later, &sooner], NOW_MS);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Packages");
        assert_eq!(
            earliest_pack_expiry(&[&later, &sooner], NOW_MS),
            Some(1_769_875_200_000),
            "the sooner pack drives auth-center ordering"
        );
        let text = rows[0].value.as_deref().unwrap_or("");
        assert!(text.contains("2 packages"), "{text}");
        assert!(text.contains("110 credits left"), "{text}");
        assert!(text.contains("next expiry"), "{text}");

        // A pack expiring within 7 days gets its own countdown row.
        let expiring_pack = json!({
            "PackageName": "Expiring", "CycleCapacitySize": 50.0, "CycleCapacityRemain": 10.0,
            "DeductionEndTime": 1_760_300_000_000i64 // ~3.5 days after NOW_MS
        });
        let soon = vec![&expiring_pack];
        let rows = package_detail_rows(&soon, NOW_MS);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].label, "Expiring");
        assert!(rows[0].value.as_deref().unwrap_or("").contains("expires in"));

        // A dozen identical packs stay one summary row plus the expiring one.
        let mut many = vec![expiring_pack.clone()];
        for index in 0..11 {
            many.push(json!({
                "PackageName": "Bulk", "CycleCapacitySize": 10.0, "CycleCapacityRemain": 5.0,
                "CycleEndTime": "2026-06-01 00:00:00", "index": index
            }));
        }
        let refs: Vec<&Value> = many.iter().collect();
        let rows = package_detail_rows(&refs, NOW_MS);
        assert_eq!(rows.len(), 2, "one expiring row + one summary row");
        assert_eq!(rows[0].label, "Expiring");
        assert_eq!(rows[1].label, "Packages");
        assert!(rows[1].value.as_deref().unwrap_or("").contains("11 packages"));

        // A spent pack contributes nothing.
        let spent_pack = json!({
            "PackageName": "Spent", "CycleCapacitySize": 50.0, "CycleCapacityRemain": 0.0
        });
        let spent = vec![&spent_pack];
        assert!(package_detail_rows(&spent, NOW_MS).is_empty());
    }

    #[test]
    fn official_usage_detail_says_requests_and_credits() {
        let summary = OfficialUsage {
            requests: 26,
            credits: 12.5,
            ..OfficialUsage::default()
        };
        assert_eq!(summary.detail(), "26 requests · 12.50 credits");
        let whole = OfficialUsage {
            requests: 3,
            credits: 8.0,
            ..OfficialUsage::default()
        };
        assert_eq!(whole.detail(), "3 requests · 8 credits");
        let partial = OfficialUsage {
            requests: 3000,
            credits: 1.0,
            partial: true,
            ..OfficialUsage::default()
        };
        assert!(partial.detail().ends_with("· partial"));
    }

    /// #31 — a 26-request month used to collapse into one opaque number. The
    /// card now names the biggest model and client, and invents nothing when
    /// the ledger named none.
    #[test]
    fn official_usage_detail_names_the_biggest_model_and_client() {
        let mut report = OfficialUsage {
            requests: 26,
            credits: 12.5,
            ..OfficialUsage::default()
        };
        report.by_model.insert("claude-sonnet-4".into(), 7.2);
        report.by_model.insert("gpt-5".into(), 5.3);
        report.by_client.insert("web".into(), 9.1);
        report.by_client.insert("ide".into(), 3.4);
        assert_eq!(
            report.detail(),
            "26 requests · 12.50 credits · claude-sonnet-4 · via web"
        );

        let bare = OfficialUsage {
            requests: 2,
            credits: 1.0,
            ..OfficialUsage::default()
        };
        assert_eq!(bare.detail(), "2 requests · 1 credits");
    }
}
