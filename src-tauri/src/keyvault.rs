//! Unified API key vault — master-password encrypted at rest.
//!
//! v2 envelope (`keyvault.json`): per-entry metadata (service / label / note /
//! masked preview) stays readable so the locked vault still shows what it
//! holds, while each key's VALUE is sealed independently with AES-256-GCM
//! under a key derived from the master password via Argon2id (OWASP floor
//! params: 19 MiB / 2 / 1). A wrong password fails the AEAD tag check on the
//! embedded canary — no separate password hash is stored.
//!
//! A legacy v1 file (plain JSON array, the pre-password state) still loads;
//! setting a master password rewrites the file encrypted and it never goes
//! back. Programmatic reads (`keys_for_service`) need the unlocked session
//! key; while locked they return empty and callers fall back to their legacy
//! sources (the provider's own key file / env), which is where the working
//! copies live anyway.
//!
//! Fail-closed parsing (keyvault v3 work, Phase 1): every read classifies the
//! file as [`ReadStore`] — Missing / LegacyV1 / SealedV2 / SealedV3 /
//! Corrupted / UnsupportedVersion / IoFailure. A damaged or unrecognized file
//! is NEVER treated as an empty vault: writes refuse, listings error, and
//! only `status()` reports the problem in-band so the UI can show a
//! protective diagnosis instead of offering to overwrite.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;
use rand_core::{OsRng, RngCore};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const VAULT_VERSION: u32 = 2;
const ARGON_M_KIB: u32 = 19_456;
const ARGON_T: u32 = 2;
const ARGON_P: u32 = 1;
/// Sealed with the derived key so unlock can tell a wrong password from a
/// right one without touching any real entry.
const CANARY_PLAINTEXT: &[u8] = b"pane-vault-unlocked-v2";
const ERR_LOCKED: &str = "vault is locked — unlock it with the master password first";
const ERR_WRONG_PASSWORD: &str = "wrong master password";

static SESSION_KEY: Mutex<Option<[u8; 32]>> = Mutex::new(None);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VaultEntry {
    pub id: String,
    pub service: String,
    pub label: String,
    /// The raw secret. Never rendered by the UI — commands hand back a
    /// masked form for listing and the raw value only for clipboard copy.
    pub key: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub created_at: i64,
}

