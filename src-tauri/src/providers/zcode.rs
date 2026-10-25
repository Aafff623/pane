//! ZCode — the Zhipu coding client's own **Start Plan** card.
//!
//! This is a separate provider from `zai` on purpose. `zai` reads the official
//! token-plan quota with an API key; ZCode's Start Plan (and every gift it
//! hands out: weekend builds, the 1024 Dev Day grant, the model-scoped trials
//! shipped with GLM-5.3 / GLM-5.3-Flash) belongs to the **ZCode account
//! session** instead. The session JWT is stored AES-256-GCM-encrypted in
//! `~/.zcode/v2/credentials.json`; the balance endpoint mirrors each
//! entitlement as a bucket with its own `show_name`, total/used units and
//! window.
//!
//! The card distinguishes the states an owner actually meets:
//! no local session / expired session (`no_credentials`), a session without any
//! Start Plan (`needs_setup`), a valid plan with no gift this period (one text
//! row), a held plan with nothing granted (a claim line), and live grants (one
//! usage row per bucket). A failed request is an error — rows are never carried
//! over from a previous account.

use super::{config_dir, json_body, read_small_text, send_with_direct_fallback};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig};
use base64::{alphabet, Engine};
use rand_core::{OsRng, RngCore};
use serde_json::Value;
use sha2::{Digest, Sha256};

const ZCODE_API: &str = "https://zcode.z.ai";
/// The ZCode build whose requests we mirror; the endpoint only sanity-checks it.
const APP_VERSION: &str = "3.14.3";

/// One row's worth of grant, already summed across buckets that share a name.
#[derive(Clone)]
pub struct WeekendAllowance {
    pub used: f64,
    pub total: f64,
    /// End of the current grant window (epoch ms) — drives the countdown.
    pub resets_at_ms: Option<i64>,
    /// Length of that window (ms) — drives the period chip on the row.
    pub period_ms: Option<i64>,
}

/// One granted gift: a Start Plan bucket, labelled by its own `show_name`.
#[derive(Clone)]
pub struct StartGrant {
    pub label: String,
    pub plan: String,
    pub allowance: WeekendAllowance,
}

const CLAIM_HINT: &str = "held · not granted (open ZCode to claim)";
const MAX_ROWS: usize = 6;

const FAMILY_ID: &str = "zcode";
const DISPLAY_NAME: &str = "ZCode";
const NO_SESSION_HINT: &str =
    "Sign in to the ZCode app on this machine, then refresh.";
const SESSION_EXPIRED_HINT: &str =
    "The local ZCode session expired — open ZCode and sign in again.";
const NO_PLAN_HINT: &str =
    "This ZCode account has no Start Plan on record.";
const NO_GIFT: &str = "No gift in this period";
const START_PLAN_BASE: &str = "Start Plan";
const NO_START_PLAN: &str = "No Start Plan";
const START_PLAN_ENDED_HINT: &str =
    "The Start Plan gift has ended — open ZCode for the next campaign.";

/// The card snapshot: one call, one honest state.
pub async fn snapshot() -> super::Snapshot {
    let Some(jwt) = read_local_jwt() else {
        return super::Snapshot::no_credentials(FAMILY_ID, DISPLAY_NAME, NO_SESSION_HINT);
    };
    let now_s = chrono::Utc::now().timestamp();
    if jwt_expired(&jwt, now_s) {
        return super::Snapshot::no_credentials(FAMILY_ID, DISPLAY_NAME, SESSION_EXPIRED_HINT);
    }
    match fetch_balance(&jwt, &device_mid()).await {
        Some(doc) => snapshot_from_doc(&doc, now_s),
        None => super::Snapshot::error(
            FAMILY_ID,
            DISPLAY_NAME,
            "ZCode balance request failed — check the ZCode session and try again".into(),
        ),
    }
}

