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

    // Prefer a token we refreshed ourselves this run; the stored one may
    // be stale if Cursor hasn't been opened in a while.
    let mut token = refreshed_token()
        .lock()
        .ok()
        .and_then(|t| t.clone())
        .unwrap_or_else(|| stored.clone());

    // Current-generation usage: percent of the plan's included usage,
    // via the same dashboard RPCs Cursor's web dashboard calls.
    // A transient failure of the new API must not strand legacy-plan
    // users whose data lives behind the old endpoint — try that before
    // giving up (keeping the original error if both paths fail).
    let mut usage = match connect_post("GetCurrentPeriodUsage", &token).await {
        Ok(u) => u,
        Err(e) => {
            return match legacy_fetch(&token).await {
                Ok(s) => Ok(s),
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
    if !enabled || plan_usage.is_none() || (limit.is_none() && total_pct.is_none()) {
        return legacy_fetch(&token).await;
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

    // Unexpired credit grants + any negative Stripe balance = money that
    // gets burned before the plan pool does.
    if let Ok(Some(grants)) = credit_grants {
        let has = grants.get("hasCreditGrants").and_then(Value::as_bool) == Some(true);
        let total = if has { num(grants.get("totalCents")).unwrap_or(0.0) } else { 0.0 };
        let used = if has { num(grants.get("usedCents")).unwrap_or(0.0) } else { 0.0 };
        if total > 0.0 {
            let remaining = (total - used).max(0.0);
            metrics.push(Metric::text("Credits", dollars(remaining)));
        }
    }

    let spend_limit = usage.get("spendLimitUsage").filter(|v| v.is_object());
    let spend_type = spend_limit
        .and_then(|s| s.get("limitType").and_then(Value::as_str))
        .map(str::to_lowercase);
    let pooled_limit = spend_limit.and_then(|s| num(s.get("pooledLimit"))).unwrap_or(0.0);
    let is_team = plan.as_deref().map(|p| p.eq_ignore_ascii_case("team")) == Some(true)
        || spend_type.as_deref() == Some("team")
        || pooled_limit > 0.0;

    let used_cents = num(plan_usage.get("totalSpend"))
        .or_else(|| limit.map(|l| l - num(plan_usage.get("remaining")).unwrap_or(0.0)))
        .unwrap_or(0.0);

    if is_team {
        // Team-shaped accounts sometimes omit the plan limit (or report
        // zero, which would divide to NaN); the legacy request endpoint
        // is what still describes them (same fallback upstream uses).
        let limit_cents = match limit {
            Some(l) if l > 0.0 => l,
            _ => return legacy_fetch(&token).await,
        };
        metrics.push(
            Metric::progress(
                "Total usage",
                (used_cents / limit_cents * 100.0).clamp(0.0, 100.0),
                Some(format!("{} / {} this cycle", dollars(used_cents), dollars(limit_cents))),
            )
            .with_reset(resets_at, Some(period_ms)),
        );
    } else if plan_usage.get("autoPercentUsed").is_some()
        || plan_usage.get("apiPercentUsed").is_some()
    {
        // Bucket-era plans: mirror Cursor's own Plan & Usage page, which
        // shows exactly two bars — "Cursor Models" (the auto bucket:
        // Composer, Cursor Grok, …) and "Other Models" — and NO total
        // bar. There is no honest total percent on these plans: the $20
        // "limit" is only the Other-Models/API floor, totalPercentUsed
        // measures against included+bonus pools (~$345 live), and the
        // API's own displayMessage does spend/$20 math — three Cursor
        // numbers that all contradict the dashboard. Dollars spent stay
        // visible as a text row; only the misleading percent is gone.
        if let Some(auto) = num(plan_usage.get("autoPercentUsed")) {
            metrics.push(
                Metric::progress("Cursor Models", auto.clamp(0.0, 100.0), None)
                    .with_reset(resets_at, Some(period_ms)),
            );
        }
        if let Some(api) = num(plan_usage.get("apiPercentUsed")) {
            metrics.push(
                Metric::progress("Other Models", api.clamp(0.0, 100.0), None)
                    .with_reset(resets_at, Some(period_ms)),
            );
        }
        metrics.push(
            Metric::text("Total usage", format!("{} this cycle", dollars(used_cents)))
                .with_reset(resets_at, Some(period_ms)),
        );
    } else {
        // Pre-bucket accounts: the classic included-pool bar. Computed
        // spend/limit (matching the caption); the API's totalPercentUsed
        // only when no limit is reported.
        let pct = match limit {
            Some(l) if l > 0.0 => used_cents / l * 100.0,
            _ => total_pct.unwrap_or(0.0),
        };
        let detail = limit.map(|l| format!("{} of {} included", dollars(used_cents), dollars(l)));
        metrics.push(
            Metric::progress("Total usage", pct.clamp(0.0, 100.0), detail)
                .with_reset(resets_at, Some(period_ms)),
        );
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

    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

/// Pre-2025 request-quota accounts: the old REST endpoint with the web
/// session cookie, counting requests instead of dollars.
async fn legacy_fetch(token: &str) -> Result<Snapshot, String> {
    // Cursor's web session cookie is "<user_id>::<jwt>"; the user id is the
    // part of the JWT `sub` claim after the "auth0|" prefix.
    let sub = jwt_sub(token).ok_or("could not decode Cursor session token")?;
    let user_id = sub.split('|').next_back().unwrap_or(&sub).to_string();
    let cookie = format!("WorkosCursorSessionToken={user_id}%3A%3A{token}");

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
            _ => {
                metrics.push(Metric::text("Requests this cycle", format!("{used:.0}")));
            }
        }
    }
    if metrics.is_empty() {
        return Err("usage response had no recognizable data".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}
