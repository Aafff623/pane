//! v3 cryptographic primitives — thin, testable wrappers over audited
//! crates (RustCrypto `aes-gcm` / `argon2` / `hkdf` / `hmac` / `sha2`)
//! plus the two hand-rolled pieces the protocol needs: GF(2^8) Shamir
//! 2-of-3 with XOR answer masking, and the canonical-byte form the
//! manifest HMAC covers.
//!
//! Every function here is *pure where randomness matters*: sealing and
//! splitting take their nonce / coefficient material as explicit
//! arguments so tests can pin known-answer vectors; the `fresh_*`
//! wrappers draw from the OS CSPRNG at the call site.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

// ── domain constants (handoff design/02 §B, §C) ─────────────────────────────

pub const SUITE: &str = "pane-v3-aes256gcm-hkdfsha256-v1";
pub const MANIFEST_MAC_ALGO: &str = "hmac-sha256";
pub const CANONICALIZATION: &str = "rfc8785-jcs";
pub const ENTRY_PAYLOAD_SCHEMA: &str = "pane-v3-entry-v1";
pub const QUESTIONS_SCHEME: &str = "shamir-gf256-masked-v1";
pub const ANSWER_NORMALIZATION: &str = "answer-normalize-v1";
pub const RECOVERY_ENCODING: &str = "crockford-base32-check10-v1";

/// HKDF info labels — every derived subkey gets its own label so a key for
/// one purpose can never be replayed for another.
pub const INFO_ENTRIES: &[u8] = b"pane/v3/entries/aes256gcm";
pub const INFO_MANIFEST: &[u8] = b"pane/v3/manifest/hmac-sha256";
pub const INFO_WRAP_PASSWORD: &[u8] = b"pane/v3/wrap/password";
pub const INFO_QUESTION_MASK: &[u8] = b"pane/v3/question/mask";
pub const INFO_RECOVERY_CODE: &[u8] = b"pane/v3/recovery/code";
pub const INFO_RECOVERY_DEVICE: &[u8] = b"pane/v3/recovery/device";

/// AAD protocol labels.
pub const AAD_ENTRY: &[u8] = b"pane/v3/entry/v1";
pub const AAD_WRAP_PASSWORD: &[u8] = b"pane/v3/wrap/password/v1";
pub const AAD_WRAP_RECOVERY: &[u8] = b"pane/v3/wrap/recovery/v1";

/// Argon2id parameters — Pane's recommended configuration, NOT a claim of
/// being an RFC 9106 recommended profile. The strict parser accepts nothing
/// else, so a future retune is a new suite id, never a silent change.
pub const ARGON_M_KIB: u32 = 65_536;
pub const ARGON_T: u32 = 3;
pub const ARGON_P: u32 = 1;
pub const ARGON_V: u32 = 19; // 0x13

pub fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

/// Canonical base64: decode must succeed AND re-encode must reproduce the
/// input byte-for-byte (rejects padding games and non-canonical trailing
/// bits before anything cryptographic runs). `name` only feeds error text.
pub fn b64_decode_canonical(name: &str, field: &str, expected_len: usize) -> Result<Vec<u8>, String> {
    let raw = b64()
        .decode(field)
        .map_err(|_| format!("{name}: invalid base64"))?;
    if b64().encode(&raw) != field {
        return Err(format!("{name}: non-canonical base64"));
    }
    if raw.len() != expected_len {
        return Err(format!("{name}: expected {expected_len} bytes, got {}", raw.len()));
    }
    Ok(raw)
}

/// HKDF-SHA256 with an exact 32-byte output — the only expansion the v3
/// protocol uses.
pub fn hkdf32(ikm: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut okm = [0u8; 32];
    hk.expand(info, &mut okm).expect("32 bytes is a valid HKDF length");
    okm
}

/// Length-prefixed binary TLV: each part as u32-BE length + bytes. AAD and
/// HKDF info fields are built from this so field boundaries are unambiguous
/// (no fuzzy string concatenation across encodings).
pub fn tlv(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        out.extend_from_slice(&(part.len() as u32).to_be_bytes());
        out.extend_from_slice(part);
    }
    out
}