// ── on-disk v2 envelope ─────────────────────────────────────────────────────

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct KdfSection {
    algo: String, // "argon2id"
    salt_b64: String,
    m_kib: u32,
    t: u32,
    p: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SealedEntry {
    id: String,
    service: String,
    label: String,
    #[serde(default)]
    note: String,
    /// Pre-computed `mask_key` of the plaintext so the locked listing can
    /// still show the `sk-abc…wxyz` preview without the session key.
    masked: String,
    #[serde(default)]
    created_at: i64,
    nonce_b64: String,
    sealed_b64: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct VaultFile {
    version: u32,
    kdf: KdfSection,
    canary_nonce_b64: String,
    canary_b64: String,
    entries: Vec<SealedEntry>,
    /// Optional Q&A recovery ("forgot master password"): a random recovery
    /// key is sealed once per answer, under the vault key, and the master
    /// password under it. Absent on pre-recovery files.
    #[serde(default)]
    recovery: Option<RecoverySection>,
}

// ── Q&A recovery ─────────────────────────────────────────────────────────────
//
// The user picks N questions (default 2, minimum 1) whose answers only they
// know. A random 32-byte recovery key R is generated and sealed three ways:
//   - under Argon2id(each answer) — so ALL answers unseal R
//   - under the vault key — so change_password can re-seal the master
//     password under R without asking the questions again
// and the master password itself is sealed under R. Forgetting the password
// then means: answer every question → R → old password → set a new one.
// Each answer also gets its own canary so a wrong answer is told apart from
// a right one question-by-question without touching R.

/// Answers are normalized before use — a recovery flow is not the place to
/// fail on case or stray spaces.
fn normalize_answer(answer: &str) -> String {
    answer.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

const RECOVERY_MIN_QUESTIONS: usize = 1;
const RECOVERY_MAX_QUESTIONS: usize = 5;
const RECOVERY_FAIL_LIMIT: u32 = 5;
const RECOVERY_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(5 * 60);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct RecoveryQuestion {
    id: String,
    /// Stored plaintext: the challenge UI must render it while locked.
    question: String,
    answer_salt_b64: String,
    canary_nonce_b64: String,
    canary_b64: String,
    key_nonce_b64: String,
    key_b64: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct RecoverySection {
    questions: Vec<RecoveryQuestion>,
    /// R sealed under the vault key (lets change_password rotate the sealed
    /// master password without the answers).
    backup_nonce_b64: String,
    backup_b64: String,
    /// The master password sealed under R.
    master_nonce_b64: String,
    master_b64: String,
}

/// Failed reset attempts (process-wide): RECOVERY_FAIL_LIMIT misses inside
/// the window → cooldown. Answers are human words, not passwords — guessing
/// "apple" must not be free.
static RECOVERY_FAILS: Mutex<Option<(u32, std::time::Instant)>> = Mutex::new(None);

fn recovery_cooldown_remaining() -> Option<std::time::Duration> {
    let g = RECOVERY_FAILS.lock().ok()?;
    let (fails, first_fail) = (*g)?;
    if fails >= RECOVERY_FAIL_LIMIT {
        let elapsed = first_fail.elapsed();
        if elapsed < RECOVERY_COOLDOWN {
            return Some(RECOVERY_COOLDOWN - elapsed);
        }
    }
    None
}

fn note_recovery_failure() {
    if let Ok(mut g) = RECOVERY_FAILS.lock() {
        let entry = g.get_or_insert_with(|| (0, std::time::Instant::now()));
        entry.0 += 1;
    }
}

fn clear_recovery_failures() {
    if let Ok(mut g) = RECOVERY_FAILS.lock() {
        *g = None;
    }
}

fn answer_kdf(answer: &str, salt_b64: &str) -> Result<[u8; 32], String> {
    let kdf = KdfSection {
        algo: "argon2id".into(),
        salt_b64: salt_b64.into(),
        m_kib: ARGON_M_KIB,
        t: ARGON_T,
        p: ARGON_P,
    };
    derive_key(&normalize_answer(answer), &kdf)
}

fn validate_recovery_questions(questions: &[(String, String)]) -> Result<(), String> {
    if questions.len() < RECOVERY_MIN_QUESTIONS || questions.len() > RECOVERY_MAX_QUESTIONS {
        return Err(format!(
            "recovery needs {RECOVERY_MIN_QUESTIONS}–{RECOVERY_MAX_QUESTIONS} questions"
        ));
    }
    for (q, a) in questions {
        let q = q.trim();
        if q.is_empty() || q.chars().count() > 60 {
            return Err("each question needs 1–60 characters".into());
        }
        let a = normalize_answer(a);
        if a.chars().count() < 2 || a.chars().count() > 40 {
            return Err("each answer needs 2–40 characters (letters, not essays)".into());
        }
    }
    Ok(())
}

/// Build the recovery section: R sealed under every answer and under the
/// vault key, the master password sealed under R.
fn build_recovery(
    vault_key: &[u8; 32],
    password: &str,
    questions: &[(String, String)],
) -> Result<RecoverySection, String> {
    validate_recovery_questions(questions)?;
    let mut recovery_key = [0u8; 32];
    OsRng.fill_bytes(&mut recovery_key);
    let mut sealed_questions = Vec::with_capacity(questions.len());
    for (i, (q, a)) in questions.iter().enumerate() {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let answer_key = answer_kdf(a, &b64().encode(salt))?;
        let (canary_nonce_b64, canary_b64) = seal(&answer_key, CANARY_PLAINTEXT)?;
        let (key_nonce_b64, key_b64) = seal(&answer_key, &recovery_key)?;
        sealed_questions.push(RecoveryQuestion {
            id: format!("q{}", i + 1),
            question: q.trim().to_string(),
            answer_salt_b64: b64().encode(salt),
            canary_nonce_b64,
            canary_b64,
            key_nonce_b64,
            key_b64,
        });
    }
    let (backup_nonce_b64, backup_b64) = seal(vault_key, &recovery_key)?;
    let (master_nonce_b64, master_b64) = seal(&recovery_key, password.trim().as_bytes())?;
    Ok(RecoverySection { questions: sealed_questions, backup_nonce_b64, backup_b64, master_nonce_b64, master_b64 })
}

/// Every answer must pass its canary; then R unseals the master password.
/// Used by the forgot-password flow (which adds rate limiting) and tests.
fn try_recovery(recovery: &RecoverySection, answers: &[String]) -> Result<String, String> {
    if answers.len() != recovery.questions.len() {
        return Err("answer every question".into());
    }
    let mut recovery_key: Option<[u8; 32]> = None;
    for (q, answer) in recovery.questions.iter().zip(answers.iter()) {
        let answer_key = answer_kdf(answer, &q.answer_salt_b64)?;
        if open(&answer_key, &q.canary_nonce_b64, &q.canary_b64).is_err() {
            return Err("an answer did not match".into());
        }
        if recovery_key.is_none() {
            let raw = open(&answer_key, &q.key_nonce_b64, &q.key_b64)?;
            recovery_key =
                Some(raw.as_slice().try_into().map_err(|_| "recovery key is malformed".to_string())?);
        }
    }
    let Some(recovery_key) = recovery_key else {
        return Err("no recovery questions are set".into());
    };
    String::from_utf8(
        open(&recovery_key, &recovery.master_nonce_b64, &recovery.master_b64)
            .map_err(|_| "recovery data is unreadable".to_string())?,
    )
    .map_err(|_| "recovery data is malformed".to_string())
}

/// (Re)set the recovery questions. The master password is verified against
/// the on-disk file first — the caller just proved it, but the file is
/// what gets rewritten. v3 replaces the whole 3-question set (same DEK).
pub fn set_recovery(
    password: &str,
    questions: &[(String, String)],
) -> Result<VaultStatus, String> {
    set_recovery_at(&vault_path(), password, questions)
}

fn set_recovery_at(
    path: &Path,
    password: &str,
    questions: &[(String, String)],
) -> Result<VaultStatus, String> {
    match read_store_from(path) {
        ReadStore::SealedV2(mut file) => {
            // Verify the password against the file's canary (not the
            // session) so a stale unlocked session can never rewrite
            // recovery with a wrong password.
            let vault_key = derive_key(password.trim(), &file.kdf)?;
            open(&vault_key, &file.canary_nonce_b64, &file.canary_b64)?;
            let recovery = build_recovery(&vault_key, password, questions)?;
            file.recovery = Some(recovery);
            save_sealed_to(&path, &file)?;
            Ok(status_of(&ReadStore::SealedV2(file)))
        }
        ReadStore::SealedV3(file) => {
            let dek = crate::keyvault_v3::unlock_with_password(&file, password)?;
            let replaced = crate::keyvault_v3::replace_questions(&file, &dek, questions)?;
            save_v3_to(&path, &replaced)?;
            set_session_key(dek);
            Ok(status_of(&ReadStore::SealedV3(replaced)))
        }
        _ => return Err("set an unlock code before recovery questions".into()),
    }
}

/// The challenge surface: (id, question) pairs, readable while locked.
pub fn recovery_questions() -> Vec<(String, String)> {
    match read_store_from(&vault_path()) {
        ReadStore::SealedV2(file) => file
            .recovery
            .map(|r| r.questions.into_iter().map(|q| (q.id, q.question)).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Rotate the sealed master password under R and re-seal R under the new
/// vault key. Called from password changes; the answers and their seals are
/// untouched.
fn rotate_recovery_master(
    recovery: &mut RecoverySection,
    old_vault_key: &[u8; 32],
    new_vault_key: &[u8; 32],
    new_password: &str,
) -> Result<(), String> {
    let recovery_key = open(old_vault_key, &recovery.backup_nonce_b64, &recovery.backup_b64)?;
    let recovery_key: [u8; 32] =
        recovery_key.as_slice().try_into().map_err(|_| "recovery key is malformed".to_string())?;
    let (master_nonce_b64, master_b64) = seal(&recovery_key, new_password.trim().as_bytes())?;
    let (backup_nonce_b64, backup_b64) = seal(new_vault_key, &recovery_key)?;
    recovery.master_nonce_b64 = master_nonce_b64;
    recovery.master_b64 = master_b64;
    recovery.backup_nonce_b64 = backup_nonce_b64;
    recovery.backup_b64 = backup_b64;
    Ok(())
}

/// Forgot-password path: every answer must verify, then the sealed master
/// password comes back and is immediately used to set `new_password`
/// (re-sealing the whole vault). Never returns the old password to the
/// caller — the frontend only learns success/failure.
pub fn recovery_reset(answers: &[String], new_password: &str) -> Result<VaultStatus, String> {
    if let Some(left) = recovery_cooldown_remaining() {
        let mins = (left.as_secs() / 60).max(1);
        return Err(format!("too many wrong attempts — try again in {mins} min"));
    }
    let path = vault_path();
    let ReadStore::SealedV2(file) = read_store_from(&path) else {
        return Err("the v2 forgot-password flow needs a v2 vault (v3 has its own reset)".into());
    };
    let Some(recovery) = file.recovery.as_ref() else {
        return Err("no recovery questions are set".into());
    };
    let master = match try_recovery(recovery, answers) {
        Ok(master) => master,
        Err(e) => {
            note_recovery_failure();
            return Err(e);
        }
    };

    // change_password in one step, with the recovered password as the old
    // one — the caller never sees the old password, only success/failure.
    let old_vault_key = derive_key(master.trim(), &file.kdf)?;
    open(&old_vault_key, &file.canary_nonce_b64, &file.canary_b64)?;
    let entries = open_all_from(&file, &old_vault_key)?;
    let status = seal_and_store(&path, new_password, &entries, file.recovery.clone().map(|r| (r, old_vault_key)))?;
    clear_recovery_failures();
    Ok(status)
}

/// Like [`open_all`] but with an explicit key (recovery must not depend on
/// the in-memory session, which the forgot-password flow cannot have).
fn open_all_from(file: &VaultFile, key: &[u8; 32]) -> Result<Vec<VaultEntry>, String> {
    file.entries
        .iter()
        .map(|e| {
            Ok(VaultEntry {
                id: e.id.clone(),
                service: e.service.clone(),
                label: e.label.clone(),
                key: open_entry(key, e)?,
                note: e.note.clone(),
                created_at: e.created_at,
            })
        })
        .collect()
}

// ── fail-closed file classification (Phase 1) ───────────────────────────────

/// Upper bound for a vault file — a hostile or runaway file must be rejected
/// before parsing, not streamed into memory.
const MAX_VAULT_FILE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug)]
enum ReadStore {
    /// File does not exist (a true missing file, not a permission error).
    Missing,
    /// Pre-password plain JSON array; still readable, upgraded on demand.
    LegacyV1(Vec<VaultEntry>),
    SealedV2(VaultFile),
    SealedV3(crate::keyvault_v3::VaultFileV3),
    /// Exists but cannot be parsed as any known vault format. The payload is
    /// a stable reason code only — never file content.
    Corrupted(&'static str),
    /// Parses as JSON but carries a version this build does not know.
    UnsupportedVersion(u64),
    /// The file exists but cannot be read (permissions, locking, …).
    IoFailure(std::io::ErrorKind),
}

/// JSON parser that rejects duplicate object keys (a classic parser-confusion
/// vector against MAC'd formats: `{"mac":x,"mac":y}` must not silently
/// become "last one wins").
fn parse_json_no_duplicate_keys(raw: &str) -> Result<Value, String> {
    use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};

    struct NoDupValue;
    impl<'de> serde::de::DeserializeSeed<'de> for NoDupValue {
        type Value = Value;
        fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
            d.deserialize_any(NoDupVisitor)
        }
    }

    struct NoDupVisitor;
    impl<'de> Visitor<'de> for NoDupVisitor {
        type Value = Value;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("any JSON value")
        }
        fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
            Ok(Value::Bool(v))
        }
        fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
            Ok(Value::Number(v.into()))
        }
        fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
            Ok(Value::Number(v.into()))
        }
        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Value, E> {
            serde_json::Number::from_f64(v)
                .map(Value::Number)
                .ok_or_else(|| serde::de::Error::custom("non-finite float"))
        }
        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
            Ok(Value::String(v.into()))
        }
        fn visit_unit<E>(self) -> Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_none<E>(self) -> Result<Value, E> {
            Ok(Value::Null)
        }
        fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
            NoDupValue.deserialize(d)
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
            let mut out = Vec::new();
            while let Some(v) = a.next_element_seed(NoDupValue)? {
                out.push(v);
            }
            Ok(Value::Array(out))
        }
        fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
            let mut seen = std::collections::HashSet::new();
            let mut map = serde_json::Map::new();
            while let Some(key) = a.next_key::<String>()? {
                if !seen.insert(key.clone()) {
                    return Err(serde::de::Error::custom(format!("duplicate key: {key}")));
                }
                let value = a.next_value_seed(NoDupValue)?;
                map.insert(key, value);
            }
            Ok(Value::Object(map))
        }
    }

    serde_json::Deserializer::from_str(raw)
        .deserialize_any(NoDupVisitor)
        .map_err(|e| e.to_string())
}

fn read_store_from(path: &Path) -> ReadStore {
    read_store_with_limit(path, MAX_VAULT_FILE_BYTES)
}

fn read_store_with_limit(path: &Path, max_bytes: u64) -> ReadStore {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return ReadStore::Missing,
        Err(e) => return ReadStore::IoFailure(e.kind()),
    };
    if meta.len() > max_bytes {
        return ReadStore::Corrupted("file exceeds the size limit");
    }
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) => return ReadStore::IoFailure(e.kind()),
    };
    let doc = match parse_json_no_duplicate_keys(raw.trim_start_matches('\u{feff}')) {
        Ok(d) => d,
        Err(_) => return ReadStore::Corrupted("invalid json"),
    };
    match doc.get("version").and_then(Value::as_u64) {
        // The v1 format is a bare array WITHOUT a version field — a version-1
        // object is someone's broken hybrid, not a vault.
        Some(1) => ReadStore::Corrupted("v1 object is not a vault"),
        Some(2) => match serde_json::from_value::<VaultFile>(doc) {
            Ok(file) => ReadStore::SealedV2(file),
            Err(_) => ReadStore::Corrupted("v2 structure invalid"),
        },
        Some(3) => match crate::keyvault_v3::parse_v3_value(doc) {
            Ok(file) => ReadStore::SealedV3(file),
            Err(_) => ReadStore::Corrupted("v3 structure invalid"),
        },
        Some(other) => ReadStore::UnsupportedVersion(other),
        None => match serde_json::from_value::<Vec<VaultEntry>>(doc) {
            Ok(entries) => ReadStore::LegacyV1(entries),
            Err(_) => ReadStore::Corrupted("not a vault file"),
        },
    }
}

// ── crypto primitives ───────────────────────────────────────────────────────

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