/// Pure state machine over a balance response, so every status is testable
/// without a session or a network call.
pub fn snapshot_from_doc(doc: &Value, now_s: i64) -> super::Snapshot {
    if doc.get("code").and_then(Value::as_i64) != Some(0) {
        let message = doc
            .get("msg")
            .or_else(|| doc.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("unexpected balance response")
            .to_string();
        return super::Snapshot::error(FAMILY_ID, DISPLAY_NAME, message);
    }
    let state = parse_start_plan_state(doc, now_s);
    if !state.found {
        return super::Snapshot::needs_setup(
            FAMILY_ID,
            DISPLAY_NAME,
            Some(START_PLAN_BASE.into()),
            vec![super::Metric::text(START_PLAN_BASE, NO_START_PLAN.into())],
            NO_PLAN_HINT,
        );
    }
    // The plan identity is always Start Plan; the campaign is a parenthetical
    // so a new promotion never renames the card.
    let campaigns: Vec<&str> = if state.has_active {
        state.active_campaigns.iter().map(String::as_str).collect()
    } else {
        state.expired_campaigns.iter().map(String::as_str).collect()
    };
    let campaign_label = (!campaigns.is_empty()).then(|| campaigns.join(" · "));
    let plan_label = super::plan_with_campaign(START_PLAN_BASE, campaign_label.as_deref());
    let grouped = state.active_plans.len() > 1;
    let mut rows = if !state.grants.is_empty() || !state.ungranted.is_empty() {
        render_rows_with_plan(&state.grants, &state.ungranted, grouped)
    } else if state.has_active {
        vec![super::Metric::text(START_PLAN_BASE, NO_GIFT.into())]
    } else {
        Vec::new()
    };
    append_expired_rows(&mut rows, &state.expired);
    if state.has_active {
        super::Snapshot::ok(FAMILY_ID, DISPLAY_NAME, Some(plan_label), rows)
    } else {
        // No live gift, but past campaigns stay visible instead of vanishing.
        super::Snapshot::needs_setup(
            FAMILY_ID,
            DISPLAY_NAME,
            Some(plan_label),
            rows,
            START_PLAN_ENDED_HINT,
        )
    }
}

/// One expired Start Plan campaign: its final bucket totals are history the
/// owner asked to keep, not live quota.
#[derive(Clone)]
pub struct ExpiredPlan {
    pub label: String,
    pub used: f64,
    pub total: f64,
}

/// Everything one balance response says about Start Plan, active or past.
#[derive(Default, Clone)]
pub struct StartPlanState {
    pub grants: Vec<StartGrant>,
    pub ungranted: Vec<String>,
    pub expired: Vec<ExpiredPlan>,
    pub active_plans: Vec<String>,
    pub active_campaigns: Vec<String>,
    pub expired_campaigns: Vec<String>,
    pub has_active: bool,
    pub found: bool,
}

/// The card rows: one usage bar per granted bucket, then one claim line per
/// plan that exists without a granted bucket.
pub fn render_rows(grants: &[StartGrant], ungranted: &[String]) -> Vec<super::Metric> {
    render_rows_with_plan(grants, ungranted, false)
}

fn render_rows_with_plan(
    grants: &[StartGrant],
    ungranted: &[String],
    show_plan: bool,
) -> Vec<super::Metric> {
    if grants.is_empty() && ungranted.is_empty() {
        return Vec::new();
    }
    // Claim lines are the actionable part; they get their slots first, and the
    // bar count yields to them instead of the other way round.
    let claims: Vec<String> = if ungranted.len() <= 2 {
        ungranted.to_vec()
    } else {
        vec![
            ungranted[0].clone(),
            format!("+{} more held plans", ungranted.len() - 1),
        ]
    };
    let bar_cap = MAX_ROWS.saturating_sub(claims.len());
    let mut rows: Vec<super::Metric> = if grants.len() > bar_cap {
        // One slot names what did not fit, so a hidden grant is never silent.
        let shown = grants.len().min(bar_cap.saturating_sub(1));
        let mut rows: Vec<super::Metric> = grants
            .iter()
            .take(shown)
            .map(|grant| grant_metric(grant, show_plan))
            .collect();
        rows.push(super::Metric::text(
            &format!("+{} more grants", grants.len() - shown),
            CLAIM_HINT.into(),
        ));
        rows
    } else {
        grants
            .iter()
            .map(|grant| grant_metric(grant, show_plan))
            .collect()
    };
    for plan in &claims {
        rows.push(super::Metric::text(plan, CLAIM_HINT.into()));
    }
    rows
}

/// Past campaigns are evidence, not live quota: they follow the active rows
/// with their final used/total and never invent a countdown.
fn append_expired_rows(rows: &mut Vec<super::Metric>, expired: &[ExpiredPlan]) {
    if expired.is_empty() {
        return;
    }
    let room = MAX_ROWS.saturating_sub(rows.len());
    if room == 0 {
        rows.push(super::Metric::text(
            &format!("+{} past Start Plan campaigns", expired.len()),
            "expired".into(),
        ));
        return;
    }
    let shown = expired.len().min(room);
    for entry in expired.iter().take(shown) {
        let value = if entry.total > 0.0 {
            format!(
                "expired · {} of {} used",
                with_thousands(entry.used),
                with_thousands(entry.total)
            )
        } else {
            "expired".into()
        };
        rows.push(super::Metric::text(&entry.label, value));
    }
    if expired.len() > shown {
        rows.push(super::Metric::text(
            &format!("+{} past Start Plan campaigns", expired.len() - shown),
            "expired".into(),
        ));
    }
}

/// One gift bucket: usage bar + used/total detail + its own window. The label
/// is the bucket's `show_name` (GLM-5.3, GLM-5.3-Flash …), so a new campaign
/// needs no code change — names are evidence, never filters.
fn grant_metric(grant: &StartGrant, show_plan: bool) -> super::Metric {
    let w = &grant.allowance;
    let percent = if w.total > 0.0 {
        (w.used / w.total * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let label = if show_plan && grant.plan != grant.label {
        format!("{} · {}", grant.plan, grant.label)
    } else {
        grant.label.clone()
    };
    super::Metric::progress(
        &label,
        percent,
        Some(format!(
            "{} of {} used",
            with_thousands(w.used),
            with_thousands(w.total),
        )),
    )
    .with_reset(w.resets_at_ms, w.period_ms)
}

/// Split a balance response into active Start Plan grants/claims and the
/// expired campaigns that should remain visible as history. Only Start Plan
/// plans qualify: a Coding Plan entry shares `plans[]` and is not a gift.
pub fn parse_start_plan_state(doc: &Value, fallback_now_s: i64) -> StartPlanState {
    let mut state = StartPlanState::default();
    if doc.get("code").and_then(Value::as_i64) != Some(0) {
        return state;
    }
    let Some(data) = doc.get("data") else {
        return state;
    };
    let now_s = num(data.get("server_time"))
        .filter(|t| *t > 0.0)
        .unwrap_or(fallback_now_s as f64) as i64;
    let (Some(plans), Some(balances)) = (
        data.get("plans").and_then(Value::as_array),
        data.get("balances").and_then(Value::as_array),
    ) else {
        return state;
    };
    let mut claimed = vec![false; balances.len()];
    for plan in plans {
        if !is_start_plan_plan(plan) {
            continue;
        }
        state.found = true;
        let plan_name = text_field(plan, "name");
        let campaign = campaign_name(&plan_name);
        let label = super::plan_with_campaign(START_PLAN_BASE, campaign.as_deref());
        if plan_expired(plan, now_s) {
            if let Some(campaign) = &campaign {
                if !state.expired_campaigns.contains(campaign) {
                    state.expired_campaigns.push(campaign.clone());
                }
            }
            let (used, total) = plan_usage(plan, balances, now_s);
            if let Some(existing) = state.expired.iter_mut().find(|row| row.label == label) {
                existing.used += used;
                existing.total += total;
            } else {
                state.expired.push(ExpiredPlan { label, used, total });
            }
            continue;
        }
        state.has_active = true;
        if !state.active_plans.contains(&label) {
            state.active_plans.push(label.clone());
        }
        if let Some(campaign) = &campaign {
            if !state.active_campaigns.contains(campaign) {
                state.active_campaigns.push(campaign.clone());
            }
        }
        let (rows, saw_bucket) = plan_buckets(
            plan,
            balances,
            now_s,
            false,
            Some(&mut claimed),
            allow_user_fallback(plan, plans, now_s),
        );
        if rows.is_empty() {
            // Only a plan that declares entitlements is a gift ZCode could
            // hand out; a coding-plan entry shares plans[] and must not be
            // presented as something to claim.
            if has_entitlements(plan) && !saw_bucket && !state.ungranted.contains(&label) {
                state.ungranted.push(label);
            }
        } else {
            state.grants.extend(rows);
        }
    }
    // A response can mix plan ids on the two sides (the classmate payload
    // measured 2026-10-25 does): the campaign plan omits `plan_id`, its
    // buckets carry one, and the base plan carries a different id. Do not
    // silently drop those unmatched buckets when two Start Plan fleets are
    // present; attach them to the plan whose campaign token they name.
    if state.active_plans.len() > 1 {
        let known_plan_ids: Vec<String> = plans
            .iter()
            .map(|plan| text_field(plan, "plan_id"))
            .filter(|id| !id.is_empty())
            .collect();
        for (index, bucket) in balances.iter().enumerate() {
            if claimed[index] || bucket_expired(bucket, now_s) {
                continue;
            }
            let bucket_id = text_field(bucket, "plan_id");
            if bucket_id.is_empty() || known_plan_ids.contains(&bucket_id) {
                continue;
            }
            let label = plans
                .iter()
                .filter(|plan| {
                    is_start_plan_plan(plan)
                        && !plan_expired(plan, now_s)
                        && plan_token_matches(&text_field(plan, "name"), &bucket_id)
                })
                .filter_map(|plan| {
                    let campaign = campaign_name(&text_field(plan, "name"));
                    Some(campaign.unwrap_or_else(|| START_PLAN_BASE.to_string()))
                })
                .next()
                .unwrap_or_else(|| START_PLAN_BASE.to_string());
            if let Some(grant) = bucket_grant(bucket, &label) {
                state.grants.push(grant);
            }
        }
    }
    state
        .ungranted
        .retain(|label| !state.grants.iter().any(|grant| grant.plan == *label));
    // Soonest deadline first; a grant without a window sorts last.
    state.grants.sort_by(|a, b| {
        let key = |g: &StartGrant| g.allowance.resets_at_ms.unwrap_or(i64::MAX);
        key(a).cmp(&key(b)).then_with(|| {
            (a.plan.as_str(), a.label.as_str()).cmp(&(b.plan.as_str(), b.label.as_str()))
        })
    });
    state.expired.sort_by(|a, b| a.label.cmp(&b.label));
    state.ungranted.sort();
    state.grants.truncate(MAX_ROWS);
    state.ungranted.truncate(MAX_ROWS);
    state.expired.truncate(MAX_ROWS);
    state
}

/// Backwards-compatible view used by tests and callers that only need live
/// rows. Expired history comes from [`parse_start_plan_state`].
pub fn parse_start_grants(doc: &Value, fallback_now_s: i64) -> (Vec<StartGrant>, Vec<String>) {
    let state = parse_start_plan_state(doc, fallback_now_s);
    (state.grants, state.ungranted)
}

fn plan_usage(plan: &Value, balances: &[Value], now_s: i64) -> (f64, f64) {
    let (rows, _) = plan_buckets(plan, balances, now_s, true, None, true);
    (
        rows.iter().map(|row| row.allowance.used).sum(),
        rows.iter().map(|row| row.allowance.total).sum(),
    )
}

fn plan_buckets(
    plan: &Value,
    balances: &[Value],
    now_s: i64,
    include_expired: bool,
    mut claimed: Option<&mut [bool]>,
    allow_user_fallback: bool,
) -> (Vec<StartGrant>, bool) {
    let plan_name = text_field(plan, "name");
    let campaign = campaign_name(&plan_name);
    let plan_label = campaign.clone().unwrap_or_else(|| START_PLAN_BASE.to_string());
    let plan_user = text_field(plan, "user_plan_id");
    let plan_id = text_field(plan, "plan_id");
    let mut plan_rows: Vec<StartGrant> = Vec::new();
    let mut saw_bucket = false;
    for (index, bucket) in balances.iter().enumerate() {
        if claimed
            .as_deref()
            .is_some_and(|flags| flags.get(index).copied().unwrap_or(false))
        {
            continue;
        }
        if !bucket_matches_for_plan(
            &plan_user,
            &plan_id,
            &plan_name,
            &text_field(bucket, "user_plan_id"),
            &text_field(bucket, "plan_id"),
            allow_user_fallback,
        ) {
            continue;
        }
        saw_bucket = true;
        if !include_expired && bucket_expired(bucket, now_s) {
            if let Some(flags) = claimed.as_deref_mut() {
                if let Some(slot) = flags.get_mut(index) {
                    *slot = true;
                }
            }
            continue;
        }
        if let Some(flags) = claimed.as_deref_mut() {
            if let Some(slot) = flags.get_mut(index) {
                *slot = true;
            }
        }
        let Some(total) = num(bucket.get("total_units")).filter(|t| *t > 0.0) else {
            continue;
        };
        let used = num(bucket.get("used_units"))
            .unwrap_or(0.0)
            .clamp(0.0, total);
        let label = {
            let shown = text_field(bucket, "show_name");
            if !shown.is_empty() {
                shown
            } else {
                plan_label.clone()
            }
        };
        let end = bucket_deadline(bucket);
        let start = num(bucket.get("period_start")).filter(|t| *t > 0.0);
        // Buckets that share a name (top-ups, split windows) sum into one row
        // and keep the latest end, exactly like the weekend row did.
        if let Some(existing) = plan_rows.iter_mut().find(|row| row.label == label) {
            existing.allowance.used += used;
            existing.allowance.total += total;
            if let Some(end) = end {
                if existing
                    .allowance
                    .resets_at_ms
                    .is_none_or(|cur| end * 1000.0 > cur as f64)
                {
                    existing.allowance.resets_at_ms = Some((end * 1000.0) as i64);
                    existing.allowance.period_ms = start
                        .filter(|s| *s < end)
                        .map(|s| ((end - s) * 1000.0) as i64);
                }
            }
            continue;
        }
        plan_rows.push(StartGrant {
            label,
            plan: plan_label.clone(),
            allowance: WeekendAllowance {
                used,
                total,
                resets_at_ms: end.map(|t| (t * 1000.0) as i64),
                period_ms: match (start, end) {
                    (Some(s), Some(e)) if e > s => Some(((e - s) * 1000.0) as i64),
                    _ => None,
                },
            },
        });
    }
    (plan_rows, saw_bucket)
}

/// One balance bucket as a renderable grant. Kept beside `plan_buckets` so
/// the fallback path below uses exactly the same window and label semantics.
fn bucket_grant(bucket: &Value, plan_label: &str) -> Option<StartGrant> {
    let total = num(bucket.get("total_units")).filter(|t| *t > 0.0)?;
    let used = num(bucket.get("used_units"))
        .unwrap_or(0.0)
        .clamp(0.0, total);
    let shown = text_field(bucket, "show_name");
    let label = if shown.is_empty() {
        plan_label.to_string()
    } else {
        shown
    };
    let end = bucket_deadline(bucket);
    let start = num(bucket.get("period_start")).filter(|t| *t > 0.0);
    Some(StartGrant {
        label,
        plan: plan_label.to_string(),
        allowance: WeekendAllowance {
            used,
            total,
            resets_at_ms: end.map(|t| (t * 1000.0) as i64),
            period_ms: match (start, end) {
                (Some(s), Some(e)) if e > s => Some(((e - s) * 1000.0) as i64),
                _ => None,
            },
        },
    })
}

fn text_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

fn has_entitlements(plan: &Value) -> bool {
    plan.get("entitlements")
        .and_then(Value::as_array)
        .is_some_and(|list| !list.is_empty())
}

fn is_coding_plan(plan: &Value) -> bool {
    let name = text_field(plan, "name").to_lowercase();
    let id = text_field(plan, "plan_id").to_lowercase();
    name.contains("coding plan")
        || id.contains("coding-plan")
        || id.contains("coding_plan")
}

fn is_marked_start_plan(plan: &Value) -> bool {
    if text_field(plan, "name")
        .to_lowercase()
        .contains("start plan")
    {
        return true;
    }
    let plan_id = text_field(plan, "plan_id").to_lowercase();
    if plan_id.contains("start-plan") || plan_id.contains("start_plan") {
        return true;
    }
    plan.get("entitlements")
        .and_then(Value::as_array)
        .is_some_and(|list| {
            list.iter().any(|entitlement| {
                let id = text_field(entitlement, "entitlement_id").to_lowercase();
                id.contains("start-plan") || id.contains("start_plan")
            })
        })
}

/// A Start Plan campaign, never a Coding Plan entry. The endpoint is Start
/// Plan billing, so an unmarked gift plan (a future campaign with an opaque
/// name) still qualifies; a coding plan is excluded by name regardless.
fn is_start_plan_plan(plan: &Value) -> bool {
    if is_coding_plan(plan) {
        return false;
    }
    is_marked_start_plan(plan)
        || has_entitlements(plan)
        || text_field(plan, "name").to_lowercase().starts_with("zcode")
}

fn is_plan_separator(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '·' | '-' | '_' | ':' | '—')
}

/// Turn a campaign plan name ("ZCode 1024 Dev Day", "ZCode Weekend Build")
/// into its campaign part. The stable base is always Start Plan.
fn campaign_name(raw: &str) -> Option<String> {
    let mut text = raw.trim();
    for prefix in ["ZCode", "Z.AI", "Zhipu", "BigModel"] {
        if text
            .get(..prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
        {
            text = text[prefix.len()..].trim_start_matches(is_plan_separator);
            break;
        }
    }
    let lower = text.to_lowercase();
    if lower == "start plan" {
        return None;
    }
    if lower.starts_with("start plan") {
        text = text["start plan".len()..].trim_start_matches(is_plan_separator);
    }
    let text = text.trim_matches(is_plan_separator);
    if text.is_empty() || text.eq_ignore_ascii_case(START_PLAN_BASE) {
        None
    } else {
        Some(text.to_string())
    }
}

fn plan_expired(plan: &Value, now_s: i64) -> bool {
    let status = plan
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let ends_at = num(plan.get("ends_at")).unwrap_or(0.0) as i64;
    status == "expired" || (status == "active" && ends_at > 0 && ends_at <= now_s)
}

/// A bucket whose own window has closed contributes nothing, whatever its
/// parent plan still claims.
fn bucket_expired(bucket: &Value, now_s: i64) -> bool {
    bucket_deadline(bucket).is_some_and(|deadline| deadline <= now_s as f64)
}

// ---------------------------------------------------------------------------
// Credential store
// ---------------------------------------------------------------------------

/// The seed ZCode derives its storage key from. `platform`/`home`/`user` are
/// parameters so tests can pin them; the live values must match what ZCode
/// (Node) sees: bare account name, no DOMAIN\ prefix, and the platform
/// spelled the Node way ("win32", not "windows").
fn credential_seed(platform: &str, home: &str, user: &str) -> String {
    format!("zcode-credential-fallback:{platform}:{home}:{user}")
}

fn derive_key(seed: &str) -> [u8; 32] {
    Sha256::digest(seed.as_bytes()).into()
}

fn b64_url() -> GeneralPurpose {
    // ZCode (Node base64url) writes unpadded; accept both spellings so a
    // padded value from some other writer still decodes.
    GeneralPurpose::new(
        &alphabet::URL_SAFE,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    )
}

/// One `enc:v1:` blob → plaintext. Layout: iv (12 bytes) . tag . ciphertext,
/// all base64url; GCM opens ct||tag.
fn decrypt_value(key: &[u8; 32], value: &str) -> Option<String> {
    let rest = value.strip_prefix("enc:v1:")?;
    let mut parts = rest.split('.');
    let iv = b64_url().decode(parts.next()?).ok()?;
    let tag = b64_url().decode(parts.next()?).ok()?;
    let ct = b64_url().decode(parts.next()?).ok()?;
    if parts.next().is_some() || iv.len() != 12 || tag.is_empty() {
        return None;
    }
    let cipher = Aes256Gcm::new_from_slice(key).ok()?;
    let mut payload = ct;
    payload.extend_from_slice(&tag);
    let plain = cipher
        .decrypt(Nonce::from_slice(&iv), payload.as_slice())
        .ok()?;
    String::from_utf8(plain).ok()
}

/// The storage key as the running machine would derive it. Env override
/// first (ZCODE_CREDENTIAL_SECRET), then the fallback seed — the same
/// string the Node probe on this machine verified.
fn current_key() -> Option<[u8; 32]> {
    if let Ok(seed) = std::env::var("ZCODE_CREDENTIAL_SECRET") {
        if !seed.is_empty() {
            return Some(derive_key(&seed));
        }
    }
    let home = dirs::home_dir()?.to_string_lossy().into_owned();
    let user = if cfg!(windows) {
        std::env::var("USERNAME")
    } else {
        std::env::var("USER")
    }
    .unwrap_or_default();
    let platform = if cfg!(windows) {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };
    Some(derive_key(&credential_seed(platform, &home, &user)))
}

/// The first entry of ~/.zcode/v2/credentials.json that decrypts to a JWT
/// (the user has exactly one ZCode account — spec, closed question 2).
/// Read-only: the file and its plaintexts never leave this function, and
/// nothing here is logged.
fn read_local_jwt() -> Option<String> {
    let path = dirs::home_dir()?
        .join(".zcode")
        .join("v2")
        .join("credentials.json");
    let raw = read_small_text(&path, 256 * 1024, "zcode credentials").ok()?;
    let doc: Value =
        serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok()?;
    let key = current_key()?;
    pick_jwt(doc.as_object()?, &key)
}

fn is_jwt_shape(s: &str) -> bool {
    let segments: Vec<&str> = s.split('.').collect();
    segments.len() == 3 && segments.iter().all(|p| !p.is_empty())
}

/// Picks the session JWT out of the credential store. The store also keeps
/// an OAuth access token (oauth:zai:access_token) that decrypts to a
/// JWT-shaped string yet answers 401 on the plan endpoint (probed live
/// 2026-10-12), so the documented entry name (`zcodejwttoken`) is preferred
/// outright; scanning falls back to file order for future layouts.
fn pick_jwt(store: &serde_json::Map<String, Value>, key: &[u8; 32]) -> Option<String> {
    let from_entry = |value: &Value| -> Option<String> {
        let plain = decrypt_value(key, value.as_str()?)?;
        is_jwt_shape(&plain).then_some(plain)
    };
    if let Some(jwt) = store.get("zcodejwttoken").and_then(from_entry) {
        return Some(jwt);
    }
    store.values().find_map(from_entry)
}

/// True only when the JWT clearly carries an `exp` that has passed; an
/// unparseable token is let through to fail (silently) at the endpoint.
fn jwt_expired(jwt: &str, now_s: i64) -> bool {
    let Some(payload) = b64_url().decode(jwt.split('.').nth(1).unwrap_or("")).ok() else {
        return false;
    };
    let Ok(doc) = serde_json::from_slice::<Value>(&payload) else {
        return false;
    };
    match doc.get("exp").and_then(Value::as_i64) {
        Some(exp) if exp > 0 => now_s >= exp,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Device id
// ---------------------------------------------------------------------------

fn valid_uuid(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    for (i, b) in bytes.iter().enumerate() {
        match i {
            8 | 13 | 18 | 23 => {
                if *b != b'-' {
                    return false;
                }
            }
            _ => {
                if !b.is_ascii_hexdigit() {
                    return false;
                }
            }
        }
    }
    true
}

fn new_uuid_v4() -> String {
    let mut b = [0u8; 16];
    OsRng.fill_bytes(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40; // version 4
    b[8] = (b[8] & 0x3f) | 0x80; // variant 10
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// This machine's X-Device-Mid: kept once beside config.json, reused after.
fn device_mid() -> String {
    let path = config_dir().join("zcode-device-mid");
    if let Ok(id) = read_small_text(&path, 256, "zcode device id") {
        let id = id.trim().to_lowercase();
        if valid_uuid(&id) {
            return id;
        }
    }
    let id = new_uuid_v4();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, format!("{id}\n"));
    id
}

// ---------------------------------------------------------------------------
// The balance endpoint
// ---------------------------------------------------------------------------

async fn fetch_balance(jwt: &str, mid: &str) -> Option<Value> {
    let url = format!("{ZCODE_API}/api/v1/zcode-plan/billing/balance?app_version={APP_VERSION}");
    let build = |client: &reqwest::Client| {
        client
            .get(&url)
            .bearer_auth(jwt)
            .header("X-Device-Mid", mid)
            .header("HTTP-Referer", "https://zcode.z.ai")
            .header("X-Client-Language", "zh-CN")
            .header("X-Client-Timezone", "Asia/Shanghai")
            .header("Accept", "application/json")
    };
    let resp = send_with_direct_fallback(build).await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    json_body(resp, 256 * 1024, "zcode balance").await.ok()
}

/// A balance number arrives as a JSON number or a decimal string (magpie's
/// zcodeNum).
fn num(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// True when a balance bucket belongs to a plan. `plan_id` decides whenever
/// both sides carry one: one account can hold several plans (a 1024 gift beside
/// the coding plan), and they share `user_plan_id` — matching on the user first
/// attached every bucket to every plan. User id is only the fallback for a
/// payload that omits plan ids on one side.
fn bucket_matches(plan_user: &str, plan_id: &str, bucket_user: &str, bucket_id: &str) -> bool {
    if !plan_id.is_empty() || !bucket_id.is_empty() {
        // One side names a plan, so plan identity is the only proof of
        // ownership: a bucket without one must not fall back to the user id,
        // or it would be claimed by every plan that account holds.
        return !plan_id.is_empty() && plan_id == bucket_id;
    }
    !plan_user.is_empty() && plan_user == bucket_user
}

/// Plan/bucket pairing used by the Start Plan reader. Exact plan ids win;
/// when a response omits one side, the caller may fall back to the account's
/// user-plan id. A missing campaign id is also paired by the campaign token
/// in its plan name (for example `1024`), which keeps older fleet records
/// visible without fanning the same bucket into every plan.
fn bucket_matches_for_plan(
    plan_user: &str,
    plan_id: &str,
    plan_name: &str,
    bucket_user: &str,
    bucket_id: &str,
    allow_user_fallback: bool,
) -> bool {
    let user_matches = !plan_user.is_empty() && plan_user == bucket_user;
    if !plan_id.is_empty() && !bucket_id.is_empty() {
        return plan_id == bucket_id;
    }
    if !plan_id.is_empty() {
        return allow_user_fallback && user_matches;
    }
    if !bucket_id.is_empty() {
        return (user_matches || plan_user.is_empty())
            && plan_token_matches(plan_name, bucket_id);
    }
    allow_user_fallback && user_matches
}

fn plan_token_matches(plan_name: &str, bucket_id: &str) -> bool {
    let bucket = bucket_id.to_ascii_lowercase();
    plan_name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|token| {
            token.len() >= 3
                && !matches!(
                    token.as_str(),
                    "zcode" | "start" | "plan" | "dev" | "day" | "build"
                )
        })
        .any(|token| bucket.contains(&token))
}

/// Multiple Start Plan fleets share one user_plan_id. Only let the user-id
/// fallback claim an id-less bucket on the plan that carries an explicit id,
/// unless every fleet on that user lacks an id.
fn allow_user_fallback(plan: &Value, plans: &[Value], now_s: i64) -> bool {
    let user = text_field(plan, "user_plan_id");
    if user.is_empty() {
        return false;
    }
    let candidates: Vec<&Value> = plans
        .iter()
        .filter(|candidate| {
            is_start_plan_plan(candidate)
                && !plan_expired(candidate, now_s)
                && text_field(candidate, "user_plan_id") == user
        })
        .collect();
    if candidates.len() <= 1 {
        return true;
    }
    let plan_id = text_field(plan, "plan_id");
    !plan_id.is_empty()
        || !candidates
            .iter()
            .any(|candidate| !text_field(candidate, "plan_id").is_empty())
}

/// A bucket's real deadline: the earlier of its window end and its own
/// expiry. Two fields can disagree; taking one and ignoring the other showed
/// expired grants as usable.
fn bucket_deadline(bucket: &Value) -> Option<f64> {
    [num(bucket.get("period_end")), num(bucket.get("expires_at"))]
        .into_iter()
        .flatten()
        .filter(|t| *t > 0.0)
        .reduce(f64::min)
}

/// The weekend grant out of a successful balance response: the one
/// unexpired plan whose name says "weekend", its token buckets summed.
/// `fallback_now_s` stands in when the response carries no server_time.
pub fn parse_weekend(doc: &Value, fallback_now_s: i64) -> Option<WeekendAllowance> {
    if doc.get("code").and_then(Value::as_i64) != Some(0) {
        return None;
    }
    let data = doc.get("data")?;
    let now_s = num(data.get("server_time"))
        .filter(|t| *t > 0.0)
        .unwrap_or(fallback_now_s as f64) as i64;

    let plans = data.get("plans")?.as_array()?;
    let balances = data.get("balances")?.as_array()?;

    // A plan past its end is over even while it still reports "active"
    // (measured 2026-10-12: the weekend plan lingered active past
    // ends_at) — normalize, then remember the dead ones.
    let expired = |p: &Value| -> bool {
        let status = p
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let ends_at = num(p.get("ends_at")).unwrap_or(0.0) as i64;
        if status == "expired" {
            return true;
        }
        status == "active" && ends_at > 0 && ends_at <= now_s
    };
    let plan_field = |p: &Value, key: &str| -> String {
        p.get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };

    let plans: Vec<(String, String, bool, bool)> = plans
        .iter()
        .map(|p| {
            (
                plan_field(p, "user_plan_id"),
                plan_field(p, "plan_id"),
                expired(p),
                p.get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains("weekend"),
            )
        })
        .collect();

    let bucket_over = |b: &Value| {
        let (bu, bi) = (
            plan_field(b, "user_plan_id"),
            plan_field(b, "plan_id"),
        );
        plans
            .iter()
            .any(|(pu, pid, dead, _)| *dead && bucket_matches(pu, pid, &bu, &bi))
    };
    let bucket_is_weekend = |b: &Value| {
        let (bu, bi) = (
            plan_field(b, "user_plan_id"),
            plan_field(b, "plan_id"),
        );
        plans
            .iter()
            .any(|(pu, pid, dead, weekend)| *weekend && !*dead && bucket_matches(pu, pid, &bu, &bi))
    };

    let mut used = 0.0;
    let mut total = 0.0;
    let mut window_end: Option<f64> = None;
    let mut window_start: Option<f64> = None;
    for b in balances.iter() {
        if bucket_over(b) || !bucket_is_weekend(b) {
            continue;
        }
        let Some(bucket_total) = num(b.get("total_units")).filter(|t| *t > 0.0) else {
            continue;
        };
        used += num(b.get("used_units")).unwrap_or(0.0).max(0.0);
        total += bucket_total;
        let end = num(b.get("period_end")).filter(|t| *t > 0.0);
        if let Some(end) = end {
            if window_end.is_none_or(|cur| end > cur) {
                window_end = Some(end);
                window_start = num(b.get("period_start")).filter(|t| *t > 0.0);
            }
        }
    }
    if total <= 0.0 {
        return None;
    }
    let resets_at_ms = window_end.map(|s| (s * 1000.0) as i64);
    let period_ms = match (window_end, window_start) {
        (Some(end), Some(start)) if end > start => Some(((end - start) * 1000.0) as i64),
        _ => None,
    };
    Some(WeekendAllowance {
        used,
        total,
        resets_at_ms,
        period_ms,
    })
}

/// 115465592 → "115,465,592"; token counts are integral, negatives clamp.
fn with_thousands(v: f64) -> String {
    let digits = (v.max(0.0).round() as u64).to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
    use serde_json::json;

    fn encrypt_blob(key: &[u8; 32], plain: &[u8]) -> String {
        use aes_gcm::aead::AeadInPlace;
        let cipher = Aes256Gcm::new_from_slice(key).unwrap();
        let mut iv = [0u8; 12];
        OsRng.fill_bytes(&mut iv);
        let mut buf = plain.to_vec();
        let tag = cipher
            .encrypt_in_place_detached(Nonce::from_slice(&iv), b"", &mut buf)
            .unwrap();
        format!(
            "enc:v1:{}.{}.{}",
            URL_SAFE_NO_PAD.encode(iv),
            URL_SAFE_NO_PAD.encode(tag),
            URL_SAFE_NO_PAD.encode(buf)
        )
    }

    /// Read-only inventory of this machine's real Start Plan balance. Ignored by
    /// default: it needs a local ZCode session plus network, and prints
    /// account amounts for calibration. IDs, emails and tokens are never printed.
    #[tokio::test]
    #[ignore = "real-machine Start Plan inventory; needs a local ZCode session"]
    async fn real_machine_start_plan_inventory() {
        let Some(jwt) = read_local_jwt() else {
            eprintln!("no local ZCode session");
            return;
        };
        let Some(doc) = fetch_balance(&jwt, &device_mid()).await else {
            eprintln!("balance request failed");
            return;
        };
        let data = &doc["data"];
        eprintln!("code={:?} server_time={:?}", doc.get("code"), data.get("server_time"));
        eprintln!(
            "data keys={:?}",
            data.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>())
        );
        if let Some(plans) = data.get("plans").and_then(Value::as_array) {
            for plan in plans {
                eprintln!(
                    "plan name={:?} status={:?} starts={:?} ends={:?} description={:?} entitlements={:?}",
                    plan.get("name"),
                    plan.get("status"),
                    plan.get("starts_at"),
                    plan.get("ends_at"),
                    plan.get("description"),
                    plan.get("entitlements").map(|v| format!("len={} {}", v.as_array().map(|a| a.len()).unwrap_or(0), v.to_string().chars().take(240).collect::<String>()))
                );
            }
        }
        if let Some(balances) = data.get("balances").and_then(Value::as_array) {
            for bucket in balances {
                eprintln!(
                    "bucket show_name={:?} meter={:?} unit={:?} caps={:?} total={:?} used={:?} remaining={:?} available={:?} start={:?} end={:?} expires={:?}",
                    bucket.get("show_name"),
                    bucket.get("meter"),
                    bucket.get("unit_type"),
                    bucket.get("capabilities").map(|v| v.to_string().chars().take(120).collect::<String>()),
                    bucket.get("total_units"),
                    bucket.get("used_units"),
                    bucket.get("remaining_units"),
                    bucket.get("available_units"),
                    bucket.get("period_start"),
                    bucket.get("period_end"),
                    bucket.get("expires_at")
                );
            }
        }
    }

    #[test]
    fn decrypt_round_trips_the_zcode_layout() {
        let seed = credential_seed("win32", "C:\\Users\\t", "alice");
        let key = derive_key(&seed);
        let blob = encrypt_blob(&key, b"header.payload.signature");
        assert_eq!(
            decrypt_value(&key, &blob).as_deref(),
            Some("header.payload.signature")
        );
    }

    #[test]
    fn decrypt_rejects_a_wrong_key_and_mangled_blobs() {
        let key = derive_key("seed-a");
        let blob = encrypt_blob(&key, b"secret");
        let other = derive_key("seed-b");
        assert!(decrypt_value(&other, &blob).is_none());
        assert!(decrypt_value(&key, "not-enc").is_none());
        assert!(decrypt_value(&key, "enc:v1:one.two").is_none());
        // A padded blob still decodes (padding indifferent).
        let padded = format!(
            "enc:v1:{}.{}.{}",
            URL_SAFE.encode([0u8; 12]),
            URL_SAFE.encode([0u8; 16]),
            URL_SAFE.encode("x")
        );
        assert!(decrypt_value(&key, &padded).is_none(), "auth must fail");
    }

    #[test]
    fn uuid_is_v4_and_the_validator_agrees() {
        let id = new_uuid_v4();
        assert!(valid_uuid(&id), "{id}");
        assert_eq!(&id[14..15], "4", "version nibble");
        assert!(matches!(&id[19..20], "8" | "9" | "a" | "b"), "variant nibble");
        assert!(!valid_uuid(&id[..35]));
        assert!(!valid_uuid("not-a-uuid"));
    }

    #[test]
    fn jwt_expiry_needs_a_real_future_exp_to_pass() {
        let mk = |payload: &str| format!("h.{payload}.s");
        let past = URL_SAFE_NO_PAD.encode(json!({"exp": 1000}).to_string());
        let future = URL_SAFE_NO_PAD.encode(json!({"exp": 9_999_999_999i64}).to_string());
        assert!(jwt_expired(&mk(&past), 2000));
        assert!(!jwt_expired(&mk(&future), 2000));
        assert!(!jwt_expired(&mk(&URL_SAFE_NO_PAD.encode("notjson")), 2000));
        assert!(!jwt_expired("garbage", 2000));
    }

    fn balance_doc(plans: Value, balances: Value, server_time: i64) -> Value {
        json!({
            "code": 0,
            "data": {
                "server_time": server_time,
                "plans": plans,
                "balances": balances
            }
        })
    }

    const NOW: i64 = 1_760_100_000; // inside the measured grant window

    #[test]
    fn a_session_without_a_start_plan_is_a_setup_state() {
        let doc = json!({"code": 0, "data": {"server_time": NOW, "plans": [], "balances": []}});
        let snap = snapshot_from_doc(&doc, NOW);
        assert_eq!(snap.status, "needs_setup");
        assert_eq!(snap.plan.as_deref(), Some("Start Plan"));
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].label, "Start Plan");
        assert_eq!(snap.metrics[0].value.as_deref(), Some(NO_START_PLAN));
    }

    #[test]
    fn a_valid_plan_without_a_gift_says_so_instead_of_showing_nothing() {
        let doc = balance_doc(
            json!([{"plan_id": "p-start", "user_plan_id": "up-1", "name": "ZCode Start Plan",
                    "status": "active", "ends_at": NOW + 50_000,
                    "entitlements": []}]),
            json!([]),
            NOW,
        );
        let snap = snapshot_from_doc(&doc, NOW);
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan.as_deref(), Some("Start Plan"));
        // The plan is held but nothing is granted this period: one explicit
        // line, not an empty card and not a fake zero bar.
        assert!(snap.metrics.iter().any(|m| m.value.as_deref() == Some(NO_GIFT)));
    }

    #[test]
    fn live_grants_render_one_row_each_and_name_the_plan() {
        let doc = balance_doc(
            json!([{"plan_id": "p-1024", "user_plan_id": "up-1", "name": "ZCode 1024 Dev Day",
                    "status": "active", "ends_at": NOW + 50_000,
                    "entitlements": [{"show_name": "GLM-5.3"}, {"show_name": "GLM-5.3-Flash"}]}]),
            json!([
                {"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3-Flash",
                 "total_units": 300, "used_units": 0, "period_end": NOW + 10_000},
                {"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 200, "used_units": 0, "period_end": NOW + 10_000}
            ]),
            NOW,
        );
        let snap = snapshot_from_doc(&doc, NOW);
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan.as_deref(), Some("Start Plan (1024 Dev Day)"));
        assert_eq!(snap.metrics.len(), 2);
        assert!(snap.metrics.iter().all(|m| m.kind == "progress"));
    }

    #[test]
    fn a_rejected_envelope_is_an_error_not_an_empty_plan() {
        let snap = snapshot_from_doc(&json!({"code": 3001, "msg": "parameter error"}), NOW);
        assert_eq!(snap.status, "error");
        assert!(snap.metrics.is_empty());
    }

    #[test]
    fn any_campaign_name_is_a_gift_row_without_code_changes() {
        // Names are evidence, not filters: a weekend build, a 618 build or the
        // next model trial all arrive as plans/entitlements.
        let doc = balance_doc(
            json!([{"plan_id": "p-next", "user_plan_id": "up-1", "name": "ZCode 618 Build",
                    "status": "active", "ends_at": NOW + 50_000,
                    "entitlements": [{"show_name": "GLM-5.4-Trial"}]}]),
            json!([{"plan_id": "p-next", "user_plan_id": "up-1", "show_name": "GLM-5.4-Trial",
                    "total_units": 100_000_000, "used_units": 0, "period_end": NOW + 10_000}]),
            NOW,
        );
        let snap = snapshot_from_doc(&doc, NOW);
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan.as_deref(), Some("Start Plan (618 Build)"));
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].label, "GLM-5.4-Trial");
        assert_eq!(snap.metrics[0].kind, "progress");
    }

    #[test]
    fn a_dev_day_plan_lists_each_gift_bucket_by_its_own_name() {
        // Shape and labels from the live probe of 2026-10-23: one plan
        // ("ZCode 1024 Dev Day") with a GLM-5.3-Flash 3亿 and a GLM-5.3 2亿
        // entitlement, mirrored as two balance buckets.
        let doc = balance_doc(
            json!([{
                "plan_id": "p-1024",
                "user_plan_id": "up-1",
                "name": "ZCode 1024 Dev Day",
                "status": "active",
                "ends_at": NOW + 10_000
            }]),
            json!([
                {"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3-Flash",
                 "total_units": 300_000_000, "used_units": 0,
                 "period_start": NOW - 1_000, "period_end": NOW + 10_000},
                {"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 200_000_000, "used_units": 50_000_000,
                 "period_start": NOW - 1_000, "period_end": NOW + 10_000}
            ]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert!(ungranted.is_empty());
        assert_eq!(grants.len(), 2, "one row per granted model bucket");
        let rows = render_rows(&grants, &ungranted);
        assert_eq!(rows.len(), 2);
        // Same deadline, so labels decide the order: GLM-5.3 before -Flash.
        assert_eq!(rows[0].label, "GLM-5.3");
        assert_eq!(rows[0].used_percent, Some(25.0));
        assert_eq!(
            rows[0].detail.as_deref(),
            Some("50,000,000 of 200,000,000 used")
        );
        assert_eq!(rows[0].resets_at, Some((NOW + 10_000) * 1000));
        assert_eq!(rows[1].label, "GLM-5.3-Flash");
        assert_eq!(rows[1].used_percent, Some(0.0));
        assert_eq!(rows[1].detail.as_deref(), Some("0 of 300,000,000 used"));
    }

    #[test]
    fn unnamed_buckets_fall_back_to_the_plan_name_and_sum() {
        let doc = balance_doc(
            json!([{
                "plan_id": "p-weekend",
                "user_plan_id": "up-1",
                "name": "ZCode Weekend Build",
                "status": "active",
                "ends_at": NOW + 100_000
            }]),
            json!([
                {"plan_id": "p-weekend", "user_plan_id": "up-1", "total_units": "200",
                 "used_units": 50, "period_start": NOW - 5_000, "period_end": NOW + 3_000},
                {"plan_id": "p-weekend", "user_plan_id": "up-1", "total_units": 100,
                 "used_units": 10, "period_start": NOW - 1_000, "period_end": NOW + 9_000}
            ]),
            NOW,
        );
        let (grants, _) = parse_start_grants(&doc, NOW);
        assert_eq!(grants.len(), 1, "same label merges into one row");
        assert_eq!(grants[0].label, "Weekend Build");
        assert_eq!(grants[0].allowance.used, 60.0);
        assert_eq!(grants[0].allowance.total, 300.0);
        assert_eq!(grants[0].allowance.resets_at_ms, Some((NOW + 9_000) * 1000));
        assert_eq!(grants[0].allowance.period_ms, Some(10_000 * 1000));
    }

    #[test]
    fn an_active_but_past_end_plan_is_dead() {
        // Measured behavior: a plan reported status=active after ends_at had
        // passed; its buckets must go with it.
        let doc = balance_doc(
            json!([{
                "plan_id": "p-weekend",
                "user_plan_id": "up-1",
                "name": "ZCode Weekend Build",
                "status": "active",
                "ends_at": NOW - 100
            }]),
            json!([{"plan_id": "p-weekend", "user_plan_id": "up-1",
                    "total_units": 300, "used_units": 1, "period_end": NOW + 500}]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert!(grants.is_empty());
        assert!(ungranted.is_empty(), "a dead plan is not something to claim");
        let state = parse_start_plan_state(&doc, NOW);
        assert!(!state.has_active);
        assert_eq!(state.expired_campaigns, vec!["Weekend Build".to_string()]);
        assert_eq!(state.expired.len(), 1);
        // The ended campaign stays visible as history, with its final usage.
        let snap = snapshot_from_doc(&doc, NOW);
        assert_eq!(snap.status, "needs_setup");
        assert_eq!(snap.plan.as_deref(), Some("Start Plan (Weekend Build)"));
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].label, "Start Plan (Weekend Build)");
        assert_eq!(
            snap.metrics[0].value.as_deref(),
            Some("expired · 1 of 300 used")
        );
    }

    #[test]
    fn a_coding_plan_never_becomes_a_start_plan_gift() {
        let doc = balance_doc(
            json!([{"plan_id": "coding-plan-p", "user_plan_id": "up-1", "name": "GLM Coding Plan",
                    "status": "active", "ends_at": NOW + 50_000}]),
            json!([{"plan_id": "coding-plan-p", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                    "total_units": 9_000_000, "used_units": 8_000_000, "period_end": NOW + 10_000}]),
            NOW,
        );
        let state = parse_start_plan_state(&doc, NOW);
        assert!(!state.found, "the coding plan is not a Start Plan campaign");
        assert!(state.grants.is_empty());
        let snap = snapshot_from_doc(&doc, NOW);
        assert_eq!(snap.status, "needs_setup");
        assert_eq!(snap.metrics[0].value.as_deref(), Some(NO_START_PLAN));
    }

    #[test]
    fn campaign_names_never_repeat_the_plan_identity() {
        assert_eq!(campaign_name("ZCode 1024 Dev Day").as_deref(), Some("1024 Dev Day"));
        assert_eq!(campaign_name("ZCode Weekend Build").as_deref(), Some("Weekend Build"));
        assert_eq!(campaign_name("ZCode Start Plan"), None);
        assert_eq!(campaign_name("Start Plan"), None);
        assert_eq!(campaign_name("  "), None);
    }

    #[test]
    fn a_closed_bucket_does_not_count_even_under_a_live_plan() {
        let doc = balance_doc(
            json!([{"plan_id": "p-1", "user_plan_id": "up-1", "name": "ZCode Weekend Build",
                    "status": "active", "ends_at": NOW + 50_000}]),
            json!([{"plan_id": "p-1", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                    "total_units": 500, "used_units": 0, "period_end": NOW - 1}]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert!(grants.is_empty(), "a closed window is not a live gift");
        assert!(ungranted.is_empty(), "an expired grant is not something to claim");
    }

    #[test]
    fn a_plan_held_without_a_bucket_renders_a_claim_line() {
        let doc = balance_doc(
            json!([{"plan_id": "p-1024", "user_plan_id": "up-1", "name": "ZCode 1024 Dev Day",
                    "status": "active", "ends_at": NOW + 50_000,
                    "entitlements": [{"show_name": "GLM-5.3"}]}]),
            json!([]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert!(grants.is_empty());
        assert_eq!(ungranted, vec!["Start Plan (1024 Dev Day)".to_string()]);
        let rows = render_rows(&grants, &ungranted);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "text");
        assert_eq!(rows[0].label, "Start Plan (1024 Dev Day)");
        assert_eq!(rows[0].value.as_deref(), Some(CLAIM_HINT));
    }

    #[test]
    fn buckets_of_other_plans_never_leak_into_a_plan() {
        // A coding plan and a gift plan share the balances array; each plan's
        // rows come only from its own buckets (user_plan_id decides).
        let doc = balance_doc(
            json!([
                {"plan_id": "p-gift", "user_plan_id": "up-1", "name": "ZCode 1024 Dev Day",
                 "status": "active", "ends_at": NOW + 50_000},
                {"plan_id": "p-coding", "user_plan_id": "up-2", "name": "GLM Coding Plan",
                 "status": "active", "ends_at": NOW + 90_000}
            ]),
            json!([
                {"plan_id": "p-gift", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 200, "used_units": 10},
                {"plan_id": "p-coding", "user_plan_id": "up-2", "show_name": "Coding Plan",
                 "total_units": 9_000_000, "used_units": 8_000_000}
            ]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert!(ungranted.is_empty());
        assert_eq!(grants.len(), 1, "a coding plan is not a Start Plan gift");
        assert_eq!(grants[0].label, "GLM-5.3");
        assert_eq!(
            grants[0].allowance.total, 200.0,
            "the coding plan's 9M must not inflate the gift"
        );
    }

    #[test]
    fn the_named_session_entry_beats_file_order() {
        // The store's other JWT-shaped entry (zai's OAuth token) sits FIRST
        // on this machine and answers 401 on the plan endpoint — the named
        // session entry must win regardless of insertion order.
        let key = derive_key("seed");
        let enc = |plain: &str| encrypt_blob(&key, plain.as_bytes());
        let mut store = serde_json::Map::new();
        store.insert(
            "oauth:zai:access_token".into(),
            Value::String(enc("aaa.bbb.ccc")),
        );
        store.insert(
            "zcodejwttoken".into(),
            Value::String(enc("real.session.token")),
        );
        assert_eq!(
            pick_jwt(&store, &key).as_deref(),
            Some("real.session.token")
        );
        // Without the named entry the scan falls back to file order.
        store.remove("zcodejwttoken");
        assert_eq!(pick_jwt(&store, &key).as_deref(), Some("aaa.bbb.ccc"));
    }

    #[test]
    fn a_plan_without_entitlements_is_not_a_gift_to_claim() {
        // The coding-plan entry shares plans[] but declares no entitlements;
        // calling it "held · not granted" would ask the owner to claim it in
        // ZCode, where no such gift exists.
        let doc = balance_doc(
            json!([{"plan_id": "p-coding", "user_plan_id": "up-1", "name": "GLM Coding Plan",
                    "status": "active", "ends_at": NOW + 50_000, "entitlements": []}]),
            json!([]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert!(grants.is_empty());
        assert!(ungranted.is_empty(), "a non-gift plan is not claimable");
    }

    #[test]
    fn claim_lines_are_never_truncated_away_by_the_bar_cap() {
        // Six live buckets plus two ungranted plans: the rows must keep both
        // claim lines, so the bars yield two slots instead of pushing claims
        // past MAX_ROWS.
        let mut balances = Vec::new();
        for i in 0..6 {
            balances.push(json!({
                "plan_id": "p-gift", "user_plan_id": "up-1",
                "show_name": format!("MODEL-{i}"), "total_units": 100, "used_units": 0,
                "period_end": NOW + 10_000
            }));
        }
        let doc = balance_doc(
            json!([
                {"plan_id": "p-gift", "user_plan_id": "up-1", "name": "ZCode 1024 Dev Day",
                 "status": "active", "ends_at": NOW + 50_000,
                 "entitlements": [{"show_name": "GLM-5.3"}]},
                {"plan_id": "p-1", "user_plan_id": "up-1", "name": "Held One",
                 "status": "active", "ends_at": NOW + 50_000,
                 "entitlements": [{"show_name": "GLM-5.3-Flash"}]},
                {"plan_id": "p-2", "user_plan_id": "up-1", "name": "Held Two",
                 "status": "active", "ends_at": NOW + 50_000,
                 "entitlements": [{"show_name": "GLM-5.3"}]}
            ]),
            json!(balances),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert_eq!(grants.len(), 6);
        assert_eq!(ungranted.len(), 2);
        let rows = render_rows(&grants, &ungranted);
        assert_eq!(rows.len(), MAX_ROWS, "the card stays bounded");
        let claim_lines = rows
            .iter()
            .filter(|r| r.value.as_deref() == Some(CLAIM_HINT))
            .count();
        // Two claim lines, one "+3 more grants" summary; the rest are bars.
        assert_eq!(claim_lines, 3, "claims and the overflow line survive");
        assert!(rows.iter().any(|r| r.label.starts_with("+3 more grants")));
    }

    #[test]
    fn one_accounts_plans_do_not_share_each_others_buckets() {
        // One account (same user_plan_id) holding two plans: matching on the
        // user first fanned every bucket out to both plans.
        let doc = balance_doc(
            json!([
                {"plan_id": "p-1024", "user_plan_id": "up-1", "name": "ZCode 1024 Dev Day",
                 "status": "active", "ends_at": NOW + 50_000,
                 "entitlements": [{"show_name": "GLM-5.3"}]},
                {"plan_id": "p-weekend", "user_plan_id": "up-1", "name": "ZCode Weekend Build",
                 "status": "active", "ends_at": NOW + 60_000,
                 "entitlements": [{"show_name": "GLM-5.3-Flash"}]}
            ]),
            json!([
                {"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 200, "used_units": 0, "period_end": NOW + 10_000},
                {"plan_id": "p-weekend", "user_plan_id": "up-1", "show_name": "GLM-5.3-Flash",
                 "total_units": 300, "used_units": 0, "period_end": NOW + 20_000}
            ]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert!(ungranted.is_empty());
        assert_eq!(grants.len(), 2, "one bucket per plan, not two per plan");
        let by_plan = |name: &str| grants.iter().find(|g| g.plan == name).unwrap();
        assert_eq!(by_plan("1024 Dev Day").allowance.total, 200.0);
        assert_eq!(by_plan("Weekend Build").allowance.total, 300.0);
    }

    #[test]
    fn two_start_plan_fleets_render_both_names_and_their_own_buckets() {
        let doc = balance_doc(
            json!([
                {"plan_id": "p-1024", "user_plan_id": "up-1", "name": "ZCode 1024 Dev Day",
                 "status": "active", "ends_at": NOW + 50_000,
                 "entitlements": [{"show_name": "GLM-5.3"}, {"show_name": "GLM-5.3-Flash"}]},
                {"plan_id": "p-start", "user_plan_id": "up-1", "name": "ZCode Start Plan",
                 "status": "active", "ends_at": NOW + 60_000,
                 "entitlements": [{"show_name": "GLM-5.3"}, {"show_name": "GLM-5.3-Flash"}]}
            ]),
            json!([
                {"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3-Flash",
                 "total_units": 300_000_000, "used_units": 0, "period_end": NOW + 10_000},
                {"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 200_000_000, "used_units": 0, "period_end": NOW + 10_000},
                {"plan_id": "p-start", "user_plan_id": "up-1", "show_name": "GLM-5.3-Flash",
                 "total_units": 5_000_000, "used_units": 2_269_745, "period_end": NOW + 60_000},
                {"plan_id": "p-start", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 3_000_000, "used_units": 0, "period_end": NOW + 60_000}
            ]),
            NOW,
        );
        let state = parse_start_plan_state(&doc, NOW);
        assert_eq!(state.active_plans.len(), 2);
        assert!(state.ungranted.is_empty());
        assert_eq!(state.grants.len(), 4);
        let totals: f64 = state.grants.iter().map(|g| g.allowance.total).sum();
        assert_eq!(totals, 508_000_000.0);
        let snap = snapshot_from_doc(&doc, NOW);
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan.as_deref(), Some("Start Plan (1024 Dev Day)"));
        assert_eq!(snap.metrics.len(), 4);
        assert!(snap
            .metrics
            .iter()
            .any(|m| m.label == "1024 Dev Day · GLM-5.3-Flash"));
        assert!(snap
            .metrics
            .iter()
            .any(|m| m.label == "Start Plan · GLM-5.3-Flash"));
    }

    #[test]
    fn mixed_plan_ids_still_render_every_start_plan_fleet() {
        // Classmate payload shape (2026-10-25): the campaign plan carries no
        // plan_id while its buckets do, and the base plan carries an id while
        // its buckets rely on user_plan_id. Matching only one direction hid
        // the 300M/200M Dev Day buckets and mislabelled the 3M/5M base rows.
        let doc = balance_doc(
            json!([
                {"user_plan_id": "up-1", "name": "ZCode 1024 Dev Day",
                 "status": "active", "ends_at": NOW + 50_000,
                 "entitlements": [{"show_name": "GLM-5.3"}, {"show_name": "GLM-5.3-Flash"}]},
                {"plan_id": "p-start", "user_plan_id": "up-1", "name": "ZCode Start Plan",
                 "status": "active", "ends_at": NOW + 60_000,
                 "entitlements": [{"show_name": "GLM-5.3"}, {"show_name": "GLM-5.3-Flash"}]}
            ]),
            json!([
                {"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3-Flash",
                 "total_units": 300_000_000, "used_units": 0, "period_end": NOW + 10_000},
                {"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 200_000_000, "used_units": 0, "period_end": NOW + 10_000},
                {"user_plan_id": "up-1", "show_name": "GLM-5.3-Flash",
                 "total_units": 5_000_000, "used_units": 2_269_745, "period_end": NOW + 60_000},
                {"user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 3_000_000, "used_units": 0, "period_end": NOW + 60_000}
            ]),
            NOW,
        );
        let state = parse_start_plan_state(&doc, NOW);
        assert_eq!(state.active_plans.len(), 2);
        assert!(state.ungranted.is_empty(), "both plans have live buckets");
        assert_eq!(state.grants.len(), 4, "no bucket may be dropped");
        assert_eq!(
            state.grants.iter().map(|g| g.allowance.total).sum::<f64>(),
            508_000_000.0
        );
        let snap = snapshot_from_doc(&doc, NOW);
        assert_eq!(snap.metrics.len(), 4);
        assert!(snap
            .metrics
            .iter()
            .any(|m| m.label == "1024 Dev Day · GLM-5.3-Flash"));
        assert!(snap
            .metrics
            .iter()
            .any(|m| m.label == "Start Plan · GLM-5.3-Flash"));
    }

    #[test]
    fn a_bucket_pairs_on_plan_id_when_only_one_side_carries_user_plan_id() {
        let doc = balance_doc(
            json!([{"plan_id": "p-1024", "name": "ZCode 1024 Dev Day",
                    "status": "active", "ends_at": NOW + 50_000,
                    "entitlements": [{"show_name": "GLM-5.3"}]}]),
            json!([{"plan_id": "p-1024", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                    "total_units": 200, "used_units": 0, "period_end": NOW + 10_000}]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert_eq!(grants.len(), 1, "same plan_id is enough to pair");
        assert_eq!(grants[0].label, "GLM-5.3");
        assert!(ungranted.is_empty(), "a paired bucket is not a held plan");
    }

    #[test]
    fn a_bucket_without_a_plan_id_is_claimed_once_not_by_every_plan() {
        // A top-up bucket that names no plan must not fan out to each plan
        // of that account: the user-id fallback attaches it to exactly one
        // fleet (an id-carrying one), and the rest stay held.
        let doc = balance_doc(
            json!([
                {"plan_id": "p-1024", "user_plan_id": "up-1", "name": "ZCode 1024 Dev Day",
                 "status": "active", "ends_at": NOW + 50_000, "entitlements": [{"show_name": "GLM-5.3"}]},
                {"plan_id": "p-weekend", "user_plan_id": "up-1", "name": "ZCode Weekend Build",
                 "status": "active", "ends_at": NOW + 60_000, "entitlements": [{"show_name": "GLM-5.3-Flash"}]}
            ]),
            json!([{"user_plan_id": "up-1", "show_name": "GLM-5.3",
                    "total_units": 200, "used_units": 0, "period_end": NOW + 10_000}]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert_eq!(grants.len(), 1, "an id-less bucket is claimed exactly once");
        assert_eq!(grants[0].plan, "1024 Dev Day", "the id-carrying fleet wins it");
        assert_eq!(ungranted.len(), 1, "the fleet that did not win it stays held");
        assert_eq!(ungranted[0], "Start Plan (Weekend Build)");
    }

    #[test]
    fn a_payload_without_plan_ids_still_pairs_by_user() {
        let doc = balance_doc(
            json!([{"user_plan_id": "up-1", "name": "ZCode Weekend Build",
                    "status": "active", "ends_at": NOW + 50_000, "entitlements": [{"show_name": "GLM-5.3"}]}]),
            json!([{"user_plan_id": "up-1", "show_name": "GLM-5.3",
                    "total_units": 200, "used_units": 0, "period_end": NOW + 10_000}]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert_eq!(grants.len(), 1);
        assert!(ungranted.is_empty());
        assert_eq!(grants[0].allowance.total, 200.0);
    }

    #[test]
    fn the_earlier_of_two_deadlines_wins() {
        // period_end in the future, expires_at already past: the grant is over.
        let doc = balance_doc(
            json!([{"plan_id": "p-1", "user_plan_id": "up-1", "name": "ZCode Weekend Build",
                    "status": "active", "ends_at": NOW + 50_000, "entitlements": [{"show_name": "GLM-5.3"}]}]),
            json!([{"plan_id": "p-1", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                    "total_units": 200, "used_units": 0,
                    "period_end": NOW + 10_000, "expires_at": NOW - 1}]),
            NOW,
        );
        let (grants, _) = parse_start_grants(&doc, NOW);
        assert!(grants.is_empty(), "the earlier deadline closed the bucket");
    }

    #[test]
    fn a_merged_window_never_keeps_a_stale_period() {
        // The later end arrives without a start: the old window's length must
        // not be attached to the new deadline.
        let doc = balance_doc(
            json!([{"plan_id": "p-1", "user_plan_id": "up-1", "name": "ZCode Weekend Build",
                    "status": "active", "ends_at": NOW + 90_000,
                    "entitlements": [{"show_name": "GLM-5.3"}]}]),
            json!([
                {"plan_id": "p-1", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 100, "used_units": 0,
                 "period_start": NOW - 5_000, "period_end": NOW + 3_000},
                {"plan_id": "p-1", "user_plan_id": "up-1", "show_name": "GLM-5.3",
                 "total_units": 100, "used_units": 0, "period_end": NOW + 9_000}
            ]),
            NOW,
        );
        let (grants, _) = parse_start_grants(&doc, NOW);
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].allowance.resets_at_ms, Some((NOW + 9_000) * 1000));
        assert_eq!(grants[0].allowance.period_ms, None, "no stale length from the earlier window");
    }

    #[test]
    fn an_empty_grant_set_renders_nothing_for_the_caller_to_invent() {
        // The snapshot supplies the "no gift" line; the row renderer must not
        // own a campaign-specific placeholder of its own.
        assert!(render_rows(&[], &[]).is_empty());
    }

    #[test]
    fn thousands_group_integers_only() {
        assert_eq!(with_thousands(115_465_592.0), "115,465,592");
        assert_eq!(with_thousands(999.0), "999");
        assert_eq!(with_thousands(0.0), "0");
        assert_eq!(with_thousands(-5.0), "0");
    }
}
