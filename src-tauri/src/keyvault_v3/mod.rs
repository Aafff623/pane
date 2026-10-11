//! Keyvault v3 — random-DEK envelope with three independent unlock paths
//! (unlock code / Shamir 2-of-3 security questions / high-entropy recovery
//! code) and whole-file integrity (HMAC-SHA256 over the canonical form).
//!
//! Protocol per `temp/keyvault-v3-handoff/pane-v3-vault-handoff/design/02-ADR-v3-密码学协议.md`:
//! the DEK is OS-CSPRNG material, never password-derived; the unlock code
//! only *wraps* it (Argon2id → HKDF → AES-256-GCM); each question's share is
//! XOR-masked with answer-derived material and carries NO per-question
//! verifier — the only acceptance test for a recovered DEK is the whole-file
//! manifest MAC; the recovery code is 160 bits of CSPRNG with a typo
//! checksum. The original unlock code is never sealed anywhere.
//!
//! This module is pure cryptography + format. Persistence, IPC and UI land
//! in later phases; nothing here touches the real `keyvault.json`.

// Transitional: the question/recovery/rotation surface is fully implemented
// and test-covered, but the IPC wiring lands with the migration phase —
// until then the lib build legitimately sees some of it as unused.
#![allow(dead_code)]

pub mod crypto;