fn derive_key(password: &str, kdf: &KdfSection) -> Result<[u8; 32], String> {
    let salt = b64()
        .decode(kdf.salt_b64.trim())
        .map_err(|e| format!("vault salt is unreadable: {e}"))?;
    let params = argon2::Params::new(kdf.m_kib, kdf.t, kdf.p, Some(32))
        .map_err(|e| format!("vault kdf params: {e}"))?;
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut out = [0u8; 32];
    argon
        .hash_password_into(password.as_bytes(), &salt, &mut out)
        .map_err(|e| format!("vault kdf failed: {e}"))?;
    Ok(out)
}

fn seal(key: &[u8; 32], plaintext: &[u8]) -> Result<(String, String), String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| "vault seal failed".to_string())?;
    Ok((b64().encode(nonce), b64().encode(ct)))
}

fn open(key: &[u8; 32], nonce_b64: &str, sealed_b64: &str) -> Result<Vec<u8>, String> {
    let nonce = b64().decode(nonce_b64).map_err(|e| e.to_string())?;
    let ct = b64().decode(sealed_b64).map_err(|e| e.to_string())?;
    if nonce.len() != 12 {
        return Err("vault nonce is malformed".into());
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(&nonce), ct.as_ref())
        .map_err(|_| ERR_WRONG_PASSWORD.to_string())
}

// ── persistence ─────────────────────────────────────────────────────────────

fn vault_path() -> PathBuf {
    crate::providers::config_dir().join("keyvault.json")
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents)
        .and_then(|()| std::fs::rename(&tmp, &path))
        .map_err(|e| format!("write keyvault.json: {e}"))?;
    let _ = crate::providers::onenewapi::store::restrict_owner_only(path);
    Ok(())
}

fn save_plain_to(path: &Path, entries: &[VaultEntry]) -> Result<(), String> {
    let text = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    write_atomic(path, &text)
}

fn save_sealed_to(path: &Path, file: &VaultFile) -> Result<(), String> {
    let text = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    write_atomic(path, &text)
}

fn seal_entry(key: &[u8; 32], e: &VaultEntry) -> Result<SealedEntry, String> {
    let (nonce_b64, sealed_b64) = seal(key, e.key.as_bytes())?;
    Ok(SealedEntry {
        id: e.id.clone(),
        service: e.service.clone(),
        label: e.label.clone(),
        note: e.note.clone(),
        masked: mask_key(&e.key),
        created_at: e.created_at,
        nonce_b64,
        sealed_b64,
    })
}

fn open_entry(key: &[u8; 32], e: &SealedEntry) -> Result<String, String> {
    let raw = open(key, &e.nonce_b64, &e.sealed_b64)?;
    String::from_utf8(raw).map_err(|e| e.to_string())
}

fn session_key() -> Option<[u8; 32]> {
    SESSION_KEY.lock().ok().and_then(|g| *g)
}

fn set_session_key(key: [u8; 32]) {
    if let Ok(mut g) = SESSION_KEY.lock() {
        *g = Some(key);
    }
}

/// Decrypt every entry (needs the unlocked session key).
fn open_all(file: &VaultFile) -> Result<Vec<VaultEntry>, String> {
    let key = session_key().ok_or_else(|| ERR_LOCKED.to_string())?;
    file.entries
        .iter()
        .map(|e| {
            Ok(VaultEntry {
                id: e.id.clone(),
                service: e.service.clone(),
                label: e.label.clone(),
                key: open_entry(&key, e)?,
                note: e.note.clone(),
                created_at: e.created_at,
            })
        })
        .collect()
}

// ── ids + masking (unchanged contract) ──────────────────────────────────────

/// Stable id: service + short digest of the key — seeding twice never
/// duplicates an entry, and re-adding the same key is a no-op update.
pub(crate) fn entry_id(service: &str, key: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in key.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{}-{:016x}", service.to_lowercase().replace([' ', '/'], "-"), h)
}

/// `sk-abc…wxyz` — head and tail visible, middle hidden.
pub fn mask_key(key: &str) -> String {
    let k = key.trim();
    if k.len() <= 10 {
        return "*".repeat(k.len().max(3));
    }
    format!("{}…{}", &k[..6], &k[k.len() - 4..])
}

fn upsert(entries: &mut Vec<VaultEntry>, service: &str, label: &str, key: &str, note: &str) {
    let key = key.trim();
    if key.is_empty() {
        return;
    }
    let id = entry_id(service, key);
    if let Some(existing) = entries.iter_mut().find(|e| e.id == id) {
        existing.label = label.to_string();
        return;
    }
    entries.push(VaultEntry {
        id,
        service: service.trim().to_string(),
        label: label.trim().to_string(),
        key: key.to_string(),
        note: note.to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
    });
}

// ── seeding: one-time import of keys already living elsewhere ──────────────

