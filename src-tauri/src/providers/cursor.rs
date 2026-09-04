use super::{http, Metric, Snapshot};
use serde_json::Value;
use std::path::PathBuf;

const ID: &str = "cursor";
const NAME: &str = "Cursor";

fn state_db_path() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let p = PathBuf::from(appdata)
        .join("Cursor")
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");
    p.exists().then_some(p)
}

/// Pure local probe for the Customize gear panel (no network): the Cursor
/// editor's state database exists on this machine.
pub fn local_credential_hint() -> Option<String> {
    state_db_path().map(|_| "Cursor editor local sign-in".to_string())
}

/// Pure local probe: the Cursor membership/subscription the editor itself
/// recorded (Stripe membership type + status from state.vscdb), so the
/// gear panel can badge "Free" vs "Pro" without a network call. Reads a
/// temporary copy, same dance as the token read.
pub fn local_membership() -> Option<String> {
    let Some(db_path) = state_db_path() else { return None };
    let tmp = std::env::temp_dir().join(format!("openusage-cursor-membership-{}.vscdb", std::process::id()));
    let read = |conn: &rusqlite::Connection| -> rusqlite::Result<(Option<String>, Option<String>)> {
        let get = |key: &str| -> rusqlite::Result<Option<String>> {
            match conn.query_row("SELECT value FROM ItemTable WHERE key = ?1", [key], |r| {
                r.get::<_, String>(0)
            }) {
                Ok(v) => Ok(Some(v)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e),
            }
        };
        Ok((get("cursorAuth/stripeMembershipType")?, get("cursorAuth/stripeSubscriptionStatus")?))
    };
    let pair = match std::fs::copy(&db_path, &tmp) {
        Ok(_) => {
            let out = rusqlite::Connection::open_with_flags(&tmp, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .and_then(|conn| read(&conn))
                .ok();
            let _ = std::fs::remove_file(&tmp);
            out
        }
        Err(_) => None,
    };
    let (membership, status) = pair?;
    let membership = membership.map(|v| unquote(&v)).filter(|v| !v.is_empty());
    let status = status.map(|v| unquote(&v)).filter(|v| !v.is_empty() && !v.eq_ignore_ascii_case("active"));
    // Normalize the same way cockpit does (types/cursor.ts): student→pro,
    // business/team→enterprise; the badge shows Free / Pro / Pro+ / Ultra.
    let badge = membership.map(|m| match m.to_ascii_lowercase().as_str() {
        "free" | "free_trial" => "Free".to_string(),
        "pro" | "pro_student" | "pro_trial" => "Pro".to_string(),
        "pro_plus" => "Pro+".to_string(),
        "ultra" => "Ultra".to_string(),
        "business" | "team" | "enterprise" => "Team/Enterprise".to_string(),
        other => title_case(other),
    });
    match (badge, status) {
        (Some(m), Some(s)) => Some(format!("{m} · {}", title_case(&s))),
        (Some(m), None) => Some(m),
        (None, Some(s)) => Some(title_case(&s)),
        (None, None) => None,
    }
}

fn read_pair(conn: &rusqlite::Connection) -> Result<(Option<String>, Option<String>), rusqlite::Error> {
    let get = |key: &str| -> Result<Option<String>, rusqlite::Error> {
        match conn.query_row("SELECT value FROM ItemTable WHERE key = ?1", [key], |r| {
            r.get::<_, String>(0)
        }) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    };
    Ok((get("cursorAuth/accessToken")?, get("cursorAuth/refreshToken")?))
}

/// Cursor stores its session token in a small SQLite database. The running
/// Cursor app may hold a lock on it, so we read from a temporary copy —
/// retried a few times because the copy loses to Cursor's own writes now
/// and then, and finally via a lock-free immutable open of the original.
fn read_state_values() -> Result<(Option<String>, Option<String>), String> {
    let Some(db_path) = state_db_path() else {
        return Ok((None, None));
    };
    let tmp = std::env::temp_dir().join(format!("openusage-cursor-{}.vscdb", std::process::id()));

    let mut copy_err = String::new();
    for attempt in 0..3 {
        match std::fs::copy(&db_path, &tmp) {
            Ok(_) => {
                let result = rusqlite::Connection::open_with_flags(
                    &tmp,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )
                .and_then(|conn| read_pair(&conn));
                let _ = std::fs::remove_file(&tmp);
                return result.map_err(|e| format!("read state.vscdb: {e}"));
            }
            Err(e) => {
                copy_err = e.to_string();
                if attempt < 2 {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                }
            }
        }
    }

    // Copy kept losing to Cursor's lock: open the real file read-only and
    // immutable (SQLite promises not to write, so no lock is taken).
    let uri = format!("file:{}?immutable=1", db_path.to_string_lossy().replace('\\', "/"));
    match rusqlite::Connection::open_with_flags(
        &uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .and_then(|conn| read_pair(&conn))
    {
        Ok(pair) => Ok(pair),
        Err(e) => Err(format!("copy state.vscdb: {copy_err}; immutable open: {e}")),
    }
}

/// Values in ItemTable are sometimes stored as JSON strings ("\"abc\"").
fn unquote(v: &str) -> String {
    v.trim().trim_matches('"').to_string()
}

fn jwt_sub(token: &str) -> Option<String> {
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let claims: Value = serde_json::from_slice(&bytes).ok()?;
    claims.get("sub").and_then(Value::as_str).map(str::to_string)
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// The dashboard's usage-events CSV export — the raw material for Cursor
/// spend tiles. Cached briefly so live usage shows up within minutes like
/// every other spend source (the 31-day export is only a few KB); a failed
/// refetch serves the last good copy instead of blanking the Cursor rows.
pub async fn fetch_usage_csv() -> Option<String> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<(i64, String)>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new((0, String::new())));
    let now = chrono::Utc::now().timestamp_millis();
    if let Ok(c) = cache.lock() {
        if now - c.0 < 300_000 && !c.1.is_empty() {
            return Some(c.1.clone());
        }
    }
    // Any failure below falls back to the last good export, however old —
    // stale spend beats a Cursor card that loses its rows on one bad fetch.
    let stale = || {
        cache
            .lock()
            .ok()
            .filter(|c| !c.1.is_empty())
            .map(|c| c.1.clone())
    };

    // Prefer a token refreshed by fetch() this run — the stored one may
    // have expired since Cursor last wrote it.
    let Some(token) = refreshed_token()
        .lock()
        .ok()
        .and_then(|t| t.clone())
        .or_else(|| read_state_values().ok()?.0.map(|t| unquote(&t)))
    else {
        return stale();
    };
    if token.is_empty() {
        return stale();
    }
    let Some(sub) = jwt_sub(&token) else { return stale() };
    let user_id = sub.split('|').next_back().unwrap_or(&sub).to_string();
    let cookie = format!("WorkosCursorSessionToken={user_id}%3A%3A{token}");

    // The export answers 200-with-empty unless it's given an explicit
    // range; strategy=tokens yields the per-model token columns the spend
    // parser prices (same query Cursor's dashboard sends).
    let end = chrono::Utc::now().timestamp_millis();
    let start = end - 31 * 24 * 3_600_000;
    let resp = match http()
        .get(format!(
            "https://cursor.com/api/dashboard/export-usage-events-csv?startDate={start}&endDate={end}&strategy=tokens"
        ))
        .header("Cookie", &cookie)
        .header("Accept", "text/csv")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return stale(),
    };
    if !resp.status().is_success() {
        eprintln!("[pane] cursor csv: HTTP {}", resp.status());
        return stale();
    }
    let Ok(body) = resp.text().await else { return stale() };
    if body.trim().is_empty() {
        return stale();
    }
    if let Ok(mut c) = cache.lock() {
        *c = (now, body.clone());
    }
    Some(body)
}

