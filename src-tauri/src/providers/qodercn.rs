//! Qoder CN — the Alibaba-hosted edition of the Qoder IDE (qoder.cn).
//!
//! The IDE is an Electron app that keeps its sign-in in
//! `%APPDATA%\com.qodercn.app.stable\auth.v1.dat`, a Chromium `os_crypt`
//! v10 blob: AES-256-GCM whose key sits DPAPI-wrapped in the sibling
//! `Local State` under `os_crypt.encrypted_key`. Pane only reads — it never
//! refreshes or writes the token, so the IDE's own refreshes can't race us.
//!
//! Quota comes from the CN OpenAPI (`openapi.qoder.com.cn` — the global
//! `openapi.qoder.sh` rejects CN tokens): `/api/v2/user/plan` names the tier
//! and `/api/v2/quota/usage` reports the credit pool plus any dedicated
//! model packages (e.g. Qwen-only credits) with their own expiry.

use super::{http, Metric, Snapshot};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

const ID: &str = "qodercn";
const NAME: &str = "Qoder CN";
const APP_DIR: &str = "com.qodercn.app.stable";
const AUTH_FILE: &str = "auth.v1.dat";
const LOCAL_STATE: &str = "Local State";
const OPENAPI_BASE: &str = "https://openapi.qoder.com.cn";
const PLAN_PATH: &str = "/api/v2/user/plan";
const USAGE_PATH: &str = "/api/v2/quota/usage";

/// auth.v1.dat holds one session token; Local State is Electron's kitchen
/// sink and grows with the profile, so it gets the looser cap.
const MAX_AUTH_BYTES: u64 = 16 * 1024;
const MAX_LOCAL_STATE_BYTES: u64 = 512 * 1024;
const MAX_API_BYTES: usize = 128 * 1024;

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Pure local probe for the Customize gear panel (no network): the Qoder CN
/// app's sign-in blob exists on this machine. Regular-file check, matching
/// what the fetch path's reader will accept.
pub fn local_credential_hint() -> Option<String> {
    auth_file_path()
        .filter(|p| {
            std::fs::symlink_metadata(p)
                .ok()
                .is_some_and(|m| m.is_file() && !m.file_type().is_symlink())
        })
        .map(|_| "Qoder CN app sign-in".to_string())
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(auth_path) = auth_file_path() else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Qoder CN sign-in not found. Open Qoder CN and sign in once, then refresh.",
        ));
    };
    let token = load_token(&auth_path)?;
    let (plan, usage) = tokio::join!(
        fetch_api(&token, PLAN_PATH, "plan"),
        fetch_api(&token, USAGE_PATH, "usage")
    );
    let usage = usage?;
    parse_snapshot(plan.as_ref().ok(), &usage)
}

fn auth_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|cfg| cfg.join(APP_DIR).join(AUTH_FILE))
}

/// auth.v1.dat → session token. The AES-GCM key travels in `Local State`,
/// so a missing/corrupt Local State is the same as no sign-in for us.
fn load_token(auth_path: &Path) -> Result<String, String> {
    // Raw bytes, not text: the v10 blob is AES-GCM ciphertext and never
    // survives a UTF-8 read.
    let raw = super::read_small_bytes(auth_path, MAX_AUTH_BYTES, "auth.v1.dat")?;
    let dir = auth_path.parent().unwrap_or(Path::new("."));
    let bytes = decode_os_crypt(&raw, &os_crypt_key(dir)?)?;
    let doc: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse auth.v1.dat: {e}"))?;
    doc.get("token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "auth.v1.dat has no token".into())
}

/// `Local State` → `os_crypt.encrypted_key` → DPAPI → 32-byte AES key.
fn os_crypt_key(dir: &Path) -> Result<[u8; 32], String> {
    let raw = super::read_small_text(&dir.join(LOCAL_STATE), MAX_LOCAL_STATE_BYTES, "Local State")?;
    let wrapped = extract_wrapped_key(&raw)?;
    let key = crate::platform::dpapi_unprotect(&wrapped)
        .ok_or("DPAPI unwrap of the Qoder CN key failed")?;
    key.try_into().map_err(|_| "os_crypt key is not 32 bytes".to_string())
}

/// Local State text → the DPAPI-wrapped key bytes (JSON → base64 → strip
/// the `DPAPI` prefix). Split out so the non-Windows half is testable.
fn extract_wrapped_key(local_state: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    let doc: Value = serde_json::from_str(local_state.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("parse Local State: {e}"))?;
    let encoded = doc
        .pointer("/os_crypt/encrypted_key")
        .and_then(Value::as_str)
        .ok_or("Local State has no os_crypt.encrypted_key")?;
    let blob = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("decode encrypted_key: {e}"))?;
    blob.strip_prefix(b"DPAPI")
        .map(|b| b.to_vec())
        .ok_or_else(|| "encrypted_key is not DPAPI-wrapped".into())
}