use base64::Engine as _;
use crypto::{
    aad_entry, aead_open, aead_seal, argon2id_frozen, canonical_json, compute_manifest_mac,
    entry_key, fresh_nonce, fresh_random, hkdf32, manifest_key, normalize_answer_v1,
    normalize_password_v3, password_wrap_key, recovery_code_decode, recovery_code_encode,
    recovery_code_key, shamir_combine, shamir_share_at, tlv, verify_manifest_mac, b64,
    ANSWER_NORMALIZATION, ARGON_M_KIB, ARGON_P, ARGON_T,
    ARGON_V, CANONICALIZATION, ENTRY_PAYLOAD_SCHEMA, MANIFEST_MAC_ALGO, QUESTIONS_SCHEME,
    RECOVERY_ENCODING, SUITE,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VAULT_VERSION: u32 = 3;
/// Structural caps (schema `keyvault-v3.schema.json`).
pub const MAX_ENTRIES: usize = 10_000;
pub const MAX_QUESTION_CHARS: usize = 60;
pub const MIN_ANSWER_CHARS: usize = 2;
pub const MAX_ANSWER_CHARS: usize = 80;
/// v3 unlock-code policy (product decision, 2026-10-10): ≥6 code points,
/// NO composition requirement — a one-line recommendation (mixed case +
/// digits) lives in the UI, not the validator. Blocklist still applies.
/// Creation/change-time only; unlocking never re-checks.
pub const MIN_UNLOCK_CODE_POINTS: usize = 6;
pub const MAX_UNLOCK_CODE_POINTS: usize = 1024;
/// JSON integers stay exactly representable in the JS IPC layer up to
/// 2^53-1 (the schema's `maximum`); larger values would silently lose
/// precision crossing the Tauri boundary.
pub const MAX_SAFE_JSON_INT: u64 = 9_007_199_254_740_991;

/// One wrong-guess canary is not needed here: a generic failure message for
/// every unlock/recovery path (no oracle about *which* factor failed).
const ERR_UNLOCK_FAILED: &str = "unlock failed — the code or the vault contents are wrong";

// ── on-disk types (schema-exact; unknown fields are rejected) ───────────────

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KdfParams {
    pub algo: String,
    pub v: u32,
    pub salt_b64: String,
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WrapSection {
    pub nonce_b64: String,
    pub sealed_dek_b64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PasswordSection {
    pub kdf: KdfParams,
    pub wrap: WrapSection,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShareEntry {
    pub id: String,
    pub question: String,
    pub x: u8,
    pub kdf: KdfParams,
    pub masked_share_b64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QuestionsSection {
    pub scheme: String,
    pub threshold: u32,
    pub normalization: String,
    pub shares: Vec<ShareEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeviceSection {
    pub provider: String,
    pub key_id: String,
    pub key_name: String,
    pub algo: String,
    pub sealed_device_secret_b64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecoverySectionV3 {
    pub id: String,
    pub encoding: String,
    pub entropy_bits: u32,
    pub salt_b64: String,
    pub binding: String,
    pub device: Option<DeviceSection>,
    pub wrap: WrapSection,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EntryV3 {
    pub id: String,
    pub service: String,
    pub label: String,
    pub masked: String,
    pub created_at: i64,
    pub public_note: Option<String>,
    pub payload_schema: String,
    pub nonce_b64: String,
    pub sealed_payload_b64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CryptoSection {
    pub suite: String,
    pub manifest_mac: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IntegritySection {
    pub canonicalization: String,
    pub mac_b64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VaultFileV3 {
    pub version: u32,
    pub vault_id: String,
    pub revision: u64,
    pub crypto: CryptoSection,
    pub password: PasswordSection,
    pub questions: Option<QuestionsSection>,
    pub recovery: Option<RecoverySectionV3>,
    pub entries: Vec<EntryV3>,
    pub integrity: IntegritySection,
}

/// A decrypted entry — the in-memory shape callers know from v2.
#[derive(Clone, Debug)]
pub struct PlainEntry {
    pub id: String,
    pub service: String,
    pub label: String,
    pub key: String,
    pub note: String,
    pub created_at: i64,
}

/// The sealed entry payload (`pane-v3-entry-v1`): the key AND the private
/// note are confidential; the note leaves the plaintext metadata.
#[derive(Serialize, Deserialize)]
struct EntryPayload {
    key: String,
    #[serde(default)]
    private_note: Option<String>,
}

// ── unlock-code / answer policy ─────────────────────────────────────────────

/// A starter deny-list of the most common passwords (OWASP-style top list).
/// Full 3000-entry list is a pending product decision; the rule structure
/// (normalized exact match) is what matters for the protocol.
const COMMON_PASSWORDS: &[&str] = &[
    "123456", "password", "123456789", "12345678", "12345", "qwerty", "1234567890", "1234567",
    "111111", "123123", "abc123", "password1", "1234", "qwerty123", "000000", "iloveyou",
    "1q2w3e4r", "qwertyuiop", "monkey", "dragon", "123321", "654321", "666666", "123qwe",
    "asdfghjkl", "zxcvbnm", "asdfgh", "123456a", "superman", "batman", "starwars", "letmein",
    "welcome", "admin", "administrator", "login", "master", "hello", "freedom", "whatever",
    "trustno1", "hunter2", "sunshine", "princess", "football", "baseball", "shadow", "michael",
    "jennifer", "charlie", "jordan", "harley", "ranger", "buster", "thomas", "tigger", "robert",
    "soccer", "hockey", "killer", "george", "sexy", "andrew", "charlie1", "passw0rd", "p@ssw0rd",
    "passwort", "contrasena", "qazwsx", "zoom143", "gmail", "102030", "1234qwer", "qwer1234",
    "1qaz2wsx", "zaq12wsx", "aaaaaa", "abc123456", "a123456", "123abc", "pokemon", "samsung",
    "password123", "password1234", "password12345", "password123456", "password1234567",
    "password12345678", "password123456789", "abcd1234", "q1w2e3r4", "12341234", "456789",
    "admin123", "root", "toor", "pass123", "qwe123", "zaq12wsx", "159753", "987654321",
    "google", "liverpool", "chelsea", "arsenal",
];

pub fn validate_unlock_code(normalized: &str) -> Result<(), String> {
    let points = normalized.chars().count();
    if points < MIN_UNLOCK_CODE_POINTS {
        return Err(format!(
            "unlock code needs at least {MIN_UNLOCK_CODE_POINTS} characters"
        ));
    }
    if points > MAX_UNLOCK_CODE_POINTS {
        return Err("unlock code is too long".into());
    }
    let folded = normalized.to_lowercase();
    if COMMON_PASSWORDS.iter().any(|p| folded == p.trim()) {
        return Err("unlock code is too common — pick something unique".into());
    }
    Ok(())
}

/// Creation-time Q&A policy: exactly three questions, texts pairwise
/// distinct after trim, normalized answers pairwise distinct (one guessed
/// answer must never unmask two shares), each answer 2–80 code points.
fn validate_three_questions(questions: &[(String, String)]) -> Result<[(String, String); 3], String> {
    if questions.len() != 3 {
        return Err("exactly three security questions are required".into());
    }
    let mut out = [("", ""); 3].map(|_| (String::new(), String::new()));
    for (i, (q, a)) in questions.iter().enumerate() {
        let q_trim = q.trim();
        if q_trim.is_empty() || q_trim.chars().count() > MAX_QUESTION_CHARS {
            return Err(format!("question {} needs 1–{MAX_QUESTION_CHARS} characters", i + 1));
        }
        let norm = normalize_answer_v1(a);
        let points = norm.chars().count();
        if !(MIN_ANSWER_CHARS..=MAX_ANSWER_CHARS).contains(&points) {
            return Err(format!(
                "answer {} needs {MIN_ANSWER_CHARS}–{MAX_ANSWER_CHARS} characters",
                i + 1
            ));
        }
        out[i] = (q_trim.to_string(), norm);
    }
    for i in 0..3 {
        for j in (i + 1)..3 {
            if out[i].0 == out[j].0 {
                return Err("the three questions must be different".into());
            }
            if out[i].1 == out[j].1 {
                return Err("the three answers must be different".into());
            }
        }
    }
    Ok(out)
}

// ── parse + integrity ───────────────────────────────────────────────────────

fn parse_frozen_kdf(kdf: &KdfParams, salt_len: usize) -> Result<(), String> {
    if kdf.algo != "argon2id"
        || kdf.v != ARGON_V
        || kdf.m_kib != ARGON_M_KIB
        || kdf.t != ARGON_T
        || kdf.p != ARGON_P
    {
        // Unknown suite parameters are rejected outright — an attacker-
        // controlled file must never dictate memory/time costs (CR-24).
        return Err("unsupported kdf parameters".into());
    }
    crypto::b64_decode_canonical("kdf salt", &kdf.salt_b64, salt_len).map(|_| ())
}

/// Strict semantic validation of an already duplicate-free JSON value.
/// Length, enum, and cross-field rules mirror `keyvault-v3.schema.json` —
/// the schema is documentation; these checks are the enforcement.
pub fn parse_v3_value(doc: Value) -> Result<VaultFileV3, String> {
    let file: VaultFileV3 = serde_json::from_value(doc).map_err(|e| format!("v3 structure: {e}"))?;
    if file.version != VAULT_VERSION {
        return Err("not a v3 vault".into());
    }
    // vault_id: 32 lowercase hex chars == 16 bytes.
    if file.vault_id.len() != 32 || !file.vault_id.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err("vault_id malformed".into());
    }
    if file.revision == 0 || file.revision > MAX_SAFE_JSON_INT {
        return Err("revision out of range".into());
    }
    if file.crypto.suite != SUITE || file.crypto.manifest_mac != MANIFEST_MAC_ALGO {
        return Err("unsupported crypto suite".into());
    }
    // password
    parse_frozen_kdf(&file.password.kdf, 16)?;
    crypto::b64_decode_canonical("password wrap", &file.password.wrap.sealed_dek_b64, 48)?;
    crypto::b64_decode_canonical("password nonce", &file.password.wrap.nonce_b64, 12)?;
    // questions
    if let Some(q) = &file.questions {
        if q.scheme != QUESTIONS_SCHEME || q.normalization != ANSWER_NORMALIZATION {
            return Err("unsupported question scheme".into());
        }
        // Only the real threshold is parseable — a file claiming threshold=1
        // is malformed, not a hint (CR-33).
        if q.threshold != 2 || q.shares.len() != 3 {
            return Err("questions must be exactly 2-of-3".into());
        }
        let mut xs = [0u8; 3];
        for (i, s) in q.shares.iter().enumerate() {
            if s.id != format!("q{}", s.x) || !matches!(s.x, 1..=3) {
                return Err("share id/index mismatch".into());
            }
            xs[i] = s.x;
            if s.question.is_empty() || s.question.chars().count() > MAX_QUESTION_CHARS {
                return Err("question text out of range".into());
            }
            parse_frozen_kdf(&s.kdf, 16)?;
            crypto::b64_decode_canonical("masked share", &s.masked_share_b64, 32)?;
        }
        if xs[0] == xs[1] || xs[0] == xs[2] || xs[1] == xs[2] {
            return Err("share indices must be distinct".into());
        }
    }
    // recovery
    if let Some(r) = &file.recovery {
        if r.encoding != RECOVERY_ENCODING || r.entropy_bits != 160 {
            return Err("unsupported recovery encoding".into());
        }
        if r.id.len() != 32
            || !r.id.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err("recovery id malformed".into());
        }
        if r.binding != "none" && r.binding != "tpm_cng_v1" {
            return Err("unsupported device binding".into());
        }
        let binding = r.binding.clone();
        match (&r.device, binding.as_str()) {
            (None, "none") => {}
            (Some(d), "tpm_cng_v1") => {
                if d.provider != "windows-cng-platform" || d.algo != "rsa-oaep-sha256" {
                    return Err("unsupported device section".into());
                }
                // Schema bounds: key_id 16–128 chars, key_name 1–256.
                let kid = d.key_id.chars().count();
                let kname = d.key_name.chars().count();
                if !(16..=128).contains(&kid) || kname == 0 || kname > 256 {
                    return Err("device identifiers out of range".into());
                }
                crypto::b64_decode_canonical(
                    "device secret",
                    &d.sealed_device_secret_b64,
                    256, // RSA-2048 OAEP ciphertext
                )?;
            }
            _ => return Err("binding/device mismatch".into()),
        }
        crypto::b64_decode_canonical("recovery salt", &r.salt_b64, 16)?;
        crypto::b64_decode_canonical("recovery nonce", &r.wrap.nonce_b64, 12)?;
        crypto::b64_decode_canonical("recovery wrap", &r.wrap.sealed_dek_b64, 48)?;
    }
    // entries
    if file.entries.len() > MAX_ENTRIES {
        return Err("too many entries".into());
    }
    for e in &file.entries {
        if e.id.is_empty() || e.id.chars().count() > 128 {
            return Err("entry id out of range".into());
        }
        if e.service.is_empty() || e.service.chars().count() > 128 {
            return Err("entry service out of range".into());
        }
        if e.label.is_empty() || e.label.chars().count() > 256 {
            return Err("entry label out of range".into());
        }
        if e.masked.chars().count() > 256 {
            return Err("entry masked out of range".into());
        }
        if e.created_at < 0 || e.created_at > MAX_SAFE_JSON_INT as i64 {
            return Err("entry created_at out of range".into());
        }
        if e.public_note.as_ref().is_some_and(|n| n.chars().count() > 4096) {
            return Err("public note out of range".into());
        }
        if e.payload_schema != ENTRY_PAYLOAD_SCHEMA {
            return Err("unsupported entry payload schema".into());
        }
        crypto::b64_decode_canonical("entry nonce", &e.nonce_b64, 12)?;
        // Entry payloads are variable-length; canonicality is the check.
        let payload = b64()
            .decode(&e.sealed_payload_b64)
            .map_err(|_| "entry payload malformed".to_string())?;
        if b64().encode(&payload) != e.sealed_payload_b64 {
            return Err("entry payload: non-canonical base64".into());
        }
    }
    if file.integrity.canonicalization != CANONICALIZATION {
        return Err("unsupported canonicalization".into());
    }
    crypto::b64_decode_canonical("integrity mac", &file.integrity.mac_b64, 32)?;
    Ok(file)
}

fn vault_id_bytes(file: &VaultFileV3) -> [u8; 16] {
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = u8::from_str_radix(&file.vault_id[i * 2..i * 2 + 2], 16).expect("checked hex");
    }
    out
}

/// Public form of [`vault_id_bytes`] for callers outside the module.
pub fn vault_id_of(file: &VaultFileV3) -> [u8; 16] {
    vault_id_bytes(file)
}

/// Canonical MAC input: the whole file minus the `integrity` object.
fn canonical_bytes(file: &VaultFileV3) -> Result<Vec<u8>, String> {
    let mut value = serde_json::to_value(file).map_err(|e| e.to_string())?;
    value.as_object_mut().ok_or("vault is not an object")?.remove("integrity");
    canonical_json(&value)
}

/// Recompute `integrity` from current content (bump revision first when the
/// change is a mutation).
pub fn reseal_integrity(file: &mut VaultFileV3, dek: &[u8; 32]) -> Result<(), String> {
    let vid = vault_id_bytes(file);
    let canonical = canonical_bytes(file)?;
    let mac = compute_manifest_mac(&manifest_key(dek, &vid), &canonical);
    file.integrity = IntegritySection {
        canonicalization: CANONICALIZATION.into(),
        mac_b64: b64().encode(mac),
    };
    Ok(())
}

/// Verify the whole-file MAC under a DEK candidate. The ONLY acceptance
/// test for any recovered DEK — no per-question or per-wrap oracle exists.
pub fn verify_integrity(file: &VaultFileV3, dek: &[u8; 32]) -> Result<(), String> {
    let vid = vault_id_bytes(file);
    let canonical = canonical_bytes(file)?;
    verify_manifest_mac(&manifest_key(dek, &vid), &canonical, &file.integrity.mac_b64)
}

/// Internal alias used across construction helpers.
fn seal_integrity(file: &mut VaultFileV3, dek: &[u8; 32]) -> Result<(), String> {
    reseal_integrity(file, dek)
}

// ── AAD builders over parsed sections ───────────────────────────────────────

fn aad_password(file: &VaultFileV3) -> Vec<u8> {
    let k = &file.password.kdf;
    // Salt is hex-checked upstream; decode failure cannot happen post-parse.
    let salt = b64().decode(&k.salt_b64).unwrap_or_default();
    crypto::aad_password_wrap(
        file.version,
        &vault_id_bytes(file),
        &k.algo,
        k.v,
        &salt,
        k.m_kib,
        k.t,
        k.p,
    )
}

/// Recovery-wrap AAD — the single builder shared by the *writer*
/// (`enable_recovery_code`, where the section does not exist yet, so the
/// fields are still local values) and the *reader* (from the parsed
/// section). One function means the two sides cannot drift apart.
fn aad_recovery_wrap(
    version: u32,
    vault_id: &[u8; 16],
    recovery_id_hex: &str,
    binding: &str,
    salt: &[u8; 16],
) -> Vec<u8> {
    tlv(&[
        crypto::AAD_WRAP_RECOVERY,
        &version.to_be_bytes(),
        vault_id,
        recovery_id_hex.as_bytes(),
        binding.as_bytes(),
        salt,
    ])
}

fn aad_recovery(file: &VaultFileV3) -> Result<Vec<u8>, String> {
    let r = file.recovery.as_ref().ok_or("no recovery section")?;
    let salt = b64().decode(&r.salt_b64).map_err(|_| "recovery salt malformed".to_string())?;
    let Ok(salt_arr) = salt.as_slice().try_into() else {
        return Err("recovery salt malformed".to_string());
    };
    Ok(aad_recovery_wrap(file.version, &vault_id_bytes(file), &r.id, &r.binding, &salt_arr))
}

// ── construction (deterministic cores + fresh wrappers) ─────────────────────

/// Explicit randomness for tests; production uses [`init_vault`].
pub struct InitMaterial {
    pub dek: [u8; 32],
    pub vault_id: [u8; 16],
    pub pw_salt: [u8; 16],
    pub pw_nonce: [u8; 12],
    pub share_slope: [u8; 32],
    pub question_salts: [[u8; 16]; 3],
}

pub fn init_vault_with(
    mat: &InitMaterial,
    unlock_code: &str,
    entries: &[PlainEntry],
    questions: Option<&[(String, String)]>,
) -> Result<VaultFileV3, String> {
    let code = normalize_password_v3(unlock_code);
    validate_unlock_code(&code)?;
    let k_pw_base = argon2id_frozen(code.as_bytes(), &mat.pw_salt)?;
    let k_pw = password_wrap_key(&k_pw_base, &mat.vault_id);
    let pw_aad = tlv(&[
        crypto::AAD_WRAP_PASSWORD,
        &VAULT_VERSION.to_be_bytes(),
        &mat.vault_id,
        b"argon2id",
        &ARGON_V.to_be_bytes(),
        &mat.pw_salt,
        &ARGON_M_KIB.to_be_bytes(),
        &ARGON_T.to_be_bytes(),
        &ARGON_P.to_be_bytes(),
    ]);
    let sealed_dek = aead_seal(&k_pw, mat.pw_nonce, &mat.dek, &pw_aad);

    let questions_section = match questions {
        None => None,
        Some(qa) => Some(build_questions_with(
            &mat.dek,
            &mat.vault_id,
            &mat.share_slope,
            &mat.question_salts,
            qa,
        )?),
    };

    let mut file = VaultFileV3 {
        version: VAULT_VERSION,
        vault_id: hex_encode(&mat.vault_id),
        revision: 1,
        crypto: CryptoSection { suite: SUITE.into(), manifest_mac: MANIFEST_MAC_ALGO.into() },
        password: PasswordSection {
            kdf: frozen_kdf_section(&mat.pw_salt),
            wrap: WrapSection {
                nonce_b64: b64().encode(mat.pw_nonce),
                sealed_dek_b64: b64().encode(&sealed_dek),
            },
        },
        questions: questions_section,
        recovery: None,
        entries: entries
            .iter()
            .map(|e| seal_entry_with(&mat.dek, &mat.vault_id, e, fresh_nonce()))
            .collect::<Result<Vec<_>, String>>()?,
        integrity: IntegritySection {
            canonicalization: CANONICALIZATION.into(),
            mac_b64: String::new(),
        },
    };
    seal_integrity(&mut file, &mat.dek)?;
    Ok(file)
}

/// Production init: all randomness from the OS CSPRNG.
pub fn init_vault(
    unlock_code: &str,
    entries: &[PlainEntry],
    questions: Option<&[(String, String)]>,
) -> Result<(VaultFileV3, [u8; 32]), String> {
    let mat = InitMaterial {
        dek: fresh_random(),
        vault_id: fresh_random(),
        pw_salt: fresh_random(),
        pw_nonce: fresh_nonce(),
        share_slope: fresh_random(),
        question_salts: [fresh_random(), fresh_random(), fresh_random()],
    };
    let file = init_vault_with(&mat, unlock_code, entries, questions)?;
    Ok((file, mat.dek))
}

fn frozen_kdf_section(salt: &[u8; 16]) -> KdfParams {
    KdfParams {
        algo: "argon2id".into(),
        v: ARGON_V,
        salt_b64: b64().encode(salt),
        m_kib: ARGON_M_KIB,
        t: ARGON_T,
        p: ARGON_P,
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// HKDF info for one question's share mask. Single source for the build and
/// unlock sides: build passes the freshly-minted `"q{x}"` id, unlock passes
/// the parsed `share.id` — equivalent only because `parse_v3_value` enforces
/// `id == "q{x}"`; keeping one function means they cannot drift apart.
fn question_mask_info(id: &str, x: u8) -> Vec<u8> {
    tlv(&[crypto::INFO_QUESTION_MASK, id.as_bytes(), &[x]])
}

fn build_questions_with(
    dek: &[u8; 32],
    vault_id: &[u8; 16],
    slope: &[u8; 32],
    salts: &[[u8; 16]; 3],
    qa: &[(String, String)],
) -> Result<QuestionsSection, String> {
    let normalized = validate_three_questions(qa)?;
    let mut shares = Vec::with_capacity(3);
    for (i, (question, answer)) in normalized.iter().enumerate() {
        let x = (i + 1) as u8;
        let share = shamir_share_at(dek, slope, x);
        let base = argon2id_frozen(answer.as_bytes(), &salts[i])?;
        let mask = hkdf32(
            &base,
            vault_id,
            &question_mask_info(&format!("q{x}"), x),
        );
        let masked: Vec<u8> = share.iter().zip(mask.iter()).map(|(s, m)| s ^ m).collect();
        shares.push(ShareEntry {
            id: format!("q{x}"),
            question: question.clone(),
            x,
            kdf: frozen_kdf_section(&salts[i]),
            masked_share_b64: b64().encode(&masked),
        });
    }
    Ok(QuestionsSection {
        scheme: QUESTIONS_SCHEME.into(),
        threshold: 2,
        normalization: ANSWER_NORMALIZATION.into(),
        shares,
    })
}

fn seal_entry_with(
    dek: &[u8; 32],
    vault_id: &[u8; 16],
    e: &PlainEntry,
    nonce: [u8; 12],
) -> Result<EntryV3, String> {
    let payload = EntryPayload { key: e.key.clone(), private_note: Some(e.note.clone()) };
    let pt = serde_json::to_vec(&payload).map_err(|er| er.to_string())?;
    let aad = aad_entry(VAULT_VERSION, vault_id, (&e.id, &e.service, ENTRY_PAYLOAD_SCHEMA));
    let k_entry = entry_key(dek, vault_id);
    let sealed = aead_seal(&k_entry, nonce, &pt, &aad);
    Ok(EntryV3 {
        id: e.id.clone(),
        service: e.service.clone(),
        label: e.label.clone(),
        masked: super::keyvault::mask_key(&e.key),
        created_at: e.created_at,
        public_note: None,
        payload_schema: ENTRY_PAYLOAD_SCHEMA.into(),
        nonce_b64: b64().encode(nonce),
        sealed_payload_b64: b64().encode(&sealed),
    })
}

/// Production entry sealing with a fresh CSPRNG nonce.
pub fn seal_entry_v3(
    dek: &[u8; 32],
    vault_id: &[u8; 16],
    e: &PlainEntry,
) -> Result<EntryV3, String> {
    seal_entry_with(dek, vault_id, e, fresh_nonce())
}

pub fn open_entry_v3(dek: &[u8; 32], vault_id: &[u8; 16], e: &EntryV3) -> Result<PlainEntry, String> {
    let aad = aad_entry(VAULT_VERSION, vault_id, (&e.id, &e.service, ENTRY_PAYLOAD_SCHEMA));
    let k_entry = entry_key(dek, vault_id);
    let pt = aead_open(&k_entry, &e.nonce_b64, &e.sealed_payload_b64, &aad)?;
    let payload: EntryPayload = serde_json::from_slice(&pt).map_err(|_| "entry payload malformed".to_string())?;
    Ok(PlainEntry {
        id: e.id.clone(),
        service: e.service.clone(),
        label: e.label.clone(),
        key: payload.key,
        note: payload.private_note.unwrap_or_default(),
        created_at: e.created_at,
    })
}

pub fn open_all_v3(dek: &[u8; 32], file: &VaultFileV3) -> Result<Vec<PlainEntry>, String> {
    let vid = vault_id_bytes(file);
    file.entries.iter().map(|e| open_entry_v3(dek, &vid, e)).collect()
}

// ── unlock paths ────────────────────────────────────────────────────────────

/// Unlock-code path: derive KEK → unwrap DEK → verify the manifest MAC.
/// Any failure is the same generic error (no oracle for which step failed).
/// Policy (length/blocklist) is a CREATION-time rule only — a vault built
/// with a code that a future, stricter policy would reject must still open.
pub fn unlock_with_password(file: &VaultFileV3, code: &str) -> Result<[u8; 32], String> {
    let code = normalize_password_v3(code);
    let salt = b64()
        .decode(&file.password.kdf.salt_b64)
        .map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    let k_pw_base = argon2id_frozen(code.as_bytes(), &salt).map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    let k_pw = password_wrap_key(&k_pw_base, &vault_id_bytes(file));
    let dek = aead_open(&k_pw, &file.password.wrap.nonce_b64, &file.password.wrap.sealed_dek_b64, &aad_password(file))
        .map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    let dek: [u8; 32] = dek.try_into().map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    verify_integrity(file, &dek).map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    Ok(dek)
}

/// Question path: derive masks for the PROVIDED answers only, try every
/// C(provided,2) pair, accept a candidate DEK solely on the manifest MAC.
/// `answers` are (question_id, raw user input); order is irrelevant and the
/// mapping is by explicit id, never by array position.
pub fn unlock_with_questions(
    file: &VaultFileV3,
    answers: &[(String, String)],
) -> Result<[u8; 32], String> {
    let questions = file.questions.as_ref().ok_or(ERR_UNLOCK_FAILED)?;
    let vid = vault_id_bytes(file);
    if answers.len() < 2 {
        // Not enough input — fail before any expensive KDF work (CR-16).
        return Err(ERR_UNLOCK_FAILED.into());
    }
    let mut recovered: Vec<(u8, [u8; 32])> = Vec::with_capacity(answers.len());
    for (id, raw) in answers.iter().take(3) {
        let Some(share) = questions.shares.iter().find(|s| &s.id == id) else {
            return Err(ERR_UNLOCK_FAILED.into());
        };
        let salt = b64().decode(&share.kdf.salt_b64).map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
        let base = argon2id_frozen(normalize_answer_v1(raw).as_bytes(), &salt)
            .map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
        let mask = hkdf32(
            &base,
            &vid,
            &question_mask_info(&share.id, share.x),
        );
        let masked = b64().decode(&share.masked_share_b64).map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
        if masked.len() != 32 {
            return Err(ERR_UNLOCK_FAILED.into());
        }
        let mut unmasked = [0u8; 32];
        for j in 0..32 {
            unmasked[j] = masked[j] ^ mask[j];
        }
        recovered.push((share.x, unmasked));
    }
    for i in 0..recovered.len() {
        for j in (i + 1)..recovered.len() {
            if let Ok(candidate) = shamir_combine(recovered[i].0, &recovered[i].1, recovered[j].0, &recovered[j].1) {
                if verify_integrity(file, &candidate).is_ok() {
                    return Ok(candidate);
                }
            }
        }
    }
    Err(ERR_UNLOCK_FAILED.into())
}

/// Recovery-code path: checksum-decode → HKDF → unwrap → MAC.
pub fn unlock_with_recovery_code(file: &VaultFileV3, code_input: &str) -> Result<[u8; 32], String> {
    let r = file.recovery.as_ref().ok_or(ERR_UNLOCK_FAILED)?;
    if r.binding != "none" {
        // TPM-bound strict mode: the code alone is not sufficient by design.
        return Err(ERR_UNLOCK_FAILED.into());
    }
    let normalized = crypto::recovery_code_normalize(code_input);
    let code = recovery_code_decode(&normalized).map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    let vid = vault_id_bytes(file);
    let mut rid = [0u8; 16];
    for i in 0..16 {
        rid[i] = u8::from_str_radix(&r.id[i * 2..i * 2 + 2], 16).map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    }
    let salt = b64().decode(&r.salt_b64).map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    let mut salt_arr = [0u8; 16];
    salt_arr.copy_from_slice(&salt);
    let kek = recovery_code_key(&code, &vid, &rid, &salt_arr);
    let dek = aead_open(&kek, &r.wrap.nonce_b64, &r.wrap.sealed_dek_b64, &aad_recovery(file)?)
        .map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    let dek: [u8; 32] = dek.try_into().map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    verify_integrity(file, &dek).map_err(|_| ERR_UNLOCK_FAILED.to_string())?;
    Ok(dek)
}

// ── rotations (same DEK, re-wrapped) ────────────────────────────────────────

/// Change the unlock code. DEK, entries, questions and recovery wraps stay
/// byte-identical — only the password section is re-created (CR-21).
pub fn change_unlock_code(file: &VaultFileV3, dek: &[u8; 32], new_code: &str) -> Result<VaultFileV3, String> {
    let code = normalize_password_v3(new_code);
    validate_unlock_code(&code)?;
    let mut out = file.clone();
    out.revision += 1;
    let salt: [u8; 16] = fresh_random();
    let base = argon2id_frozen(code.as_bytes(), &salt)?;
    let k_pw = password_wrap_key(&base, &vault_id_bytes(file));
    let nonce = fresh_nonce();
    let aad = crypto::aad_password_wrap(
        VAULT_VERSION,
        &vault_id_bytes(file),
        "argon2id",
        ARGON_V,
        &salt,
        ARGON_M_KIB,
        ARGON_T,
        ARGON_P,
    );
    let sealed = aead_seal(&k_pw, nonce, dek, &aad);
    out.password = PasswordSection {
        kdf: frozen_kdf_section(&salt),
        wrap: WrapSection { nonce_b64: b64().encode(nonce), sealed_dek_b64: b64().encode(&sealed) },
    };
    seal_integrity(&mut out, dek)?;
    Ok(out)
}

/// Replace ALL three questions (never a single share in place). Same DEK;
/// the old answer set stops working immediately (CR-20).
pub fn replace_questions(
    file: &VaultFileV3,
    dek: &[u8; 32],
    qa: &[(String, String)],
) -> Result<VaultFileV3, String> {
    let mut out = file.clone();
    out.revision += 1;
    let slope: [u8; 32] = fresh_random();
    let salts = [fresh_random::<16>(), fresh_random::<16>(), fresh_random::<16>()];
    out.questions = Some(build_questions_with(dek, &vault_id_bytes(file), &slope, &salts, qa)?);
    seal_integrity(&mut out, dek)?;
    Ok(out)
}

pub fn disable_questions(mut file: VaultFileV3, dek: &[u8; 32]) -> Result<VaultFileV3, String> {
    file.revision += 1;
    file.questions = None;
    seal_integrity(&mut file, dek)?;
    Ok(file)
}

/// Generate a fresh recovery code (binding=none until the Phase-4 TPM
/// prototype proves strict mode on real hardware). Returns the printed
/// code exactly once — it is never stored or returned again.
pub fn enable_recovery_code(file: &VaultFileV3, dek: &[u8; 32]) -> Result<(VaultFileV3, String), String> {
    let code_bytes: [u8; 20] = fresh_random();
    let printed = recovery_code_encode(&code_bytes);
    let mut out = file.clone();
    out.revision += 1;
    let vid = vault_id_bytes(file);
    let rid: [u8; 16] = fresh_random();
    let salt: [u8; 16] = fresh_random();
    let kek = recovery_code_key(&code_bytes, &vid, &rid, &salt);
    let aad = aad_recovery_wrap(VAULT_VERSION, &vid, &hex_encode(&rid), "none", &salt);
    let nonce = fresh_nonce();
    let sealed = aead_seal(&kek, nonce, dek, &aad);
    out.recovery = Some(RecoverySectionV3 {
        id: hex_encode(&rid),
        encoding: RECOVERY_ENCODING.into(),
        entropy_bits: 160,
        salt_b64: b64().encode(salt),
        binding: "none".into(),
        device: None,
        wrap: WrapSection {
            nonce_b64: b64().encode(nonce),
            sealed_dek_b64: b64().encode(&sealed),
        },
    });
    seal_integrity(&mut out, dek)?;
    Ok((out, printed))
}

pub fn disable_recovery(mut file: VaultFileV3, dek: &[u8; 32]) -> Result<VaultFileV3, String> {
    file.revision += 1;
    file.recovery = None;
    seal_integrity(&mut file, dek)?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keyvault_v3::crypto::{
        recovery_code_normalize, shamir_share_at, verify_manifest_mac, INFO_QUESTION_MASK,
    };

    fn plain_entries() -> Vec<PlainEntry> {
        vec![
            PlainEntry {
                id: "tavily-1".into(),
                service: "tavily".into(),
                label: "Tavily key 1".into(),
                key: "tvly-secret-one".into(),
                note: "main".into(),
                created_at: 1,
            },
            PlainEntry {
                id: "bocha-1".into(),
                service: "bocha".into(),
                label: "BochaAI".into(),
                key: "bocha-secret-two".into(),
                note: String::new(),
                created_at: 2,
            },
        ]
    }

    fn fixed_material() -> InitMaterial {
        InitMaterial {
            dek: [0x11; 32],
            vault_id: *b"0123456789abcdef",
            pw_salt: [0x22; 16],
            pw_nonce: [0x33; 12],
            share_slope: [0x44; 32],
            question_salts: [[0x55; 16], [0x56; 16], [0x57; 16]],
        }
    }

    fn qa3() -> Vec<(String, String)> {
        vec![
            ("你最喜欢的水果是什么？".into(), "Apple".into()),
            ("你最常用的编程工具是什么？".into(), "Rust Lang".into()),
            ("第一只宠物的名字是什么？".into(), "Mochi".into()),
        ]
    }

    fn make_vault(questions: bool) -> (VaultFileV3, [u8; 32]) {
        let mat = fixed_material();
        let qa = qa3();
        let file = init_vault_with(
            &mat,
            "correct-horse-battery-42",
            &plain_entries(),
            if questions { Some(qa.as_slice()) } else { None },
        )
        .unwrap();
        (file, mat.dek)
    }

    // CR-01 — the DEK is CSPRNG material, not a password derivation.
    #[test]
    fn dek_is_random_not_password_derived() {
        let (file, dek) = make_vault(false);
        let salt = b64().decode(&file.password.kdf.salt_b64).unwrap();
        let derived = argon2id_frozen(normalize_password_v3("correct-horse-battery-42").as_bytes(), &salt).unwrap();
        assert_ne!(dek, derived);
        assert_eq!(dek.len(), 32);
    }

    // Happy path: init → parse (round-trip through JSON) → unlock.
    #[test]
    fn init_parse_unlock_roundtrip() {
        let (file, dek) = make_vault(true);
        let json = serde_json::to_string(&file).unwrap();
        let doc: Value = serde_json::from_str(&json).unwrap();
        let parsed = parse_v3_value(doc).unwrap();
        assert_eq!(parsed, file);
        let dek2 = unlock_with_password(&parsed, "correct-horse-battery-42").unwrap();
        assert_eq!(dek, dek2);
        // Entries decrypt to the originals.
        let opened = open_all_v3(&dek2, &parsed).unwrap();
        assert_eq!(opened[0].key, "tvly-secret-one");
        assert_eq!(opened[0].note, "main");
    }

    // CR-11 / CR-12 / CR-13 — any two of the three questions work.
    #[test]
    fn any_two_questions_recover_the_dek() {
        let (file, dek) = make_vault(true);
        let all = qa3();
        let pairs = [(0, 1), (0, 2), (1, 2)];
        for (a, b) in pairs {
            let got = unlock_with_questions(
                &file,
                &[(format!("q{}", a + 1), all[a].1.clone()), (format!("q{}", b + 1), all[b].1.clone())],
            )
            .unwrap();
            assert_eq!(got, dek);
        }
        // All three also works (any pair among them).
        let got = unlock_with_questions(
            &file,
            &all.iter()
                .enumerate()
                .map(|(i, (_, a))| (format!("q{}", i + 1), a.clone()))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(got, dek);
    }

    // CR-14 / CR-15 / CR-16 — one correct answer is never enough, zero
    // answers fail before any expensive work, and errors are uniform.
    #[test]
    fn single_or_zero_questions_fail_without_oracle() {
        let (file, _) = make_vault(true);
        // Exactly one right answer → fail.
        let err = unlock_with_questions(&file, &[("q1".into(), "apple".into())]).unwrap_err();
        assert_eq!(err, ERR_UNLOCK_FAILED);
        // Zero → fail (cheap).
        assert!(unlock_with_questions(&file, &[]).is_err());
        // Two with one wrong → fail, same message (no partial credit).
        let err = unlock_with_questions(
            &file,
            &[("q1".into(), "apple".into()), ("q2".into(), "wrong".into())],
        )
        .unwrap_err();
        assert_eq!(err, ERR_UNLOCK_FAILED);
    }

    // CR-10 / attack_verify_one_question — no per-question verifier exists:
    // a single share never produces MAC-able material and the file schema
    // carries no canary/tag/hash for questions (CR-19).
    #[test]
    fn file_schema_has_no_single_question_verifier() {
        let (file, _) = make_vault(true);
        let raw = serde_json::to_string(&file).unwrap();
        for banned in ["canary", "answer_hash", "answerHash", "share_tag", "master_b64", "backup_b64", "key_b64"] {
            assert!(!raw.contains(banned), "schema must not contain {banned}");
        }
        let q = file.questions.as_ref().unwrap();
        for s in &q.shares {
            let names: Vec<&str> = ["id", "question", "x", "kdf", "masked_share_b64"]
                .iter()
                .copied()
                .collect();
            let ser = serde_json::to_value(s).unwrap();
            let fields: Vec<&String> = ser.as_object().unwrap().keys().collect();
            assert_eq!(fields.len(), 5);
            for n in names {
                assert!(fields.contains(&&n.to_string()), "missing {n}");
            }
        }
    }

    // CR-18 — normalization rules apply on the question path too.
    #[test]
    fn question_answers_normalize_before_mask_derivation() {
        let (file, dek) = make_vault(true);
        let got = unlock_with_questions(
            &file,
            &[("q1".into(), "  APPLE  ".into()), ("q2".into(), "rust   LANG".into())],
        )
        .unwrap();
        assert_eq!(got, dek);
    }

    // CR-18a — duplicate questions/answers rejected at creation.
    #[test]
    fn duplicate_questions_or_answers_rejected() {
        let mat = fixed_material();
        assert!(init_vault_with(
            &mat,
            "correct-horse-battery-42",
            &[],
            Some(&[
                ("Q one?".into(), "apple".into()),
                ("Q one?".into(), "pear".into()), // duplicate text
                ("Q two?".into(), "mochi".into()),
            ]),
        )
        .is_err());
        assert!(init_vault_with(
            &mat,
            "correct-horse-battery-42",
            &[],
            Some(&[
                ("Q one?".into(), "apple".into()),
                ("Q two?".into(), "  Apple ".into()), // duplicate normalized answer
                ("Q three?".into(), "mochi".into()),
            ]),
        )
        .is_err());
        assert!(init_vault_with(&mat, "correct-horse-battery-42", &[], Some(&qa3()[..2])).is_err());
    }

    // CR-20 — replacing questions invalidates the old set, keeps the DEK.
    #[test]
    fn replace_questions_rotates_shares_not_dek() {
        let (file, dek) = make_vault(true);
        let new_qa = vec![
            ("新问题一？".into(), "answer-one".into()),
            ("新问题二？".into(), "answer-two".into()),
            ("新问题三？".into(), "answer-three".into()),
        ];
        let rotated = replace_questions(&file, &dek, &new_qa).unwrap();
        assert_ne!(rotated, file);
        assert!(rotated.revision > file.revision);
        // Old answers no longer work…
        assert!(unlock_with_questions(&rotated, &[("q1".into(), "apple".into()), ("q2".into(), "rust lang".into())]).is_err());
        // …the new set works from any two…
        assert_eq!(
            unlock_with_questions(&rotated, &[("q1".into(), "ANSWER-one".into()), ("q3".into(), "answer-three".into())]).unwrap(),
            dek
        );
        // …and entries were NOT re-encrypted.
        assert_eq!(rotated.entries, file.entries);
    }

    // CR-21 — password change: old fails, new works, other paths intact,
    // entries untouched.
    #[test]
    fn change_unlock_code_keeps_dek_and_other_paths() {
        let (file, dek) = make_vault(true);
        let (with_code, printed_code) = enable_recovery_code(&file, &dek).unwrap();
        let changed = change_unlock_code(&with_code, &dek, "a-much-newer-unlock-code-7").unwrap();
        assert!(changed.revision > with_code.revision);
        assert!(unlock_with_password(&changed, "correct-horse-battery-42").is_err());
        assert_eq!(unlock_with_password(&changed, "a-much-newer-unlock-code-7").unwrap(), dek);
        // Questions still recover the same DEK.
        assert_eq!(
            unlock_with_questions(&changed, &[("q1".into(), "apple".into()), ("q2".into(), "rust lang".into())]).unwrap(),
            dek
        );
        // Recovery code still unwraps the same DEK.
        assert_eq!(unlock_with_recovery_code(&changed, &printed_code).unwrap(), dek);
        // Entries untouched.
        assert_eq!(changed.entries, with_code.entries);
        // Everything except the password wrap survives byte-identical.
        assert_eq!(changed.questions, with_code.questions);
        assert_eq!(changed.recovery, with_code.recovery);
    }

    // CR-23 — unlock-code policy: ≥6 code points, blocklist, NO composition
    // rule (pure digits / pure letters are the user's call — the UI only
    // recommends mixed case + digits).
    #[test]
    fn unlock_code_policy_enforced() {
        assert!(validate_unlock_code(&normalize_password_v3("short")).is_err()); // 5 points
        assert!(validate_unlock_code(&normalize_password_v3("abc123")).is_err()); // blocklist
        assert!(validate_unlock_code(&normalize_password_v3("1357-2468-1357-2468-99")).is_ok()); // digits only
        assert!(validate_unlock_code(&normalize_password_v3("only-letters-no-digits-here")).is_ok());
        assert!(validate_unlock_code(&normalize_password_v3("zz9x7q")).is_ok()); // 6 mixed chars
        assert!(validate_unlock_code(&normalize_password_v3(&"汉字密".repeat(8))).is_ok());
        assert!(validate_unlock_code(&normalize_password_v3(&"x".repeat(2000))).is_err());
    }

    // Policy is creation-time only: unlocking never consults it, so a
    // future-stricter rule cannot orphan existing vaults (review MEDIUM fix).
    #[test]
    fn unlock_ignores_policy_but_creation_enforces_it() {
        let (file, dek) = make_vault(false);
        assert_eq!(unlock_with_password(&file, "correct-horse-battery-42").unwrap(), dek);
        // A code current policy would reject still reaches the crypto check
        // and fails there with the uniform error — never a policy refusal.
        assert_eq!(
            unlock_with_password(&file, "short").unwrap_err(),
            ERR_UNLOCK_FAILED
        );
    }

    // CR-25 — every sealed entry carries its own nonce.
    #[test]
    fn entries_get_distinct_nonces() {
        let (file, _) = make_vault(false);
        assert_eq!(file.entries.len(), 2);
        assert_ne!(file.entries[0].nonce_b64, file.entries[1].nonce_b64);
    }

    // CR-24 / CR-33 — parser rejects tampered KDF parameters and thresholds.
    #[test]
    fn parser_rejects_tampered_kdf_and_threshold() {
        let (file, _) = make_vault(true);
        let mut v = serde_json::to_value(&file).unwrap();
        v["password"]["kdf"]["m_kib"] = serde_json::json!(12345);
        assert!(parse_v3_value(v).is_err());
        let (file2, _) = make_vault(true);
        let mut v2 = serde_json::to_value(&file2).unwrap();
        v2["questions"]["threshold"] = serde_json::json!(1);
        assert!(parse_v3_value(v2).is_err());
    }

    // CR-26 — the manifest MAC covers every field; constant-time compare is
    // exercised by hmac::verify_slice on every failure path above.
    #[test]
    fn manifest_mac_catches_any_tamper() {
        fn set_path(v: &mut Value, keys: &[&str], val: Value) {
            let mut cur = v;
            for key in &keys[..keys.len() - 1] {
                cur = match cur {
                    Value::Object(map) => map.get_mut(*key).unwrap(),
                    Value::Array(list) => &mut list[key.parse::<usize>().unwrap()],
                    _ => unreachable!(),
                };
            }
            let last = keys[keys.len() - 1];
            match cur {
                Value::Object(map) => map.insert(last.to_string(), val),
                Value::Array(list) => {
                    list[last.parse::<usize>().unwrap()] = val;
                    return;
                }
                _ => unreachable!(),
            };
        }
        let (file, dek) = make_vault(true);
        verify_integrity(&file, &dek).unwrap();
        let swaps: Vec<(&[&str], Value)> = vec![
            (&["revision"], serde_json::json!(99)),
            (&["entries", "0", "label"], serde_json::json!("evil")),
            (
                &["questions", "shares", "0", "masked_share_b64"],
                serde_json::json!(file.questions.as_ref().unwrap().shares[1].masked_share_b64),
            ),
            (
                &["password", "wrap", "sealed_dek_b64"],
                serde_json::json!(b64().encode([7u8; 48])),
            ),
        ];
        for (path, val) in swaps {
            let mut doc = serde_json::to_value(&file).unwrap();
            set_path(&mut doc, path, val);
            let parsed = parse_v3_value(doc).unwrap();
            assert!(
                verify_integrity(&parsed, &dek).is_err(),
                "MAC must catch tampering at {path:?}"
            );
        }
    }

    // CR-27 — entry AEAD AAD binds metadata: swapping service/id under a
    // valid DEK still fails the open (even with the MAC skipped internally).
    #[test]
    fn entry_aad_binds_metadata() {
        let (file, dek) = make_vault(false);
        let vid = *b"0123456789abcdef";
        let mut e = file.entries[0].clone();
        e.service = "evil".into();
        assert!(open_entry_v3(&dek, &vid, &e).is_err());
        e.service = file.entries[0].service.clone();
        e.id = "other".into();
        assert!(open_entry_v3(&dek, &vid, &e).is_err());
    }

    // CR-30 — recovery code: right code unwraps, wrong code/vault fails.
    #[test]
    fn recovery_code_unwrap_and_rejection() {
        let (file, dek) = make_vault(false);
        let (rotated, code) = enable_recovery_code(&file, &dek).unwrap();
        assert_eq!(unlock_with_recovery_code(&rotated, &code).unwrap(), dek);
        // Confusable-tolerant input path.
        assert_eq!(unlock_with_recovery_code(&rotated, &recovery_code_normalize(&code)).unwrap(), dek);
        // Wrong code (checksum-valid? no — checksum fails first).
        assert!(unlock_with_recovery_code(&rotated, "AAAA-AAAA-AAAA-AAAA-AAAA-AAAA-AAAA-AAAAX").is_err());
        // A different vault rejects the code (id/salt mismatch → AEAD/MAC fail).
        let (other, _) = {
            let mut mat = fixed_material();
            mat.dek = [0x99; 32];
            mat.vault_id = *b"fedcba9876543210";
            let f = init_vault_with(&mat, "another-unlock-code-7", &[], None).unwrap();
            (f, mat.dek)
        };
        assert!(unlock_with_recovery_code(&other, &code).is_err());
    }

    // CR-31 / CR-32 — rotation revokes the old code on the new file.
    #[test]
    fn recovery_rotation_revokes_old_code() {
        let (file, dek) = make_vault(false);
        let (gen1, code1) = enable_recovery_code(&file, &dek).unwrap();
        assert_eq!(unlock_with_recovery_code(&gen1, &code1).unwrap(), dek);
        let (gen2, code2) = enable_recovery_code(&gen1, &dek).unwrap();
        assert_eq!(unlock_with_recovery_code(&gen2, &code2).unwrap(), dek);
        assert_ne!(code1, code2);
        // Old code against the CURRENT file → fail.
        assert!(unlock_with_recovery_code(&gen2, &code1).is_err());
        // Old code still opens the OLD snapshot (documented rollback limit,
        // IO-29: not preventable by the protocol).
        assert_eq!(unlock_with_recovery_code(&gen1, &code1).unwrap(), dek);
    }

    // CR-34 / attack_forge_other_shares — an attacker who rewrites the
    // questions with their own DEK-derived shares (and re-MACs everything)
    // still cannot read the original entries: K_entry derives from THEIR dek.
    #[test]
    fn forged_shares_and_self_mac_cannot_read_original_entries() {
        let (file, _real_dek) = make_vault(true);
        // Attacker's own world:
        let attacker_dek = [0xA7; 32];
        let mut evil = file.clone();
        evil.vault_id = file.vault_id.clone();
        let slope = [0xAB; 32];
        let salts = [[9u8; 16], [8u8; 16], [7u8; 16]];
        evil.questions = Some(
            build_questions_with(&attacker_dek, &vault_id_bytes(&file), &slope, &salts, &qa3())
                .unwrap(),
        );
        evil.revision += 1;
        seal_integrity(&mut evil, &attacker_dek).unwrap();
        // The file parses and its MAC verifies against the attacker DEK…
        let parsed = parse_v3_value(serde_json::to_value(&evil).unwrap()).unwrap();
        verify_integrity(&parsed, &attacker_dek).unwrap();
        // …but the ORIGINAL provider keys stay sealed (entry AEAD fails).
        let vid = vault_id_bytes(&parsed);
        for e in &parsed.entries {
            assert!(open_entry_v3(&attacker_dek, &vid, e).is_err());
            assert!(open_entry_v3(&_real_dek, &vid, e).is_ok());
        }
    }

    // CR-35 — replacing the password wrap with attacker material fails the
    // same way.
    #[test]
    fn forged_password_wrap_cannot_read_original_entries() {
        let (file, _real_dek) = make_vault(true);
        let attacker_dek = [0xB2; 32];
        let mut evil = file.clone();
        let salt = [0x77; 16];
        let base = argon2id_frozen(b"attacker code here", &salt).unwrap();
        let k_pw = password_wrap_key(&base, &vault_id_bytes(&file));
        let nonce = [5u8; 12];
        let mut aad_file = evil.clone();
        aad_file.password.kdf = frozen_kdf_section(&salt);
        let sealed = aead_seal(&k_pw, nonce, &attacker_dek, &aad_password(&aad_file));
        evil.password = PasswordSection {
            kdf: frozen_kdf_section(&salt),
            wrap: WrapSection { nonce_b64: b64().encode(nonce), sealed_dek_b64: b64().encode(&sealed) },
        };
        evil.revision += 1;
        seal_integrity(&mut evil, &attacker_dek).unwrap();
        let parsed = parse_v3_value(serde_json::to_value(&evil).unwrap()).unwrap();
        // Attacker can unlock their own wrap…
        assert_eq!(unlock_with_password(&parsed, "attacker code here").unwrap(), attacker_dek);
        // …but cannot read the original entries.
        let vid = vault_id_bytes(&parsed);
        for e in &parsed.entries {
            assert!(open_entry_v3(&attacker_dek, &vid, e).is_err());
        }
    }

    // attack_one_answer with random wrong guesses gives no signal beyond a
    // uniform error; masked share bytes leak nothing checkable per question.
    #[test]
    fn masked_share_is_useless_without_answers() {
        let (file, dek) = make_vault(true);
        let q = file.questions.as_ref().unwrap();
        let masked = b64().decode(&q.shares[0].masked_share_b64).unwrap();
        // The share alone must not equal the DEK or any plausible transform.
        assert_ne!(masked, dek);
        // Even knowing the correct answer for q1, a single share cannot
        // produce a MAC-able candidate: combine needs two points.
        let salt = b64().decode(&q.shares[0].kdf.salt_b64).unwrap();
        let base = argon2id_frozen(b"apple", &salt).unwrap();
        let mask = hkdf32(&base, &vault_id_bytes(&file), &question_mask_info("q1", 1));
        let mut s1 = [0u8; 32];
        for j in 0..32 {
            s1[j] = masked[j] ^ mask[j];
        }
        // Verify s1 IS the true share at x=1…
        assert_eq!(shamir_share_at(&dek, &[0x44; 32], 1), s1);
        // …yet no combination from one point exists; MAC rejects every guess.
        let mut guess = dek;
        guess[0] ^= 1;
        assert!(verify_manifest_mac(
            &manifest_key(&guess, &vault_id_bytes(&file)),
            &canonical_bytes(&file).unwrap(),
            &file.integrity.mac_b64
        )
        .is_err());
    }

    // IO-06 — structurally invalid protocol fields are rejected precisely.
    #[test]
    fn parser_rejects_unknown_fields_and_bad_values() {
        let (file, _) = make_vault(false);
        let mut v = serde_json::to_value(&file).unwrap();
        v["extra_field"] = serde_json::json!(1);
        assert!(parse_v3_value(v).is_err());
        let (file2, _) = make_vault(false);
        let mut v2 = serde_json::to_value(&file2).unwrap();
        v2["entries"][0]["created_at"] = serde_json::json!(-5);
        assert!(parse_v3_value(v2).is_err());
        let (file3, _) = make_vault(false);
        let mut v3 = serde_json::to_value(&file3).unwrap();
        v3["vault_id"] = serde_json::json!("UPPER");
        assert!(parse_v3_value(v3).is_err());
        // Integers beyond the JS-safe 2^53-1 bound are rejected (schema max).
        let (file4, _) = make_vault(false);
        let mut v4 = serde_json::to_value(&file4).unwrap();
        v4["revision"] = serde_json::json!(9_007_199_254_740_992u64);
        assert!(parse_v3_value(v4).is_err());
        let (file5, _) = make_vault(false);
        let mut v5 = serde_json::to_value(&file5).unwrap();
        v5["entries"][0]["created_at"] = serde_json::json!(9_007_199_254_740_992i64);
        assert!(parse_v3_value(v5).is_err());
    }

    // CR-12 (determinism) — same material builds the same file; different
    // vault_id changes masks (shares are vault-scoped, CR-17).
    #[test]
    fn shares_are_vault_scoped() {
        let (a, _) = make_vault(true);
        let mut mat = fixed_material();
        mat.vault_id = *b"ffffffffffffffff";
        let b = init_vault_with(&mat, "correct-horse-battery-42", &plain_entries(), Some(&qa3())).unwrap();
        let sa = &a.questions.as_ref().unwrap().shares;
        let sb = &b.questions.as_ref().unwrap().shares;
        assert_ne!(sa[0].masked_share_b64, sb[0].masked_share_b64);
    }
}