/// OAuth client id Cursor's own dashboard uses for token refreshes
/// (research credit: robinebers/openusage's Cursor provider).
const CLIENT_ID: &str = "KbZUR41cY7W6zRSdpSUJ7I7mLYBKOCmB";

/// Access token refreshed via the OAuth endpoint this app run. Kept in
/// memory only — Cursor's own state.vscdb is never written to.
fn refreshed_token() -> &'static std::sync::Mutex<Option<String>> {
    static T: std::sync::OnceLock<std::sync::Mutex<Option<String>>> = std::sync::OnceLock::new();
    T.get_or_init(|| std::sync::Mutex::new(None))
}

/// Connect-RPC POST to Cursor's dashboard service. Returns Ok(None) on
/// 401/403 so the caller can refresh and retry.
async fn connect_post(method: &str, token: &str) -> Result<Option<Value>, String> {
    let resp = http()
        .post(format!("https://api2.cursor.sh/aiserver.v1.DashboardService/{method}"))
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .body("{}")
        .send()
        .await
        .map_err(|e| format!("{method}: {e}"))?;
    match resp.status().as_u16() {
        401 | 403 => Ok(None),
        s if !(200..300).contains(&(s as i32)) => Err(format!("{method}: HTTP {s}")),
        _ => resp
            .json::<Value>()
            .await
            .map(Some)
            .map_err(|e| format!("{method} parse: {e}")),
    }
}

async fn refresh_access_token(refresh: &str) -> Option<String> {
    let resp = http()
        .post("https://api2.cursor.sh/oauth/token")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": CLIENT_ID,
            "refresh_token": refresh,
        }))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        eprintln!("[pane] cursor token refresh: HTTP {}", resp.status());
        return None;
    }
    let v: Value = resp.json().await.ok()?;
    let token = v.get("access_token")?.as_str()?.to_string();
    if let Ok(mut t) = refreshed_token().lock() {
        *t = Some(token.clone());
    }
    Some(token)
}

fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(|x| x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok())))
}

fn dollars(cents: f64) -> String {
    if cents >= 10_000.0 {
        format!("${:.0}", cents / 100.0)
    } else {
        format!("${:.2}", cents / 100.0)
    }
}

/// Promo / grant balance from `GetCreditGrantsBalance`. Live shape
/// (2026-08): `{hasCreditGrants, creditBalanceCents, totalCents,
/// usedCents}` with cents as strings or numbers. Returns None when
/// there is nothing to show — never a 0% bar for a missing pool.
fn credit_grants_metric(grants: &Value) -> Option<Metric> {
    let has = grants.get("hasCreditGrants").and_then(Value::as_bool);
    let total = num(grants.get("totalCents")).unwrap_or(0.0);
    let used = num(grants.get("usedCents")).unwrap_or(0.0);
    let remaining = num(grants.get("creditBalanceCents"))
        .unwrap_or_else(|| (total - used).max(0.0));
    // Explicit false wins even if leftover totals are still on the payload
    // (expired grant). A 100% bar from that would also trip Almost Out.
    if has == Some(false) {
        return None;
    }
    if total <= 0.0 && remaining <= 0.0 {
        if has == Some(true) {
            eprintln!(
                "[pane] cursor credit grants: hasCreditGrants but no totalCents/creditBalanceCents"
            );
        }
        return None;
    }
    if total > 0.0 {
        let pct = ((total - remaining) / total * 100.0).clamp(0.0, 100.0);
        Some(Metric::progress(
            "Credits",
            pct,
            Some(format!("{} left of {}", dollars(remaining), dollars(total))),
        ))
    } else {
        Some(Metric::text("Credits", dollars(remaining)))
    }
}

