//! Trae CN — the China edition of ByteDance's Trae IDE (trae.cn, internally
//! "icube").
//!
//! The IDE keeps its sign-in in
//! `%APPDATA%\Trae CN\User\globalStorage\storage.json` under
//! `iCubeAuthInfo://icube.cloudide`, value-encrypted with "ByteCrypto":
//! base64 of `magic(6) ‖ random key(32) ‖ AES-128-CBC(SHA512-derived key+iv,
//! SHA512(plain) ‖ plain)`. The random key ships inside the blob and the
//! salt is a pair of compile-time constants, so the value decrypts locally
//! without any machine binding. Pane only reads — Trae rotates the refresh
//! token while running, and racing it signs the IDE out, so an expired
//! token here means "open Trae CN once", not a refresh attempt.
//!
//! Credits come from `POST {host}/trae/api/v2/pay/ide_user_ent_usage`
//! (host is stored beside the token; CN = `https://api.trae.cn`) with a
//! `Cloud-IDE-JWT` authorization header. `usage_summary` aggregates every
//! credit pack; the pay-status call beside it only names the tier (Free/Pro).

use super::{http, Metric, Snapshot};
use serde_json::Value;
use std::time::Duration;

const ID: &str = "traecn";
const NAME: &str = "Trae CN";
const APP_DIR: &str = "Trae CN";
const AUTH_KEY: &str = "iCubeAuthInfo://icube.cloudide";
const DEFAULT_HOST: &str = "https://api.trae.cn";
const ENT_USAGE_PATH: &str = "/trae/api/v2/pay/ide_user_ent_usage";
const PAY_STATUS_PATH: &str = "/trae/api/v2/pay/ide_user_pay_status";

const MAX_STORAGE_BYTES: u64 = 1024 * 1024; // grows with the profile/plugin count
const MAX_API_BYTES: usize = 128 * 1024;

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Pure local probe for the Customize gear panel (no network): the Trae CN
/// app's storage.json carries an encrypted sign-in blob.
pub fn local_credential_hint() -> Option<String> {
    storage_path()
        .and_then(|p| {
            super::read_small_text(&p, MAX_STORAGE_BYTES, "storage.json").ok()
        })
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .is_some_and(|doc| doc.get(AUTH_KEY).is_some())
        .then(|| "Trae CN app sign-in".to_string())
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(path) = storage_path() else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Trae CN sign-in not found. Open Trae CN and sign in once, then refresh.",
        ));
    };
    let raw = super::read_small_text(&path, MAX_STORAGE_BYTES, "storage.json")?;
    let doc: Value =
        serde_json::from_str(raw.trim_start_matches('\u{feff}')).map_err(|e| format!("parse storage.json: {e}"))?;
    let encrypted = doc
        .get(AUTH_KEY)
        .and_then(Value::as_str)
        .ok_or_else(|| "storage.json has no Trae sign-in blob".to_string())?;
    let auth = decode_auth_blob(encrypted)?;
    let token = auth
        .get("token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "Trae sign-in blob has no token".to_string())?
        .to_string();
    let host = auth
        .get("host")
        .and_then(Value::as_str)
        .filter(|h| h.starts_with("https://"))
        .unwrap_or(DEFAULT_HOST)
        .to_string();

    let (usage, pay_status) = tokio::join!(
        fetch_json(&host, ENT_USAGE_PATH, "usage endpoint", &token),
        fetch_json(&host, PAY_STATUS_PATH, "pay-status endpoint", &token)
    );
    let usage = usage?;
    let plan = pay_status
        .ok()
        .and_then(|p| {
            p.get("user_pay_identity_str")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    parse_snapshot(plan.as_deref(), &usage)
}

fn storage_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|cfg| {
        cfg.join(APP_DIR)
            .join("User")
            .join("globalStorage")
            .join("storage.json")
    })
}

