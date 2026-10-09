//! Kiro (AWS) — reads kiro-cli's own state database for credentials and
//! asks the same `GetUsageLimits` service the CLI's `/usage` command calls.
//! The CLI itself is never spawned: it runs an update check on every
//! invocation and downloads a fresh installer to %TEMP%, which flooded the
//! temp folder back when refreshes spawned it every few minutes.

use super::{http, Metric, Snapshot};
use serde_json::{json, Value};

const ID: &str = "kiro";
const NAME: &str = "Kiro";

const TARGET: &str = "AmazonCodeWhispererService.GetUsageLimits";
/// Profile-ARN region → the endpoint that region answers on (eu-central-1
/// lives on the renamed q. hostname). Same table CodexBar ships.
const ENDPOINTS: &[(&str, &str)] = &[
    ("us-east-1", "https://codewhisperer.us-east-1.amazonaws.com/"),
    ("eu-central-1", "https://q.eu-central-1.amazonaws.com/"),
];
/// Plausible Unix seconds for a billing reset (2001-09-09 … 2100-01-01).
/// Outside this range the unit changed, not the date — milliseconds would
/// land far beyond any real reset.
const RESET_RANGE: std::ops::RangeInclusive<f64> = 1_000_000_000.0..=4_102_444_800.0;

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Snapshot for one Pane-managed Kiro login (login_accounts): the usage
/// call runs with the login's own bearer + profileArn; a rejected token
/// is refreshed once (rotated pair written back) before the error stands.
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
        return format!("Kiro — {}", login.email);
    }
    format!("Kiro @{}", login.account_id.rsplit('/').next().unwrap_or("profile"))
}