/// Cursor's sponsored / gifted bonus pool (`planUsage.bonusSpend`) — free
/// usage model providers cover beyond the plan, not money the user owes.
/// Shown as a text row (tucked behind Show more like other balances), not
/// a bar: the RPC never names the pool size, so the ceiling would have to
/// be derived from `totalPercentUsed` (`totalSpend / pct - includedSpend`)
/// — Cursor's percent fields have contradicted each other before (the
/// bucket-era 0% bug, `remainingBonus:false` against a 36% total), so the
/// derived number rides along as context when it's sane, never as a meter.
fn bonus_metric(plan_usage: &Value, total_pct: Option<f64>) -> Option<Metric> {
    let bonus_spend = num(plan_usage.get("bonusSpend")).filter(|v| *v > 0.0)?;
    let included = num(plan_usage.get("includedSpend")).unwrap_or(0.0);
    let bonus_pool = match (num(plan_usage.get("totalSpend")), total_pct) {
        (Some(spent), Some(pct)) if pct >= 1.0 && spent > 0.0 => {
            (spent / pct * 100.0 - included).max(0.0)
        }
        _ => 0.0,
    };
    Some(if bonus_pool >= bonus_spend && bonus_pool > 0.0 {
        Metric::text(
            "Bonus",
            format!("{} of {} used", dollars(bonus_spend), dollars(bonus_pool)),
        )
    } else {
        Metric::text("Bonus", format!("{} used", dollars(bonus_spend)))
    })
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Overwrites a fallback snapshot's identity (summary/legacy return the
/// bare family id) so a multi-account card keeps its own id.
fn rename_snapshot(mut snap: Snapshot, id: &str, name: &str) -> Snapshot {
    snap.id = id.to_string();
    snap.name = name.to_string();
    snap
}

async fn fetch() -> Result<Snapshot, String> {
    let (access_raw, refresh_raw) = read_state_values()?;
    let Some(token_raw) = access_raw else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Cursor sign-in not found. Open Cursor and log in.",
        ));
    };
    let stored = unquote(&token_raw);
    if stored.is_empty() {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Cursor sign-in not found. Open Cursor and log in.",
        ));
    }
    let refresh = refresh_raw.map(|r| unquote(&r)).filter(|r| !r.is_empty());
    fetch_with_token(stored, refresh, ID, NAME, None).await
}

/// Quota snapshot for one imported/multi-account Cursor login: the same
/// dashboard RPCs, driven by the account's own token pair instead of the
/// local editor's. `membership` is the account's own tier badge (from the
/// import/OAuth payload) — the local editor's membership never applies to
/// an imported account.
pub async fn snapshot_with_token_as(
    access_token: &str,
    refresh_token: Option<&str>,
    card_id: &str,
    card_name: &str,
    membership: Option<&str>,
) -> Snapshot {
    match fetch_with_token(
        access_token.to_string(),
        refresh_token.map(str::to_string),
        card_id,
        card_name,
        membership.map(str::to_string),
    )
    .await
    {
        Ok(s) => s,
        Err(e) => Snapshot::error(card_id, card_name, e),
    }
}