async fn fetch_json(host: &str, path: &str, what: &str, token: &str) -> Result<Value, String> {
    let url = format!("{host}{path}");
    let resp = http()
        .post(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Cloud-IDE-JWT {token}"))
        .body("{\"require_usage\":true}")
        .timeout(Duration::from_secs(8))
        .send()
        .await
        .map_err(|e| format!("request {what}: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err("Trae CN session token was rejected — open Trae CN once to refresh it".into());
        }
        return Err(format!("{what}: HTTP {status}"));
    }
    super::json_body(resp, MAX_API_BYTES, what).await
}

fn parse_snapshot(plan: Option<&str>, usage: &Value) -> Result<Snapshot, String> {
    let summary = usage
        .pointer("/usage_summary")
        .ok_or("usage response has no usage_summary")?;
    let consumed = json_f64(summary.get("consumed_amount"));
    let total = json_f64(summary.get("total_amount"));
    let (Some(consumed), Some(total)) = (consumed, total) else {
        return Err("usage_summary has no credit totals".into());
    };

    let packs = usage
        .get("user_entitlement_pack_list")
        .and_then(Value::as_array)
        .map(|a| a.len())
        .unwrap_or(0);
    let mut detail = format!("{consumed:.2} of {total:.0} credits used");
    if packs > 1 {
        detail.push_str(&format!(" · {packs} packs"));
    }

    let pct = if total > 0.0 { (consumed / total * 100.0).clamp(0.0, 100.0) } else { 0.0 };
    let metrics = vec![Metric::progress("Credits", pct, Some(detail))];
    Ok(Snapshot::ok(ID, NAME, plan.map(str::to_string), metrics))
}

// ---- ByteCrypto -------------------------------------------------------------

const SHA512_LEN: usize = 64;
const RANDOM_KEY_LEN: usize = 32;
const MAGIC_AES: [u8; 6] = [116, 99, 5, 16, 0, 0];
const MAGIC_AES_PRIVATE: [u8; 6] = [18, 57, 32, 32, 2, 3];
const SALT_A: [u8; SHA512_LEN] = [
    82, 9, 106, 213, 48, 54, 165, 56, 191, 64, 163, 158, 129, 243, 215, 251, 124, 227, 57, 130,
    155, 47, 255, 135, 52, 142, 67, 68, 196, 222, 233, 203, 84, 123, 148, 50, 166, 194, 35, 61,
    238, 76, 149, 11, 66, 250, 195, 78, 8, 46, 161, 102, 40, 217, 36, 178, 118, 91, 162, 73,
    109, 139, 209, 37,
];
const SALT_B: [u8; SHA512_LEN] = [
    31, 221, 168, 51, 136, 7, 199, 49, 177, 18, 16, 89, 39, 128, 236, 95, 96, 81, 127, 169, 25,
    181, 74, 13, 45, 229, 122, 159, 147, 201, 156, 239, 160, 224, 59, 77, 174, 42, 245, 176, 200,
    235, 187, 60, 131, 83, 153, 97, 23, 43, 4, 126, 186, 119, 214, 38, 225, 105, 20, 99, 85, 33,
    12, 125,
];
const SALT_PRIVATE_A: [u8; SHA512_LEN] = [
    191, 192, 216, 250, 122, 246, 220, 97, 31, 254, 98, 27, 8, 72, 71, 176, 135, 99, 96, 18,
    127, 101, 203, 104, 211, 102, 191, 125, 37, 72, 150, 156, 51, 229, 121, 35, 17, 153, 141,
    177, 110, 131, 150, 128, 172, 255, 254, 6, 18, 140, 55, 62, 236, 249, 135, 64, 135, 12, 117,
    4, 89, 149, 168, 209,
];
const SALT_PRIVATE_B: [u8; SHA512_LEN] = [
    246, 204, 26, 232, 232, 70, 129, 109, 223, 146, 169, 242, 23, 241, 105, 145, 50, 196, 165,
    42, 254, 120, 3, 54, 244, 207, 209, 85, 53, 6, 138, 106, 175, 148, 31, 204, 186, 186, 165,
    182, 87, 142, 49, 10, 39, 110, 26, 154, 86, 56, 173, 125, 18, 64, 198, 225, 99, 99, 83, 82,
    191, 134, 76, 170,
];

fn salt_xor(a: &[u8; SHA512_LEN], b: &[u8; SHA512_LEN]) -> [u8; SHA512_LEN] {
    let mut out = [0u8; SHA512_LEN];
    for i in 0..SHA512_LEN {
        out[i] = a[i] ^ b[i];
    }
    out
}

fn derive_key_iv(random_key: &[u8], salt: &[u8; SHA512_LEN]) -> Option<([u8; 16], [u8; 16])> {
    use sha2::{Digest, Sha512};
    if random_key.len() != RANDOM_KEY_LEN {
        return None;
    }
    let mut merged = [0u8; SHA512_LEN * 2];
    merged[..SHA512_LEN].copy_from_slice(&Sha512::digest(random_key));
    merged[SHA512_LEN..].copy_from_slice(salt);
    let merged_hash = Sha512::digest(merged);
    let mut key = [0u8; 16];
    let mut iv = [0u8; 16];
    key.copy_from_slice(&merged_hash[..16]);
    iv.copy_from_slice(&merged_hash[16..32]);
    Some((key, iv))
}

/// One ByteCrypto value → the decrypted JSON document of the sign-in blob.
fn decode_auth_blob(b64_value: &str) -> Result<Value, String> {
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(b64_value.trim())
        .map_err(|e| format!("decode sign-in blob: {e}"))?;
    let plain = byte_crypto_decrypt(&raw).ok_or("Trae sign-in blob decryption failed")?;
    serde_json::from_slice(&plain).map_err(|e| format!("parse sign-in blob: {e}"))
}

/// base64-decoded blob → plaintext (SHA512 checksum header stripped).
fn byte_crypto_decrypt(raw: &[u8]) -> Option<Vec<u8>> {
    use aes::cipher::block_padding::Pkcs7;
    use aes::cipher::{BlockDecryptMut, KeyIvInit};
    use cbc::Decryptor;
    type Aes128CbcDec = Decryptor<aes::Aes128>;

    if raw.len() <= 6 + RANDOM_KEY_LEN {
        return None;
    }
    let salt = match &raw[..6] {
        m if *m == MAGIC_AES => salt_xor(&SALT_A, &SALT_B),
        m if *m == MAGIC_AES_PRIVATE => salt_xor(&SALT_PRIVATE_A, &SALT_PRIVATE_B),
        _ => return None,
    };
    let key_material = &raw[6..6 + RANDOM_KEY_LEN];
    let ciphertext = &raw[6 + RANDOM_KEY_LEN..];
    if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
        return None;
    }
    let (key, iv) = derive_key_iv(key_material, &salt)?;
    let mut buffer = ciphertext.to_vec();
    let decrypted = Aes128CbcDec::new_from_slices(&key, &iv)
        .ok()?
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .ok()?;
    if decrypted.len() < SHA512_LEN {
        return None;
    }
    // Integrity: the first 64 bytes must be SHA512 of the payload.
    use sha2::{Digest, Sha512};
    if Sha512::digest(&decrypted[SHA512_LEN..]).as_slice() != &decrypted[..SHA512_LEN] {
        return None;
    }
    Some(decrypted[SHA512_LEN..].to_vec())
}

