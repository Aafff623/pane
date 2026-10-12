/// Generic Chromium `os_crypt` helpers — `Local State` → DPAPI → AES-GCM
/// v10 peel. Doubao's desktop cookies and WorkBuddy's web-session cookie
/// import both go through here; anything Chromium-profile-shaped belongs.
use aes_gcm::{aead::Aead, Aes256Gcm, Key, KeyInit, Nonce};
use serde_json::Value;

use super::read_small_text;

const LOCAL_STATE_REL: &str = "Local State";
const MAX_LOCAL_STATE_BYTES: u64 = 256 * 1024;

/// The peel sanity bar: cookie values are printable text, so a wrong key
/// or wrong header length (binary junk) is rejected here.
pub(crate) fn looks_like_text(s: &str) -> bool {
    let printable = s.chars().filter(|c| !c.is_control()).count();
    printable * 2 > s.len()
}

/// `Local State` → `os_crypt.encrypted_key` → DPAPI → 32-byte AES key.
pub(crate) fn os_crypt_key(dir: &std::path::Path) -> Result<[u8; 32], String> {
    let raw = read_small_text(
        &dir.join(LOCAL_STATE_REL),
        MAX_LOCAL_STATE_BYTES,
        "Local State",
    )?;
    use base64::Engine;
    let doc: Value = serde_json::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("parse Local State: {e}"))?;
    let encoded = doc
        .pointer("/os_crypt/encrypted_key")
        .and_then(Value::as_str)
        .ok_or("Local State has no os_crypt.encrypted_key")?;
    let wrapped = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("decode encrypted_key: {e}"))?;
    let wrapped = wrapped
        .strip_prefix(b"DPAPI")
        .ok_or("encrypted_key lacks the DPAPI prefix")?;
    let key =
        crate::platform::dpapi_unprotect(wrapped).ok_or("DPAPI unwrap of the os_crypt key failed")?;
    key.try_into()
        .map_err(|_| "os_crypt key is not 32 bytes".to_string())
}

/// Chromium os_crypt v10: nonce = bytes 3..15, ciphertext+tag = 15...
/// (Same scheme as Qoder CN's auth.v1.dat.)
pub(crate) fn decode_os_crypt(raw: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    if raw.len() < 3 + 12 + 16 || &raw[..3] != b"v10" {
        return Err("not a v10 os_crypt blob".into());
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(&raw[3..15]), &raw[15..])
        .map_err(|_| "cookie decryption failed (key mismatch?)".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_peel_sanity_bar() {
        // Real cookie values are printable text — accepted. (Synthetic
        // samples only — never paste a live token here.)
        assert!(looks_like_text("session-token-value"));
        assert!(looks_like_text("aBc123XyZ_-synthetic-sample-value-000111"));
        // Binary junk (wrong key / wrong header length) — rejected.
        assert!(!looks_like_text("\u{1}\u{2}\u{3}"));
        assert!(!looks_like_text("ab\u{0}\u{1}\u{2}\u{3}cd"));
    }
}