/// Chromium `v10` blob: `v10` + 12-byte nonce + ciphertext‖tag, AES-256-GCM.
fn decode_os_crypt(raw: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
    if raw.len() < 3 + 12 + 16 {
        return Err("auth.v1.dat is truncated".into());
    }
    if &raw[..3] != b"v10" {
        return Err("auth.v1.dat is not a v10 os_crypt blob".into());
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(&raw[3..15]), &raw[15..])
        .map_err(|_| "auth.v1.dat decryption failed (profile key mismatch?)".into())
}

/// One authenticated OpenAPI GET. A rejected session token is the IDE's
/// sign-in dying — surface it as guidance instead of a raw HTTP code.
async fn fetch_api(token: &str, path: &str, what: &str) -> Result<Value, String> {
    let resp = http()
        .get(format!("{OPENAPI_BASE}{path}"))
        .bearer_auth(token)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| format!("{what} request: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err("Qoder CN session token was rejected — sign in again in Qoder CN".into());
        }
        return Err(format!("{what} endpoint: HTTP {status}"));
    }
    super::json_body(resp, MAX_API_BYTES, what).await
}

fn parse_snapshot(plan: Option<&Value>, usage: &Value) -> Result<Snapshot, String> {
    let quota = usage
        .get("userQuota")
        .ok_or("usage response has no userQuota")?;
    let total = json_f64(quota.get("total"));
    let used = json_f64(quota.get("used"));
    let (Some(total), Some(used)) = (total, used) else {
        return Err("usage response has no credit total/used".into());
    };

    // Collect package rows first: the main "Credits" row is an AGGREGATE —
    // main pool + every available package (sum used / sum total), the
    // "total credit wealth" reading the dashboard anchors on. Its reset is
    // the earliest expiry across all pools (the denominator's first change).
    // Trae CN's main Credits row is the same merged semantics.
    let main_reset = json_f64(usage.get("expiresAt")).map(epoch_ms);
    let mut pkg_used = 0.0;
    let mut pkg_total = 0.0;
    let mut earliest_reset = main_reset;
    let mut package_rows: Vec<Metric> = Vec::new();

    for package in usage
        .get("dedicatedResourcePackages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if !package.get("available").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        let (Some(p_total), Some(p_used)) =
            (json_f64(package.get("total")), json_f64(package.get("used")))
        else {
            // A package missing used/total is skipped entirely — row AND
            // aggregate — so a hidden denominator never inflates the pct.
            continue;
        };
        let label = package
            .get("displayLabels")
            .and_then(Value::as_array)
            .and_then(|labels| {
                labels
                    .iter()
                    .find(|l| l.get("dimension") == Some(&Value::from("title")))
            })
            .and_then(|l| {
                l.pointer("/valueI18n/en-US")
                    .or_else(|| l.get("value"))
                    .and_then(Value::as_str)
            })
            .unwrap_or("Dedicated credits");
        // The main pool owns the plain "Credits" label everywhere in the
        // UI (pools, layout order, overview primary) — a vendor-authored
        // package title must not shadow it. Dropped from the aggregate too:
        // it may be the main pool reported twice.
        if label == "Credits" {
            continue;
        }
        pkg_used += p_used;
        pkg_total += p_total;
        let reset = json_f64(package.get("expiresAt")).map(epoch_ms);
        if let Some(r) = reset {
            earliest_reset = Some(match earliest_reset {
                Some(cur) => cur.min(r),
                None => r,
            });
        }
        package_rows.push(credit_row(label, p_used, p_total).with_reset(reset, None));
    }

    // Paid add-on / granted credit packs live in a sibling top-level bucket
    // (verified live 2026-10-07: {total, used, remaining, percentage, unit,
    // detailUrl} — no name, no expiry of its own). Same treatment as
    // dedicated packages: into the aggregate AND its own detail row, so pack
    // credits the main pool doesn't know about stay visible instead of
    // hiding behind a maxed-out red ring.
    if let Some(add) = usage.get("addOnQuota") {
        let a_total = json_f64(add.get("total")).or_else(|| json_f64(add.get("cap")));
        let a_used = json_f64(add.get("used")).or_else(|| {
            match (a_total, json_f64(add.get("remaining"))) {
                (Some(t), Some(r)) => Some((t - r).max(0.0)),
                _ => None,
            }
        });
        if let (Some(a_total), Some(a_used)) = (a_total, a_used) {
            if a_total > 0.0 {
                pkg_used += a_used;
                pkg_total += a_total;
                package_rows.push(credit_row("Add-on credits", a_used, a_total));
            }
        }
    }

    let mut metrics = vec![credit_row("Credits", used + pkg_used, total + pkg_total)
        .with_reset(earliest_reset, None)];
    metrics.extend(package_rows);

    let plan = plan
        .and_then(|p| p.get("plan_tier_name"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

fn credit_row(label: &str, used: f64, total: f64) -> Metric {
    let pct = if total > 0.0 { (used / total * 100.0).clamp(0.0, 100.0) } else { 0.0 };
    Metric::progress(
        label,
        pct,
        Some(format!("{used:.0} of {total:.0} credits used")),
    )
}

fn json_f64(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Qoder sends epoch millis in `expiresAt`/`end_date`; tolerate seconds.
fn epoch_ms(n: f64) -> i64 {
    if n.abs() >= 1e12 {
        n as i64
    } else {
        (n * 1000.0) as i64
    }
}

// ---- Daily benefit check-in (default off; config key `qoderCheckin`) ----
//
// The growth API hands every signed-in account 100 credits per day
// (CLAIM_BENEFIT campaigns, refreshed 10:00 UTC+8). The device token Pane
// already holds works on /sash/ unchanged (verified 2026-10-07), and
// claiming an already-claimed campaign is idempotent (HTTP 200 +
// replayed:true), so a retry can never double-claim. One text metric row
// reports the state; the ledger file keeps the once-per-window rhythm.

const CAMPAIGNS_PATH: &str = "/sash/api/v1/me/campaigns";
const CHECKIN_STATE_FILE: &str = "qoder_checkin.json";
const CHECKIN_LABEL: &str = "Daily check-in";
/// After a failed attempt, wait this long before the next try so a broken
/// endpoint doesn't turn every refresh cycle into a claim hammer.
const CHECKIN_RETRY_MS: i64 = 30 * 60 * 1000;
/// "inactive" days re-check at this pace — a campaign can appear any time.
const CHECKIN_IDLE_RECHECK_MS: i64 = 6 * 60 * 60 * 1000;

/// One credential's check-in record. Pane's Qoder CN is single-credential
/// by design, so the file is one flat record, not a per-uid map.
#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct CheckinState {
    /// Beijing-day index the record belongs to (diagnostics only; the gate
    /// is `next_check_at`).
    day: i64,
    /// "claimed" | "done" | "inactive" | "failed"
    status: String,
    amount: f64,
    last_at: i64,
    #[serde(default)]
    error: String,
    /// Earliest instant (epoch ms) the next network attempt makes sense:
    /// the current claim window's end for settled states, now + idle pace
    /// for inactive, last attempt + retry for failed.
    #[serde(default)]
    next_check_at: i64,
}

/// Beijing-day index (UTC+8, no DST): rolls over at 16:00 UTC.
fn beijing_day(unix_secs: i64) -> i64 {
    (unix_secs + 8 * 3600).div_euclid(86_400)
}

/// What to do with today's campaigns list: claim these ids, or the day is
/// already settled ("done" = every benefit claimed, "inactive" = the list
/// carries no claimable-benefit campaign at all).
enum CheckinStep {
    Claim(Vec<String>),
    Settle(&'static str),
}

fn checkin_step_from_campaigns(doc: &Value) -> Result<CheckinStep, String> {
    let campaigns = doc
        .get("campaigns")
        .and_then(Value::as_array)
        .ok_or("campaigns response has no list")?;
    let mut to_claim = Vec::new();
    let mut seen_benefit = false;
    let mut all_claimed = true;
    for c in campaigns {
        if c.get("actionType").and_then(Value::as_str) != Some("CLAIM_BENEFIT") {
            continue;
        }
        seen_benefit = true;
        match c.get("claimStatus").and_then(Value::as_str) {
            Some("CLAIMABLE") => {
                all_claimed = false;
                if let Some(id) = c.get("campaignId").and_then(Value::as_str) {
                    to_claim.push(id.to_string());
                }
            }
            Some("CLAIMED") => {}
            _ => all_claimed = false,
        }
    }
    if !to_claim.is_empty() {
        return Ok(CheckinStep::Claim(to_claim));
    }
    if !seen_benefit {
        return Ok(CheckinStep::Settle("inactive"));
    }
    Ok(CheckinStep::Settle("done"))
}

/// POST claim response → (amount, replayed). Live shape is flat
/// `{status, replayed?, benefit:{amount}}`; the reference client also saw a
/// `{data:{...}}` wrapper — unwrap it when present. Anything that is not a
/// fresh CLAIMED counts as replayed so odd responses can't cause re-claim
/// spam.
fn claim_outcome(doc: &Value) -> (f64, bool) {
    let body = doc.get("data").filter(|d| d.is_object()).unwrap_or(doc);
    let amount = json_f64(body.pointer("/benefit/amount")).unwrap_or(0.0);
    let replayed = body.get("replayed").and_then(Value::as_bool).unwrap_or(false)
        || body.get("status").and_then(Value::as_str) != Some("CLAIMED");
    (amount, replayed)
}

/// When the settled state may look at the network again: the far end among
/// the benefit campaigns' windows (the next one opens right after the
/// current closes), with a fallback pace when the list is bare.
fn next_open_from_campaigns(doc: &Value) -> i64 {
    let mut ends: Vec<f64> = doc
        .get("campaigns")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|c| c.get("actionType").and_then(Value::as_str) == Some("CLAIM_BENEFIT"))
        .filter_map(|c| json_f64(c.get("endAt")))
        .collect();
    ends.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    ends.first()
        .map(|e| epoch_ms(*e) + 60_000)
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() + CHECKIN_IDLE_RECHECK_MS)
}

fn checkin_text(status: &str, amount: f64, error: &str) -> String {
    match status {
        "claimed" => format!("+{} credits claimed today", amount.max(0.0) as i64),
        "done" => "already claimed today".into(),
        "inactive" => "no check-in campaign today".into(),
        "failed" if error.is_empty() => "check-in failed".into(),
        "failed" => format!("check-in failed: {error}"),
        _ => status.to_string(),
    }
}

fn checkin_state_path() -> Option<PathBuf> {
    Some(super::config_dir().join(CHECKIN_STATE_FILE))
}

fn load_checkin_state() -> CheckinState {
    let Some(path) = checkin_state_path() else {
        return CheckinState::default();
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_checkin_state(state: &CheckinState) {
    if let Some(path) = checkin_state_path() {
        if let Ok(body) = serde_json::to_string(state) {
            let _ = crate::accounts::write_json_atomic(&path, body);
        }
    }
}

fn push_checkin_row(snap: &mut Snapshot, state: &CheckinState) {
    snap.metrics
        .push(Metric::text(CHECKIN_LABEL, checkin_text(&state.status, state.amount, &state.error)));
}

/// Runs the daily check-in for this card, appending one text row to the
/// snapshot. Call sites are the same two places long-wall mirroring runs —
/// the global refresh cycle and the manual ⟳ — so both paths agree; the
/// connectivity test never claims. Config gating happens in lib.rs (the
/// parse-tests harness has no config module); here only family and
/// snapshot health gate.
pub(crate) async fn maybe_checkin(card_id: &str, snap: &mut Snapshot) {
    let family = card_id.split('@').next().unwrap_or(card_id);
    if family != ID || snap.status != "ok" {
        return;
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut state = load_checkin_state();

    // Settled states answer from the ledger until their window reopens;
    // failures retry at most every CHECKIN_RETRY_MS.
    if state.status != "failed" && now_ms < state.next_check_at {
        push_checkin_row(snap, &state);
        return;
    }
    if state.status == "failed" && now_ms - state.last_at < CHECKIN_RETRY_MS {
        push_checkin_row(snap, &state);
        return;
    }

    let Some(auth_path) = auth_file_path() else {
        return;
    };
    let Ok(token) = load_token(&auth_path) else {
        return;
    };

    let (status, amount, next_check_at, error) = match run_checkin(&token).await {
        Ok((s, a, n)) => (s, a, n, String::new()),
        Err(e) => (
            "failed".to_string(),
            0.0,
            now_ms + CHECKIN_RETRY_MS,
            e,
        ),
    };
    state = CheckinState {
        day: beijing_day(chrono::Utc::now().timestamp()),
        status,
        amount,
        last_at: now_ms,
        error,
        next_check_at,
    };
    save_checkin_state(&state);
    push_checkin_row(snap, &state);
}

async fn run_checkin(token: &str) -> Result<(String, f64, i64), String> {
    let doc = checkin_request(token, CAMPAIGNS_PATH, false).await?;
    let next_open = next_open_from_campaigns(&doc);
    match checkin_step_from_campaigns(&doc)? {
        CheckinStep::Settle(s) => Ok((s.to_string(), 0.0, next_open)),
        CheckinStep::Claim(ids) => {
            let mut amount = 0.0;
            let mut fresh = false;
            for id in ids {
                let out = checkin_request(token, &format!("{CAMPAIGNS_PATH}/{id}/claim"), true)
                    .await?;
                let (a, replayed) = claim_outcome(&out);
                amount += a;
                fresh |= !replayed;
            }
            Ok((
                if fresh { "claimed" } else { "done" }.to_string(),
                amount,
                next_open,
            ))
        }
    }
}

/// One /sash/ call with the header set the growth API expects
/// (Cosy-ClientType + a plain "Qoder" UA, same as the reference client).
async fn checkin_request(token: &str, path: &str, post: bool) -> Result<Value, String> {
    let mut req = if post {
        http()
            .post(format!("{OPENAPI_BASE}{path}"))
            .header("Content-Type", "application/json")
            .body("{}".to_string())
    } else {
        http().get(format!("{OPENAPI_BASE}{path}"))
    };
    req = req
        .bearer_auth(token)
        .header("Accept", "application/json")
        .header("Cosy-ClientType", "10")
        .header("User-Agent", "Qoder");
    let resp = req
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| format!("check-in request: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err("session token rejected — sign in again in Qoder CN".into());
        }
        return Err(format!("check-in endpoint: HTTP {status}"));
    }
    super::json_body(resp, MAX_API_BYTES, "check-in").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes_gcm::aead::Aead;
    use serde_json::json;

    fn os_crypt_blob(key: &[u8; 32], plain: &[u8]) -> Vec<u8> {
        use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let nonce = [7u8; 12];
        let mut out = b"v10".to_vec();
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&cipher.encrypt(Nonce::from_slice(&nonce), plain).unwrap());
        out
    }

    #[test]
    fn decodes_a_v10_os_crypt_roundtrip() {
        let key = [42u8; 32];
        let plain = br#"{"token":"sess-token"}"#;
        let blob = os_crypt_blob(&key, plain);
        let decoded = decode_os_crypt(&blob, &key).expect("decrypt");
        assert_eq!(decoded, plain);
    }

    #[test]
    fn rejects_short_or_wrong_prefix_blobs() {
        let key = [0u8; 32];
        assert!(decode_os_crypt(b"v1", &key).is_err());
        // 33 bytes (> the 31-byte minimum) so the length gate passes and
        // the non-v10 prefix is what rejects it.
        assert!(decode_os_crypt(b"v20abcdefghijklmnopqrstuvwxyz0123", &key).is_err());
    }

    #[test]
    fn wrong_key_fails_the_gcm_tag() {
        let blob = os_crypt_blob(&[1u8; 32], b"{}");
        assert!(decode_os_crypt(&blob, &[2u8; 32]).is_err());
    }

    #[test]
    fn binary_credential_blob_survives_the_file_read() {
        // auth.v1.dat is raw AES-GCM ciphertext — never valid UTF-8. The
        // byte-level reader must round-trip it and the text reader must
        // refuse it (a regression guard for reading via read_to_string).
        let dir = std::env::temp_dir().join(format!("pane-qoder-bin-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("auth.v1.dat");
        let key = [7u8; 32];
        let blob = os_crypt_blob(&key, br#"{"token":"t"}"#);
        std::fs::write(&path, &blob).unwrap();
        let read_back = super::super::read_small_bytes(&path, MAX_AUTH_BYTES, "auth.v1.dat");
        assert_eq!(read_back.expect("binary read"), blob);
        assert!(super::super::read_small_text(&path, MAX_AUTH_BYTES, "auth.v1.dat").is_err());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn wrapped_key_extraction_strips_dpapi_prefix() {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD
            .encode([b'D', b'P', b'A', b'P', b'I', 1u8, 2, 3]);
        let local_state = format!(r#"{{"os_crypt":{{"encrypted_key":"{b64}"}}}}"#);
        assert_eq!(extract_wrapped_key(&local_state).unwrap(), vec![1, 2, 3]);
        // A BOM in front of the JSON is tolerated (Electron writes one).
        assert_eq!(
            extract_wrapped_key(&format!("\u{feff}{local_state}")).unwrap(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn wrapped_key_extraction_rejects_malformed_state() {
        assert!(extract_wrapped_key("{}").is_err()); // no os_crypt
        assert!(extract_wrapped_key("not json").is_err());
        // Wrong prefix: base64 of "NOPE..." rather than "DPAPI...".
        assert!(extract_wrapped_key(r#"{"os_crypt":{"encrypted_key":"Tk9QRQ=="}}"#).is_err());
    }

    #[test]
    fn parses_the_real_usage_shape() {
        let usage = json!({
            "userId": "01a08a64-2bf5-7224-9561-2ff9b9f13ece",
            "usageType": "credits",
            "expiresAt": 1791648000000i64,
            "userQuota": {"total": 2000.0, "used": 5.0, "remaining": 1995.0, "unit": "credits"},
            "dedicatedResourcePackages": [
                {
                    "name": "act-20260901-170",
                    "total": 2000.0, "used": 5.0, "remaining": 1995.0,
                    "expiresAt": 1791620214801i64,
                    "available": true,
                    "displayLabels": [
                        {"dimension": "description", "value": "qwen model series description"},
                        {"dimension": "title", "value": "qwen model series",
                         "valueI18n": {"en-US": "Qwen Exclusive Credits", "zh-CN": "Qwen 专属积分"}}
                    ]
                },
                {"total": 100.0, "used": 0.0, "available": false}
            ]
        });
        let plan = json!({"plan_tier_name": "Pro", "is_paid_plan": true, "end_date": 1791648000000i64});
        let snap = parse_snapshot(Some(&plan), &usage).expect("parse");
        assert_eq!(snap.id, "qodercn");
        assert_eq!(snap.plan.as_deref(), Some("Pro"));
        assert_eq!(snap.status, "ok");
        // Main pool + the one available package; the unavailable one is skipped.
        assert_eq!(snap.metrics.len(), 2);
        assert_eq!(snap.metrics[0].label, "Credits");
        // Main row is the aggregate: (5+5)/(2000+2000) — numerically the
        // same 0.25 here by coincidence, detail and reset prove the merge.
        assert!((snap.metrics[0].used_percent.unwrap() - 0.25).abs() < 0.001);
        assert_eq!(snap.metrics[0].detail.as_deref(), Some("10 of 4000 credits used"));
        // Aggregate reset = earliest expiry across pools (package 10-10
        // beats the main pool's monthly 10-11).
        assert_eq!(snap.metrics[0].resets_at, Some(1791620214801));
        assert_eq!(snap.metrics[1].label, "Qwen Exclusive Credits");
        assert!(snap.metrics[1].detail.as_deref().unwrap().contains("5 of 2000"));
        assert_eq!(snap.metrics[1].resets_at, Some(1791620214801));
    }

    #[test]
    fn missing_plan_still_meters() {
        let usage = json!({
            "userQuota": {"total": 2000.0, "used": 0.0},
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        assert_eq!(snap.plan, None);
        assert_eq!(snap.metrics.len(), 1);
        assert!((snap.metrics[0].used_percent.unwrap() - 0.0).abs() < 0.001);
    }

    #[test]
    fn usage_without_quota_is_an_error() {
        assert!(parse_snapshot(None, &json!({})).is_err());
        assert!(parse_snapshot(None, &json!({"userQuota": {"total": 2000.0}})).is_err());
    }

    #[test]
    fn package_without_i18n_gets_a_generic_label() {
        let usage = json!({
            "userQuota": {"total": 100.0, "used": 0.0},
            "dedicatedResourcePackages": [{"total": 50.0, "used": 10.0, "available": true}]
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        assert_eq!(snap.metrics[1].label, "Dedicated credits");
        // Aggregate main row: (0+10)/(100+50).
        assert!((snap.metrics[0].used_percent.unwrap() - 6.667).abs() < 0.01);
        assert_eq!(snap.metrics[0].detail.as_deref(), Some("10 of 150 credits used"));
    }

    #[test]
    fn package_titled_credits_cannot_shadow_the_main_pool() {
        let usage = json!({
            "userQuota": {"total": 100.0, "used": 0.0},
            "dedicatedResourcePackages": [{
                "total": 50.0, "used": 10.0, "available": true,
                "displayLabels": [
                    {"dimension": "title", "value": "credits",
                     "valueI18n": {"en-US": "Credits"}}
                ]
            }]
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        // The package row is dropped rather than colliding with the main
        // pool's label (pool/maxed/layout engines are label-keyed) — and
        // it stays OUT of the aggregate: it may be the main pool reported
        // twice, so 0/100 (not 10/150) locks the no-double-count rule.
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].label, "Credits");
        assert_eq!(snap.metrics[0].detail.as_deref(), Some("0 of 100 credits used"));
    }

    #[test]
    fn aggregate_skips_unavailable_packages() {
        let usage = json!({
            "userQuota": {"total": 100.0, "used": 10.0},
            "dedicatedResourcePackages": [
                {"total": 100.0, "used": 90.0, "available": false,
                 "displayLabels": [{"dimension": "title", "value": "gone"}]},
                {"total": 100.0, "used": 30.0, "available": true,
                 "displayLabels": [{"dimension": "title", "value": "live"}]}
            ]
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        // Only the available package joins: (10+30)/(100+100) = 20%.
        assert_eq!(snap.metrics.len(), 2);
        assert!((snap.metrics[0].used_percent.unwrap() - 20.0).abs() < 0.001);
        assert_eq!(snap.metrics[1].label, "live");
    }

    #[test]
    fn aggregate_reset_takes_the_earliest_expiry() {
        // Package expires before the main pool's monthly reset. (Values are
        // real epoch-ms magnitudes — epoch_ms promotes second-scale numbers.)
        let early = json!({
            "expiresAt": 1791648000000.0,
            "userQuota": {"total": 100.0, "used": 0.0},
            "dedicatedResourcePackages": [
                {"total": 100.0, "used": 0.0, "expiresAt": 1791620214801.0, "available": true,
                 "displayLabels": [{"dimension": "title", "value": "p"}]}]
        });
        assert_eq!(
            parse_snapshot(None, &early).unwrap().metrics[0].resets_at,
            Some(1791620214801)
        );
        // And the reverse order — the main pool's earlier expiry wins.
        let late = json!({
            "expiresAt": 1791620214801.0,
            "userQuota": {"total": 100.0, "used": 0.0},
            "dedicatedResourcePackages": [
                {"total": 100.0, "used": 0.0, "expiresAt": 1791648000000.0, "available": true,
                 "displayLabels": [{"dimension": "title", "value": "p"}]}]
        });
        assert_eq!(
            parse_snapshot(None, &late).unwrap().metrics[0].resets_at,
            Some(1791620214801)
        );
    }

    #[test]
    fn package_missing_fields_stays_out_of_the_aggregate() {
        let usage = json!({
            "userQuota": {"total": 100.0, "used": 0.0},
            "dedicatedResourcePackages": [
                // No `used` — row skipped AND denominator untouched.
                {"total": 50.0, "available": true,
                 "displayLabels": [{"dimension": "title", "value": "p1"}]},
                // No `total` — same treatment.
                {"used": 20.0, "available": true,
                 "displayLabels": [{"dimension": "title", "value": "p2"}]}]
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].detail.as_deref(), Some("0 of 100 credits used"));
    }

    #[test]
    fn all_zero_totals_do_not_panic() {
        let usage = json!({
            "userQuota": {"total": 0.0, "used": 0.0},
            "dedicatedResourcePackages": [
                {"total": 0.0, "used": 0.0, "available": true,
                 "displayLabels": [{"dimension": "title", "value": "p"}]}]
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        assert_eq!(snap.metrics[0].used_percent, Some(0.0));
    }

    #[test]
    fn epoch_seconds_are_promoted_to_millis() {
        assert_eq!(epoch_ms(1_789_102_821.0), 1_789_102_821_000);
        assert_eq!(epoch_ms(1_791_648_000_000.0), 1_791_648_000_000);
    }

    #[test]
    fn addon_bucket_merges_into_aggregate_with_its_own_row() {
        // Live-verified shape (2026-10-07): flat bucket, no name/expiry.
        let usage = json!({
            "userQuota": {"total": 2000.0, "used": 2000.0},
            "addOnQuota": {"total": 500.0, "used": 500.0, "remaining": 0.0, "unit": "credits"}
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        assert_eq!(snap.metrics.len(), 2);
        assert_eq!(snap.metrics[0].label, "Credits");
        assert_eq!(snap.metrics[0].used_percent, Some(100.0));
        assert_eq!(snap.metrics[1].label, "Add-on credits");
        assert_eq!(snap.metrics[1].used_percent, Some(100.0));
    }

    #[test]
    fn addon_bucket_absent_changes_nothing() {
        let usage = json!({"userQuota": {"total": 100.0, "used": 40.0}});
        let snap = parse_snapshot(None, &usage).expect("parse");
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].used_percent, Some(40.0));
    }

    #[test]
    fn addon_zero_total_is_skipped() {
        let usage = json!({
            "userQuota": {"total": 100.0, "used": 40.0},
            "addOnQuota": {"total": 0.0, "used": 0.0}
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        assert_eq!(snap.metrics.len(), 1);
    }

    #[test]
    fn addon_used_falls_back_to_total_minus_remaining() {
        let usage = json!({
            "userQuota": {"total": 100.0, "used": 10.0},
            "addOnQuota": {"total": 50.0, "remaining": 30.0}
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        assert_eq!(snap.metrics.len(), 2);
        // used falls back to 50-30=20; aggregate (10+20)/(100+50) = 20%,
        // the add-on row alone is 40%.
        assert_eq!(snap.metrics[0].used_percent, Some(20.0));
        assert_eq!(snap.metrics[1].used_percent, Some(40.0));
    }

    #[test]
    fn checkin_claimable_campaign_yields_claim() {
        let doc = json!({"claimable": true, "campaigns": [
            {"campaignId": "c1", "actionType": "CLAIM_BENEFIT",
             "claimStatus": "CLAIMABLE", "endAt": 1791424740},
            {"campaignId": "c2", "actionType": "VIEW_DETAILS", "claimStatus": "CLAIMED"}
        ]});
        match checkin_step_from_campaigns(&doc).expect("step") {
            CheckinStep::Claim(ids) => assert_eq!(ids, vec!["c1".to_string()]),
            CheckinStep::Settle(s) => panic!("expected a claim step, got settle {s}"),
        }
    }

    #[test]
    fn checkin_all_claimed_settles_done() {
        let doc = json!({"claimable": false, "campaigns": [
            {"campaignId": "c1", "actionType": "CLAIM_BENEFIT", "claimStatus": "CLAIMED"}
        ]});
        match checkin_step_from_campaigns(&doc).expect("step") {
            CheckinStep::Settle("done") => {}
            _ => panic!("expected done"),
        }
    }

    #[test]
    fn checkin_without_benefit_campaign_is_inactive() {
        let doc = json!({"campaigns": [
            {"campaignId": "c2", "actionType": "VIEW_DETAILS", "claimStatus": "CLAIMED"}
        ]});
        match checkin_step_from_campaigns(&doc).expect("step") {
            CheckinStep::Settle("inactive") => {}
            _ => panic!("expected inactive"),
        }
    }

    #[test]
    fn claim_response_replayed_means_done() {
        // Fresh claim: status CLAIMED, no replay flag.
        assert_eq!(claim_outcome(&json!({"status": "CLAIMED", "benefit": {"amount": 100}})), (100.0, false));
        // Idempotent replay (verified live): replayed:true, still 200.
        assert_eq!(claim_outcome(&json!({"status": "CLAIMED", "replayed": true, "benefit": {"amount": 100}})), (100.0, true));
        // The {data:{...}} wrapper some clients see unwraps fine.
        assert_eq!(
            claim_outcome(&json!({"data": {"status": "CLAIMED", "replayed": true, "benefit": {"amount": 100}}})),
            (100.0, true)
        );
        // An unrecognized status counts as replayed — never re-claim spam.
        assert_eq!(claim_outcome(&json!({"status": "WEIRD"})), (0.0, true));
    }

    #[test]
    fn checkin_next_open_follows_the_window_end() {
        let doc = json!({"campaigns": [
            {"campaignId": "c1", "actionType": "CLAIM_BENEFIT", "endAt": 1791424740}
        ]});
        assert_eq!(next_open_from_campaigns(&doc), 1_791_424_740_000 + 60_000);
        // Bare list → idle pace from now (no panic, sane fallback).
        assert!(next_open_from_campaigns(&json!({"campaigns": []})) > 0);
    }

    #[test]
    fn beijing_day_rolls_at_sixteen_utc() {
        // 57599 = 15:59:59 UTC → still the previous Beijing day; 57600 =
        // 16:00:00 UTC = Beijing midnight → next day.
        assert_eq!(beijing_day(57_599), 0);
        assert_eq!(beijing_day(57_600), 1);
    }

    #[test]
    fn checkin_text_covers_the_four_states() {
        assert_eq!(checkin_text("claimed", 100.0, ""), "+100 credits claimed today");
        assert_eq!(checkin_text("done", 0.0, ""), "already claimed today");
        assert_eq!(checkin_text("inactive", 0.0, ""), "no check-in campaign today");
        assert_eq!(checkin_text("failed", 0.0, "HTTP 503"), "check-in failed: HTTP 503");
    }
}
