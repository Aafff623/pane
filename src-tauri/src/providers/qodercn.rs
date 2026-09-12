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

    let resets_at = json_f64(usage.get("expiresAt")).map(epoch_ms);
    let mut metrics = vec![credit_row("Credits", used, total).with_reset(resets_at, None)];

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
        // package title must not shadow it.
        if label == "Credits" {
            continue;
        }
        let reset = json_f64(package.get("expiresAt")).map(epoch_ms);
        metrics.push(credit_row(label, p_used, p_total).with_reset(reset, None));
    }

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
        assert!((snap.metrics[0].used_percent.unwrap() - 0.25).abs() < 0.001);
        assert_eq!(snap.metrics[0].resets_at, Some(1791648000000));
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
        // pool's label (pool/maxed/layout engines are label-keyed).
        assert_eq!(snap.metrics.len(), 1);
        assert_eq!(snap.metrics[0].label, "Credits");
    }

    #[test]
    fn epoch_seconds_are_promoted_to_millis() {
        assert_eq!(epoch_ms(1_789_102_821.0), 1_789_102_821_000);
        assert_eq!(epoch_ms(1_791_648_000_000.0), 1_791_648_000_000);
    }
}