fn json_f64(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::block_padding::Pkcs7;
    use aes::cipher::{BlockEncryptMut, KeyIvInit};
    use base64::Engine;
    use serde_json::json;
    use sha2::{Digest, Sha512};

    type Aes128CbcEnc = cbc::Encryptor<aes::Aes128>;

    /// Mirror of the IDE's encrypt path so tests can round-trip real blobs.
    fn byte_crypto_encrypt(plain: &[u8], random_key: &[u8; 32], private: bool) -> Vec<u8> {
        let salt = if private {
            salt_xor(&SALT_PRIVATE_A, &SALT_PRIVATE_B)
        } else {
            salt_xor(&SALT_A, &SALT_B)
        };
        let (key, iv) = derive_key_iv(random_key, &salt).unwrap();
        let mut payload = Sha512::digest(plain).to_vec();
        payload.extend_from_slice(plain);
        // Scratch tail so the buffer spans whole blocks; the cipher call
        // below writes the real PKCS7 padding itself.
        let pad = 16 - (payload.len() % 16);
        payload.extend(std::iter::repeat_n(0u8, pad));
        let mut out = if private { MAGIC_AES_PRIVATE } else { MAGIC_AES }.to_vec();
        out.extend_from_slice(random_key);
        out.extend_from_slice(
            Aes128CbcEnc::new_from_slices(&key, &iv)
                .unwrap()
                .encrypt_padded_mut::<Pkcs7>(&mut payload, plain.len() + SHA512_LEN)
                .unwrap(),
        );
        out
    }

    #[test]
    fn byte_crypto_roundtrip_both_versions() {
        let key = [9u8; 32];
        let plain = br#"{"token":"jwt-abc","host":"https://api.trae.cn"}"#;
        for private in [false, true] {
            let blob = byte_crypto_encrypt(plain, &key, private);
            let decoded = byte_crypto_decrypt(&blob).expect("decrypt");
            assert_eq!(decoded, plain, "private={private}");
        }
    }

    #[test]
    fn byte_crypto_rejects_garbage() {
        assert!(byte_crypto_decrypt(b"short").is_none());
        assert!(byte_crypto_decrypt(&[0u8; 64]).is_none()); // bad magic
        // Corrupt the payload of an otherwise valid blob → checksum fails.
        let key = [9u8; 32];
        let mut blob = byte_crypto_encrypt(br#"{"a":1}"#, &key, false);
        let last = blob.len() - 1;
        blob[last] ^= 0xFF;
        assert!(byte_crypto_decrypt(&blob).is_none());
    }

    #[test]
    fn decodes_a_real_shaped_auth_value() {
        let key = [3u8; 32];
        let auth = json!({
            "token": "eyJhbGciOi...long-jwt",
            "refreshToken": "r-tok",
            "host": "https://api.trae.cn",
            "userId": "3863014157662970"
        });
        let blob = byte_crypto_encrypt(serde_json::to_string(&auth).unwrap().as_bytes(), &key, false);
        let b64 = base64::engine::general_purpose::STANDARD.encode(blob);
        let decoded = decode_auth_blob(&b64).expect("decode");
        assert_eq!(decoded["host"], "https://api.trae.cn");
        assert_eq!(decoded["userId"], "3863014157662970");
    }

    #[test]
    fn parses_the_real_usage_shape() {
        let usage = json!({
            "is_credits_billing": true,
            "usage_summary": {
                "consumed_amount": 84.02,
                "total_amount": 4800,
                "consumption_ratio": 0.017504166666666664
            },
            "user_entitlement_pack_list": [
                {"display_desc": "老用户福利", "group_name": "用户福利",
                 "entitlement_base_info": {"quota": {"credits_limit": 2000}, "end_time": 1791716393}},
                {"display_desc": "新用户积分", "group_name": "新用户",
                 "entitlement_base_info": {"quota": {"credits_limit": 800}, "end_time": 1791716393}}
            ]
        });
        let snap = parse_snapshot(Some("Free"), &usage).expect("parse");
        assert_eq!(snap.id, "traecn");
        assert_eq!(snap.name, "Trae CN");
        assert_eq!(snap.plan.as_deref(), Some("Free"));
        assert_eq!(snap.metrics.len(), 1);
        let row = &snap.metrics[0];
        assert_eq!(row.label, "Credits");
        assert!((row.used_percent.unwrap() - (84.02 / 4800.0 * 100.0)).abs() < 0.001);
        assert_eq!(row.detail.as_deref(), Some("84.02 of 4800 credits used · 2 packs"));
    }

    #[test]
    fn single_pack_detail_has_no_pack_suffix() {
        let usage = json!({
            "usage_summary": {"consumed_amount": 0.0, "total_amount": 500},
            "user_entitlement_pack_list": [
                {"display_desc": "x", "entitlement_base_info": {"quota": {"credits_limit": 500}}}
            ]
        });
        let snap = parse_snapshot(None, &usage).expect("parse");
        assert_eq!(snap.plan, None);
        assert_eq!(
            snap.metrics[0].detail.as_deref(),
            Some("0.00 of 500 credits used")
        );
    }

    #[test]
    fn usage_without_summary_is_an_error() {
        assert!(parse_snapshot(None, &json!({})).is_err());
        assert!(parse_snapshot(None, &json!({"usage_summary": {"consumed_amount": 1.0}})).is_err());
    }
}
