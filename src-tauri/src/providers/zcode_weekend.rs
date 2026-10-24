//! ZCode (Zhipu's desktop client) hands its signed-in accounts a series of
//! token gifts: weekend builds, the 1024 Dev Day grant, and the model-scoped
//! trials that ship with new models (GLM-5.3, GLM-5.3-Flash, …). They all
//! belong to the **ZCode Start Plan**, i.e. the account session — a different
//! credential than the Z.ai API key this card is built around. The session JWT
//! is stored AES-256-GCM-encrypted in ~/.zcode/v2/credentials.json and the
//! balance is served by zcode.z.ai/api/v1/zcode-plan/billing/balance, whose
//! `plans[].entitlements[]` and `balances[]` describe every grant as a bucket
//! with its own `show_name`, total/used units and window.
//!
//! Each granted bucket is its own row on the Z.ai card (2026-10-24 amendment:
//! the earlier weekend-only filter hid every other gift). A plan held without a
//! bucket renders a claim line; a session with no grant renders the historical
//! "Weekend not started" row; a machine without a local session shows no row at
//! all. A failed request holds this run's last verdict so rows never flicker
//! (`.scratch/zcode-weekend/spec.md`).
//!
//! Crypto and endpoint semantics mirror magpie's Go implementation
//! (zcode.go zcodeSecret/zcodeDecrypt, zcode_start.go zcodeStartBalance):
//! `ends_at` and the period bounds are unix SECONDS, numbers-or-strings;
//! a plan still "active" past its `ends_at` is over, and its buckets are
//! dropped. The header contract is asymmetric: the endpoint refuses a
//! request without X-Device-Mid (400, code 3001) yet only checks that the
//! header exists, so Pane keeps a UUID of its own, made once beside
//! config.json — never ZCode's, never magpie's.

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
const LABEL: &str = "ZCode Weekend";
/// Canonical English caption for the no-live-grant placeholder; the
/// frontend translates it at paint time (detail.weekendNotStarted).
const NOT_STARTED: &str = "Weekend not started";

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

/// Every live gift row for the Z.ai card.
///
/// The Start Plan is the **ZCode session's own plan** (the JWT in
/// `~/.zcode/v2/credentials.json`), not the Z.ai API key. Its `plans[]` carry
/// `entitlements[]` (model-scoped grants such as `GLM-5.3` 2亿 /
/// `GLM-5.3-Flash` 3亿, weekend builds, 1024 Dev Day) and `balances[]` mirror
/// them as buckets. So each granted bucket is a row — the old weekend-only
/// filter hid every other gift.
///
/// `None` when this machine has no local ZCode session: the card shows nothing
/// rather than a lying placeholder. A session with no grant at all renders the
/// established "Weekend not started" row.
pub async fn start_plan_rows() -> Option<Vec<super::Metric>> {
    let jwt = read_local_jwt()?;
    let now_s = chrono::Utc::now().timestamp();
    // An expired session is a logged-out machine as far as these rows are
    // concerned; skip the dead call until the user signs in again.
    if jwt_expired(&jwt, now_s) {
        return None;
    }
    match fetch_balance(&jwt, &device_mid()).await {
        Some(doc) => {
            let (grants, ungranted) = parse_start_grants(&doc, now_s);
            let rows = render_rows(&grants, &ungranted);
            store_last(&rows);
            Some(rows)
        }
        // Transient failure: hold this run's last verdict so rows do not
        // flicker on a blip; a cold-start miss shows nothing this cycle.
        None => cached_last(),
    }
}

fn last_state() -> &'static std::sync::Mutex<Option<Vec<super::Metric>>> {
    static LAST: std::sync::OnceLock<std::sync::Mutex<Option<Vec<super::Metric>>>> =
        std::sync::OnceLock::new();
    LAST.get_or_init(|| std::sync::Mutex::new(None))
}

fn store_last(rows: &[super::Metric]) {
    if let Ok(mut slot) = last_state().lock() {
        *slot = Some(rows.to_vec());
    }
}