// ── AES-256-GCM with mandatory AAD ──────────────────────────────────────────

pub fn aead_seal(key: &[u8; 32], nonce: [u8; 12], plaintext: &[u8], aad: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .encrypt(Nonce::from_slice(&nonce), Payload { msg: plaintext, aad })
        .expect("AES-GCM encryption cannot fail for a valid key/nonce")
}

pub fn aead_open(
    key: &[u8; 32],
    nonce_b64: &str,
    sealed_b64: &str,
    aad: &[u8],
) -> Result<Vec<u8>, String> {
    let nonce = b64_decode_canonical("nonce", nonce_b64, 12).map_err(|_| "malformed nonce".to_string())?;
    // Sealed payloads are variable-length; only the nonce has a fixed size.
    let sealed = b64()
        .decode(sealed_b64)
        .map_err(|_| "malformed sealed payload".to_string())?;
    if b64().encode(&sealed) != sealed_b64 {
        return Err("malformed sealed payload".into());
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(&nonce), Payload { msg: sealed.as_ref(), aad })
        .map_err(|_| "authentication failed".to_string())
}

pub fn fresh_nonce() -> [u8; 12] {
    let mut n = [0u8; 12];
    OsRng.fill_bytes(&mut n);
    n
}

pub fn fresh_random<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    OsRng.fill_bytes(&mut buf);
    buf
}

// ── manifest HMAC ───────────────────────────────────────────────────────────

/// `K_manifest` from the DEK — an independent derivation, never the DEK or
/// an entry key itself.
pub fn manifest_key(dek: &[u8; 32], vault_id: &[u8; 16]) -> [u8; 32] {
    hkdf32(dek, vault_id, INFO_MANIFEST)
}

pub fn compute_manifest_mac(key: &[u8; 32], canonical: &[u8]) -> [u8; 32] {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("hmac accepts any key size");
    mac.update(canonical);
    mac.finalize().into_bytes().into()
}

/// Constant-time verification — `Hmac::verify_slice` compares the full
/// digest without branching on content.
pub fn verify_manifest_mac(key: &[u8; 32], canonical: &[u8], mac_b64: &str) -> Result<(), String> {
    let expected =
        b64_decode_canonical("manifest mac", mac_b64, 32).map_err(|_| "manifest mac malformed".to_string())?;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("hmac accepts any key size");
    mac.update(canonical);
    mac.verify_slice(&expected).map_err(|_| "manifest authentication failed".to_string())
}

// ── canonical JSON (RFC 8785 subset — see module note) ──────────────────────

/// The v3 file's canonical byte form — an explicit RFC 8785 (JCS) subset
/// writer that does NOT depend on which `Map` type serde_json uses.
/// `serde_json`'s `Map` is only the sorted `BTreeMap` when `preserve_order`
/// is OFF; the app build pulls that feature in through
/// tauri-utils→schemars, so a plain `to_vec` there serializes keys in
/// INSERTION order and the MAC bytes silently diverge between builds. Keys
/// are sorted explicitly here, and the two remaining JCS assumptions of
/// this closed format are enforced as hard errors instead of assumed:
/// object keys must be ASCII (byte sort == UTF-16 code-unit sort) and
/// numbers must be integers (no ES6 float serialization).
pub fn canonical_json(v: &serde_json::Value) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    write_canonical(v, &mut out)?;
    Ok(out)
}