/// The profileArn-backed usage flow for one login. `allow_refresh` gates
/// the single retry: a refresh write-back then re-runs the query once.
async fn fetch_with_login(
    login: &crate::login_accounts::LoginAccount,
    allow_refresh: bool,
) -> Result<Snapshot, String> {
    let id = crate::login_accounts::card_id_for_account(ID, login);
    let name = login_card_name(login);
    let Some(endpoint) = endpoint_for_arn(&login.account_id) else {
        return Err("unsupported profile ARN — no known endpoint for its region".into());
    };
    let resp = http()
        .post(endpoint)
        .header("Content-Type", "application/x-amz-json-1.0")
        .header("X-Amz-Target", TARGET)
        .bearer_auth(&login.access_token)
        .json(&json!({ "profileArn": login.account_id }))
        .send()
        .await
        .map_err(|e| format!("GetUsageLimits request: {e}"))?;
    if resp.status().as_u16() == 401 && allow_refresh && !login.refresh_token.trim().is_empty() {
        let mut doc = crate::kiro_login::refresh(&login.refresh_token).await?;
        crate::kiro_login::ensure_expires_at(&mut doc);
        let new_access = doc
            .get("accessToken")
            .or_else(|| doc.get("access_token"))
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .ok_or("refresh response missing accessToken")?
            .to_string();
        let new_refresh = doc
            .get("refreshToken")
            .or_else(|| doc.get("refresh_token"))
            .and_then(Value::as_str)
            .unwrap_or(&login.refresh_token)
            .to_string();
        // Write the rotated pair back (reload first so a concurrent
        // rename/remove from the UI is not clobbered).
        let mut accounts = crate::login_accounts::load(ID);
        if let Some(entry) = accounts.iter_mut().find(|a| a.account_id == login.account_id) {
            entry.access_token = new_access.clone();
            entry.refresh_token = new_refresh;
            if let Some(expires_at) = doc.get("expiresAt").and_then(Value::as_str) {
                entry.expires_at = expires_at.to_string();
            }
        }
        let _ = crate::login_accounts::save(ID, &accounts);
        let mut updated = login.clone();
        updated.access_token = new_access;
        return Box::pin(fetch_with_login(&updated, false)).await;
    }
    if resp.status().as_u16() == 401 {
        return Err("token was rejected — sign in again from the auth center".into());
    }
    if !resp.status().is_success() {
        return Err(format!("GetUsageLimits: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("GetUsageLimits parse: {e}"))?;
    parse_usage(&doc).map(|mut s| {
        s.id = id;
        s.name = name;
        s
    })
}

pub fn local_credential_hint() -> Option<String> {
    state_db_candidates().iter().any(|p| p.is_file()).then(|| "Kiro CLI sign-in".to_string())
}

/// The local CLI login's profileArn, for the account-card dedup — the same
/// identity Pane logins key on.
pub fn default_identity() -> Option<String> {
    let db = state_db_candidates().into_iter().find(|p| p.is_file())?;
    read_identity(&db).ok().flatten().map(|i| i.profile_arn)
}

/// data.sqlite3 candidates in priority order: `KIRO_DATA_DIR` wins, then
/// the Windows locations (LOCALAPPDATA first — kiro-cli is the renamed
/// Amazon Q Developer CLI and writes there), then the macOS/Linux spots.
fn state_db_candidates() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(dir) = std::env::var("KIRO_DATA_DIR") {
        if !dir.trim().is_empty() {
            out.push(std::path::Path::new(dir.trim()).join("data.sqlite3"));
        }
    }
    for var in ["LOCALAPPDATA", "APPDATA"] {
        if let Ok(base) = std::env::var(var) {
            if !base.trim().is_empty() {
                out.push(std::path::Path::new(base.trim()).join("kiro-cli").join("data.sqlite3"));
            }
        }
    }
    if let Some(home) = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from)
    {
        out.push(home.join("Library").join("Application Support").join("kiro-cli").join("data.sqlite3"));
        out.push(home.join(".local").join("share").join("kiro-cli").join("data.sqlite3"));
    }
    out
}

struct Identity {
    access_token: String,
    profile_arn: String,
}

/// Reads the CLI's credentials without disturbing them — the CLI owns the
/// token and its refresh, so the database is opened read-only. `Ok(None)`
/// means signed out (rows or JSON fields missing); `Err` means the database
/// is there but unreadable.
fn read_identity(db: &std::path::Path) -> Result<Option<Identity>, String> {
    let conn =
        rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("open kiro-cli state database: {e}"))?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(250));
    let row = |sql: &str| -> Result<Option<String>, String> {
        match conn.query_row(sql, [], |r| r.get::<_, String>(0)) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("read kiro-cli state: {e}")),
        }
    };
    let token_json = row("SELECT value FROM auth_kv WHERE key = 'kirocli:odic:token'")?;
    let profile_json = row("SELECT value FROM state WHERE key = 'api.codewhisperer.profile'")?;
    let Some(access_token) =
        token_json.as_deref().and_then(|t| non_empty_json_str(t, "access_token"))
    else {
        return Ok(None);
    };
    let Some(profile_arn) = profile_json.as_deref().and_then(|p| non_empty_json_str(p, "arn"))
    else {
        return Ok(None);
    };
    Ok(Some(Identity { access_token, profile_arn }))
}

fn non_empty_json_str(raw: &str, key: &str) -> Option<String> {
    let doc: Value = serde_json::from_str(raw).ok()?;
    let s = doc.get(key)?.as_str()?;
    (!s.is_empty()).then(|| s.to_string())
}