fn read_provider_key(file: &str) -> Option<String> {
    let p = crate::providers::config_dir().join(file);
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).ok()?).ok()?;
    v.get("apiKey")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn env_nonempty(var: &str) -> Option<String> {
    std::env::var(var).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// The active Firecrawl key: ZCode CLI's MCP config carries the literal
/// current key; env may still hold a legacy one.
fn firecrawl_seed_key() -> Option<String> {
    let cfg = dirs::home_dir()?.join(".zcode").join("cli").join("config.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(cfg).ok()?).ok()?;
    v.pointer("/mcp/servers/firecrawl/env/FIRECRAWL_API_KEY")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| env_nonempty("FIRECRAWL_FIRECRAWL_API_KEY"))
        .or_else(|| env_nonempty("FIRECRAWL_API_KEY"))
}

fn seed_entries(entries: &mut Vec<VaultEntry>) {
    // Tavily multi-key file
    let tavily_path = crate::providers::config_dir().join("tavily-keys.json");
    if let Ok(raw) = std::fs::read_to_string(&tavily_path) {
        if let Ok(v) = serde_json::from_str::<Value>(&raw) {
            for (i, k) in v
                .get("keys")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .enumerate()
            {
                upsert(entries, "tavily", &format!("Tavily key {}", i + 1), k, "imported");
            }
        }
    }
    if let Some(k) = read_provider_key("apigoto.json") {
        upsert(entries, "apigoto", "APIGOTO (RouterCode Free)", &k, "imported");
    }
    if let Some(k) = read_provider_key("clinepass.json") {
        upsert(entries, "clinepass", "ClinePass", &k, "imported");
    }
    if let Some(k) = env_nonempty("BOCHA_API_KEY") {
        upsert(entries, "bocha", "BochaAI", &k, "imported from env");
    }
    if let Some(k) = firecrawl_seed_key() {
        upsert(entries, "firecrawl", "Firecrawl (active)", &k, "imported");
    }
}

// ── public API (called by thin #[tauri::command] wrappers in lib.rs) ────────

#[derive(serde::Serialize)]
pub struct VaultRow {
    pub id: String,
    pub service: String,
    pub label: String,
    pub masked: String,
    pub note: String,
}

#[derive(serde::Serialize)]
pub struct VaultStatus {
    pub has_password: bool,
    pub unlocked: bool,
    pub count: usize,
    /// Set recovery questions; 0 = none (forgot-password path unavailable).
    pub recovery_questions: usize,
    /// v3 only: a recovery code is enrolled (one is printable per
    /// generation, never stored).
    pub recovery_code: bool,
    /// Lifecycle state for the v3 UI state machine: `uninitialized |
    /// legacy_plain | ok | corrupted | unsupported_version | io_error`.
    /// Extra fields are ignored by the current frontend.
    pub state: &'static str,
    pub version: Option<u32>,
    /// Stable reason code for problem states (`corrupted` carries the parse
    /// reason); never file content.
    pub issue_code: Option<&'static str>,
}

/// The unlocked flag must reflect a session that actually verifies against
/// THIS file — a v2 key left over from a since-replaced file, or a v3 DEK
/// against a swapped v3 file, must not read as "unlocked".
fn v2_session_for(file: &VaultFile) -> bool {
    let Some(key) = session_key() else { return false };
    open(&key, &file.canary_nonce_b64, &file.canary_b64).is_ok()
}

fn status_of(state: &ReadStore) -> VaultStatus {
    match state {
        ReadStore::Missing => VaultStatus {
            has_password: false,
            unlocked: true,
            count: 0,
            recovery_questions: 0,
            recovery_code: false,
            state: "uninitialized",
            version: None,
            issue_code: None,
        },
        ReadStore::LegacyV1(entries) => VaultStatus {
            has_password: false,
            unlocked: true,
            count: entries.len(),
            recovery_questions: 0,
            recovery_code: false,
            state: "legacy_plain",
            version: Some(1),
            issue_code: None,
        },
        ReadStore::SealedV2(file) => VaultStatus {
            has_password: true,
            unlocked: v2_session_for(file),
            count: file.entries.len(),
            recovery_questions: file.recovery.as_ref().map(|r| r.questions.len()).unwrap_or(0),
            recovery_code: false,
            state: "ok",
            version: Some(2),
            issue_code: None,
        },
        ReadStore::SealedV3(file) => VaultStatus {
            has_password: true,
            // The session DEK is only meaningful after a verified unlock.
            unlocked: v3_session_for(file).is_some(),
            count: file.entries.len(),
            recovery_questions: file.questions.as_ref().map(|q| q.shares.len()).unwrap_or(0),
            recovery_code: file.recovery.is_some(),
            state: "ok",
            version: Some(3),
            issue_code: None,
        },
        ReadStore::Corrupted(reason) => VaultStatus {
            // Protective mapping: a corrupted vault must look LOCKED to the
            // current UI, never like an empty vault awaiting initialization
            // (UI-07). Writes are refused everywhere regardless.
            has_password: true,
            unlocked: false,
            count: 0,
            recovery_questions: 0,
            recovery_code: false,
            state: "corrupted",
            version: None,
            issue_code: Some(reason),
        },
        ReadStore::UnsupportedVersion(v) => VaultStatus {
            has_password: true,
            unlocked: false,
            count: 0,
            recovery_questions: 0,
            recovery_code: false,
            state: "unsupported_version",
            version: Some(u32::try_from(*v).ok().unwrap_or(0)),
            issue_code: None,
        },
        ReadStore::IoFailure(_) => VaultStatus {
            has_password: true,
            unlocked: false,
            count: 0,
            recovery_questions: 0,
            recovery_code: false,
            state: "io_error",
            version: None,
            issue_code: None,
        },
    }
}

pub fn status() -> VaultStatus {
    status_of(&read_store_from(&vault_path()))
}

/// Errors on every problem state — the frontend's existing try/catch shows
/// the reason instead of ever rendering a fake empty vault.
pub fn list() -> Result<Vec<VaultRow>, String> {
    // Search-service keys live in a plaintext Pane-managed file, not the
    // vault — they are listed alongside the vault rows so Settings shows one
    // unified key list.
    let mut out = search_key_rows();
    out.extend(vault_rows()?);
    Ok(out)
}

fn search_key_rows() -> Vec<VaultRow> {
    crate::search_keys::list()
        .into_iter()
        .map(|k| VaultRow {
            masked: mask_key(&k.key),
            id: k.id,
            service: k.service,
            label: k.label,
            note: String::new(),
        })
        .collect()
}

fn vault_rows() -> Result<Vec<VaultRow>, String> {
    match read_store_from(&vault_path()) {
        ReadStore::Missing => Ok(Vec::new()),
        ReadStore::LegacyV1(mut entries) => {
            // Seed lazily on first listing; idempotent by construction.
            // Only pre-existing v1 files get this — a missing file is no
            // longer auto-created as plaintext (v3 requires an unlock code).
            let before = entries.len();
            seed_entries(&mut entries);
            if entries.len() != before {
                let _ = save_plain_to(&vault_path(), &entries);
            }
            Ok(entries.into_iter().map(row_from_plain).collect())
        }
        ReadStore::SealedV2(file) => Ok(file.entries.iter().map(row_from_sealed).collect()),
        ReadStore::SealedV3(file) => {
            // v3 notes are sealed, so the locked listing shows public
            // metadata only — same trust level as the v2 masked listing.
            Ok(file
                .entries
                .iter()
                .map(|e| VaultRow {
                    id: e.id.clone(),
                    service: e.service.clone(),
                    label: e.label.clone(),
                    masked: e.masked.clone(),
                    note: e.public_note.clone().unwrap_or_default(),
                })
                .collect())
        }
        ReadStore::Corrupted(_) => Err(
            "keyvault.json is corrupted — Pane will not touch it; restore a backup or remove the file manually"
                .into(),
        ),
        ReadStore::UnsupportedVersion(v) => {
            Err(format!("keyvault.json uses unsupported version {v} — upgrade Pane"))
        }
        ReadStore::IoFailure(_) => Err("keyvault.json is unreadable (I/O error)".into()),
    }
}

fn row_from_plain(e: VaultEntry) -> VaultRow {
    VaultRow { masked: mask_key(&e.key), id: e.id, service: e.service, label: e.label, note: e.note }
}

fn row_from_sealed(e: &SealedEntry) -> VaultRow {
    VaultRow {
        id: e.id.clone(),
        service: e.service.clone(),
        label: e.label.clone(),
        masked: e.masked.clone(),
        note: e.note.clone(),
    }
}

/// Set (or change) the master password and encrypt the whole vault with it.
/// Any previously set recovery questions are dropped — they sealed the OLD
/// password, and the set-password flow re-collects questions right after.
pub fn set_password(password: &str) -> Result<VaultStatus, String> {
    set_password_at(&vault_path(), password)
}

fn set_password_at(path: &Path, password: &str) -> Result<VaultStatus, String> {
    match read_store_from(path) {
        // Brand-new vaults start as v3 (random-DEK envelope; the 16+-code-
        // point policy is enforced inside init_vault). v1/v2 files keep
        // their own shape until the Phase-3 migration ships.
        ReadStore::Missing => {
            let (file, dek) = crate::keyvault_v3::init_vault(password, &[], None)?;
            save_v3_to(path, &file)?;
            set_session_key(dek);
            Ok(status_of(&ReadStore::SealedV3(file)))
        }
        ReadStore::LegacyV1(entries) => seal_and_store(path, password, &entries, None),
        ReadStore::SealedV2(file) => {
            let entries = open_all(&file)?;
            seal_and_store(path, password, &entries, None)
        }
        ReadStore::SealedV3(_) => {
            return Err("a v3 vault sets its unlock code at creation — use the migration flow".into())
        }
        ReadStore::Corrupted(_) => {
            return Err(
                "keyvault.json is corrupted — set-password refused to avoid overwriting it".into(),
            )
        }
        ReadStore::UnsupportedVersion(v) => return Err(format!("unsupported vault version {v}")),
        ReadStore::IoFailure(_) => return Err("keyvault.json is unreadable".into()),
    }
}

/// Change the master password: the old one must verify against the on-disk
/// canary first, then the whole vault is re-sealed under the new one — the
/// recovery questions survive with their seals rotated to the new password.
pub fn change_password(old: &str, new: &str) -> Result<VaultStatus, String> {
    let path = vault_path();
    match read_store_from(&path) {
        ReadStore::Missing | ReadStore::LegacyV1(_) => Err("no master password is set".into()),
        ReadStore::SealedV2(file) => {
            let old_key = derive_key(old.trim(), &file.kdf)?;
            open(&old_key, &file.canary_nonce_b64, &file.canary_b64)?;
            let entries = open_all_from(&file, &old_key)?;
            seal_and_store(&path, new, &entries, file.recovery.map(|r| (r, old_key)))
        }
        // v3: the DEK never changes — only the password wrap is re-created
        // (CR-21), so entries/questions/recovery stay byte-identical.
        ReadStore::SealedV3(file) => {
            let dek = crate::keyvault_v3::unlock_with_password(&file, old)?;
            let changed = crate::keyvault_v3::change_unlock_code(&file, &dek, new)?;
            save_v3_to(&path, &changed)?;
            set_session_key(dek);
            Ok(status_of(&ReadStore::SealedV3(changed)))
        }
        ReadStore::Corrupted(_) => Err("keyvault.json is corrupted — password change refused".into()),
        ReadStore::UnsupportedVersion(v) => Err(format!("unsupported vault version {v}")),
        ReadStore::IoFailure(_) => Err("keyvault.json is unreadable".into()),
    }
}

/// True while a verified key is held in memory (vault unlocked).
pub fn session_open() -> bool {
    SESSION_KEY.lock().map(|g| g.is_some()).unwrap_or(false)
}

/// Verify `password` against the on-disk canary and open the session — the
/// provider/account-key reveal gate shares this with the vault so users
/// face one password and one lock. Errors [`ERR_WRONG_PASSWORD`] on a
/// mismatch; "no master password is set" when none exists yet.
pub fn verify_for_reveal(password: &str) -> Result<(), String> {
    match read_store_from(&vault_path()) {
        ReadStore::Missing | ReadStore::LegacyV1(_) => Err("no master password is set".into()),
        ReadStore::SealedV2(file) => {
            let key = derive_key(password.trim(), &file.kdf)?;
            open(&key, &file.canary_nonce_b64, &file.canary_b64)?;
            set_session_key(key);
            Ok(())
        }
        ReadStore::SealedV3(file) => {
            let dek = crate::keyvault_v3::unlock_with_password(&file, password)?;
            set_session_key(dek);
            Ok(())
        }
        ReadStore::Corrupted(_) => Err("keyvault.json is corrupted — unlock refused".into()),
        ReadStore::UnsupportedVersion(v) => Err(format!("unsupported vault version {v}")),
        ReadStore::IoFailure(_) => Err("keyvault.json is unreadable".into()),
    }
}

/// Build + write the sealed vault. `keep` carries the previous recovery
/// section and the OLD vault key so its seals rotate to the new password;
/// `None` writes the file without recovery.
fn seal_and_store(
    path: &Path,
    password: &str,
    entries: &[VaultEntry],
    keep: Option<(RecoverySection, [u8; 32])>,
) -> Result<VaultStatus, String> {
    let password = password.trim();
    if password.len() < 4 {
        return Err("master password needs at least 4 characters".into());
    }
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let kdf = KdfSection {
        algo: "argon2id".into(),
        salt_b64: b64().encode(salt),
        m_kib: ARGON_M_KIB,
        t: ARGON_T,
        p: ARGON_P,
    };
    let key = derive_key(password, &kdf)?;
    let (canary_nonce_b64, canary_b64) = seal(&key, CANARY_PLAINTEXT)?;
    let mut sealed = Vec::with_capacity(entries.len());
    for e in entries {
        sealed.push(seal_entry(&key, e)?);
    }
    let mut recovery = None;
    if let Some((mut r, old_key)) = keep {
        rotate_recovery_master(&mut r, &old_key, &key, password)?;
        recovery = Some(r);
    }
    let file = VaultFile { version: VAULT_VERSION, kdf, canary_nonce_b64, canary_b64, entries: sealed, recovery };
    save_sealed_to(path, &file)?;
    set_session_key(key);
    Ok(status_of(&ReadStore::SealedV2(file)))
}

pub fn unlock(password: &str) -> Result<VaultStatus, String> {
    match read_store_from(&vault_path()) {
        // No password set yet — the vault is inherently open.
        ReadStore::Missing | ReadStore::LegacyV1(_) => Ok(status_of(&read_store_from(&vault_path()))),
        ReadStore::SealedV2(file) => {
            let key = derive_key(password.trim(), &file.kdf)?;
            open(&key, &file.canary_nonce_b64, &file.canary_b64)?;
            set_session_key(key);
            Ok(status_of(&ReadStore::SealedV2(file)))
        }
        ReadStore::SealedV3(file) => {
            let dek = crate::keyvault_v3::unlock_with_password(&file, password)?;
            set_session_key(dek);
            Ok(status_of(&ReadStore::SealedV3(file)))
        }
        ReadStore::Corrupted(_) => Err("keyvault.json is corrupted — unlock refused".into()),
        ReadStore::UnsupportedVersion(v) => Err(format!("unsupported vault version {v}")),
        ReadStore::IoFailure(_) => Err("keyvault.json is unreadable".into()),
    }
}

pub fn lock() -> VaultStatus {
    if let Ok(mut g) = SESSION_KEY.lock() {
        if let Some(mut key) = g.take() {
            key.fill(0);
        }
    }
    status()
}

/// Persist a v3 file (transactional writer lands in Phase 3; the v2
/// atomic-write helper is shared until then).
fn save_v3_to(path: &Path, file: &crate::keyvault_v3::VaultFileV3) -> Result<(), String> {
    let text = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    write_atomic(path, &text)
}

/// The session DEK is only honored for a file whose CURRENT manifest MAC it
/// verifies — a swapped or replaced file can never be read with a stale
/// session (IO-25).
fn v3_session_for(file: &crate::keyvault_v3::VaultFileV3) -> Option<[u8; 32]> {
    let dek = session_key()?;
    crate::keyvault_v3::verify_integrity(file, &dek).ok()?;
    Some(dek)
}

fn problem_err(state: &ReadStore) -> Option<String> {
    match state {
        ReadStore::Corrupted(_) => Some(
            "keyvault.json is corrupted — write refused to protect the file".into(),
        ),
        ReadStore::UnsupportedVersion(v) => Some(format!("unsupported vault version {v}")),
        ReadStore::IoFailure(_) => Some("keyvault.json is unreadable (I/O error)".into()),
        _ => None,
    }
}

// ── v3 recovery-code management + forgot-code reset ─────────────────────────

/// One-shot v3 initialization for the setup wizard: unlock code + exactly
/// three security questions + an enrolled recovery code, all written in a
/// single file replace. Returns the printed recovery code exactly once.
/// Any policy violation fails before the file is ever created.
pub fn setup_v3(code: &str, questions: &[(String, String)]) -> Result<(VaultStatus, String), String> {
    setup_v3_at(&vault_path(), code, questions)
}

fn setup_v3_at(
    path: &Path,
    code: &str,
    questions: &[(String, String)],
) -> Result<(VaultStatus, String), String> {
    if !matches!(read_store_from(path), ReadStore::Missing) {
        return Err("a vault already exists".into());
    }
    let (file0, dek) = crate::keyvault_v3::init_vault(code, &[], Some(questions))?;
    let (file, printed) = crate::keyvault_v3::enable_recovery_code(&file0, &dek)?;
    save_v3_to(path, &file)?;
    set_session_key(dek);
    Ok((status_of(&ReadStore::SealedV3(file)), printed))
}

pub fn enable_recovery_code(password: &str) -> Result<(VaultStatus, String), String> {
    enable_recovery_code_at(&vault_path(), password)
}

fn enable_recovery_code_at(path: &Path, password: &str) -> Result<(VaultStatus, String), String> {
    let ReadStore::SealedV3(file) = read_store_from(path) else {
        return Err("recovery codes need a v3 vault — set an unlock code first".into());
    };
    let dek = crate::keyvault_v3::unlock_with_password(&file, password)?;
    // The printed code comes back exactly once and is never stored.
    let (out, printed) = crate::keyvault_v3::enable_recovery_code(&file, &dek)?;
    save_v3_to(path, &out)?;
    set_session_key(dek);
    Ok((status_of(&ReadStore::SealedV3(out)), printed))
}

pub fn disable_recovery_code(password: &str) -> Result<VaultStatus, String> {
    disable_recovery_code_at(&vault_path(), password)
}

fn disable_recovery_code_at(path: &Path, password: &str) -> Result<VaultStatus, String> {
    let ReadStore::SealedV3(file) = read_store_from(path) else {
        return Err("recovery codes need a v3 vault — set an unlock code first".into());
    };
    let dek = crate::keyvault_v3::unlock_with_password(&file, password)?;
    let out = crate::keyvault_v3::disable_recovery(file, &dek)?;
    save_v3_to(path, &out)?;
    set_session_key(dek);
    Ok(status_of(&ReadStore::SealedV3(out)))
}

/// v3 forgot-code path: recover the DEK via the security questions (any two
/// of three, matched by explicit question id) OR the recovery code (typed,
/// or lifted out of an exported TXT — the same normalization applies), then
/// set a new unlock code in the same step. Online guessing is braked with
/// the same cooldown the v2 flow uses; cooldown itself does not count.
pub fn reset_unlock_code_v3(
    answers: &[(String, String)],
    recovery_code: Option<&str>,
    new_code: &str,
) -> Result<VaultStatus, String> {
    if let Some(left) = recovery_cooldown_remaining() {
        let mins = (left.as_secs() / 60).max(1);
        return Err(format!("too many wrong attempts — try again in {mins} min"));
    }
    match reset_unlock_code_v3_at(&vault_path(), answers, recovery_code, new_code) {
        Ok(st) => {
            clear_recovery_failures();
            Ok(st)
        }
        Err(e) => {
            note_recovery_failure();
            Err(e)
        }
    }
}

/// Lift a recovery code out of user input: the whole string when it folds
/// to exactly 34 Crockford characters, else the first line that does (an
/// exported TXT carries prose around the code). Anything else falls through
/// so the decoder produces the proper uniform error.
fn lift_recovery_code(input: &str) -> String {
    let fold = |s: &str| crate::keyvault_v3::crypto::recovery_code_normalize(s);
    let whole = fold(input);
    if whole.chars().count() == 34 {
        return whole;
    }
    for line in input.lines() {
        let folded = fold(line);
        if folded.chars().count() == 34 {
            return folded;
        }
    }
    whole
}

fn reset_unlock_code_v3_at(
    path: &Path,
    answers: &[(String, String)],
    recovery_code: Option<&str>,
    new_code: &str,
) -> Result<VaultStatus, String> {
    let ReadStore::SealedV3(file) = read_store_from(path) else {
        return Err("no v3 vault is set up".into());
    };
    // Exactly one factor decides; both paths fail with the same generic
    // error from the v3 module (no oracle about which step failed).
    let dek = if let Some(code) = recovery_code {
        crate::keyvault_v3::unlock_with_recovery_code(&file, &lift_recovery_code(code))
    } else {
        crate::keyvault_v3::unlock_with_questions(&file, answers)
    }?;
    let changed = crate::keyvault_v3::change_unlock_code(&file, &dek, new_code)?;
    save_v3_to(path, &changed)?;
    set_session_key(dek);
    Ok(status_of(&ReadStore::SealedV3(changed)))
}

pub fn add(service: &str, label: &str, key: &str, note: &str) -> Result<Vec<VaultRow>, String> {
    // Search services are stored in the plaintext Pane file — readable on
    // every launch without unlocking the vault.
    if crate::search_keys::is_search_service(service) {
        crate::search_keys::add(service, label, key)?;
        return list();
    }
    if service.trim().is_empty() || key.trim().is_empty() {
        return Err("service and key are required".into());
    }
    let path = vault_path();
    match read_store_from(&path) {
        ReadStore::Missing => Err("set an unlock code before storing keys".into()),
        ReadStore::LegacyV1(mut entries) => {
            upsert(&mut entries, service, label, key, note);
            save_plain_to(&path, &entries)?;
            Ok(entries.into_iter().map(row_from_plain).collect())
        }
        ReadStore::SealedV2(mut file) => {
            let session = session_key().ok_or_else(|| ERR_LOCKED.to_string())?;
            let id = entry_id(service, key.trim());
            if let Some(existing) = file.entries.iter_mut().find(|e| e.id == id) {
                existing.label = label.trim().to_string();
                if !note.is_empty() {
                    existing.note = note.to_string();
                }
            } else {
                let mut entries = Vec::new();
                upsert(&mut entries, service, label, key, note);
                let Some(e) = entries.into_iter().next() else {
                    return Err("service and key are required".into());
                };
                file.entries.push(seal_entry(&session, &e)?);
            }
            save_sealed_to(&path, &file)?;
            Ok(file.entries.iter().map(row_from_sealed).collect())
        }
        ReadStore::SealedV3(mut file) => {
            let dek = v3_session_for(&file).ok_or_else(|| ERR_LOCKED.to_string())?;
            let id = entry_id(service, key.trim());
            if let Some(existing) = file.entries.iter_mut().find(|e| e.id == id) {
                // Label is public metadata; the sealed payload is untouched.
                existing.label = label.trim().to_string();
            } else {
                let e = crate::keyvault_v3::PlainEntry {
                    id,
                    service: service.trim().to_string(),
                    label: label.trim().to_string(),
                    key: key.trim().to_string(),
                    note: note.to_string(),
                    created_at: chrono::Utc::now().timestamp_millis(),
                };
                file.entries
                    .push(crate::keyvault_v3::seal_entry_v3(&dek, &vault_id_bytes_pub(&file), &e)?);
            }
            file.revision += 1;
            crate::keyvault_v3::reseal_integrity(&mut file, &dek)?;
            save_v3_to(&path, &file)?;
            Ok(file
                .entries
                .iter()
                .map(|e| VaultRow {
                    id: e.id.clone(),
                    service: e.service.clone(),
                    label: e.label.clone(),
                    masked: e.masked.clone(),
                    note: e.public_note.clone().unwrap_or_default(),
                })
                .collect())
        }
        other => Err(problem_err(&other).unwrap_or_else(|| ERR_LOCKED.to_string())),
    }
}

fn vault_id_bytes_pub(file: &crate::keyvault_v3::VaultFileV3) -> [u8; 16] {
    crate::keyvault_v3::vault_id_of(file)
}

pub fn remove(id: &str) -> Result<Vec<VaultRow>, String> {
    if crate::search_keys::service_of(id).is_some() {
        crate::search_keys::remove(id)?;
        return list();
    }
    let path = vault_path();
    match read_store_from(&path) {
        ReadStore::Missing => Err("no such key".into()),
        ReadStore::LegacyV1(mut entries) => {
            let before = entries.len();
            entries.retain(|e| e.id != id);
            if entries.len() == before {
                return Err("no such key".into());
            }
            save_plain_to(&path, &entries)?;
            Ok(entries.into_iter().map(row_from_plain).collect())
        }
        ReadStore::SealedV2(mut file) => {
            let before = file.entries.len();
            file.entries.retain(|e| e.id != id);
            if file.entries.len() == before {
                return Err("no such key".into());
            }
            save_sealed_to(&path, &file)?;
            Ok(file.entries.iter().map(row_from_sealed).collect())
        }
        ReadStore::SealedV3(mut file) => {
            // Unlike v2, the v3 manifest is MAC'd under a DEK-derived key —
            // any manifest change (including removals) needs the session.
            let dek = v3_session_for(&file).ok_or_else(|| ERR_LOCKED.to_string())?;
            let before = file.entries.len();
            file.entries.retain(|e| e.id != id);
            if file.entries.len() == before {
                return Err("no such key".into());
            }
            file.revision += 1;
            crate::keyvault_v3::reseal_integrity(&mut file, &dek)?;
            save_v3_to(&path, &file)?;
            Ok(file
                .entries
                .iter()
                .map(|e| VaultRow {
                    id: e.id.clone(),
                    service: e.service.clone(),
                    label: e.label.clone(),
                    masked: e.masked.clone(),
                    note: e.public_note.clone().unwrap_or_default(),
                })
                .collect())
        }
        other => Err(problem_err(&other).unwrap_or_else(|| "no such key".into())),
    }
}

/// Metadata-only note edit — notes live outside the sealed section, so this
/// works while locked too.
pub fn set_note(id: &str, note: &str) -> Result<Vec<VaultRow>, String> {
    let path = vault_path();
    match read_store_from(&path) {
        ReadStore::Missing => Err("no such key".into()),
        ReadStore::LegacyV1(mut entries) => {
            let Some(e) = entries.iter_mut().find(|e| e.id == id) else {
                return Err("no such key".into());
            };
            e.note = note.trim().to_string();
            save_plain_to(&path, &entries)?;
            Ok(entries.into_iter().map(row_from_plain).collect())
        }
        ReadStore::SealedV2(mut file) => {
            let Some(e) = file.entries.iter_mut().find(|e| e.id == id) else {
                return Err("no such key".into());
            };
            e.note = note.trim().to_string();
            save_sealed_to(&path, &file)?;
            Ok(file.entries.iter().map(row_from_sealed).collect())
        }
        ReadStore::SealedV3(_) => Err(
            "v3 notes are sealed with the key — unlock the vault before editing notes".into(),
        ),
        other => Err(problem_err(&other).unwrap_or_else(|| "no such key".into())),
    }
}

/// Raw key for viewing/clipboard — the only path that un-masks, and only the
/// in-app frontend can call it. A sealed vault must be unlocked first.
pub fn reveal(id: &str) -> Result<String, String> {
    if crate::search_keys::service_of(id).is_some() {
        return crate::search_keys::reveal(id);
    }
    match read_store_from(&vault_path()) {
        ReadStore::Missing => Err("no such key".into()),
        ReadStore::LegacyV1(entries) => entries
            .into_iter()
            .find(|e| e.id == id)
            .map(|e| e.key)
            .ok_or_else(|| "no such key".to_string()),
        ReadStore::SealedV2(file) => {
            let key = session_key().ok_or_else(|| ERR_LOCKED.to_string())?;
            let entry = file.entries.iter().find(|e| e.id == id).ok_or_else(|| "no such key".to_string())?;
            open_entry(&key, entry)
        }
        ReadStore::SealedV3(file) => {
            let dek = v3_session_for(&file).ok_or_else(|| ERR_LOCKED.to_string())?;
            let vid = vault_id_bytes_pub(&file);
            let entry = file.entries.iter().find(|e| e.id == id).ok_or_else(|| "no such key".to_string())?;
            crate::keyvault_v3::open_entry_v3(&dek, &vid, entry).map(|p| p.key)
        }
        other => Err(problem_err(&other).unwrap_or_else(|| "no such key".into())),
    }
}

pub fn copy(id: &str) -> Result<String, String> {
    reveal(id)
}

/// (label, key) pairs for one service, in stored order — the read side for
/// quota providers. A locked sealed vault yields nothing here and callers
/// fall back to their legacy sources (provider key file / env), which hold
/// the working copies anyway. A corrupted/unsupported vault behaves like a
/// locked one (safe fallback, no writes) while `status()` surfaces the
/// problem — the working copies never live only in the vault.
pub fn keys_for_service(service: &str) -> Vec<(String, String)> {
    // Search services come from the plaintext Pane file — always readable,
    // never gated behind an unlock.
    if crate::search_keys::is_search_service(service) {
        return crate::search_keys::for_service(service);
    }
    match read_store_from(&vault_path()) {
        ReadStore::LegacyV1(entries) => entries
            .into_iter()
            .filter(|e| e.service.eq_ignore_ascii_case(service))
            .map(|e| (e.label, e.key))
            .collect(),
        ReadStore::SealedV2(file) => {
            let Some(key) = session_key() else { return Vec::new() };
            file.entries
                .iter()
                .filter(|e| e.service.eq_ignore_ascii_case(service))
                .filter_map(|e| open_entry(&key, e).ok().map(|k| (e.label.clone(), k)))
                .collect()
        }
        ReadStore::SealedV3(file) => {
            let Some(dek) = v3_session_for(&file) else { return Vec::new() };
            let vid = vault_id_bytes_pub(&file);
            file.entries
                .iter()
                .filter(|e| e.service.eq_ignore_ascii_case(service))
                .filter_map(|e| {
                    crate::keyvault_v3::open_entry_v3(&dek, &vid, e)
                        .ok()
                        .map(|p| (e.label.clone(), p.key))
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// (label, masked) for a service's sealed entries while the vault is LOCKED.
/// Labels and masks sit outside the sealed section, so no password is needed —
/// this is how a card still lists every key the user stored instead of
/// silently dropping the ones it cannot read. An unlocked (or plain) vault
/// returns nothing: [`keys_for_service`] already hands those over.
pub fn locked_service_entries(service: &str) -> Vec<(String, String)> {
    // Search-service keys are plaintext, so they are never "locked".
    if crate::search_keys::is_search_service(service) {
        return Vec::new();
    }
    match read_store_from(&vault_path()) {
        ReadStore::SealedV2(file) if session_key().is_none() => file
            .entries
            .iter()
            .filter(|e| e.service.eq_ignore_ascii_case(service))
            .map(|e| (e.label.clone(), e.masked.clone()))
            .collect(),
        ReadStore::SealedV3(file) if session_key().is_none() => file
            .entries
            .iter()
            .filter(|e| e.service.eq_ignore_ascii_case(service))
            .map(|e| (e.label.clone(), e.masked.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Which service an entry id belongs to (needed before removal, so cache
/// invalidation knows which provider to refetch).
pub fn service_of(id: &str) -> Option<String> {
    if let Some(service) = crate::search_keys::service_of(id) {
        return Some(service);
    }
    match read_store_from(&vault_path()) {
        ReadStore::LegacyV1(entries) => entries.into_iter().find(|e| e.id == id).map(|e| e.service),
        ReadStore::SealedV2(file) => file.entries.into_iter().find(|e| e.id == id).map(|e| e.service),
        ReadStore::SealedV3(file) => file.entries.into_iter().find(|e| e.id == id).map(|e| e.service),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_vault(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "pane-keyvault-test-{}-{}.json",
            std::process::id(),
            tag
        ));
        let _ = std::fs::remove_file(&p);
        p
    }

    fn write_plain(path: &Path, entries: &[VaultEntry]) {
        std::fs::write(path, serde_json::to_string(entries).unwrap()).unwrap();
    }

    fn sample() -> Vec<VaultEntry> {
        vec![
            VaultEntry {
                id: entry_id("tavily", "tvly-secret-one"),
                service: "tavily".into(),
                label: "Tavily key 1".into(),
                key: "tvly-secret-one".into(),
                note: "main".into(),
                created_at: 1,
            },
            VaultEntry {
                id: entry_id("bocha", "bocha-secret-two"),
                service: "bocha".into(),
                label: "BochaAI".into(),
                key: "bocha-secret-two".into(),
                note: String::new(),
                created_at: 2,
            },
        ]
    }

    /// set_password/unlock against an arbitrary path (the public API reads
    /// vault_path(); tests drive the same steps through the internals).
    fn seal_file(path: &Path, password: &str, entries: &[VaultEntry]) -> [u8; 32] {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);
        let kdf = KdfSection {
            algo: "argon2id".into(),
            salt_b64: b64().encode(salt),
            m_kib: ARGON_M_KIB,
            t: ARGON_T,
            p: ARGON_P,
        };
        let key = derive_key(password, &kdf).unwrap();
        let (canary_nonce_b64, canary_b64) = seal(&key, CANARY_PLAINTEXT).unwrap();
        let sealed = entries.iter().map(|e| seal_entry(&key, e).unwrap()).collect();
        let file = VaultFile { version: VAULT_VERSION, kdf, canary_nonce_b64, canary_b64, entries: sealed, recovery: None };
        save_sealed_to(path, &file).unwrap();
        key
    }

    #[test]
    fn mask_keeps_head_and_tail_only() {
        assert_eq!(mask_key("sk-abcdefghijklmnop"), "sk-abc…mnop");
        assert_eq!(mask_key("short"), "*****");
        assert_eq!(mask_key(""), "***");
    }

    #[test]
    fn ids_are_stable_and_service_scoped() {
        assert_eq!(entry_id("Tavily", "k1"), entry_id("tavily", "k1"));
        assert_ne!(entry_id("tavily", "k1"), entry_id("tavily", "k2"));
        assert_ne!(entry_id("tavily", "k1"), entry_id("bocha", "k1"));
    }

    #[test]
    fn upsert_is_idempotent_and_updates_label() {
        let mut v = Vec::new();
        upsert(&mut v, "tavily", "A", "key-one", "");
        upsert(&mut v, "tavily", "B", "key-one", "");
        upsert(&mut v, "tavily", "C", "key-two", "");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].label, "B");
        // blank keys never land
        upsert(&mut v, "x", "y", "   ", "");
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn vault_rows_always_mask() {
        let mut v = Vec::new();
        upsert(&mut v, "svc", "l", "sk-verylongsecretkey1234", "n");
        assert!(mask_key(&v[0].key).starts_with("sk-ver"));
        assert!(!mask_key(&v[0].key).contains("secret"));
    }

    #[test]
    fn sealed_roundtrip_restores_every_entry() {
        let path = temp_vault("roundtrip");
        let key = seal_file(&path, "correct horse", &sample());
        let ReadStore::SealedV2(file) = read_store_from(&path) else { panic!("expected sealed store") };
        assert_eq!(file.entries.len(), 2);
        for (sealed, plain) in file.entries.iter().zip(sample().iter()) {
            assert_eq!(open_entry(&key, sealed).unwrap(), plain.key);
            assert_eq!(sealed.masked, mask_key(&plain.key));
            assert_eq!(sealed.note, plain.note);
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn wrong_password_fails_the_canary() {
        let path = temp_vault("wrong");
        seal_file(&path, "right-password", &sample());
        let ReadStore::SealedV2(file) = read_store_from(&path) else { panic!("expected sealed store") };
        let bad = derive_key("wrong-password", &file.kdf).unwrap();
        assert_eq!(
            open(&bad, &file.canary_nonce_b64, &file.canary_b64).unwrap_err(),
            ERR_WRONG_PASSWORD
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn every_seal_uses_a_fresh_nonce() {
        let path = temp_vault("nonce");
        seal_file(&path, "pw", &sample());
        let ReadStore::SealedV2(file) = read_store_from(&path) else { panic!("expected sealed store") };
        assert_ne!(file.entries[0].nonce_b64, file.entries[1].nonce_b64);
        assert_ne!(file.entries[0].sealed_b64, file.entries[1].sealed_b64);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_plain_array_still_loads() {
        let path = temp_vault("legacy");
        write_plain(&path, &sample());
        let ReadStore::LegacyV1(entries) = read_store_from(&path) else { panic!("expected plain store") };
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "tvly-secret-one");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sealed_file_holds_no_plaintext_key_material() {
        let path = temp_vault("atrest");
        seal_file(&path, "pw", &sample());
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("tvly-secret-one"));
        assert!(!raw.contains("bocha-secret-two"));
        assert!(raw.contains("argon2id"));
        // Metadata stays readable for the locked listing.
        assert!(raw.contains("Tavily key 1"));
        let _ = std::fs::remove_file(&path);
    }

    // ── Q&A recovery ────────────────────────────────────────────────────────

    fn questions_2() -> Vec<(String, String)> {
        vec![
            ("你最喜欢的水果是什么？".into(), "Apple".into()),
            ("你最常用的编程工具是什么？".into(), "  Rust   Lang  ".into()),
        ]
    }

    #[test]
    fn recovery_requires_every_answer_normalized() {
        let vault_key = [7u8; 32];
        let recovery = build_recovery(&vault_key, "master-pw-1", &questions_2()).unwrap();
        // Both right (answers normalized: case + collapsed spaces).
        let ok = try_recovery(
            &recovery,
            &["  apple ".into(), "rust lang".into()],
        )
        .unwrap();
        assert_eq!(ok, "master-pw-1");
        // One wrong → rejected, no partial credit.
        assert!(try_recovery(&recovery, &["apple".into(), "python".into()]).is_err());
        // Wrong count → rejected.
        assert!(try_recovery(&recovery, &["apple".into()]).is_err());
    }

    #[test]
    fn recovery_seals_store_no_plaintext_answers_or_password() {
        let recovery = build_recovery(&[7u8; 32], "master-pw-1", &questions_2()).unwrap();
        let raw = serde_json::to_string(&recovery).unwrap();
        assert!(!raw.contains("master-pw-1"));
        assert!(!raw.to_lowercase().contains("apple"));
        assert!(!raw.to_lowercase().contains("rust"));
        // Questions themselves are plaintext by design (challenge UI).
        assert!(raw.contains("水果"));
    }

    #[test]
    fn recovery_survives_a_master_password_change() {
        let old_key = [7u8; 32];
        let mut recovery = build_recovery(&old_key, "old-master", &questions_2()).unwrap();
        let new_key = [9u8; 32];
        rotate_recovery_master(&mut recovery, &old_key, &new_key, "new-master").unwrap();
        // Same answers now recover the NEW password.
        assert_eq!(
            try_recovery(&recovery, &["apple".into(), "rust lang".into()]).unwrap(),
            "new-master"
        );
    }

    #[test]
    fn recovery_question_validation_bounds() {
        let key = [1u8; 32];
        assert!(build_recovery(&key, "pw", &[]).is_err()); // zero questions
        let six: Vec<(String, String)> =
            (0..6).map(|i| (format!("Q{i}?"), "ans".into())).collect();
        assert!(build_recovery(&key, "pw", &six).is_err()); // more than five
        assert!(build_recovery(&key, "pw", &[("".into(), "ans".into())]).is_err()); // empty question
        assert!(build_recovery(&key, "pw", &[("Q".repeat(61), "ans".into())]).is_err()); // long question
        assert!(build_recovery(&key, "pw", &[("Q?".into(), "x".into())]).is_err()); // 1-char answer
        assert!(build_recovery(&key, "pw", &[("Q?".into(), "a".repeat(41))]).is_err()); // long answer
        // One question is a legal (if discouraged) configuration.
        assert!(build_recovery(&key, "pw", &[("Q?".into(), "ans".into())]).is_ok());
    }

    #[test]
    fn answer_normalization_collapses_case_and_spaces() {
        assert_eq!(normalize_answer("  Apple   Pie "), "apple pie");
        assert_eq!(normalize_answer("apple pie"), normalize_answer("APPLE  PIE"));
    }

    #[test]
    fn recovery_failure_counter_reaches_cooldown_and_resets() {
        clear_recovery_failures();
        for _ in 0..RECOVERY_FAIL_LIMIT {
            note_recovery_failure();
        }
        assert!(recovery_cooldown_remaining().is_some());
        clear_recovery_failures();
        assert!(recovery_cooldown_remaining().is_none());
    }

    // ── Phase 1: fail-closed classification (IO-01..08) ─────────────────────
    // The public status()/list()/add() read the real config dir, so these
    // drive `read_store_from` on isolated paths — the same classifier every
    // public entry point routes through.

    #[test]
    fn missing_file_classifies_as_missing() {
        let p = temp_vault("missing");
        assert!(matches!(read_store_from(&p), ReadStore::Missing));
    }

    #[test]
    fn directory_at_path_is_io_failure_not_missing() {
        let p = std::env::temp_dir().join(format!("pane-kv-dir-{}-t.json", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir(&p).unwrap();
        assert!(matches!(read_store_from(&p), ReadStore::IoFailure(_)));
        std::fs::remove_dir(&p).unwrap();
    }

    #[test]
    fn malformed_json_is_corrupted() {
        let p = temp_vault("malformed");
        std::fs::write(&p, "{\"version\": 2, \"kd").unwrap();
        assert!(matches!(read_store_from(&p), ReadStore::Corrupted(_)));
    }

    // IO-04 — a JSON object that is not a vault is Corrupted, never "empty".
    #[test]
    fn json_object_without_version_is_corrupted() {
        let p = temp_vault("notvault");
        std::fs::write(&p, r#"{"hello": "world"}"#).unwrap();
        assert!(matches!(read_store_from(&p), ReadStore::Corrupted(_)));
        // An object that even LOOKS like the v1 array shape is still not v1.
        std::fs::write(&p, r#"{"version": 1}"#).unwrap();
        assert!(matches!(read_store_from(&p), ReadStore::Corrupted(_)));
    }

    // IO-05 — unknown versions never fall through to v1/v2 handling.
    #[test]
    fn unknown_version_classifies_unsupported() {
        let p = temp_vault("future");
        std::fs::write(&p, r#"{"version": 999, "entries": []}"#).unwrap();
        match read_store_from(&p) {
            ReadStore::UnsupportedVersion(999) => {}
            other => panic!("expected unsupported version 999, got {other:?}"),
        }
    }

    // IO-07 — duplicate keys must not silently become "last one wins".
    #[test]
    fn duplicate_keys_are_rejected() {
        assert!(parse_json_no_duplicate_keys(r#"{"a":1}"#).is_ok());
        assert!(parse_json_no_duplicate_keys(r#"{"a":1,"a":2}"#).is_err());
        assert!(parse_json_no_duplicate_keys(r#"{"v":{"x":1,"x":2}}"#).is_err());
        assert!(parse_json_no_duplicate_keys(r#"[1,2,3]"#).is_ok());
        let p = temp_vault("dupkeys");
        std::fs::write(&p, r#"{"version":2,"version":3}"#).unwrap();
        assert!(matches!(read_store_from(&p), ReadStore::Corrupted(_)));
    }

    // IO-08 — oversized files are rejected before parsing.
    #[test]
    fn oversized_file_is_rejected_before_parse() {
        let p = temp_vault("huge");
        std::fs::write(&p, "x".repeat(4096)).unwrap();
        assert!(matches!(
            read_store_with_limit(&p, 1024),
            ReadStore::Corrupted("file exceeds the size limit")
        ));
        // Under the cap, the same file classifies by content.
        assert!(matches!(
            read_store_with_limit(&p, 1024 * 1024),
            ReadStore::Corrupted(_) // "xxxx…" is invalid json, but it parsed-size-wise
        ));
    }

    // A v3 file produced by the pure module classifies as SealedV3 and its
    // metadata lists through the v3 branch shape.
    #[test]
    fn v3_file_classifies_as_sealed_v3() {
        let p = temp_vault("v3");
        let (file, _dek) = crate::keyvault_v3::init_vault("unlock-code-77", &[], None).unwrap();
        std::fs::write(&p, serde_json::to_string_pretty(&file).unwrap()).unwrap();
        match read_store_from(&p) {
            ReadStore::SealedV3(parsed) => {
                assert_eq!(parsed.entries.len(), 0);
                assert_eq!(parsed.vault_id, file.vault_id);
            }
            other => panic!("expected sealed v3, got {other:?}"),
        }
        let _ = std::fs::remove_file(&p);
    }

    // ── v3 lifecycle through the public-shaped internals (temp paths) ──────
    // These drive the *_at functions so nothing touches the real config
    // dir; session side effects (the global unlock state) are irrelevant to
    // the assertions below, which check FILE semantics only.

    #[test]
    fn set_password_on_missing_creates_v3() {
        let p = temp_vault("v3init");
        let st = set_password_at(&p, "unlock-code-77").unwrap();
        assert_eq!(st.version, Some(3));
        assert!(st.has_password);
        assert_eq!(st.state, "ok");
        assert_eq!(st.recovery_questions, 0);
        assert!(!st.recovery_code);
        let ReadStore::SealedV3(file) = read_store_from(&p) else {
            panic!("expected a v3 file")
        };
        assert!(crate::keyvault_v3::unlock_with_password(&file, "unlock-code-77").is_ok());
        // Policy is creation-time: a short code never creates the file.
        let p2 = temp_vault("v3short");
        assert!(set_password_at(&p2, "short").is_err());
        assert!(matches!(read_store_from(&p2), ReadStore::Missing));
        // A corrupted file is refused and left byte-identical.
        let p3 = temp_vault("v3corrupt");
        std::fs::write(&p3, "{\"version\": 2, \"kd").unwrap();
        assert!(set_password_at(&p3, "unlock-code-77").is_err());
        assert_eq!(std::fs::read_to_string(&p3).unwrap(), "{\"version\": 2, \"kd");
    }

    // The setup wizard's one-shot path: vault + three questions + recovery
    // code in a single write; any failure (policy, weak questions) leaves
    // NO file behind; refuses when a vault already exists.
    #[test]
    fn setup_v3_is_atomic() {
        let p = temp_vault("v3setup");
        // Policy failure → nothing written.
        assert!(setup_v3_at(&p, "short", &v3_qa3()).is_err());
        assert!(matches!(read_store_from(&p), ReadStore::Missing));
        // Bad questions (two only) → nothing written.
        assert!(setup_v3_at(&p, "unlock-code-77", &v3_qa3()[..2]).is_err());
        assert!(matches!(read_store_from(&p), ReadStore::Missing));
        // The happy path: everything in one file, recovery code printable.
        let (st, printed) = setup_v3_at(&p, "unlock-code-77", &v3_qa3()).unwrap();
        assert_eq!(st.version, Some(3));
        assert_eq!(st.recovery_questions, 3);
        assert!(st.recovery_code);
        assert_eq!(printed.chars().filter(|c| *c == '-').count(), 1);
        let ReadStore::SealedV3(file) = read_store_from(&p) else { panic!() };
        assert!(file.questions.is_some() && file.recovery.is_some());
        // The printed code actually unlocks through the recovery path.
        assert_eq!(
            crate::keyvault_v3::unlock_with_recovery_code(&file, &printed).unwrap().len(),
            32
        );
        // Second setup on the same path is refused.
        assert!(setup_v3_at(&p, "another-unlock-code-7", &v3_qa3()).is_err());
    }

    fn v3_qa3() -> Vec<(String, String)> {
        vec![
            ("Q one?".into(), "apple".into()),
            ("Q two?".into(), "pear".into()),
            ("Q three?".into(), "mochi".into()),
        ]
    }

    #[test]
    fn v3_questions_and_reset_via_questions() {
        let p = temp_vault("v3qa");
        set_password_at(&p, "unlock-code-77").unwrap();
        // v3 questions are exactly three; two-question input is rejected
        // without touching the file.
        let before = std::fs::read_to_string(&p).unwrap();
        assert!(set_recovery_at(&p, "unlock-code-77", &v3_qa3()[..2]).is_err());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), before);
        let st = set_recovery_at(&p, "unlock-code-77", &v3_qa3()).unwrap();
        assert_eq!(st.recovery_questions, 3);
        // Forgot-code reset via ANY two questions, addressed by explicit id.
        let st = reset_unlock_code_v3_at(
            &p,
            &[("q1".into(), "apple".into()), ("q3".into(), "mochi".into())],
            None,
            "newer-unlock-code-16ch-77",
        )
        .unwrap();
        assert_eq!(st.version, Some(3));
        let ReadStore::SealedV3(file) = read_store_from(&p) else { panic!() };
        assert!(crate::keyvault_v3::unlock_with_password(&file, "unlock-code-77").is_err());
        assert!(crate::keyvault_v3::unlock_with_password(&file, "newer-unlock-code-16ch-77").is_ok());
        // The questions survived the unlock-code change (same DEK).
        assert!(file.questions.is_some());
    }

    #[test]
    fn v3_recovery_code_reset_and_txt_lift() {
        let p = temp_vault("v3code");
        set_password_at(&p, "unlock-code-77").unwrap();
        let (st, printed) = enable_recovery_code_at(&p, "unlock-code-77").unwrap();
        assert!(st.recovery_code);
        // An exported TXT carries prose around the code — the lifter must
        // find it (also through lowercase + de-separated input).
        let txt = format!(
            "Pane 密钥保险箱 — 恢复码\n\n{}\n\n此码仅显示一次，请妥善保管。",
            printed.to_lowercase().replace('-', " ")
        );
        let st = reset_unlock_code_v3_at(&p, &[], Some(&txt), "third-unlock-code-16ch-77").unwrap();
        assert_eq!(st.version, Some(3));
        let ReadStore::SealedV3(file) = read_store_from(&p) else { panic!() };
        assert!(crate::keyvault_v3::unlock_with_password(&file, "third-unlock-code-16ch-77").is_ok());
        // A wrong factor never rewrites the file.
        let before = std::fs::read_to_string(&p).unwrap();
        assert!(reset_unlock_code_v3_at(
            &p,
            &[("q1".into(), "nope".into()), ("q2".into(), "still-nope".into())],
            None,
            "fourth-unlock-code-16ch-77",
        )
        .is_err());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), before);
        // Disabling drops the enrollment.
        let st = disable_recovery_code_at(&p, "third-unlock-code-16ch-77").unwrap();
        assert!(!st.recovery_code);
    }

    #[test]
    fn lift_recovery_code_folds_and_finds_lines() {
        let printed = "0123456789ABCDEFGHJKMNPQRSTVWXYZ-AB"; // 32 + check, folded 34
        let folded = "0123456789ABCDEFGHJKMNPQRSTVWXYZAB";
        // Whole-string folding (spaces for the dash, lowercase input)…
        assert_eq!(lift_recovery_code(&printed.replace('-', "  ")), folded);
        assert_eq!(lift_recovery_code(&printed.to_lowercase()), folded);
        // …and first-34-char-line extraction out of prose.
        let txt = format!("Pane recovery code\n{printed}\nkeep it safe");
        assert_eq!(lift_recovery_code(&txt), folded);
        // Garbage falls through unchanged (folded: O→0, I/L→1) for the
        // decoder to reject.
        assert_eq!(lift_recovery_code("not a code at all"), "N0TAC0DEATA11");
    }

    // IO-03 (protection) — a corrupted file must NEVER classify as a plain
    // v1 vault, whatever the damage.
    #[test]
    fn corrupted_files_never_classify_as_plain() {
        let cases = [
            "",
            "null",
            "42",
            r#""a string""#,
            r#"{"version": 2}"#,               // v2 missing required fields
            r#"{"version": 2, "entries": []}"#, // v2 missing kdf/canary
            r#"[{"id": 1}]"#,                  // v1 array with wrong types
        ];
        for (i, case) in cases.iter().enumerate() {
            let p = temp_vault(&format!("corrupt{i}"));
            std::fs::write(&p, case).unwrap();
            let cls = read_store_from(&p);
            assert!(
                matches!(cls, ReadStore::Corrupted(_) | ReadStore::UnsupportedVersion(_)),
                "{case:?} classified as {cls:?}"
            );
            let _ = std::fs::remove_file(&p);
        }
    }
}