fn write_canonical(v: &serde_json::Value, out: &mut Vec<u8>) -> Result<(), String> {
    use serde_json::Value;
    match v {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Number(n) => {
            // Minimal decimal digits == JCS for integers; a float would need
            // the full ES6 number formatter, which this format never uses.
            if let Some(i) = n.as_i64() {
                out.extend_from_slice(i.to_string().as_bytes());
            } else if let Some(u) = n.as_u64() {
                out.extend_from_slice(u.to_string().as_bytes());
            } else {
                return Err(
                    "canonical form: non-integer numbers are not part of the v3 format".into(),
                );
            }
        }
        Value::String(s) => {
            // serde_json's string escaping (minimal escapes, raw UTF-8 for
            // non-control chars, lowercase \u hex) matches JCS exactly.
            serde_json::to_writer(&mut *out, s)
                .map_err(|e| format!("canonical serialization: {e}"))?;
        }
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canonical(item, out)?;
            }
            out.push(b']');
        }
        Value::Object(map) => {
            if map.keys().any(|k| !k.is_ascii()) {
                return Err(
                    "canonical form: non-ASCII object keys are not part of the v3 format".into(),
                );
            }
            let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
            keys.sort_unstable();
            out.push(b'{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key)
                    .map_err(|e| format!("canonical serialization: {e}"))?;
                out.push(b':');
                write_canonical(&map[*key], out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

// ── GF(2^8) + Shamir 2-of-3 ─────────────────────────────────────────────────

/// Multiplication modulo the AES irreducible polynomial 0x11B.
pub fn gf_mul(a: u8, b: u8) -> u8 {
    let mut a = a;
    let mut b = b;
    let mut out = 0u8;
    for _ in 0..8 {
        if b & 1 != 0 {
            out ^= a;
        }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= 0x1B;
        }
        b >>= 1;
    }
    out
}

/// Multiplicative inverse via a^254 (Fermat; 0 has no inverse).
pub fn gf_inv(a: u8) -> u8 {
    let mut result = 1u8;
    let mut power = a;
    let mut e = 254u32;
    while e > 0 {
        if e & 1 != 0 {
            result = gf_mul(result, power);
        }
        power = gf_mul(power, power);
        e >>= 1;
    }
    result
}

pub fn gf_div(a: u8, b: u8) -> u8 {
    gf_mul(a, gf_inv(b))
}

/// One Shamir share evaluation. The secret IS f(0): each byte j lives on the
/// line `f_j(x) = secret[j] XOR gf_mul(slope[j], x)` with an independent
/// uniformly random slope (zero included). Shares are evaluated at the
/// NON-ZERO points x = 1, 2, 3 — x=0 would be the secret itself.
pub fn shamir_share_at(secret: &[u8; 32], slope: &[u8; 32], x: u8) -> [u8; 32] {
    debug_assert!((1..=3).contains(&x));
    let mut out = [0u8; 32];
    for j in 0..32 {
        out[j] = secret[j] ^ gf_mul(slope[j], x);
    }
    out
}

/// Reconstruct f(0) from two shares at distinct non-zero x via Lagrange
/// interpolation at 0: for degree-1 lines,
/// `s = (x2·y1 ⊕ x1·y2) / (x1 ⊕ x2)`.
pub fn shamir_combine(x1: u8, y1: &[u8; 32], x2: u8, y2: &[u8; 32]) -> Result<[u8; 32], String> {
    if x1 == 0 || x2 == 0 || x1 == x2 {
        return Err("share indices must be distinct and non-zero".into());
    }
    let den = x1 ^ x2;
    let mut out = [0u8; 32];
    for j in 0..32 {
        out[j] = gf_div(gf_mul(x2, y1[j]) ^ gf_mul(x1, y2[j]), den);
    }
    Ok(out)
}

// ── recovery code codec (Crockford Base32, 160-bit + 10-bit check) ─────────

const CROCKFORD: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const CHECK_PREFIX: &[u8] = b"pane-recovery-check-v1\0";

fn base32_body(c: &[u8; 20]) -> String {
    // MSB-first: 160 bits → exactly 32 canonical chars.
    let mut out = String::with_capacity(32);
    let mut acc: u32 = 0;
    let mut acc_len: u32 = 0;
    for &byte in c {
        acc = (acc << 8) | byte as u32;
        acc_len += 8;
        while acc_len >= 5 {
            acc_len -= 5;
            out.push(CROCKFORD[((acc >> acc_len) & 0x1F) as usize] as char);
        }
    }
    out
}

fn check_chars(check10: u16) -> String {
    let mut s = String::with_capacity(2);
    s.push(CROCKFORD[((check10 >> 5) & 0x1F) as usize] as char);
    s.push(CROCKFORD[(check10 & 0x1F) as usize] as char);
    s
}

fn check10_of(c: &[u8; 20]) -> u16 {
    let digest = Sha256::digest([CHECK_PREFIX, c.as_slice()].concat());
    (((digest[0] as u16) << 2) | (digest[1] >> 6) as u16) & 0x3FF
}

/// The printed recovery code: 32 canonical body chars (160 bits) + `-` +
/// 2 check chars (first 10 bits of SHA-256 over a domain-prefixed body).
/// The checksum only catches typos — it is NOT an authentication factor.
pub fn recovery_code_encode(c: &[u8; 20]) -> String {
    format!("{}-{}", base32_body(c), check_chars(check10_of(c)))
}

/// Normalize user input: strip separators/whitespace, uppercase, and map the
/// Crockford confusables (O→0, I/L→1). Canonical output only; parsing stays
/// strict after this.
pub fn recovery_code_normalize(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .map(|c| match c {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect()
}

/// Extract a shaped code from an exported text file; checksum validation
/// remains the decoder's responsibility, not the text importer's.
pub fn recovery_code_extract(input: &str) -> Option<String> {
    std::iter::once(input)
        .chain(input.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line)))
        .map(recovery_code_normalize)
        .find(|folded| {
            folded.len() == 34
                && folded
                    .bytes()
                    .all(|byte| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&byte))
        })
}

/// Decode a normalized code back to the 20 bytes, verifying the 2-char
/// check. Err on wrong length, illegal characters, or a mismatched checksum.
pub fn recovery_code_decode(normalized: &str) -> Result<[u8; 20], String> {
    if normalized.len() != 34 {
        return Err("recovery code length is wrong".into());
    }
    let (body, check) = normalized.split_at(32);
    let mut c = [0u8; 20];
    let mut acc: u32 = 0;
    let mut acc_len: u32 = 0;
    let mut byte_idx = 0;
    for ch in body.chars() {
        let val = CROCKFORD
            .iter()
            .position(|&e| e as char == ch)
            .ok_or_else(|| "recovery code has illegal characters".to_string())? as u32;
        acc = (acc << 5) | val;
        acc_len += 5;
        if acc_len >= 8 {
            acc_len -= 8;
            c[byte_idx] = ((acc >> acc_len) & 0xFF) as u8;
            byte_idx += 1;
        }
    }
    if check != check_chars(check10_of(&c)) {
        return Err("recovery code checksum is wrong".into());
    }
    Ok(c)
}

/// `K_code` — the recovery-code KEK base, domain-separated per vault and
/// per recovery generation.
pub fn recovery_code_key(
    code: &[u8; 20],
    vault_id: &[u8; 16],
    recovery_id: &[u8; 16],
    salt: &[u8; 16],
) -> [u8; 32] {
    hkdf32(code, salt, &tlv(&[INFO_RECOVERY_CODE, vault_id, recovery_id]))
}

/// `KEK` for the (Phase-4) TPM-bound recovery: K_code mixed with the TPM-
/// sealed device secret D. Binding the wrap to this value is what makes a
/// stolen JSON+code pair useless on another device.
pub fn recovery_device_key(
    code_key: &[u8; 32],
    device_secret: &[u8; 32],
    vault_id: &[u8; 16],
    recovery_id: &[u8; 16],
    salt: &[u8; 16],
) -> [u8; 32] {
    let mut ikm = [0u8; 64];
    ikm[..32].copy_from_slice(code_key);
    ikm[32..].copy_from_slice(device_secret);
    hkdf32(&ikm, salt, &tlv(&[INFO_RECOVERY_DEVICE, vault_id, recovery_id]))
}

// ── Argon2id (frozen v3 parameters) ─────────────────────────────────────────

pub fn argon2id_frozen(password: &[u8], salt: &[u8]) -> Result<[u8; 32], String> {
    let params =
        argon2::Params::new(ARGON_M_KIB, ARGON_T, ARGON_P, Some(32)).map_err(|e| e.to_string())?;
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut out = [0u8; 32];
    argon
        .hash_password_into(password, salt, &mut out)
        .map_err(|e| format!("argon2: {e}"))?;
    Ok(out)
}

// ── normalization ───────────────────────────────────────────────────────────

/// v3 unlock codes: Unicode NFC, no trimming, no case folding. What the
/// user types (after NFC) IS the password.
pub fn normalize_password_v3(code: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    code.nfc().collect()
}

/// `answer-normalize-v1`: trim → Unicode lowercase → collapse inner
/// whitespace. Version-tagged in the file so a future rule change cannot
/// silently orphan existing shares.
pub fn normalize_answer_v1(answer: &str) -> String {
    answer.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

// ── subkey derivations over the DEK ─────────────────────────────────────────

pub fn entry_key(dek: &[u8; 32], vault_id: &[u8; 16]) -> [u8; 32] {
    hkdf32(dek, vault_id, INFO_ENTRIES)
}

pub fn password_wrap_key(k_pw_base: &[u8; 32], vault_id: &[u8; 16]) -> [u8; 32] {
    hkdf32(k_pw_base, vault_id, INFO_WRAP_PASSWORD)
}

/// AAD for one entry's AEAD seal — binds ciphertext to its public metadata.
pub fn aad_entry(version: u32, vault_id: &[u8; 16], e: (&str, &str, &str)) -> Vec<u8> {
    // (entry_id, service, payload_schema)
    tlv(&[
        AAD_ENTRY,
        &version.to_be_bytes(),
        vault_id,
        e.0.as_bytes(),
        e.1.as_bytes(),
        e.2.as_bytes(),
    ])
}

/// AAD for the password wrap — binds the sealed DEK to the KDF parameters,
/// so a tampered salt/params cannot be swapped under a valid MAC-less edit.
pub fn aad_password_wrap(
    version: u32,
    vault_id: &[u8; 16],
    kdf_algo: &str,
    kdf_v: u32,
    salt: &[u8],
    m_kib: u32,
    t: u32,
    p: u32,
) -> Vec<u8> {
    tlv(&[
        AAD_WRAP_PASSWORD,
        &version.to_be_bytes(),
        vault_id,
        kdf_algo.as_bytes(),
        &kdf_v.to_be_bytes(),
        salt,
        &m_kib.to_be_bytes(),
        &t.to_be_bytes(),
        &p.to_be_bytes(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    // CR-03 — RFC 5869 Test Case 1 (SHA-256)
    #[test]
    fn hkdf_rfc5869_test_case_1() {
        let ikm = [0x0bu8; 22];
        let salt = hex("000102030405060708090a0b0c");
        let info = hex("f0f1f2f3f4f5f6f7f8f9");
        let hk = Hkdf::<Sha256>::new(Some(&salt), &ikm);
        let mut okm = [0u8; 42];
        hk.expand(&info, &mut okm).unwrap();
        assert_eq!(
            hex("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"),
            okm.to_vec()
        );
    }

    // CR-02 — domain separation actually separates
    #[test]
    fn hkdf_domain_separation_yields_distinct_keys() {
        let ikm = [1u8; 32];
        let salt = [2u8; 16];
        assert_ne!(hkdf32(&ikm, &salt, INFO_ENTRIES), hkdf32(&ikm, &salt, INFO_MANIFEST));
        assert_ne!(hkdf32(&ikm, &salt, INFO_ENTRIES), hkdf32(&ikm, &salt, INFO_WRAP_PASSWORD));
    }

    // CR-05 — NIST GCM specification test case 13 (AES-256, zero key/IV,
    // empty plaintext, pinned tag) + tamper rejection.
    #[test]
    fn aes_gcm_known_answer_and_tamper_rejection() {
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        let sealed = aead_seal(&key, nonce, b"", b"");
        assert_eq!(sealed.len(), 16);
        assert_eq!(hex("530f8afbc74536b9a963b4f1c4cb738b"), sealed);
        // Roundtrip opens cleanly…
        let pt = b"pane test payload";
        let sealed = aead_seal(&key, nonce, pt, b"aad");
        assert_eq!(aead_open(&key, &b64().encode(nonce), &b64().encode(&sealed), b"aad").unwrap(), pt);
        // …and any tamper — tag, AAD, nonce — fails.
        let mut bad = sealed.clone();
        bad[0] ^= 1;
        assert!(aead_open(&key, &b64().encode(nonce), &b64().encode(&bad), b"aad").is_err());
        assert!(aead_open(&key, &b64().encode(nonce), &b64().encode(&sealed), b"other").is_err());
        let mut other_nonce = nonce;
        other_nonce[0] ^= 1;
        assert!(aead_open(&key, &b64().encode(other_nonce), &b64().encode(&sealed), b"aad").is_err());
    }

    // CR-06 — GF(2^8) arithmetic
    #[test]
    fn gf256_arithmetic_known_and_inverse_roundtrip() {
        assert_eq!(gf_mul(0x57, 0x83), 0xC1);
        for a in 1..=255u8 {
            assert_eq!(gf_mul(a, gf_inv(a)), 1, "inverse of {a:#04x}");
        }
    }

    // CR-07 — the handoff's deterministic Shamir vector
    #[test]
    fn shamir_known_vector_single_byte() {
        let secret = [0x42u8; 32];
        let slope = [0x57u8; 32];
        let s1 = shamir_share_at(&secret, &slope, 1);
        let s2 = shamir_share_at(&secret, &slope, 2);
        let s3 = shamir_share_at(&secret, &slope, 3);
        assert_eq!(s1[0], 0x15);
        assert_eq!(s2[0], 0xEC);
        assert_eq!(s3[0], 0xBB);
        assert_eq!(shamir_combine(1, &s1, 2, &s2).unwrap(), secret);
        assert_eq!(shamir_combine(1, &s1, 3, &s3).unwrap(), secret);
        assert_eq!(shamir_combine(2, &s2, 3, &s3).unwrap(), secret);
    }

    #[test]
    fn shamir_rejects_bad_indices() {
        let y = [0u8; 32];
        assert!(shamir_combine(0, &y, 1, &y).is_err());
        assert!(shamir_combine(2, &y, 2, &y).is_err());
        assert!(shamir_combine(3, &y, 0, &y).is_err());
    }

    // CR-08 — randomized multi-byte property: every pair of every trial
    // reconstructs the original secret (slopes sampled uniformly, zero
    // included, per byte).
    #[test]
    fn shamir_all_pairs_reconstruct_across_random_trials() {
        for _ in 0..256 {
            let secret = fresh_random::<32>();
            let slope = fresh_random::<32>();
            let shares = [
                shamir_share_at(&secret, &slope, 1),
                shamir_share_at(&secret, &slope, 2),
                shamir_share_at(&secret, &slope, 3),
            ];
            assert_eq!(shamir_combine(1, &shares[0], 2, &shares[1]).unwrap(), secret);
            assert_eq!(shamir_combine(1, &shares[0], 3, &shares[2]).unwrap(), secret);
            assert_eq!(shamir_combine(2, &shares[1], 3, &shares[2]).unwrap(), secret);
        }
    }

    #[test]
    fn base64_canonical_gate_rejects_padding_games() {
        assert!(b64_decode_canonical("f", &b64().encode([9u8; 16]), 16).is_ok());
        // 16 bytes encode with "=" padding; strip it → non-canonical → rejected.
        let stripped = b64().encode([9u8; 16]).trim_end_matches('=').to_string();
        assert!(b64_decode_canonical("f", &stripped, 16).is_err());
        // wrong length
        assert!(b64_decode_canonical("f", &b64().encode([9u8; 15]), 16).is_err());
    }

    // CR-28 — recovery code codec: canonical body + 2 check chars
    #[test]
    fn recovery_code_roundtrip_and_confusable_folding() {
        let c = fresh_random::<20>();
        let code = recovery_code_encode(&c);
        assert_eq!(code.len(), 35); // 32 body + '-' + 2 check
        assert_eq!(code.chars().filter(|ch| *ch == '-').count(), 1);
        // Lowercase, de-grouped, confusable-substituted input normalizes to
        // the same bytes. The check chars are foldable too (their Crockford
        // values never contain O/I/L equivalents).
        let mangled: String = code
            .chars()
            .map(|ch| if ch == '-' { ' ' } else { ch.to_ascii_lowercase() })
            .collect();
        assert_eq!(recovery_code_decode(&recovery_code_normalize(&mangled)).unwrap(), c);
        // A flipped body character must not decode to the same secret.
        let mut bytes = code.into_bytes();
        let idx = 5;
        bytes[idx] = if bytes[idx] == b'A' { b'B' } else { b'A' };
        let flipped = String::from_utf8(bytes).expect("crockford chars are ascii");
        let normalized = recovery_code_normalize(&flipped);
        assert!(
            recovery_code_decode(&normalized).is_err()
                || recovery_code_decode(&normalized).unwrap() != c
        );
    }

    #[test]
    fn recovery_code_check_chars_exclude_confusables() {
        for v in [0u16, 1, 31, 1023, 0x2AA] {
            for ch in check_chars(v).chars() {
                assert!(CROCKFORD.contains(&(ch as u8)));
            }
        }
    }

    // CR-22 — NFC normalization of unlock codes
    #[test]
    fn password_nfc_equivalence() {
        // "é" as U+00E9 vs U+0065 U+0301 — same NFC form.
        assert_eq!(normalize_password_v3("caf\u{00e9}"), normalize_password_v3("cafe\u{0301}"));
        // No trimming: leading/trailing spaces are significant.
        assert_ne!(normalize_password_v3("abc"), normalize_password_v3(" abc"));
    }

    // CR-18 — answer normalization v1
    #[test]
    fn answer_normalization_v1_collapses_case_and_spaces() {
        assert_eq!(normalize_answer_v1("  Apple   Pie "), "apple pie");
        assert_eq!(normalize_answer_v1("APPLE PIE"), normalize_answer_v1("apple   pie"));
    }

    // Canonical form is byte-stable, key-sorted, and unicode-safe in values.
    #[test]
    fn canonical_json_is_byte_stable_and_sorted() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"b":2,"a":"x","c":{"z":1,"y":null}}"#).unwrap();
        assert_eq!(
            canonical_json(&v).unwrap(),
            br#"{"a":"x","b":2,"c":{"y":null,"z":1}}"#.to_vec()
        );
        // Stability across parse → canonical → parse → canonical.
        let again: serde_json::Value =
            serde_json::from_slice(&canonical_json(&v).unwrap()).unwrap();
        assert_eq!(canonical_json(&again).unwrap(), canonical_json(&v).unwrap());
    }

    // KAT pinned to exact bytes — the guard that the canonical form is
    // independent of serde_json's Map type. The app build turns
    // `preserve_order` on (via tauri-utils→schemars) while a bare parse-tests
    // checkout leaves it off; this vector must produce identical bytes in
    // BOTH, which only the explicit-sort writer guarantees. The JCS subset
    // assumptions are hard errors, not silent divergent output.
    #[test]
    fn canonical_json_kat_is_map_type_independent() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"z":1,"a":{"b":[true,null,"中文值"]},"num":9007199254740991}"#,
        )
        .unwrap();
        assert_eq!(
            canonical_json(&v).unwrap(),
            r#"{"a":{"b":[true,null,"中文值"]},"num":9007199254740991,"z":1}"#.as_bytes()
        );
        // Non-ASCII keys and non-integer numbers are rejected outright.
        assert!(canonical_json(&serde_json::json!({ "键": 1 })).is_err());
        assert!(canonical_json(&serde_json::json!(1.5)).is_err());
        // Negative and u64-boundary integers take the minimal-digits path.
        let v: serde_json::Value =
            serde_json::from_str(r#"{"n":-42,"u":18446744073709551615}"#).unwrap();
        assert_eq!(
            canonical_json(&v).unwrap(),
            br#"{"n":-42,"u":18446744073709551615}"#.to_vec()
        );
    }

    #[test]
    fn tlv_is_unambiguous_across_field_boundaries() {
        // ["ab","c"] and ["abc"] must never collide.
        assert_ne!(tlv(&[b"ab", b"c"]), tlv(&[b"abc"]));
        assert_eq!(tlv(&[b"abc"]), tlv(&[b"abc"]));
    }
}
