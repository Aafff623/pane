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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct KdfSection {
    algo: String, // "argon2id"
    salt_b64: String,
    m_kib: u32,
    t: u32,
    p: u32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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
/// the on-disk canary first — the caller just proved it, but the file is
/// what gets rewritten.
pub fn set_recovery(
    password: &str,
    questions: &[(String, String)],
) -> Result<VaultStatus, String> {
    let path = vault_path();
    let Store::Sealed(mut file) = load_store_from(&path) else {
        return Err("set a master password before recovery questions".into());
    };
    // Verify the password against the file's canary (not the session) so a
    // stale unlocked session can never rewrite recovery with a wrong password.
    let vault_key = derive_key(password.trim(), &file.kdf)?;
    open(&vault_key, &file.canary_nonce_b64, &file.canary_b64)?;
    let recovery = build_recovery(&vault_key, password, questions)?;
    file.recovery = Some(recovery);
    save_sealed_to(&path, &file)?;
    Ok(status_from(&file, session_key().is_some()))
}

/// The challenge surface: (id, question) pairs, readable while locked.
pub fn recovery_questions() -> Vec<(String, String)> {
    match load_store_from(&vault_path()) {
        Store::Sealed(file) => file
            .recovery
            .map(|r| r.questions.into_iter().map(|q| (q.id, q.question)).collect())
            .unwrap_or_default(),
        Store::Plain(_) => Vec::new(),
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
    let Store::Sealed(file) = load_store_from(&path) else {
        return Err("no master password is set".into());
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
    let status = seal_and_store(new_password, &entries, file.recovery.clone().map(|r| (r, old_vault_key)))?;
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

enum Store {
    Plain(Vec<VaultEntry>),
    Sealed(VaultFile),
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

fn load_store_from(path: &Path) -> Store {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Store::Plain(Vec::new());
    };
    let Ok(doc) = serde_json::from_str::<Value>(raw.trim_start_matches('\u{feff}')) else {
        return Store::Plain(Vec::new());
    };
    if doc.get("version").and_then(Value::as_u64) == Some(VAULT_VERSION as u64) {
        if let Ok(file) = serde_json::from_value::<VaultFile>(doc) {
            return Store::Sealed(file);
        }
        return Store::Plain(Vec::new());
    }
    serde_json::from_value::<Vec<VaultEntry>>(doc)
        .map(Store::Plain)
        .unwrap_or(Store::Plain(Vec::new()))
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
fn entry_id(service: &str, key: &str) -> String {
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
}

pub fn status() -> VaultStatus {
    match load_store_from(&vault_path()) {
        Store::Plain(entries) => VaultStatus {
            has_password: false,
            unlocked: true,
            count: entries.len(),
            recovery_questions: 0,
        },
        Store::Sealed(file) => status_from(&file, session_key().is_some()),
    }
}

fn status_from(file: &VaultFile, unlocked: bool) -> VaultStatus {
    VaultStatus {
        has_password: true,
        unlocked,
        count: file.entries.len(),
        recovery_questions: file.recovery.as_ref().map(|r| r.questions.len()).unwrap_or(0),
    }
}

pub fn list() -> Vec<VaultRow> {
    match load_store_from(&vault_path()) {
        Store::Plain(mut entries) => {
            // Seed lazily on first listing; idempotent by construction.
            let before = entries.len();
            seed_entries(&mut entries);
            if entries.len() != before {
                let _ = save_plain_to(&vault_path(), &entries);
            }
            entries.into_iter().map(row_from_plain).collect()
        }
        Store::Sealed(file) => file.entries.iter().map(row_from_sealed).collect(),
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
    let path = vault_path();
    let entries = match load_store_from(&path) {
        Store::Plain(entries) => entries,
        Store::Sealed(file) => open_all(&file)?,
    };
    seal_and_store(password, &entries, None)
}

/// Change the master password: the old one must verify against the on-disk
/// canary first, then the whole vault is re-sealed under the new one — the
/// recovery questions survive with their seals rotated to the new password.
pub fn change_password(old: &str, new: &str) -> Result<VaultStatus, String> {
    match load_store_from(&vault_path()) {
        Store::Plain(_) => Err("no master password is set".into()),
        Store::Sealed(file) => {
            let old_key = derive_key(old.trim(), &file.kdf)?;
            open(&old_key, &file.canary_nonce_b64, &file.canary_b64)?;
            let entries = open_all_from(&file, &old_key)?;
            seal_and_store(new, &entries, file.recovery.map(|r| (r, old_key)))
        }
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
    match load_store_from(&vault_path()) {
        Store::Plain(_) => Err("no master password is set".into()),
        Store::Sealed(file) => {
            let key = derive_key(password.trim(), &file.kdf)?;
            open(&key, &file.canary_nonce_b64, &file.canary_b64)?;
            set_session_key(key);
            Ok(())
        }
    }
}

/// Build + write the sealed vault. `keep` carries the previous recovery
/// section and the OLD vault key so its seals rotate to the new password;
/// `None` writes the file without recovery.
fn seal_and_store(
    password: &str,
    entries: &[VaultEntry],
    keep: Option<(RecoverySection, [u8; 32])>,
) -> Result<VaultStatus, String> {
    let password = password.trim();
    if password.len() < 4 {
        return Err("master password needs at least 4 characters".into());
    }
    let path = vault_path();
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
    save_sealed_to(&path, &file)?;
    set_session_key(key);
    Ok(status_from(&file, true))
}

pub fn unlock(password: &str) -> Result<VaultStatus, String> {
    match load_store_from(&vault_path()) {
        // No password set yet — the vault is inherently open.
        Store::Plain(entries) => Ok(VaultStatus {
            has_password: false,
            unlocked: true,
            count: entries.len(),
            recovery_questions: 0,
        }),
        Store::Sealed(file) => {
            let key = derive_key(password.trim(), &file.kdf)?;
            open(&key, &file.canary_nonce_b64, &file.canary_b64)?;
            set_session_key(key);
            Ok(status_from(&file, true))
        }
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

pub fn add(service: &str, label: &str, key: &str, note: &str) -> Result<Vec<VaultRow>, String> {
    if service.trim().is_empty() || key.trim().is_empty() {
        return Err("service and key are required".into());
    }
    let path = vault_path();
    match load_store_from(&path) {
        Store::Plain(mut entries) => {
            upsert(&mut entries, service, label, key, note);
            save_plain_to(&path, &entries)?;
            Ok(entries.into_iter().map(row_from_plain).collect())
        }
        Store::Sealed(mut file) => {
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
    }
}

pub fn remove(id: &str) -> Result<Vec<VaultRow>, String> {
    let path = vault_path();
    match load_store_from(&path) {
        Store::Plain(mut entries) => {
            let before = entries.len();
            entries.retain(|e| e.id != id);
            if entries.len() == before {
                return Err("no such key".into());
            }
            save_plain_to(&path, &entries)?;
            Ok(entries.into_iter().map(row_from_plain).collect())
        }
        Store::Sealed(mut file) => {
            let before = file.entries.len();
            file.entries.retain(|e| e.id != id);
            if file.entries.len() == before {
                return Err("no such key".into());
            }
            save_sealed_to(&path, &file)?;
            Ok(file.entries.iter().map(row_from_sealed).collect())
        }
    }
}

/// Metadata-only note edit — notes live outside the sealed section, so this
/// works while locked too.
pub fn set_note(id: &str, note: &str) -> Result<Vec<VaultRow>, String> {
    let path = vault_path();
    match load_store_from(&path) {
        Store::Plain(mut entries) => {
            let Some(e) = entries.iter_mut().find(|e| e.id == id) else {
                return Err("no such key".into());
            };
            e.note = note.trim().to_string();
            save_plain_to(&path, &entries)?;
            Ok(entries.into_iter().map(row_from_plain).collect())
        }
        Store::Sealed(mut file) => {
            let Some(e) = file.entries.iter_mut().find(|e| e.id == id) else {
                return Err("no such key".into());
            };
            e.note = note.trim().to_string();
            save_sealed_to(&path, &file)?;
            Ok(file.entries.iter().map(row_from_sealed).collect())
        }
    }
}

/// Raw key for viewing/clipboard — the only path that un-masks, and only the
/// in-app frontend can call it. A sealed vault must be unlocked first.
pub fn reveal(id: &str) -> Result<String, String> {
    match load_store_from(&vault_path()) {
        Store::Plain(entries) => entries
            .into_iter()
            .find(|e| e.id == id)
            .map(|e| e.key)
            .ok_or_else(|| "no such key".to_string()),
        Store::Sealed(file) => {
            let key = session_key().ok_or_else(|| ERR_LOCKED.to_string())?;
            let entry = file.entries.iter().find(|e| e.id == id).ok_or_else(|| "no such key".to_string())?;
            open_entry(&key, entry)
        }
    }
}

pub fn copy(id: &str) -> Result<String, String> {
    reveal(id)
}

/// (label, key) pairs for one service, in stored order — the read side for
/// quota providers. A locked sealed vault yields nothing here and callers
/// fall back to their legacy sources (provider key file / env), which hold
/// the working copies anyway.
pub fn keys_for_service(service: &str) -> Vec<(String, String)> {
    match load_store_from(&vault_path()) {
        Store::Plain(entries) => entries
            .into_iter()
            .filter(|e| e.service.eq_ignore_ascii_case(service))
            .map(|e| (e.label, e.key))
            .collect(),
        Store::Sealed(file) => {
            let Some(key) = session_key() else { return Vec::new() };
            file.entries
                .iter()
                .filter(|e| e.service.eq_ignore_ascii_case(service))
                .filter_map(|e| open_entry(&key, e).ok().map(|k| (e.label.clone(), k)))
                .collect()
        }
    }
}

/// Which service an entry id belongs to (needed before removal, so cache
/// invalidation knows which provider to refetch).
pub fn service_of(id: &str) -> Option<String> {
    match load_store_from(&vault_path()) {
        Store::Plain(entries) => entries.into_iter().find(|e| e.id == id).map(|e| e.service),
        Store::Sealed(file) => file.entries.into_iter().find(|e| e.id == id).map(|e| e.service),
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
        let Store::Sealed(file) = load_store_from(&path) else { panic!("expected sealed store") };
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
        let Store::Sealed(file) = load_store_from(&path) else { panic!("expected sealed store") };
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
        let Store::Sealed(file) = load_store_from(&path) else { panic!("expected sealed store") };
        assert_ne!(file.entries[0].nonce_b64, file.entries[1].nonce_b64);
        assert_ne!(file.entries[0].sealed_b64, file.entries[1].sealed_b64);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_plain_array_still_loads() {
        let path = temp_vault("legacy");
        write_plain(&path, &sample());
        let Store::Plain(entries) = load_store_from(&path) else { panic!("expected plain store") };
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
}
