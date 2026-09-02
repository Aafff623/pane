use super::{http, Metric, Snapshot};
use base64::Engine;
use chrono::Utc;
use serde_json::{json, Value};
use std::path::PathBuf;

// Codex CLI's public OAuth client id.
const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const ID: &str = "codex";
const NAME: &str = "Codex";

fn default_home() -> PathBuf {
    std::env::var("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default().join(".codex"))
}

/// One discovered Codex login. Mirrors the Claude account model (and
/// upstream OpenUsage's Phase 5a design): the default CODEX_HOME keeps the
/// bare "codex" id; extras mint "codex@<hash8>" from the account id.
pub struct CodexAccount {
    pub id: String,
    pub name: String,
    pub dir: PathBuf,
}

/// The account identity in a Codex home, under upstream's strict rule: a
/// credential file that can't name its account (tokens.account_id, else
/// the id_token's ChatGPT account claim) never becomes a card. The email
/// claim doubles as the card label.
fn dir_identity(dir: &std::path::Path) -> Option<(String, Option<String>)> {
    identity_from(&read_auth(dir)?)
}

fn read_auth(dir: &std::path::Path) -> Option<Value> {
    let raw = std::fs::read_to_string(dir.join("auth.json")).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Proof a discovered auth.json actually belongs to OpenAI: its id_token
/// carries OpenAI's own claim namespace. `auth.json` + `tokens.account_id`
/// is not an OpenAI-specific shape — broad scanning could otherwise
/// misclassify another tool's credential file as a Codex login and send
/// its tokens to OpenAI endpoints (refresh could even write back into it).
fn openai_provenance(doc: &Value) -> bool {
    doc.pointer("/tokens/id_token")
        .and_then(Value::as_str)
        .and_then(jwt_claims)
        .is_some_and(|c| c.get("https://api.openai.com/auth").is_some())
}

/// (account id, email label) from a parsed auth.json: tokens.account_id
/// first, the id_token's ChatGPT account claim as the fallback; a file
/// that names neither has no identity and never becomes a card.
fn identity_from(doc: &Value) -> Option<(String, Option<String>)> {
    let tokens = doc.get("tokens")?;
    let claims = tokens
        .get("id_token")
        .and_then(Value::as_str)
        .and_then(jwt_claims);
    let account_id = tokens
        .get("account_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            claims
                .as_ref()?
                .pointer("/https:~1~1api.openai.com~1auth/chatgpt_account_id")
                .and_then(Value::as_str)
                .map(str::to_string)
        })?;
    let email = claims
        .as_ref()
        .and_then(|c| c.get("email"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    Some((account_id, email))
}

/// Extra Codex logins beyond the default home — same scan roots, identity
/// rules, and dedup-by-account as the Claude discovery.
pub fn discover_extra_accounts() -> Vec<CodexAccount> {
    let default = default_home();
    let default_identity = dir_identity(&default);
    // Same conservative rule as Claude: a default login that can't name
    // its account voids the dedup guarantee — discover nothing.
    if default.join("auth.json").exists() && default_identity.is_none() {
        return Vec::new();
    }
    let mut seen: Vec<String> = default_identity.map(|(a, _)| a).into_iter().collect();

    let mut out = Vec::new();
    for dir in super::account_scan_roots() {
        if dir == default {
            continue;
        }
        // The default home is user-designated; broadly scanned dirs must
        // additionally PROVE they're OpenAI's before their tokens can enter
        // the Codex request path.
        let Some(doc) = read_auth(&dir) else { continue };
        if !openai_provenance(&doc) {
            continue;
        }
        let Some((account_id, email)) = identity_from(&doc) else { continue };
        if seen.iter().any(|a| a == &account_id) {
            continue;
        }
        seen.push(account_id.clone());
        let hash8: String = account_id.chars().filter(|c| *c != '-').take(8).collect();
        let name = match email {
            Some(e) => format!("Codex — {e}"),
            None => format!("Codex @{hash8}"),
        };
        out.push(CodexAccount { id: format!("codex@{hash8}"), name, dir });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// The default login's account identity, for the snapshot-cache stamp.
pub fn default_identity() -> Option<String> {
    dir_identity(&default_home()).map(|(a, _)| a)
}

/// Pure local probe for the Customize gear panel (existence only, no
/// network): which Codex CLI sign-in this machine already has.
pub fn local_credential_hint() -> Option<String> {
    if default_home().join("auth.json").exists() {
        return Some("Codex CLI sign-in".into());
    }
    if !discover_extra_accounts().is_empty() {
        return Some("Codex CLI sign-in (extra CODEX_HOME)".into());
    }
    None
}

/// Access tokens are JWTs: three base64 chunks separated by dots. The middle
/// chunk is a JSON object with the expiry time and plan info.
fn jwt_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub async fn snapshot() -> Snapshot {
    snapshot_at(default_home(), ID.to_string(), NAME.to_string()).await
}

/// Snapshot for one account's Codex home — the default card and every
/// discovered extra account run the same flow scoped to their dir.
pub async fn snapshot_at(dir: PathBuf, id: String, name: String) -> Snapshot {
    match fetch(&dir, &id, &name).await {
        Ok(s) => s,
        Err(e) => Snapshot::error(&id, &name, e),
    }
}

struct Access {
    token: String,
    account_id: String,
    plan: Option<String>,
}

/// Credential resolution for the default Codex card: the CLI's own
/// sign-in first, then Pane's own OAuth login from the gear panel
/// (%APPDATA%\Pane\oauth\codex.json — refreshed and written back there,
/// the CLI's files untouched). Extra accounts are always CLI homes.
async fn default_card_access(dir: &std::path::Path) -> Result<Option<Access>, String> {
    if dir.join("auth.json").exists() {
        return load_access(dir).await.map(Some);
    }
    match crate::oauth::valid_tokens(ID).await {
        Ok(Some(t)) => Ok(Some(Access {
            token: t.access_token,
            account_id: t.account_id.unwrap_or_default(),
            plan: t.id_token.as_deref().and_then(jwt_claims).and_then(|c| {
                c.pointer("/https:~1~1api.openai.com~1auth/chatgpt_plan_type")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            }),
        })),
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Loads (and if needed refreshes + writes back) the Codex OAuth access
/// token. Shared by the usage fetch and the reset-credit redeem command.
async fn load_access(dir: &std::path::Path) -> Result<Access, String> {
    let path = dir.join("auth.json");
    let raw = std::fs::read_to_string(&path).map_err(|e| format!("read auth.json: {e}"))?;
    let mut doc: Value = serde_json::from_str(&raw).map_err(|e| format!("parse auth.json: {e}"))?;
    let tokens = doc
        .get("tokens")
        .cloned()
        .ok_or("auth.json has no OAuth tokens (signed in with an API key instead?)")?;

    let mut access = tokens
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let refresh = tokens
        .get("refresh_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let id_token = tokens
        .get("id_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let mut account_id = tokens
        .get("account_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let mut plan: Option<String> = None;
    if let Some(claims) = jwt_claims(&id_token) {
        if let Some(auth) = claims.get("https://api.openai.com/auth") {
            plan = auth
                .get("chatgpt_plan_type")
                .and_then(Value::as_str)
                .map(str::to_string);
            if account_id.is_empty() {
                if let Some(id) = auth.get("chatgpt_account_id").and_then(Value::as_str) {
                    account_id = id.to_string();
                }
            }
        }
    }

    let exp = jwt_claims(&access)
        .and_then(|c| c.get("exp").and_then(Value::as_i64))
        .unwrap_or(0);
    if access.is_empty() || exp <= Utc::now().timestamp() + 60 {
        if refresh.is_empty() {
            return Err("access token expired and no refresh token — run `codex login` again".into());
        }
        let resp = http()
            .post("https://auth.openai.com/oauth/token")
            .json(&json!({
                "client_id": CLIENT_ID,
                "grant_type": "refresh_token",
                "refresh_token": refresh,
                "scope": "openid profile email",
            }))
            .send()
            .await
            .map_err(|e| format!("token refresh: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("token refresh failed: HTTP {}", resp.status()));
        }
        let tok: Value = resp.json().await.map_err(|e| format!("token refresh parse: {e}"))?;
        let new_access = tok
            .get("access_token")
            .and_then(Value::as_str)
            .ok_or("refresh response missing access_token")?
            .to_string();

        access = new_access.clone();

        if let Some(t) = doc.get_mut("tokens").filter(|v| v.is_object()) {
            t["access_token"] = Value::from(new_access);
            if let Some(r) = tok.get("refresh_token").and_then(Value::as_str) {
                t["refresh_token"] = Value::from(r);
            }
            if let Some(i) = tok.get("id_token").and_then(Value::as_str) {
                t["id_token"] = Value::from(i);
            }
            doc["last_refresh"] = Value::from(Utc::now().to_rfc3339());
            // Keep a copy of the CLI's own file before touching it, so a bad
            // write can never cost the user their login.
            let _ = std::fs::copy(&path, path.with_extension("json.pane-bak"));
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_string_pretty(&doc).unwrap_or(raw))
                .and_then(|_| std::fs::rename(&tmp, &path))
                .map_err(|e| format!("write refreshed auth.json: {e}"))?;
        }
    }

    Ok(Access { token: access, account_id, plan })
}

async fn fetch(dir: &std::path::Path, id: &str, name: &str) -> Result<Snapshot, String> {
    // The Pane OAuth fallback is the DEFAULT card's only — an extra
    // account whose auth.json vanished must not silently show the OAuth
    // account's numbers.
    let auth = if id == ID {
        match default_card_access(dir).await? {
            Some(a) => a,
            None => {
                return Ok(Snapshot::no_credentials(
                    id,
                    name,
                    "Codex sign-in not found. Run `codex login` in a terminal, or sign in from the gear panel.",
                ));
            }
        }
    } else {
        if !dir.join("auth.json").exists() {
            return Ok(Snapshot::no_credentials(
                id,
                name,
                "Codex sign-in not found. Run `codex login` in a terminal, or sign in from the gear panel.",
            ));
        }
        load_access(dir).await?
    };
    let (access, account_id) = (auth.token, auth.account_id);
    let mut plan = auth.plan;

    let mut req = http()
        .get("https://chatgpt.com/backend-api/wham/usage")
        .bearer_auth(&access);
    if !account_id.is_empty() {
        req = req.header("chatgpt-account-id", &account_id);
    }
    let resp = req.send().await.map_err(|e| format!("usage request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("usage endpoint: HTTP {}", resp.status()));
    }
    let usage: Value = resp.json().await.map_err(|e| format!("usage parse: {e}"))?;

    let mut metrics = Vec::new();
    let rate_limits = usage
        .get("rate_limit")
        .or_else(|| usage.get("rate_limits"))
        .unwrap_or(&usage);
    push_window(
        &mut metrics,
        rate_limits.get("primary_window").or_else(|| rate_limits.get("primary")),
        "Session",
    );
    push_window(
        &mut metrics,
        rate_limits.get("secondary_window").or_else(|| rate_limits.get("secondary")),
        "Weekly",
    );
    // Spark (a separate metered model family) lives in additional_rate_limits;
    // only the spark entry is shown, matching the Mac app.
    if let Some(extra_limits) = usage.get("additional_rate_limits").and_then(Value::as_array) {
        let spark = extra_limits.iter().find(|e| {
            ["limit_name", "metered_feature"].iter().any(|k| {
                e.get(*k)
                    .and_then(Value::as_str)
                    .is_some_and(|s| s.to_lowercase().contains("spark"))
            })
        });
        if let Some(entry) = spark {
            let rl = entry.get("rate_limit").unwrap_or(entry);
            push_window_labeled(&mut metrics, rl.get("primary_window"), "Spark");
            push_window_labeled(&mut metrics, rl.get("secondary_window"), "Spark Weekly");
        }
    }

    // Extra Usage: pay-as-you-go credit balance ($0.04 per credit). A
    // positive balance gets a plan-style meter against the highest balance
    // seen (a top-up raises it, same mechanism as Moonshot/DeepSeek); a
    // spent balance still reads "$0.00 · 0 credits" — that's information,
    // not noise.
    // The same serializer that quotes the balance may quote this flag.
    let unlimited = usage
        .pointer("/credits/unlimited")
        .is_some_and(|v| v.as_bool() == Some(true) || v.as_str().map(str::trim) == Some("true"));
    if unlimited {
        metrics.push(Metric::text("Extra credits", "Unlimited".into()));
    } else if let Some(credits) = credits_balance(&usage) {
        if credits > 0.0 {
            let dollars = credits * 0.04;
            let suffix = format!(" · {credits:.0} credits");
            // High-water baseline keyed per CARD, not per family — two
            // accounts' balances must never share one baseline.
            let meter_key = format!("{id}-extra");
            match super::credit_meter_labeled(&meter_key, "$", dollars, "Extra credits", &suffix)
            {
                Some(m) => metrics.push(m),
                None => metrics.push(Metric::text(
                    "Extra credits",
                    format!("${dollars:.2}{suffix}"),
                )),
            }
        } else {
            metrics.push(Metric::text("Extra usage", "$0.00 · 0 credits".into()));
        }
    }

    // Per-credit rows with exact expiry (and a Use button in the UI) from
    // the dedicated endpoint; fall back to the usage body's bare count.
    match fetch_reset_credits(&access, &account_id).await {
        Some(credits) if !credits.is_empty() => {
            let many = credits.len() > 1;
            for (i, (credit_id, expires_at)) in credits.iter().enumerate() {
                let label = if many {
                    format!("Reset credit {}", i + 1)
                } else {
                    "Reset credit".to_string()
                };
                metrics.push(Metric {
                    label,
                    kind: "action".into(),
                    used_percent: None,
                    detail: Some(credit_id.clone()),
                    value: Some("Available".into()),
                    resets_at: *expires_at,
                    period_ms: None,
                });
            }
        }
        _ => {
            if let Some(count) = usage
                .pointer("/rate_limit_reset_credits/available_count")
                .and_then(Value::as_i64)
            {
                if count > 0 {
                    metrics.push(Metric::text("Reset credits", count.to_string()));
                }
            }
        }
    }
    if plan.is_none() {
        plan = usage
            .get("plan_type")
            .and_then(Value::as_str)
            .map(str::to_string);
    }
    if metrics.is_empty() {
        return Err("usage response had no recognizable rate limits".into());
    }
    Ok(Snapshot::ok(id, name, plan, metrics))
}

/// The credit balance from the usage body. The API serializes it
/// inconsistently — a JSON number in some responses, a quoted string in
/// others (rollout logs show `"balance":"0"`) — so both spellings parse.
/// A missing balance with `has_credits: false` reads as an explicit zero.
fn credits_balance(usage: &Value) -> Option<f64> {
    usage
        .pointer("/credits/balance")
        .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())))
        .map(|b| b.floor().max(0.0))
        .or_else(|| {
            (usage.pointer("/credits/has_credits").and_then(Value::as_bool) == Some(false))
                .then_some(0.0)
        })
}

#[cfg(test)]
mod tests {
    use super::{credits_balance, identity_from, openai_provenance};
    use base64::Engine;
    use serde_json::json;

    fn fake_id_token(claims: serde_json::Value) -> String {
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&claims).unwrap());
        format!("x.{payload}.y")
    }

    #[test]
    fn provenance_requires_openais_claim_namespace() {
        // A foreign auth.json with the right field SHAPE but no OpenAI
        // claim — the misclassification case — must not pass.
        let foreign = json!({"tokens": {"account_id": "some-other-account",
            "access_token": "foreign-access", "refresh_token": "foreign-refresh"}});
        assert!(!openai_provenance(&foreign));
        // Even with a JWT id_token, foreign claims don't count.
        let foreign_jwt = json!({"tokens": {"account_id": "acct",
            "id_token": fake_id_token(json!({"iss": "https://example.com"}))}});
        assert!(!openai_provenance(&foreign_jwt));
        // A real Codex login carries OpenAI's claim namespace.
        let real = json!({"tokens": {"account_id": "acct",
            "id_token": fake_id_token(json!({
                "https://api.openai.com/auth": {"chatgpt_account_id": "acct"}}))}});
        assert!(openai_provenance(&real));
        assert!(!openai_provenance(&json!({})));
    }

    #[test]
    fn codex_identity_extraction_is_validation() {
        // account_id field wins; email claim labels the card.
        let direct = json!({"tokens": {"account_id": "acct-1",
            "id_token": fake_id_token(json!({"email": "e@corp.com"}))}});
        assert_eq!(identity_from(&direct), Some(("acct-1".into(), Some("e@corp.com".into()))));
        // Empty account_id falls through to the id_token's ChatGPT claim.
        let via_claim = json!({"tokens": {"account_id": "",
            "id_token": fake_id_token(json!({
                "https://api.openai.com/auth": {"chatgpt_account_id": "acct-2"}}))}});
        assert_eq!(identity_from(&via_claim), Some(("acct-2".into(), None)));
        // Neither → no identity → no card.
        let anonymous = json!({"tokens": {"access_token": "k"}});
        assert_eq!(identity_from(&anonymous), None);
        assert_eq!(identity_from(&json!({})), None);
    }

    #[test]
    fn credit_balance_parses_number_and_string_spellings() {
        // String balance (the shape rollout logs show) — the bug that made
        // a freshly bought balance vanish from the card entirely.
        let s = json!({"credits": {"has_credits": true, "balance": "2500"}});
        assert_eq!(credits_balance(&s), Some(2500.0));
        // Number balance.
        let n = json!({"credits": {"has_credits": true, "balance": 125.0}});
        assert_eq!(credits_balance(&n), Some(125.0));
        // No balance field, explicitly no credits → explicit zero row.
        let none = json!({"credits": {"has_credits": false}});
        assert_eq!(credits_balance(&none), Some(0.0));
        // No credits object at all → no row.
        assert_eq!(credits_balance(&json!({})), None);
    }
}

const CREDITS_URL: &str = "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits";

/// Epoch seconds, epoch ms, or RFC3339 → epoch ms.
fn parse_expiry_ms(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Number(n) => {
            let n = n.as_i64()?;
            Some(if n < 1_000_000_000_000 { n * 1000 } else { n })
        }
        Value::String(s) => chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.timestamp_millis()),
        _ => None,
    }
}

/// Best-effort: still-available credits as (id, expires_at ms), soonest
/// expiry first. The extra headers mirror the Codex desktop client.
async fn fetch_reset_credits(access: &str, account_id: &str) -> Option<Vec<(String, Option<i64>)>> {
    let mut req = http()
        .get(CREDITS_URL)
        .bearer_auth(access)
        .header("Accept", "application/json")
        .header("OpenAI-Beta", "codex-1")
        .header("originator", "Codex Desktop");
    if !account_id.is_empty() {
        req = req.header("chatgpt-account-id", account_id);
    }
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let doc: Value = resp.json().await.ok()?;
    let credits = doc.get("credits").and_then(Value::as_array)?;
    let mut out: Vec<(String, Option<i64>)> = credits
        .iter()
        .filter(|c| match c.get("status").and_then(Value::as_str) {
            // Some tenants omit status even when available_count says credits exist.
            Some(s) => s.eq_ignore_ascii_case("available"),
            None => true,
        })
        .filter_map(|c| {
            let id = c
                .get("id")
                .or_else(|| c.get("credit_id"))
                .and_then(Value::as_str)?
                .to_string();
            Some((id, parse_expiry_ms(c.get("expires_at"))))
        })
        .collect();
    out.sort_by_key(|(_, e)| e.unwrap_or(i64::MAX));
    Some(out)
}

/// Consumes one banked reset credit — irreversible; the UI confirms first.
/// POST /consume with a fresh idempotency key; the windows reset server-side.
pub async fn redeem_credit(provider_id: &str, credit_id: &str) -> Result<String, String> {
    // Route the redeem to the account whose card offered the credit — an
    // extra account's Use button must spend ITS credit, not the default
    // login's (upstream's CodexResetClaimRouter, in one lookup).
    let dir = if provider_id == ID {
        default_home()
    } else {
        discover_extra_accounts()
            .into_iter()
            .find(|a| a.id == provider_id)
            .map(|a| a.dir)
            .ok_or_else(|| format!("unknown Codex account: {provider_id}"))?
    };
    let auth = if provider_id == ID {
        default_card_access(&dir)
            .await?
            .ok_or("Codex sign-in not found")?
    } else {
        load_access(&dir).await?
    };
    let redeem_request_id = format!(
        "openusage-{}-{}",
        Utc::now().timestamp_millis(),
        std::process::id()
    );
    let mut req = http()
        .post(format!("{CREDITS_URL}/consume"))
        .bearer_auth(&auth.token)
        .header("Accept", "application/json")
        .header("OpenAI-Beta", "codex-1")
        .header("originator", "Codex Desktop")
        .json(&json!({ "credit_id": credit_id, "redeem_request_id": redeem_request_id }));
    if !auth.account_id.is_empty() {
        req = req.header("chatgpt-account-id", &auth.account_id);
    }
    let resp = req.send().await.map_err(|e| format!("consume request: {e}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        let msg = body
            .get("detail")
            .and_then(Value::as_str)
            .or_else(|| body.get("error").and_then(Value::as_str))
            .unwrap_or("request failed");
        return Err(format!("HTTP {status}: {msg}"));
    }
    let windows = body.get("windows_reset").and_then(Value::as_i64).unwrap_or(0);
    Ok(if windows > 0 {
        format!("Codex limits reset ({windows} window{})", if windows == 1 { "" } else { "s" })
    } else {
        "Reset credit redeemed".to_string()
    })
}

fn push_window(metrics: &mut Vec<Metric>, node: Option<&Value>, fallback_label: &str) {
    push_window_inner(metrics, node, fallback_label, false);
}

/// Like push_window but keeps the given label (Spark rows must not be
/// auto-renamed to Session/Weekly by window length).
fn push_window_labeled(metrics: &mut Vec<Metric>, node: Option<&Value>, label: &str) {
    push_window_inner(metrics, node, label, true);
}

fn push_window_inner(metrics: &mut Vec<Metric>, node: Option<&Value>, label_in: &str, forced: bool) {
    let Some(node) = node else { return };
    let Some(used) = node.get("used_percent").and_then(Value::as_f64) else { return };
    let window_seconds = node
        .get("limit_window_seconds")
        .and_then(Value::as_i64)
        .or_else(|| node.get("window_minutes").and_then(Value::as_i64).map(|m| m * 60));
    let label = if forced {
        label_in
    } else {
        match window_seconds {
            Some(s) if s > 21_600 => "Weekly", // longer than 6 hours
            Some(_) => "Session",
            None => label_in,
        }
    };
    let period_ms = window_seconds
        .map(|s| s * 1000)
        .unwrap_or(if label.contains("Weekly") { 7 * 86_400_000 } else { 5 * 3_600_000 });
    let now_ms = Utc::now().timestamp_millis();
    let resets_at = node
        .get("reset_at")
        .and_then(Value::as_i64)
        .map(|s| if s < 1_000_000_000_000 { s * 1000 } else { s })
        .or_else(|| {
            node.get("reset_after_seconds")
                .or_else(|| node.get("resets_in_seconds"))
                .and_then(Value::as_i64)
                .map(|s| now_ms + s * 1000)
        });
    // Percentages show as Codex reports them (it floors to whole percents,
    // so an untouched window can read 1%) — the Mac dropped its old ≤1%→0
    // normalization because it masked real early usage; near-empty windows
    // are kept calm on the pacing side instead.
    metrics.push(Metric::progress(label, used, None).with_reset(resets_at, Some(period_ms)));
}