async fn fetch_with_token(
    stored: String,
    refresh: Option<String>,
    card_id: &str,
    card_name: &str,
    account_membership: Option<String>,
) -> Result<Snapshot, String> {
    // Prefer a token we refreshed ourselves this run; the stored one may
    // be stale if Cursor hasn't been opened in a while.
    let mut token = refreshed_token()
        .lock()
        .ok()
        .and_then(|t| t.clone())
        .filter(|_| card_id == ID)
        .unwrap_or_else(|| stored.clone());

    // Current-generation usage: percent of the plan's included usage,
    // via the same dashboard RPCs Cursor's web dashboard calls.
    // A transient failure of the new API must not strand legacy-plan
    // users whose data lives behind the old endpoint — try that before
    // giving up. But on bucket-era accounts the old endpoint still
    // answers, with `numRequests: 0` and no cap: an "ok" card holding only
    // "Requests this cycle 0" would replace the real bars until the next
    // successful call (no last-good restore, since the status is ok). So
    // only a legacy answer with a real request quota counts; otherwise the
    // original error surfaces and the last-good cache keeps the bars.
    // Outdated only after the three-minute stale grace — one failed
    // refresh still looks like the previous card, unmarked.
    //
    // api2.cursor.sh is a different host from cursor.com, and on some
    // networks its TLS handshake times out for minutes while cursor.com
    // keeps answering. The dashboard's REST usage-summary carries the same
    // plan figures (what upstream OpenUsage reads for Enterprise), so it
    // is tried first: the card stays live rather than stale.
    let mut usage = match connect_post("GetCurrentPeriodUsage", &token).await {
        Ok(u) => u,
        Err(e) => {
            if let Ok(s) = summary_fetch(&token).await {
                return Ok(rename_snapshot(s, card_id, card_name));
            }
            return match legacy_fetch(&token).await {
                Ok(s) => Ok(rename_snapshot(s, card_id, card_name)),
                Err(_) => Err(e),
            };
        }
    };
    if usage.is_none() {
        if let Some(fresh) = match &refresh {
            Some(r) => refresh_access_token(r).await,
            None => None,
        } {
            token = fresh;
            usage = connect_post("GetCurrentPeriodUsage", &token).await?;
        }
    }
    let Some(usage) = usage else {
        return Err("Cursor session expired — open Cursor once to refresh it".into());
    };

    let enabled = usage.get("enabled").and_then(Value::as_bool) != Some(false);
    let plan_usage = usage.get("planUsage").filter(|v| v.is_object());
    let limit = plan_usage.and_then(|p| num(p.get("limit")));
    let total_pct = plan_usage.and_then(|p| num(p.get("totalPercentUsed")));

    // Legacy request-quota accounts (and team/enterprise plans that hide
    // dollar pools) still answer the old REST endpoint. Use the effective
    // token — the stored one may be the stale token we just replaced.
    // usage-summary goes first: Enterprise/team accounts that hide
    // planUsage from the RPC still report percentages there.
    if !enabled || plan_usage.is_none() || (limit.is_none() && total_pct.is_none()) {
        if let Ok(s) = summary_fetch(&token).await {
            return Ok(rename_snapshot(s, card_id, card_name));
        }
        return legacy_fetch(&token).await.map(|s| rename_snapshot(s, card_id, card_name));
    }
    let plan_usage = plan_usage.unwrap();

    let plan_req = connect_post("GetPlanInfo", &token);
    let credits_req = connect_post("GetCreditGrantsBalance", &token);
    let (plan_info, credit_grants) = tokio::join!(plan_req, credits_req);

    let mut plan = plan_info
        .ok()
        .flatten()
        .and_then(|p| p.get("planName").and_then(Value::as_str).map(title_case))
        .filter(|p| !p.is_empty());
    // Some accounts answer GetPlanInfo without a name — the Stripe
    // membership endpoint still knows ("pro", "ultra", ...).
    if plan.is_none() {
        if let Some(sub) = jwt_sub(&token) {
            let user_id = sub.split('|').next_back().unwrap_or(&sub).to_string();
            let cookie = format!("WorkosCursorSessionToken={user_id}%3A%3A{token}");
            if let Ok(r) = http()
                .get("https://cursor.com/api/auth/stripe")
                .header("Cookie", &cookie)
                .send()
                .await
            {
                if r.status().is_success() {
                    if let Ok(v) = r.json::<Value>().await {
                        plan = v
                            .get("membershipType")
                            .and_then(Value::as_str)
                            .map(title_case)
                            .filter(|p| !p.is_empty());
                    }
                }
            }
        }
    }
    // The editor's own local record of the membership is the ground truth
    // for the badge and, on Free accounts, for dropping the fake "spend
    // limit" pool Cursor's API reports (a Free plan has no dollar quota —
    // $50 in planUsage is a display artifact the dashboard never shows).
    // Membership ground truth: the local editor's DB only describes the
    // LOCALLY logged-in account — an imported account carries its own
    // badge from the import/OAuth payload and must not read the local one.
    let membership = if card_id == ID {
        local_membership()
    } else {
        account_membership.clone()
    };
    if plan.is_none() {
        plan = membership.clone();
    }
    let free_account = membership
        .as_deref()
        .map(|m| m.to_ascii_lowercase().starts_with("free"))
        .unwrap_or(false);

    // Billing cycle bounds (epoch ms) drive the pace projection.
    let cycle_start = num(usage.get("billingCycleStart"));
    let cycle_end = num(usage.get("billingCycleEnd"));
    const MONTH_MS: i64 = 30 * 24 * 3_600_000;
    let (resets_at, period_ms) = match (cycle_start, cycle_end) {
        (Some(s), Some(e)) if e > s => (Some(e as i64), (e - s) as i64),
        (_, Some(e)) => (Some(e as i64), MONTH_MS),
        _ => (None, MONTH_MS),
    };

    let mut metrics = Vec::new();

    // Unexpired credit grants — money that gets burned before the plan
    // pool does. Cursor reports the pool size (`totalCents`) so this is
    // a real used/total bar like Codex Extra credits, not a high-water
    // guess. Cents often arrive as strings; `num()` accepts both.
    match credit_grants {
        Ok(Some(grants)) => {
            if let Some(row) = credit_grants_metric(&grants) {
                metrics.push(row);
            }
        }
        Ok(None) => {}
        Err(e) => eprintln!("[pane] cursor credit grants: {e}"),
    }

    let spend_limit = usage.get("spendLimitUsage").filter(|v| v.is_object());
    let spend_type = spend_limit
        .and_then(|s| s.get("limitType").and_then(Value::as_str))
        .map(str::to_lowercase);
    let pooled_limit = spend_limit.and_then(|s| num(s.get("pooledLimit"))).unwrap_or(0.0);
    let is_team = plan.as_deref().map(|p| p.eq_ignore_ascii_case("team")) == Some(true)
        || spend_type.as_deref() == Some("team")
        || pooled_limit > 0.0;

    // Spend is only KNOWN when Cursor actually reports it: totalSpend,
    // else limit-remaining when BOTH exist. Defaulting a missing
    // `remaining` to 0 made used == limit — a 100% bar, "Limit reached",
    // and run-out notifications for an account whose planUsage carries
    // only the limit (Devin's find on the untestable pre-bucket path).
    let used_cents_opt = num(plan_usage.get("totalSpend")).or_else(|| {
        match (limit, num(plan_usage.get("remaining"))) {
            (Some(l), Some(r)) => Some((l - r).max(0.0)),
            _ => None,
        }
    });
    let used_cents = used_cents_opt.unwrap_or(0.0);

    // The per-bucket bars mirror Cursor's own Plan & Usage page — "Cursor
    // Models" (the auto bucket: Composer, Cursor Grok, …) and "Other
    // Models" — and render for EVERY account shape that reports them,
    // team included (they always did; a restructure briefly scoped them
    // to non-team accounts and Devin caught the regression). Free
    // accounts skip them: Free has no model buckets, only slow requests.
    let auto_pct = num(plan_usage.get("autoPercentUsed"));
    let api_pct = num(plan_usage.get("apiPercentUsed"));
    if !free_account {
        if let Some(auto) = auto_pct {
            metrics.push(
                Metric::progress("Cursor Models", auto.clamp(0.0, 100.0), None)
                    .with_reset(resets_at, Some(period_ms)),
            );
        }
        if let Some(api) = api_pct {
            metrics.push(
                Metric::progress("Other Models", api.clamp(0.0, 100.0), None)
                    .with_reset(resets_at, Some(period_ms)),
            );
        }
        if let Some(row) = bonus_metric(plan_usage, total_pct) {
            metrics.push(row);
        }
    }

    if is_team {
        // Team-shaped accounts sometimes omit the plan limit (or report
        // zero, which would divide to NaN); the legacy request endpoint
        // is what still describes them (same fallback upstream uses).
        let limit_cents = match limit {
            Some(l) if l > 0.0 => l,
            _ => {
                return legacy_fetch(&token)
                    .await
                    .map(|s| rename_snapshot(s, card_id, card_name))
            }
        };
        metrics.push(
            Metric::progress(
                "Total usage",
                (used_cents / limit_cents * 100.0).clamp(0.0, 100.0),
                Some(format!("{} / {} this cycle", dollars(used_cents), dollars(limit_cents))),
            )
            .with_reset(resets_at, Some(period_ms)),
        );
    } else if auto_pct.is_some() || api_pct.is_some() {
        // Bucket-era personal plans: Cursor's page shows the two bars and
        // NO total bar. There is no honest total percent here: the $20
        // "limit" is only the Other-Models/API floor, totalPercentUsed
        // measures against included+bonus pools (~$345 live), and the
        // API's own displayMessage does spend/$20 math — three Cursor
        // numbers that all contradict the dashboard. Dollars spent stay
        // visible as a text row; only the misleading percent is gone.
        // The cycle reset stays visible on the bars above; the text row's
        // with_reset rides along for local-API consumers only.
        if let Some(u) = used_cents_opt {
            metrics.push(
                Metric::text("Total usage", format!("{} this cycle", dollars(u)))
                    .with_reset(resets_at, Some(period_ms)),
            );
        }
    } else {
        // Pre-bucket accounts: the classic included-pool bar — computed
        // spend/limit so the bar always matches its own caption, but ONLY
        // when spend is actually reported; otherwise fall back to the
        // API's own percent rather than fabricating one.
        // Free accounts: skip the bar entirely — planUsage's "limit" is a
        // display artifact on Free, where there is no dollar quota at all.
        if !free_account {
            let pct = match (used_cents_opt, limit) {
                (Some(u), Some(l)) if l > 0.0 => u / l * 100.0,
                _ => total_pct.unwrap_or(0.0),
            };
            let detail = match (used_cents_opt, limit) {
                (Some(u), Some(l)) => Some(format!("{} of {} included", dollars(u), dollars(l))),
                _ => None,
            };
            metrics.push(
                Metric::progress("Total usage", pct.clamp(0.0, 100.0), detail)
                    .with_reset(resets_at, Some(period_ms)),
            );
        }
    }

    if let Some(s) = spend_limit {
        let od_limit = num(s.get("individualLimit")).or(num(s.get("pooledLimit"))).unwrap_or(0.0);
        let od_remaining =
            num(s.get("individualRemaining")).or(num(s.get("pooledRemaining"))).unwrap_or(0.0);
        let od_spent = [
            num(s.get("individualUsed")),
            num(s.get("pooledUsed")),
            num(s.get("totalSpend")),
        ]
        .into_iter()
        .flatten()
        .find(|v| *v > 0.0)
        .unwrap_or_else(|| (od_limit - od_remaining).max(0.0));
        if od_limit > 0.0 {
            metrics.push(Metric::progress(
                "On-demand",
                (od_spent / od_limit * 100.0).clamp(0.0, 100.0),
                Some(format!("{} / {}", dollars(od_spent), dollars(od_limit))),
            ));
        } else if od_spent > 0.0 {
            metrics.push(Metric::text("On-demand", dollars(od_spent)));
        }
    }

    Ok(Snapshot::ok(card_id, card_name, plan, metrics))
}