fn cached_last() -> Option<Vec<super::Metric>> {
    last_state().lock().ok().and_then(|slot| slot.clone())
}

/// The card rows: one usage bar per granted bucket, then one claim line per
/// plan that exists without a granted bucket. A session with nothing at all
/// keeps the weekend placeholder, which is the copy this row has always used.
pub fn render_rows(grants: &[StartGrant], ungranted: &[String]) -> Vec<super::Metric> {
    if grants.is_empty() && ungranted.is_empty() {
        return vec![super::Metric::text(LABEL, NOT_STARTED.into())];
    }
    let mut rows: Vec<super::Metric> = grants.iter().map(grant_metric).collect();
    for plan in ungranted {
        rows.push(super::Metric::text(plan, CLAIM_HINT.into()));
    }
    rows.truncate(MAX_ROWS);
    rows
}

/// One gift bucket: usage bar + used/total detail + its own window. The label
/// is the bucket's `show_name` (GLM-5.3, GLM-5.3-Flash …), never a generic
/// "ZCode Weekend" that hides which model the grant belongs to.
fn grant_metric(grant: &StartGrant) -> super::Metric {
    let w = &grant.allowance;
    let percent = if w.total > 0.0 {
        (w.used / w.total * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    super::Metric::progress(
        &grant.label,
        percent,
        Some(format!(
            "{} of {} used · {}",
            with_thousands(w.used),
            with_thousands(w.total),
            grant.plan
        )),
    )
    .with_reset(w.resets_at_ms, w.period_ms)
}

/// Split a balance response into granted buckets and plans held without one.
/// A plan past its `ends_at` is over even while it still reports `active`
/// (measured 2026-10-12), and its buckets go with it. Bucket ids repeat across
/// users, so `user_plan_id` decides the pairing when both sides carry one.
pub fn parse_start_grants(doc: &Value, fallback_now_s: i64) -> (Vec<StartGrant>, Vec<String>) {
    let mut grants: Vec<StartGrant> = Vec::new();
    let mut ungranted: Vec<String> = Vec::new();
    if doc.get("code").and_then(Value::as_i64) != Some(0) {
        return (grants, ungranted);
    }
    let Some(data) = doc.get("data") else {
        return (grants, ungranted);
    };
    let now_s = num(data.get("server_time"))
        .filter(|t| *t > 0.0)
        .unwrap_or(fallback_now_s as f64) as i64;
    let (Some(plans), Some(balances)) = (
        data.get("plans").and_then(Value::as_array),
        data.get("balances").and_then(Value::as_array),
    ) else {
        return (grants, ungranted);
    };
    let text_field = |v: &Value, key: &str| -> String {
        v.get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string()
    };
    for plan in plans {
        if plan_expired(plan, now_s) {
            continue;
        }
        let plan_name = text_field(plan, "name");
        let plan_user = text_field(plan, "user_plan_id");
        let plan_id = text_field(plan, "plan_id");
        let mut plan_rows: Vec<StartGrant> = Vec::new();
        for bucket in balances {
            if bucket_expired(bucket, now_s) {
                continue;
            }
            if !bucket_matches(
                &plan_user,
                &plan_id,
                &text_field(bucket, "user_plan_id"),
                &text_field(bucket, "plan_id"),
            ) {
                continue;
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
                } else if plan_name.is_empty() {
                    LABEL.to_string()
                } else {
                    plan_name.clone()
                }
            };
            let end = num(bucket.get("period_end"))
                .or_else(|| num(bucket.get("expires_at")))
                .filter(|t| *t > 0.0);
            let start = num(bucket.get("period_start")).filter(|t| *t > 0.0);
            // Buckets that share a name (top-ups, split windows) sum into one
            // row and keep the latest end, exactly like the weekend row did.
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
                plan: plan_name.clone(),
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
        if plan_rows.is_empty() {
            if !plan_name.is_empty() && !ungranted.contains(&plan_name) {
                ungranted.push(plan_name);
            }
        } else {
            grants.extend(plan_rows);
        }
    }
    // Soonest deadline first; a grant without a window sorts last.
    grants.sort_by(|a, b| {
        let key = |g: &StartGrant| g.allowance.resets_at_ms.unwrap_or(i64::MAX);
        key(a).cmp(&key(b)).then_with(|| {
            (a.plan.as_str(), a.label.as_str()).cmp(&(b.plan.as_str(), b.label.as_str()))
        })
    });
    ungranted.sort();
    grants.truncate(MAX_ROWS);
    ungranted.truncate(MAX_ROWS);
    (grants, ungranted)
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
    let deadline = num(bucket.get("period_end"))
        .or_else(|| num(bucket.get("expires_at")))
        .unwrap_or(0.0) as i64;
    deadline > 0 && deadline <= now_s
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

/// True when a balance bucket belongs to a plan. `user_plan_id` decides
/// when both sides carry one (plan ids repeat across users), plan id
/// otherwise — magpie's pairing.
fn bucket_matches(plan_user: &str, plan_id: &str, bucket_user: &str, bucket_id: &str) -> bool {
    if !plan_user.is_empty() && !bucket_user.is_empty() {
        return plan_user == bucket_user;
    }
    if plan_user.is_empty() || bucket_user.is_empty() {
        return plan_id == bucket_id;
    }
    false
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
        eprintln!("code={:?}", doc.get("code"));
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
                    plan.get("entitlements").map(|v| v.to_string().chars().take(400).collect::<String>())
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
            Some("50,000,000 of 200,000,000 used · ZCode 1024 Dev Day")
        );
        assert_eq!(rows[0].resets_at, Some((NOW + 10_000) * 1000));
        assert_eq!(rows[1].label, "GLM-5.3-Flash");
        assert_eq!(rows[1].used_percent, Some(0.0));
        assert_eq!(rows[1].detail.as_deref(), Some("0 of 300,000,000 used · ZCode 1024 Dev Day"));
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
        assert_eq!(grants[0].label, "ZCode Weekend Build");
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
        let rows = render_rows(&grants, &ungranted);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value.as_deref(), Some(NOT_STARTED));
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
        assert_eq!(ungranted, vec!["ZCode Weekend Build".to_string()], "still worth claiming");
    }

    #[test]
    fn a_plan_held_without_a_bucket_renders_a_claim_line() {
        let doc = balance_doc(
            json!([{"plan_id": "p-1024", "user_plan_id": "up-1", "name": "ZCode 1024 Dev Day",
                    "status": "active", "ends_at": NOW + 50_000}]),
            json!([]),
            NOW,
        );
        let (grants, ungranted) = parse_start_grants(&doc, NOW);
        assert!(grants.is_empty());
        assert_eq!(ungranted, vec!["ZCode 1024 Dev Day".to_string()]);
        let rows = render_rows(&grants, &ungranted);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "text");
        assert_eq!(rows[0].label, "ZCode 1024 Dev Day");
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
        assert_eq!(grants.len(), 2, "both plans contribute their own rows");
        let labels: Vec<&str> = grants.iter().map(|g| g.label.as_str()).collect();
        assert!(labels.contains(&"GLM-5.3"));
        assert!(labels.contains(&"Coding Plan"));
        let gift = grants.iter().find(|g| g.label == "GLM-5.3").unwrap();
        assert_eq!(gift.allowance.total, 200.0, "the coding plan's 9M must not inflate the gift");
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
    fn a_session_with_no_grant_keeps_the_weekend_placeholder() {
        let rows = render_rows(&[], &[]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "text");
        assert_eq!(rows[0].label, "ZCode Weekend");
        assert_eq!(rows[0].value.as_deref(), Some("Weekend not started"));
        assert_eq!(rows[0].used_percent, None);
    }

    #[test]
    fn thousands_group_integers_only() {
        assert_eq!(with_thousands(115_465_592.0), "115,465,592");
        assert_eq!(with_thousands(999.0), "999");
        assert_eq!(with_thousands(0.0), "0");
        assert_eq!(with_thousands(-5.0), "0");
    }
}