/// `arn:aws:codewhisperer:<region>:<account>:profile/<id>` → endpoint.
/// Anything else (wrong service, unknown region, stray whitespace) is
/// rejected — a guessed endpoint would ship the bearer token to the wrong
/// host.
fn endpoint_for_arn(arn: &str) -> Option<&'static str> {
    let parts: Vec<&str> = arn.splitn(6, ':').collect();
    if parts.len() != 6
        || parts[0] != "arn"
        || parts[1] != "aws"
        || parts[2] != "codewhisperer"
        || !parts[5].starts_with("profile/")
        || parts[5].len() == "profile/".len()
        || arn.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return None;
    }
    ENDPOINTS.iter().find(|(r, _)| *r == parts[3]).map(|(_, url)| *url)
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(db) = state_db_candidates().into_iter().find(|p| p.is_file()) else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Kiro CLI (kiro-cli) is not installed.",
        ));
    };
    let Some(identity) = read_identity(&db)? else {
        return Ok(Snapshot::no_credentials(ID, NAME, "Sign in with `kiro-cli login` first."));
    };
    let Some(endpoint) = endpoint_for_arn(&identity.profile_arn) else {
        return Err("unsupported profile ARN — no known endpoint for its region".into());
    };
    let resp = http()
        .post(endpoint)
        .header("Content-Type", "application/x-amz-json-1.0")
        .header("X-Amz-Target", TARGET)
        .bearer_auth(&identity.access_token)
        .json(&json!({ "profileArn": identity.profile_arn }))
        .send()
        .await
        .map_err(|e| format!("GetUsageLimits request: {e}"))?;
    if resp.status().as_u16() == 401 {
        return Err("token was rejected — run `kiro-cli login` again".into());
    }
    if !resp.status().is_success() {
        return Err(format!("GetUsageLimits: HTTP {}", resp.status()));
    }
    let doc: Value = resp.json().await.map_err(|e| format!("GetUsageLimits parse: {e}"))?;
    parse_usage(&doc)
}

fn usable(value: Option<&Value>, field: &str) -> Result<f64, String> {
    value
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or_else(|| format!("no usable {field}"))
}