/// Cursor's web session cookie is "<user_id>::<jwt>"; the user id is the
/// part of the JWT `sub` claim after the "auth0|" prefix.
fn session_cookie(token: &str) -> Option<(String, String)> {
    let sub = jwt_sub(token)?;
    let user_id = sub.split('|').next_back().unwrap_or(&sub).to_string();
    let cookie = format!("WorkosCursorSessionToken={user_id}%3A%3A{token}");
    Some((user_id, cookie))
}

fn iso_ms(v: Option<&Value>) -> Option<i64> {
    let s = v?.as_str()?;
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp_millis())
}

/// The dashboard's REST usage report ΓÇö the plan figures the Connect RPC
/// carries, served from cursor.com instead of api2.cursor.sh. Live shape
/// (Ultra, 2026-09): `individualUsage.plan.{used,limit,remaining,
/// autoPercentUsed,apiPercentUsed,totalPercentUsed}`,
/// `individualUsage.onDemand.{enabled,used,limit,remaining}`,
/// `membershipType`, `limitType`, ISO `billingCycleStart/End`. Enterprise
/// accounts add `teamUsage.{pooled,onDemand}`. Credits and bonus rows live
/// only behind the RPC, so this card is the plan bars alone.
async fn summary_fetch(token: &str) -> Result<Snapshot, String> {
    let (_, cookie) = session_cookie(token).ok_or("could not decode Cursor session token")?;
    let resp = http()
        .get("https://cursor.com/api/usage-summary")
        .header("Cookie", &cookie)
        .send()
        .await
        .map_err(|e| format!("usage-summary request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("usage-summary: HTTP {}", resp.status()));
    }
    let doc: Value = resp
        .json()
        .await
        .map_err(|e| format!("usage-summary parse: {e}"))?;
    summary_snapshot(&doc).ok_or_else(|| "usage-summary had no plan figures".into())
}

fn summary_snapshot(doc: &Value) -> Option<Snapshot> {
    let individual = doc.get("individualUsage").filter(|v| v.is_object());
    let team = doc.get("teamUsage").filter(|v| v.is_object());
    // A disabled individual plan is "no personal meter", not "no card".
    // Enterprise reports that placeholder next to a real team pool.
    let plan_usage = individual
        .and_then(|i| i.get("plan"))
        .filter(|v| v.is_object())
        .filter(|p| p.get("enabled").and_then(Value::as_bool) != Some(false));

    const MONTH_MS: i64 = 30 * 24 * 3_600_000;
    let cycle_start = iso_ms(doc.get("billingCycleStart"));
    let cycle_end = iso_ms(doc.get("billingCycleEnd"));
    let (resets_at, period_ms) = match (cycle_start, cycle_end) {
        (Some(s), Some(e)) if e > s => (Some(e), e - s),
        (_, Some(e)) => (Some(e), MONTH_MS),
        _ => (None, MONTH_MS),
    };

    let plan = doc
        .get("membershipType")
        .and_then(Value::as_str)
        .map(title_case)
        .filter(|p| !p.is_empty());
    let limit_type = doc
        .get("limitType")
        .and_then(Value::as_str)
        .map(str::to_lowercase);
    let is_team = limit_type.as_deref() == Some("team")
        || plan.as_deref().map(|p| p.eq_ignore_ascii_case("team")) == Some(true);

    let auto_pct = plan_usage.and_then(|p| num(p.get("autoPercentUsed")));
    let api_pct = plan_usage.and_then(|p| num(p.get("apiPercentUsed")));
    let total_pct = plan_usage.and_then(|p| num(p.get("totalPercentUsed")));
    let limit = plan_usage
        .and_then(|p| num(p.get("limit")))
        .filter(|l| *l > 0.0);
    let used_cents_opt = plan_usage.and_then(|p| {
        num(p.get("used")).or_else(|| match (limit, num(p.get("remaining"))) {
            (Some(l), Some(r)) => Some((l - r).max(0.0)),
            _ => None,
        })
    });

    let mut metrics = Vec::new();
    if let Some(auto) = auto_pct {
        metrics.push(
            Metric::progress("Cursor Models", auto.clamp(0.0, 100.0), None)
                .with_reset(resets_at, Some(period_ms)),
        );
    }
    if let Some(api) = api_pct {
        metrics.push(
            Metric::progress("Other Models", api.clamp(0.0, 100.0), None)
                .with_reset(resets_at, Some(period_ms)),
        );
    }

    // Same total-row rules as the RPC path: team accounts get the dollar
    // pool bar (individual, else pooled), bucket-era personal plans a text
    // row (Cursor's page shows no total bar), pre-bucket plans the classic
    // included-pool bar.
    let pooled = team
        .and_then(|t| t.get("pooled"))
        .filter(|v| v.is_object())
        .filter(|p| p.get("enabled").and_then(Value::as_bool) != Some(false));
    let pooled_limit = pooled
        .and_then(|p| num(p.get("limit")))
        .filter(|l| *l > 0.0);
    if is_team || pooled_limit.is_some() {
        // No individual limit and no pooled cap is the live Enterprise
        // shape: keep Cursor Models / Other Models / On-demand and skip
        // the Total usage dollar bar instead of discarding the card.
        let dollar_pool = match (used_cents_opt, limit) {
            (Some(u), Some(l)) => Some((u, l)),
            _ => match (pooled_limit, pooled) {
                (Some(l), Some(p)) => {
                    let used = num(p.get("used"))
                        .filter(|u| *u > 0.0)
                        .or_else(|| num(p.get("remaining")).map(|r| (l - r).max(0.0)))
                        .unwrap_or(0.0);
                    Some((used, l))
                }
                _ => None,
            },
        };
        if let Some((used, cap)) = dollar_pool {
            metrics.push(
                Metric::progress(
                    "Total usage",
                    (used / cap * 100.0).clamp(0.0, 100.0),
                    Some(format!("{} / {} this cycle", dollars(used), dollars(cap))),
                )
                .with_reset(resets_at, Some(period_ms)),
            );
        }
    } else if auto_pct.is_some() || api_pct.is_some() {
        if let Some(u) = used_cents_opt {
            metrics.push(
                Metric::text("Total usage", format!("{} this cycle", dollars(u)))
                    .with_reset(resets_at, Some(period_ms)),
            );
        }
    } else {
        let pct = match (used_cents_opt, limit) {
            (Some(u), Some(l)) => u / l * 100.0,
            _ => total_pct?,
        };
        let detail = match (used_cents_opt, limit) {
            (Some(u), Some(l)) => Some(format!("{} of {} included", dollars(u), dollars(l))),
            _ => None,
        };
        metrics.push(
            Metric::progress("Total usage", pct.clamp(0.0, 100.0), detail)
                .with_reset(resets_at, Some(period_ms)),
        );
    }

    // The headline On-demand card is user-scoped; the team aggregate only
    // when Cursor omits the individual bucket (placeholder buckets come
    // with `enabled: false` or no limit and must not block the fallback).
    let od = [individual, team]
        .into_iter()
        .flatten()
        .filter_map(|scope| scope.get("onDemand").filter(|v| v.is_object()))
        .find(|b| {
            b.get("enabled").and_then(Value::as_bool) != Some(false)
                && (num(b.get("limit")).is_some_and(|l| l > 0.0)
                    || num(b.get("used")).is_some_and(|u| u > 0.0))
        });
    if let Some(b) = od {
        let od_limit = num(b.get("limit")).unwrap_or(0.0);
        let od_spent = num(b.get("used"))
            .filter(|u| *u > 0.0)
            .or_else(|| num(b.get("remaining")).map(|r| (od_limit - r).max(0.0)))
            .unwrap_or(0.0);
        if od_limit > 0.0 {
            metrics.push(Metric::progress(
                "On-demand",
                (od_spent / od_limit * 100.0).clamp(0.0, 100.0),
                Some(format!("{} / {}", dollars(od_spent), dollars(od_limit))),
            ));
        } else if od_spent > 0.0 {
            metrics.push(Metric::text("On-demand", dollars(od_spent)));
        }
    }

    if metrics.iter().all(|m| m.kind != "progress") {
        return None;
    }
    Some(Snapshot::ok(ID, NAME, plan, metrics))
}

/// Pre-2025 request-quota accounts: the old REST endpoint with the web
/// session cookie, counting requests instead of dollars.
async fn legacy_fetch(token: &str) -> Result<Snapshot, String> {
    let (user_id, cookie) = session_cookie(token).ok_or("could not decode Cursor session token")?;

    let usage_req = http()
        .get(format!("https://cursor.com/api/usage?user={user_id}"))
        .header("Cookie", &cookie)
        .send();
    let plan_req = http()
        .get("https://cursor.com/api/auth/stripe")
        .header("Cookie", &cookie)
        .send();
    let (usage_resp, plan_resp) = tokio::join!(usage_req, plan_req);

    let usage_resp = usage_resp.map_err(|e| format!("usage request: {e}"))?;
    if usage_resp.status().as_u16() == 401 || usage_resp.status().as_u16() == 403 {
        return Err("Cursor session expired — open Cursor once to refresh it".into());
    }
    if !usage_resp.status().is_success() {
        return Err(format!("usage endpoint: HTTP {}", usage_resp.status()));
    }
    let usage: Value = usage_resp.json().await.map_err(|e| format!("usage parse: {e}"))?;

    let mut plan: Option<String> = None;
    if let Ok(r) = plan_resp {
        if r.status().is_success() {
            if let Ok(v) = r.json::<Value>().await {
                plan = v
                    .get("membershipType")
                    .and_then(Value::as_str)
                    .map(title_case);
            }
        }
    }

    legacy_snapshot(&usage, plan)
}

/// A request cap, or a non-zero count. Capless `numRequests: 0` is the
/// empty answer bucket-era accounts get from the old endpoint — not a card.
fn legacy_snapshot(usage: &Value, plan: Option<String>) -> Result<Snapshot, String> {
    let mut metrics = Vec::new();
    if let Some(gpt4) = usage.get("gpt-4") {
        let used = gpt4.get("numRequests").and_then(Value::as_f64).unwrap_or(0.0);
        match gpt4.get("maxRequestUsage").and_then(Value::as_f64) {
            Some(max) if max > 0.0 => {
                metrics.push(Metric::progress(
                    "Requests",
                    used / max * 100.0,
                    Some(format!("{used:.0} / {max:.0} this cycle")),
                ));
            }
            _ if used > 0.0 => {
                metrics.push(Metric::text("Requests this cycle", format!("{used:.0}")));
            }
            _ => {}
        }
    }
    if metrics.is_empty() {
        return Err("usage response had no recognizable data".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn credit_grants_string_cents_become_a_bar() {
        let grants = json!({
            "hasCreditGrants": true,
            "creditBalanceCents": "228",
            "totalCents": "2500",
            "usedCents": "2272"
        });
        let m = credit_grants_metric(&grants).expect("row");
        assert_eq!(m.kind, "progress");
        assert_eq!(m.label, "Credits");
        assert!((m.used_percent.unwrap() - 90.88).abs() < 0.02);
        assert_eq!(m.detail.as_deref(), Some("$2.28 left of $25.00"));
    }

    #[test]
    fn credit_grants_numeric_cents_also_parse() {
        let grants = json!({
            "hasCreditGrants": true,
            "totalCents": 20000,
            "usedCents": 0,
            "creditBalanceCents": 20000
        });
        let m = credit_grants_metric(&grants).expect("row");
        assert_eq!(m.used_percent, Some(0.0));
        assert_eq!(m.detail.as_deref(), Some("$200 left of $200"));
    }

    #[test]
    fn summary_ultra_shape_builds_the_bucket_card() {
        // Live 2026-09 Ultra response, identifiers omitted.
        let doc = json!({
            "billingCycleStart": "2026-08-20T15:30:39.000Z",
            "billingCycleEnd": "2026-09-20T15:30:39.000Z",
            "membershipType": "ultra",
            "limitType": "user",
            "individualUsage": {
                "plan": {
                    "enabled": true, "used": 33090, "limit": 40000, "remaining": 6910,
                    "autoPercentUsed": 3.497, "apiPercentUsed": 45.196, "totalPercentUsed": 9.454
                },
                "onDemand": { "enabled": false, "used": 0, "limit": null, "remaining": null }
            },
            "teamUsage": {}
        });
        let snap = summary_snapshot(&doc).expect("card");
        assert_eq!(snap.plan.as_deref(), Some("Ultra"));
        let labels: Vec<_> = snap
            .metrics
            .iter()
            .map(|m| (m.kind.as_str(), m.label.as_str()))
            .collect();
        assert_eq!(
            labels,
            [
                ("progress", "Cursor Models"),
                ("progress", "Other Models"),
                ("text", "Total usage")
            ]
        );
        assert_eq!(snap.metrics[2].value.as_deref(), Some("$331 this cycle"));
        assert_eq!(snap.metrics[0].resets_at, Some(1789918239000));
        assert_eq!(snap.metrics[0].period_ms, Some(31 * 24 * 3_600_000));
    }

    #[test]
    fn summary_team_pool_and_individual_on_demand() {
        let doc = json!({
            "membershipType": "enterprise",
            "limitType": "team",
            "individualUsage": {
                "plan": { "autoPercentUsed": 10.0, "apiPercentUsed": 20.0 },
                "onDemand": { "enabled": true, "used": 1500, "limit": 5000, "remaining": 3500 }
            },
            "teamUsage": {
                "pooled": { "enabled": true, "used": 0, "limit": 200000, "remaining": 150000 },
                "onDemand": { "enabled": true, "used": 99999, "limit": 1000000 }
            }
        });
        let snap = summary_snapshot(&doc).expect("card");
        let total = snap
            .metrics
            .iter()
            .find(|m| m.label == "Total usage")
            .expect("pool bar");
        assert_eq!(total.kind, "progress");
        assert_eq!(total.detail.as_deref(), Some("$500 / $2000 this cycle"));
        assert_eq!(total.used_percent, Some(25.0));
        let od = snap
            .metrics
            .iter()
            .find(|m| m.label == "On-demand")
            .expect("on-demand");
        assert_eq!(od.detail.as_deref(), Some("$15.00 / $50.00"));
    }

    #[test]
    fn summary_team_without_pool_keeps_percentages() {
        // Live Enterprise shape from upstream research: limitType=team,
        // Auto/API percents + individual on-demand, no pooled cap.
        let doc = json!({
            "membershipType": "enterprise",
            "limitType": "team",
            "individualUsage": {
                "plan": { "autoPercentUsed": 10.0, "apiPercentUsed": 20.0 },
                "onDemand": { "enabled": true, "used": 1500, "limit": 5000, "remaining": 3500 }
            },
            "teamUsage": {}
        });
        let snap = summary_snapshot(&doc).expect("card");
        let labels: Vec<_> = snap
            .metrics
            .iter()
            .map(|m| (m.kind.as_str(), m.label.as_str()))
            .collect();
        assert_eq!(
            labels,
            [
                ("progress", "Cursor Models"),
                ("progress", "Other Models"),
                ("progress", "On-demand")
            ]
        );
    }

    #[test]
    fn summary_without_figures_is_no_card() {
        assert!(summary_snapshot(&json!({})).is_none());
        assert!(
            summary_snapshot(&json!({ "individualUsage": { "plan": { "enabled": false } } }))
                .is_none()
        );
        // Only a spend figure, no percentages or limit — nothing to draw a bar from.
        assert!(
            summary_snapshot(&json!({ "individualUsage": { "plan": { "used": 12 } } })).is_none()
        );
    }

    #[test]
    fn summary_disabled_individual_plan_still_uses_team() {
        let doc = json!({
            "membershipType": "enterprise",
            "limitType": "team",
            "individualUsage": {
                "plan": { "enabled": false, "autoPercentUsed": 10.0 }
            },
            "teamUsage": {
                "pooled": { "enabled": true, "used": 50000, "limit": 200000, "remaining": 150000 }
            }
        });
        let snap = summary_snapshot(&doc).expect("team card");
        let total = snap
            .metrics
            .iter()
            .find(|m| m.label == "Total usage")
            .expect("pool bar");
        assert_eq!(total.kind, "progress");
        assert_eq!(total.detail.as_deref(), Some("$500 / $2000 this cycle"));
        assert!(snap.metrics.iter().all(|m| m.label != "Cursor Models"));
    }

    #[test]
    fn summary_disabled_team_pool_is_not_a_card() {
        let doc = json!({
            "membershipType": "enterprise",
            "limitType": "team",
            "individualUsage": {
                "plan": { "enabled": false }
            },
            "teamUsage": {
                "pooled": {
                    "enabled": false, "used": 50000, "limit": 200000, "remaining": 150000
                }
            }
        });
        assert!(summary_snapshot(&doc).is_none());
    }

    #[test]
    fn legacy_zero_requests_without_cap_is_not_a_real_quota() {
        let noise = json!({ "gpt-4": { "numRequests": 0, "maxRequestUsage": null } });
        assert!(legacy_snapshot(&noise, None).is_err());
        let counted = json!({ "gpt-4": { "numRequests": 37 } });
        let snap = legacy_snapshot(&counted, None).expect("count");
        assert_eq!(snap.metrics[0].kind, "text");
        assert_eq!(snap.metrics[0].value.as_deref(), Some("37"));
        let capped = json!({ "gpt-4": { "numRequests": 60, "maxRequestUsage": 500 } });
        let snap = legacy_snapshot(&capped, None).expect("cap");
        assert_eq!(snap.metrics[0].kind, "progress");
        assert_eq!(snap.metrics[0].detail.as_deref(), Some("60 / 500 this cycle"));
        assert!(legacy_snapshot(&json!({}), None).is_err());
    }

    #[test]
    fn credit_grants_absent_or_empty_hide() {
        assert!(credit_grants_metric(&json!({"hasCreditGrants": false})).is_none());
        assert!(credit_grants_metric(&json!({})).is_none());
        assert!(credit_grants_metric(&json!({"hasCreditGrants": true})).is_none());
    }

    #[test]
    fn credit_grants_false_hides_even_with_leftover_totals() {
        let grants = json!({
            "hasCreditGrants": false,
            "totalCents": "2500",
            "usedCents": "2500",
            "creditBalanceCents": "0"
        });
        assert!(credit_grants_metric(&grants).is_none());
        let leftover = json!({
            "hasCreditGrants": false,
            "totalCents": 20000,
            "creditBalanceCents": 228
        });
        assert!(credit_grants_metric(&leftover).is_none());
    }

    #[test]
    fn credit_grants_balance_only_is_text() {
        let grants = json!({
            "hasCreditGrants": true,
            "creditBalanceCents": "1500"
        });
        let m = credit_grants_metric(&grants).expect("row");
        assert_eq!(m.kind, "text");
        assert_eq!(m.value.as_deref(), Some("$15.00"));
    }

    #[test]
    fn bonus_is_a_text_row_with_pool_context() {
        // Live 2026-08 shape: $122.51 spent, 35.51% of included+bonus,
        // $20 included, $102.51 bonus spend → bonus pool ≈ $325.
        let plan = json!({
            "totalSpend": 12251,
            "includedSpend": 2000,
            "bonusSpend": 10251,
        });
        let m = bonus_metric(&plan, Some(35.51014492753623)).expect("row");
        assert_eq!(m.kind, "text");
        assert_eq!(m.label, "Bonus");
        // dollars() rounds ≥$100 to whole dollars: 10251¢ → $103, pool $325.
        assert_eq!(m.value.as_deref(), Some("$103 of $325 used"));
    }

    #[test]
    fn bonus_tiny_percent_drops_the_derived_pool() {
        let plan = json!({ "bonusSpend": 500, "totalSpend": 500, "includedSpend": 0 });
        let m = bonus_metric(&plan, Some(0.2)).expect("row");
        assert_eq!(m.kind, "text");
        assert_eq!(m.value.as_deref(), Some("$5.00 used"));
    }

    #[test]
    fn bonus_zero_hides() {
        assert!(bonus_metric(&json!({"bonusSpend": 0}), Some(10.0)).is_none());
        assert!(bonus_metric(&json!({}), Some(10.0)).is_none());
    }
}