/// The credit math mirrors CodexBar's KiroUsageLimitsAPI: each component is
/// validated on its own (a negative one would hide inside a positive sum),
/// `currentUsageWithPrecision` is the total *including* overage, and bonus
/// spend is folded into it — so plan usage can legitimately sit above the
/// plan ceiling when `bonuses[]` is non-empty.
fn parse_usage(doc: &Value) -> Result<Snapshot, String> {
    let credits: Vec<&Value> = doc
        .get("usageBreakdownList")
        .and_then(Value::as_array)
        .ok_or("no usageBreakdownList in response")?
        .iter()
        .filter(|e| e.get("resourceType").and_then(Value::as_str) == Some("CREDIT"))
        .collect();
    match credits.len() {
        0 => return Err("no credit balance reported".into()),
        1 => {}
        _ => return Err("several credit balances reported".into()),
    }
    let c = credits[0];

    let plan_limit = usable(c.get("usageLimitWithPrecision"), "plan limit")?;
    let total_used = usable(c.get("currentUsageWithPrecision"), "usage")?;
    let overage_used = c
        .get("currentOveragesWithPrecision")
        .map(|v| usable(Some(v), "overage usage"))
        .transpose()?
        .unwrap_or(0.0);
    if total_used < overage_used {
        return Err("overage exceeds total usage".into());
    }
    let plan_used = total_used - overage_used;
    let has_bonus =
        c.get("bonuses").and_then(Value::as_array).is_some_and(|a| !a.is_empty());
    if !has_bonus && plan_used > plan_limit {
        return Err("plan usage exceeds plan limit".into());
    }

    let overage_enabled =
        match doc.pointer("/overageConfiguration/overageStatus").and_then(Value::as_str) {
            None => None,
            Some(s) => match s.trim().to_ascii_uppercase().as_str() {
                "ENABLED" => Some(true),
                "DISABLED" => Some(false),
                _ => None,
            },
        };
    // ENABLED without a cap is incomplete, not disabled — overage rows stay.
    let overage_cap = if overage_enabled == Some(true) {
        c.get("overageCapWithPrecision").map(|v| usable(Some(v), "overage cap")).transpose()?
    } else {
        None
    };
    let resets_at = c
        .get("nextDateReset")
        .or_else(|| doc.get("nextDateReset"))
        .and_then(Value::as_f64)
        .filter(|s| RESET_RANGE.contains(s))
        .map(|s| (s * 1000.0) as i64)
        .ok_or("no plausible reset date reported")?;

    let pct = if plan_limit > 0.0 {
        plan_used / plan_limit * 100.0
    } else if plan_used > 0.0 {
        100.0
    } else {
        0.0
    };
    let detail = if has_bonus {
        format!("{plan_used:.1} of {plan_limit:.0} credits (bonus spend included)")
    } else {
        format!("{plan_used:.1} of {plan_limit:.0} credits")
    };
    let mut metrics =
        vec![Metric::progress("Credits", pct.clamp(0.0, 100.0), Some(detail)).with_reset(Some(resets_at), None)];

    if overage_used > 0.0 || overage_cap.is_some() {
        match overage_cap.filter(|cap| *cap > 0.0) {
            Some(cap) => metrics.push(Metric::progress(
                "Overage",
                (overage_used / cap * 100.0).clamp(0.0, 100.0),
                Some(format!("{overage_used:.1} of {cap:.0} credits")),
            )),
            None => {
                metrics.push(Metric::text("Overage", format!("{overage_used:.1} credits used")))
            }
        }
        let currency = c.get("currency").and_then(Value::as_str).unwrap_or("USD");
        if let Some(charges) = c
            .get("overageCharges")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0.0)
        {
            let amount =
                if currency == "USD" { format!("${charges:.2}") } else { format!("{charges:.2} {currency}") };
            metrics.push(Metric::text("Overage charges", amount));
        }
    }

    Ok(Snapshot::ok(ID, NAME, None, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arn_resolves_to_its_regions_endpoint() {
        assert_eq!(
            endpoint_for_arn("arn:aws:codewhisperer:us-east-1:123456789012:profile/ABCDEF"),
            Some("https://codewhisperer.us-east-1.amazonaws.com/")
        );
        assert_eq!(
            endpoint_for_arn("arn:aws:codewhisperer:eu-central-1:1:profile/X"),
            Some("https://q.eu-central-1.amazonaws.com/")
        );
    }

    #[test]
    fn foreign_or_malformed_arns_are_rejected() {
        assert_eq!(endpoint_for_arn("arn:aws:bedrock:us-east-1:1:profile/X"), None); // wrong service
        assert_eq!(endpoint_for_arn("arn:aws:codewhisperer:ap-south-1:1:profile/X"), None); // unknown region
        assert_eq!(endpoint_for_arn("arn:aws:codewhisperer:us-east-1:1:user/X"), None); // not a profile
        assert_eq!(endpoint_for_arn("arn:aws:codewhisperer:us-east-1:1:profile/"), None); // empty id
        assert_eq!(endpoint_for_arn("arn:aws:codewhisperer:us-east-1:1:profile/ X"), None); // whitespace
        assert_eq!(endpoint_for_arn("codewhisperer:us-east-1:1:profile/X"), None); // too few parts
    }

    #[test]
    fn identity_reads_the_cli_state_database() {
        let path =
            std::env::temp_dir().join(format!("pane-kiro-state-{}.sqlite3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute("CREATE TABLE auth_kv (key TEXT PRIMARY KEY, value TEXT NOT NULL)", [])
                .unwrap();
            conn.execute("CREATE TABLE state (key TEXT PRIMARY KEY, value TEXT NOT NULL)", [])
                .unwrap();
            conn.execute(
                "INSERT INTO auth_kv (key, value) VALUES ('kirocli:odic:token', ?1)",
                [r#"{"access_token":"tok-123","refresh_token":"r"}"#],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO state (key, value) VALUES ('api.codewhisperer.profile', ?1)",
                [r#"{"arn":"arn:aws:codewhisperer:us-east-1:1:profile/ABC"}"#],
            )
            .unwrap();
        }
        let identity = read_identity(&path).unwrap().unwrap();
        assert_eq!(identity.access_token, "tok-123");
        assert_eq!(identity.profile_arn, "arn:aws:codewhisperer:us-east-1:1:profile/ABC");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn signed_out_state_reports_no_identity_not_an_error() {
        let path =
            std::env::temp_dir().join(format!("pane-kiro-signedout-{}.sqlite3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute("CREATE TABLE auth_kv (key TEXT PRIMARY KEY, value TEXT NOT NULL)", [])
                .unwrap();
            conn.execute("CREATE TABLE state (key TEXT PRIMARY KEY, value TEXT NOT NULL)", [])
                .unwrap();
        }
        assert!(read_identity(&path).unwrap().is_none());
        std::fs::remove_file(&path).ok();
    }

    fn usage_doc(
        limit: f64,
        used: f64,
        overage: Option<f64>,
        bonuses: usize,
        overage_status: Option<&str>,
        cap: Option<f64>,
        reset: Option<f64>,
    ) -> Value {
        let mut credit = json!({
            "resourceType": "CREDIT",
            "usageLimitWithPrecision": limit,
            "currentUsageWithPrecision": used,
        });
        if let Some(o) = overage {
            credit["currentOveragesWithPrecision"] = json!(o);
        }
        if let Some(c) = cap {
            credit["overageCapWithPrecision"] = json!(c);
        }
        if bonuses > 0 {
            credit["bonuses"] = json!(vec![json!({}); bonuses]);
        }
        if let Some(r) = reset {
            credit["nextDateReset"] = json!(r);
        }
        let mut doc = json!({ "usageBreakdownList": [credit] });
        if let Some(status) = overage_status {
            doc["overageConfiguration"] = json!({ "overageStatus": status });
        }
        doc
    }

    #[test]
    fn plan_and_overage_split_from_the_reported_total() {
        // 62.5 total = 50 plan + 12.5 overage → plan reads 100% used.
        let snap = parse_usage(&usage_doc(
            50.0,
            62.5,
            Some(12.5),
            0,
            Some("ENABLED"),
            Some(100.0),
            Some(1_750_000_000.0),
        ))
        .unwrap();
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.metrics.len(), 2); // Credits + Overage (no charges field)
        let credits = &snap.metrics[0];
        assert_eq!(credits.label, "Credits");
        assert_eq!(credits.used_percent, Some(100.0));
        assert_eq!(credits.detail.as_deref(), Some("50.0 of 50 credits"));
        assert_eq!(credits.resets_at, Some(1_750_000_000_000));
        let overage = &snap.metrics[1];
        assert_eq!(overage.label, "Overage");
        assert_eq!(overage.used_percent, Some(12.5));
    }

    #[test]
    fn overage_charges_format_in_the_response_currency() {
        let mut doc = usage_doc(50.0, 62.5, Some(12.5), 0, Some("ENABLED"), Some(100.0), Some(1_750_000_000.0));
        doc["usageBreakdownList"][0]["overageCharges"] = json!(1.5);
        doc["usageBreakdownList"][0]["currency"] = json!("EUR");
        let snap = parse_usage(&doc).unwrap();
        assert_eq!(snap.metrics[2].label, "Overage charges");
        assert_eq!(snap.metrics[2].value.as_deref(), Some("1.50 EUR"));
    }

    #[test]
    fn bonuses_let_plan_usage_exceed_the_limit() {
        let legal = usage_doc(50.0, 60.0, None, 1, None, None, Some(1_750_000_000.0));
        assert!(parse_usage(&legal).is_ok());
        // Same payload without bonuses is relationally impossible.
        let illegal = usage_doc(50.0, 60.0, None, 0, None, None, Some(1_750_000_000.0));
        assert!(parse_usage(&illegal).is_err());
    }

    #[test]
    fn impossible_payloads_are_rejected() {
        let no_credit = json!({
            "usageBreakdownList": [
                { "resourceType": "TOKEN", "usageLimitWithPrecision": 1.0, "currentUsageWithPrecision": 0.5 }
            ]
        });
        assert!(parse_usage(&no_credit).is_err());
        // Overage larger than the total it is supposed to be part of.
        let overage_over_total =
            usage_doc(50.0, 10.0, Some(12.0), 0, None, None, Some(1_750_000_000.0));
        assert!(parse_usage(&overage_over_total).is_err());
        // A reset in milliseconds is a unit change, not a date.
        let reset_in_ms = usage_doc(50.0, 10.0, None, 0, None, None, Some(1_750_000_000_000.0));
        assert!(parse_usage(&reset_in_ms).is_err());
        // No reset at all is unusable for the pace engine.
        let no_reset = usage_doc(50.0, 10.0, None, 0, None, None, None);
        assert!(parse_usage(&no_reset).is_err());
    }
}
