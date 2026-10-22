//! Local-only encrypted backup: accounts, the vault file and selected
//! settings in one Argon2id + AES-256-GCM envelope.
//!
//! The vault file is carried as ciphertext — this module never decrypts it.
//! Account secrets are read from the OS vault through an injected seam so the
//! parse-tests harness can exercise the same code without touching a real
//! credential store.

use crate::keyvault_v3::crypto::{
    aead_open, aead_seal, argon2id_frozen, b64, fresh_nonce, fresh_random, hkdf32, tlv,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const FORMAT: &str = "pane-backup-v1";
pub const PAYLOAD_FORMAT: &str = "pane-backup-payload-v1";
pub const MAX_BACKUP_BYTES: usize = 64 * 1024 * 1024;
const KDF_ALGO: &str = "argon2id";
const KDF_M: u32 = 65_536;
const KDF_T: u32 = 3;
const KDF_P: u32 = 1;
const INFO_BACKUP: &[u8] = b"pane/backup/v1/aes256gcm";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub accounts: bool,
    pub keys: bool,
    pub settings: bool,
}

impl Selection {
    pub fn all() -> Self {
        Self { accounts: true, keys: true, settings: true }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupData {
    pub format: String,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<BTreeMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accounts: Option<AccountBackup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<KeysBackup>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountBackup {
    /// relative path -> raw JSON (non-secret metadata and any plaintext
    /// fallback that the owning store still keeps)
    pub files: BTreeMap<String, String>,
    /// OS-vault key -> secret value, sealed inside the envelope
    #[serde(default)]
    pub secrets: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeysBackup {
    /// base64 of the already-encrypted keyvault.json
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyvault: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreview {
    pub created_at: i64,
    pub setting_keys: Vec<String>,
    pub account_files: usize,
    pub account_ids: Vec<String>,
    pub added_accounts: usize,
    pub conflicting_accounts: usize,
    pub secret_count: usize,
    pub has_keyvault: bool,
    pub keyvault_bytes: usize,
    pub keyvault_conflict: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub settings: BTreeMap<String, Value>,
    pub account_files: usize,
    pub secrets: usize,
    pub keyvault_written: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PreviewFile {
    pub path: String,
    pub preview: BackupPreview,
}

pub trait BackupSecrets {
    /// `Ok(None)` = the key is absent; `Err` = the store is unavailable and
    /// the caller must abort instead of producing a backup with holes.
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn put(&mut self, key: &str, value: &str) -> Result<(), String>;
    fn remove(&mut self, key: &str);
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: String,
    created_at: i64,
    kdf: Kdf,
    nonce: String,
    sealed: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kdf {
    algo: String,
    m: u32,
    t: u32,
    p: u32,
    salt: String,
}

fn envelope_aad() -> Vec<u8> {
    tlv(&[FORMAT.as_bytes()])
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn normalize_backup_password(password: &str) -> Result<String, String> {
    let password = crate::keyvault_v3::crypto::normalize_password_v3(password);
    if password.chars().count() < 8 {
        return Err("backup password must be at least 8 characters".into());
    }
    Ok(password)
}

fn encode_envelope(data: &BackupData, password: &str) -> Result<Vec<u8>, String> {
    let password = normalize_backup_password(password)?;
    let payload = serde_json::to_vec(data).map_err(|error| format!("serialize backup: {error}"))?;
    let salt = fresh_random::<16>();
    let base = argon2id_frozen(password.as_bytes(), &salt)?;
    let key = hkdf32(&base, &salt, INFO_BACKUP);
    let nonce = fresh_nonce();
    let sealed = aead_seal(&key, nonce, &payload, &envelope_aad());
    let envelope = Envelope {
        format: FORMAT.into(),
        created_at: data.created_at,
        kdf: Kdf {
            algo: KDF_ALGO.into(),
            m: KDF_M,
            t: KDF_T,
            p: KDF_P,
            salt: b64().encode(salt),
        },
        nonce: b64().encode(nonce),
        sealed: b64().encode(sealed),
    };
    serde_json::to_vec(&envelope).map_err(|error| format!("serialize envelope: {error}"))
}

fn decode_envelope(bytes: &[u8], password: &str) -> Result<BackupData, String> {
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err("backup file is too large".into());
    }
    let password = crate::keyvault_v3::crypto::normalize_password_v3(password);
    if password.chars().count() < 8 {
        return Err("wrong password or tampered backup".into());
    }
    let envelope: Envelope =
        serde_json::from_slice(bytes).map_err(|_| "backup file is not valid JSON".to_string())?;
    if envelope.format != FORMAT {
        return Err("unsupported backup format".into());
    }
    if envelope.kdf.algo != KDF_ALGO
        || envelope.kdf.m != KDF_M
        || envelope.kdf.t != KDF_T
        || envelope.kdf.p != KDF_P
    {
        return Err("unsupported backup KDF parameters".into());
    }
    let salt = b64()
        .decode(&envelope.kdf.salt)
        .map_err(|_| "malformed backup salt".to_string())?;
    if salt.len() != 16 {
        return Err("malformed backup salt".into());
    }
    let base = argon2id_frozen(password.as_bytes(), &salt)?;
    let key = hkdf32(&base, &salt, INFO_BACKUP);
    let payload = aead_open(&key, &envelope.nonce, &envelope.sealed, &envelope_aad())
        .map_err(|_| "wrong password or tampered backup".to_string())?;
    let data: BackupData =
        serde_json::from_slice(&payload).map_err(|_| "backup payload is malformed".to_string())?;
    if data.format != PAYLOAD_FORMAT {
        return Err("unsupported backup payload".into());
    }
    Ok(data)
}

/// Read the known account stores plus the OS-vault secrets they reference.
/// A missing account file is normal; an unavailable secret store is not.
pub fn collect_accounts(base: &Path, secrets: &dyn BackupSecrets) -> Result<AccountBackup, String> {
    let files = collect_files(base)?;
    let mut backup = AccountBackup { files, secrets: BTreeMap::new() };
    let keys = secret_keys(&backup.files)?;
    for key in keys {
        if let Some(value) = secrets.get(&key)? {
            backup.secrets.insert(key, value);
        }
    }
    Ok(backup)
}

fn known_top_level_files() -> &'static [&'static str] {
    &[
        "codex-accounts.json",
        "cursor-accounts.json",
        "antigravity-accounts.json",
        "archived_accounts.json",
        "archived_external_accounts.json",
        "antigravity-token.json",
        "onenewapi.json",
        "search-keys.json",
    ]
}

fn collect_files(base: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut files = BTreeMap::new();
    for directory in ["accounts", "login-accounts"] {
        let path = base.join(directory);
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.ends_with(".json") {
                continue;
            }
            let full = entry.path();
            let text = std::fs::read_to_string(&full)
                .map_err(|error| format!("read {}: {error}", full.display()))?;
            files.insert(format!("{directory}/{name}"), text);
        }
    }
    for name in known_top_level_files() {
        let full = base.join(name);
        if let Ok(text) = std::fs::read_to_string(&full) {
            files.insert((*name).to_string(), text);
        }
    }
    Ok(files)
}

fn secret_keys(files: &BTreeMap<String, String>) -> Result<BTreeSet<String>, String> {
    let mut keys = BTreeSet::new();
    for (relative, text) in files {
        let Ok(value) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) else {
            continue;
        };
        if relative == "search-keys.json" {
            if let Some(entries) = value.get("keys").and_then(Value::as_array) {
                for row in entries {
                    if let Some(id) = row.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()) {
                        keys.insert(crate::secretstore::token_key(&format!("searchkey/{id}")));
                    }
                }
            }
            continue;
        }
        let Some(rows) = value.as_array() else {
            continue;
        };
        if let Some(provider) = relative.strip_prefix("accounts/").and_then(|name| name.strip_suffix(".json")) {
            keys.insert(crate::secretstore::provider_key(provider));
            for row in rows {
                if let Some(id) = row.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()) {
                    keys.insert(crate::secretstore::account_key(id));
                }
            }
        } else if let Some(family) = relative.strip_prefix("login-accounts/").and_then(|name| name.strip_suffix(".json")) {
            for row in rows {
                if let Ok(account) = serde_json::from_value::<crate::login_accounts::LoginAccount>(row.clone()) {
                    let card = crate::login_accounts::card_id_for_account(family, &account);
                    keys.insert(crate::secretstore::token_key(&format!("login/{card}")));
                }
            }
        } else if relative == "codex-accounts.json" {
            for row in rows {
                if let Ok(account) = serde_json::from_value::<crate::codex_accounts::CodexLogin>(row.clone()) {
                    let card = crate::codex_accounts::card_id_for_account(&account);
                    keys.insert(crate::secretstore::token_key(&format!("codex/{card}")));
                }
            }
        } else if relative == "cursor-accounts.json" {
            for row in rows {
                if let Some(card) = row
                    .get("cardId")
                    .or_else(|| row.get("card_id"))
                    .and_then(Value::as_str)
                {
                    keys.insert(crate::secretstore::token_key(&format!("cursor/{card}")));
                }
            }
        } else if relative == "archived_accounts.json" {
            for row in rows {
                if let Some(card) = row.get("card_id").and_then(Value::as_str) {
                    keys.insert(crate::secretstore::archived_key(card));
                }
            }
        } else if relative == "archived_external_accounts.json" {
            for row in rows {
                if let Some(card) = row.get("card_id").and_then(Value::as_str) {
                    keys.insert(crate::secretstore::token_key(&format!("archived-external/{card}")));
                }
            }
        }
    }
    Ok(keys)
}

fn allowed_secret_key(key: &str) -> bool {
    [
        "account:",
        "provider:",
        "token:login/",
        "token:codex/",
        "token:cursor/",
        "token:searchkey/",
        "token:archived-external/",
        "archived:",
    ]
    .iter()
    .any(|prefix| key.starts_with(prefix))
}

pub fn export_with_password(
    base: &Path,
    settings: BTreeMap<String, Value>,
    selection: Selection,
    secrets: &dyn BackupSecrets,
    password: &str,
) -> Result<Vec<u8>, String> {
    let mut data = BackupData {
        format: PAYLOAD_FORMAT.into(),
        created_at: now_ms(),
        settings: selection.settings.then_some(settings),
        accounts: None,
        keys: None,
    };
    if selection.accounts {
        data.accounts = Some(collect_accounts(base, secrets)?);
    }
    if selection.keys {
        let path = base.join("keyvault.json");
        let keyvault = match std::fs::read(&path) {
            Ok(bytes) => {
                validate_vault_bytes(&bytes)?;
                Some(b64().encode(bytes))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("read {}: {error}", path.display())),
        };
        data.keys = Some(KeysBackup { keyvault });
    }
    encode_envelope(&data, password)
}

fn validate_vault_bytes(bytes: &[u8]) -> Result<(), String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| "vault payload is not valid JSON".to_string())?;
    let version = value
        .get("version")
        .and_then(Value::as_u64)
        .ok_or("vault payload has no version")?;
    match version {
        3 => {
            crate::keyvault_v3::parse_v3_value(value.clone())
                .map_err(|error| format!("invalid v3 vault: {error}"))?;
        }
        2 => {
            if value.get("kdf").and_then(Value::as_object).is_none() {
                return Err("v2 vault has no kdf".into());
            }
            if value.get("entries").and_then(Value::as_array).is_none() {
                return Err("v2 vault has no entries".into());
            }
        }
        other => return Err(format!("unsupported vault version {other}")),
    }
    Ok(())
}

pub fn open(bytes: &[u8], password: &str) -> Result<BackupData, String> {
    decode_envelope(bytes, password)
}

pub fn preview(
    base: &Path,
    bytes: &[u8],
    password: &str,
    selection: Selection,
) -> Result<BackupPreview, String> {
    let data = decode_envelope(bytes, password)?;
    let local_files = collect_files(base).unwrap_or_default();
    let settings = data.settings.as_ref().filter(|_| selection.settings);
    let keys = data.keys.as_ref().filter(|_| selection.keys);
    let has_keyvault = keys.and_then(|keys| keys.keyvault.as_ref()).is_some();
    let mut preview = BackupPreview {
        created_at: data.created_at,
        setting_keys: settings
            .map(|settings| settings.keys().cloned().collect())
            .unwrap_or_default(),
        has_keyvault,
        keyvault_bytes: keys
            .and_then(|keys| keys.keyvault.as_ref())
            .map(|encoded| b64().decode(encoded).map(|bytes| bytes.len()).unwrap_or(0))
            .unwrap_or(0),
        keyvault_conflict: has_keyvault && base.join("keyvault.json").exists(),
        ..BackupPreview::default()
    };
    if let Some(accounts) = data.accounts.as_ref().filter(|_| selection.accounts) {
        preview.account_files = accounts.files.len();
        preview.secret_count = accounts.secrets.len();
        let incoming = identities(&accounts.files);
        let local = identities(&local_files);
        preview.account_ids = incoming.iter().take(20).cloned().collect();
        preview.conflicting_accounts = incoming.iter().filter(|id| local.contains(*id)).count();
        preview.added_accounts = incoming.len().saturating_sub(preview.conflicting_accounts);
    }
    Ok(preview)
}

fn identities(files: &BTreeMap<String, String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (relative, text) in files {
        let Ok(value) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) else {
            continue;
        };
        let Some(rows) = value.as_array() else {
            continue;
        };
        for row in rows {
            let id = ["id", "account_id", "accountId", "cardId", "card_id"]
                .iter()
                .find_map(|key| row.get(*key).and_then(Value::as_str))
                .or_else(|| row.get("label").and_then(Value::as_str))
                .unwrap_or("unknown");
            out.insert(format!("{relative}:{id}"));
        }
    }
    out
}

fn backup_path(base: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.contains("..") || relative.starts_with('/') || relative.contains('\\') {
        return Err("backup contains an unsafe path".into());
    }
    let allowed = relative.starts_with("accounts/")
        || relative.starts_with("login-accounts/")
        || known_top_level_files().contains(&relative);
    if !allowed {
        return Err(format!("backup contains an unsupported path: {relative}"));
    }
    Ok(base.join(relative))
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("backup.tmp");
    std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|error| {
            let _ = std::fs::remove_file(&tmp);
            format!("replace {}: {error}", path.display())
        })
}

fn snapshot_file(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("read {}: {error}", path.display())),
    }
}

fn row_identity(row: &Value) -> Option<String> {
    ["card_id", "cardId", "id", "account_id", "accountId"]
        .iter()
        .find_map(|key| row.get(*key).and_then(Value::as_str))
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .or_else(|| {
            row.get("label")
                .and_then(Value::as_str)
                .filter(|label| !label.is_empty())
                .map(|label| format!("label:{label}"))
        })
}

fn merge_rows(local: &[Value], incoming: &[Value]) -> Vec<Value> {
    let mut merged = local.to_vec();
    let mut index = BTreeMap::new();
    for (position, row) in merged.iter().enumerate() {
        if let Some(id) = row_identity(row) {
            index.insert(id, position);
        }
    }
    for row in incoming {
        if let Some(id) = row_identity(row) {
            if let Some(&position) = index.get(&id) {
                merged[position] = row.clone();
                continue;
            }
            index.insert(id, merged.len());
        }
        merged.push(row.clone());
    }
    merged
}

/// Restore merges into the existing store instead of replacing the whole
/// file: accounts added locally after the backup was taken must survive.
fn merge_file(local: Option<&str>, incoming: &str) -> Result<String, String> {
    let incoming_value: Value = serde_json::from_str(incoming.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("parse backup file: {error}"))?;
    let local_value = local.and_then(|text| {
        serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')).ok()
    });
    if let (Some(local_value), Value::Array(incoming_rows)) = (&local_value, &incoming_value) {
        if let Some(local_rows) = local_value.as_array() {
            let merged = merge_rows(local_rows, incoming_rows);
            return serde_json::to_string_pretty(&Value::Array(merged))
                .map_err(|error| error.to_string());
        }
    }
    if let (Some(Value::Object(local_object)), Value::Object(incoming_object)) =
        (&local_value, &incoming_value)
    {
        let mut merged = local_object.clone();
        for (key, incoming_value) in incoming_object {
            if let (Some(local_rows), Some(incoming_rows)) = (
                merged.get(key).and_then(Value::as_array),
                incoming_value.as_array(),
            ) {
                merged.insert(
                    key.clone(),
                    Value::Array(merge_rows(local_rows, incoming_rows)),
                );
            } else {
                merged.insert(key.clone(), incoming_value.clone());
            }
        }
        return serde_json::to_string_pretty(&Value::Object(merged))
            .map_err(|error| error.to_string());
    }
    serde_json::to_string_pretty(&incoming_value).map_err(|error| error.to_string())
}

/// Apply the selected sections. Settings are returned for the command layer
/// (it owns the allowlisted config writer); account files, OS-vault secrets
/// and the vault file are applied here with rollback on failure.
pub fn apply(
    base: &Path,
    bytes: &[u8],
    password: &str,
    selection: Selection,
    secrets: &mut dyn BackupSecrets,
) -> Result<ApplyResult, String> {
    apply_with(base, bytes, password, selection, secrets, |_| Ok(()))
}

/// Same as [`apply`], but the settings section is handed to `apply_settings`
/// inside the same transaction. If that callback fails, account files, OS
/// secrets and the vault file are rolled back before returning.
pub fn apply_with<F>(
    base: &Path,
    bytes: &[u8],
    password: &str,
    selection: Selection,
    secrets: &mut dyn BackupSecrets,
    apply_settings: F,
) -> Result<ApplyResult, String>
where
    F: FnOnce(&BTreeMap<String, Value>) -> Result<(), String>,
{
    let data = decode_envelope(bytes, password)?;
    let settings = if selection.settings {
        data.settings.clone().ok_or("backup has no settings section")?
    } else {
        BTreeMap::new()
    };
    let accounts = if selection.accounts {
        Some(data.accounts.as_ref().ok_or("backup has no account section")?)
    } else {
        None
    };
    let keys = if selection.keys {
        Some(data.keys.as_ref().ok_or("backup has no key section")?)
    } else {
        None
    };

    // Refuse to start if the OS vault cannot take a secret. A one-key probe
    // is removed immediately; this is the only way to keep a half-applied
    // import from silently writing plaintext fallbacks, even when the backup
    // itself carries no OS-vault secrets.
    if accounts.is_some() {
        preflight_secret_store(secrets)?;
    }

    let vault_bytes = keys
        .and_then(|section| section.keyvault.as_ref())
        .map(|encoded| {
            let bytes = b64()
                .decode(encoded)
                .map_err(|_| "malformed vault payload".to_string())?;
            validate_vault_bytes(&bytes)?;
            Ok::<Vec<u8>, String>(bytes)
        })
        .transpose()?;

    let mut old_files: BTreeMap<PathBuf, Option<Vec<u8>>> = BTreeMap::new();
    let mut old_secrets: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut old_vault: Option<Option<Vec<u8>>> = None;
    let mut account_files = 0;
    let mut secret_count = 0;

    let outcome = (|| -> Result<(), String> {
        if let Some(accounts) = accounts {
            if accounts.secrets.len() > 2_000 {
                return Err("backup contains too many secrets".into());
            }
            for (relative, text) in &accounts.files {
                let path = backup_path(base, relative)?;
                let old = snapshot_file(&path)?;
                let local_text = old
                    .as_deref()
                    .and_then(|bytes| std::str::from_utf8(bytes).ok());
                let merged = merge_file(local_text, text)?;
                old_files.insert(path.clone(), old);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|error| format!("create {}: {error}", parent.display()))?;
                }
                crate::accounts::write_json_atomic(&path, merged)?;
                account_files += 1;
            }
            for (key, value) in &accounts.secrets {
                if !allowed_secret_key(key) {
                    return Err("backup contains an unsupported secret key".into());
                }
                old_secrets.insert(key.clone(), secrets.get(key)?);
                secrets.put(key, value)?;
                secret_count += 1;
            }
        }
        if let Some(vault) = &vault_bytes {
            let path = base.join("keyvault.json");
            old_vault = Some(snapshot_file(&path)?);
            write_bytes_atomic(&path, vault)?;
        }
        if selection.settings {
            apply_settings(&settings)?;
        }
        Ok(())
    })();

    if let Err(error) = outcome {
        let rollback_errors = rollback(&old_files, &old_secrets, &old_vault, base, secrets);
        let suffix = if rollback_errors.is_empty() {
            String::new()
        } else {
            format!("; rollback failed: {}", rollback_errors.join("; "))
        };
        return Err(format!("{error}{suffix}"));
    }

    Ok(ApplyResult {
        settings,
        account_files,
        secrets: secret_count,
        keyvault_written: vault_bytes.is_some(),
    })
}

fn preflight_secret_store(secrets: &mut dyn BackupSecrets) -> Result<(), String> {
    let probe = "__pane_backup_probe";
    secrets.put(probe, "1")?;
    secrets.remove(probe);
    Ok(())
}

fn rollback(
    old_files: &BTreeMap<PathBuf, Option<Vec<u8>>>,
    old_secrets: &BTreeMap<String, Option<String>>,
    old_vault: &Option<Option<Vec<u8>>>,
    base: &Path,
    secrets: &mut dyn BackupSecrets,
) -> Vec<String> {
    let mut errors = Vec::new();
    for (path, old) in old_files {
        let result = match old {
            Some(bytes) => write_bytes_atomic(path, bytes),
            None => match std::fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!("remove {}: {error}", path.display())),
            },
        };
        if let Err(error) = result {
            errors.push(error);
        }
    }
    for (key, old) in old_secrets {
        match old {
            Some(value) => {
                if let Err(error) = secrets.put(key, value) {
                    errors.push(format!("restore secret {key}: {error}"));
                }
            }
            None => secrets.remove(key),
        }
    }
    if let Some(old) = old_vault {
        let path = base.join("keyvault.json");
        let result = match old {
            Some(bytes) => write_bytes_atomic(&path, bytes),
            None => match std::fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!("remove {}: {error}", path.display())),
            },
        };
        if let Err(error) = result {
            errors.push(error);
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MapSecrets(BTreeMap<String, String>, bool);

    impl BackupSecrets for MapSecrets {
        fn get(&self, key: &str) -> Result<Option<String>, String> {
            if !self.1 {
                return Err("vault unavailable".into());
            }
            Ok(self.0.get(key).cloned())
        }
        fn put(&mut self, key: &str, value: &str) -> Result<(), String> {
            if !self.1 {
                return Err("vault unavailable".into());
            }
            self.0.insert(key.into(), value.into());
            Ok(())
        }
        fn remove(&mut self, key: &str) {
            self.0.remove(key);
        }
    }

    fn temp_base(tag: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "pane-backup-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    fn seed(base: &Path) -> MapSecrets {
        std::fs::create_dir_all(base.join("accounts")).unwrap();
        std::fs::create_dir_all(base.join("login-accounts")).unwrap();
        std::fs::write(
            base.join("accounts").join("workbuddy.json"),
            r#"[{"id":"workbuddy@abc","label":"Work"}]"#,
        )
        .unwrap();
        std::fs::write(
            base.join("login-accounts").join("workbuddy.json"),
            r#"[{"account_id":"uid-1","label":"One","access_token":"","refresh_token":"","id_token":"","expires_at":"","added_at":0}]"#,
        )
        .unwrap();
        std::fs::write(
            base.join("keyvault.json"),
            r#"{"version":2,"kdf":{},"canary_nonce_b64":"","canary_b64":"","entries":[]}"#,
        )
        .unwrap();
        let mut secrets = MapSecrets(BTreeMap::new(), true);
        secrets
            .0
            .insert("account:workbuddy@abc".into(), "secret-key".into());
        let login: crate::login_accounts::LoginAccount = serde_json::from_str(
            r#"{"account_id":"uid-1","label":"One","access_token":"","refresh_token":"","id_token":"","expires_at":"","added_at":0}"#,
        )
        .unwrap();
        let card = crate::login_accounts::card_id_for_account("workbuddy", &login);
        secrets
            .0
            .insert(format!("token:login/{card}"), "token-triple".into());
        secrets
    }

    fn settings() -> BTreeMap<String, Value> {
        BTreeMap::from([("theme".to_string(), Value::String("dark".into()))])
    }

    #[test]
    fn roundtrip_carries_files_secrets_settings_and_vault() {
        let base = temp_base("roundtrip");
        let secrets = seed(&base);
        let bytes = export_with_password(&base, settings(), Selection::all(), &secrets, "backup-pass-123").unwrap();
        let encoded = String::from_utf8_lossy(&bytes);
        assert!(!encoded.contains("secret-key"));
        assert!(!encoded.contains("token-triple"));

        let preview = preview(&base, &bytes, "backup-pass-123", Selection::all()).unwrap();
        assert_eq!(preview.setting_keys, vec!["theme"]);
        assert!(preview.has_keyvault);
        assert!(preview.keyvault_conflict);

        let target = temp_base("roundtrip-target");
        let mut target_secrets = MapSecrets(BTreeMap::new(), true);
        let applied = apply(&target, &bytes, "backup-pass-123", Selection::all(), &mut target_secrets).unwrap();
        assert_eq!(applied.settings, settings());
        assert_eq!(applied.account_files, 2);
        assert_eq!(applied.secrets, 2);
        assert!(applied.keyvault_written);
        let restored: Value = serde_json::from_str(
            &std::fs::read_to_string(target.join("accounts").join("workbuddy.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(restored, serde_json::json!([{"id": "workbuddy@abc", "label": "Work"}]));
        assert_eq!(
            target_secrets.0.get("account:workbuddy@abc").map(String::as_str),
            Some("secret-key")
        );
        let _ = secrets;
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn wrong_password_and_tamper_are_rejected() {
        let base = temp_base("tamper");
        let secrets = seed(&base);
        let bytes = export_with_password(&base, settings(), Selection::all(), &secrets, "backup-pass-123").unwrap();
        assert!(open(&bytes, "wrong").is_err());

        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        let sealed = value.get_mut("sealed").unwrap();
        let mut text = sealed.as_str().unwrap().to_string();
        text.push('A');
        *sealed = Value::String(text);
        assert!(open(&serde_json::to_vec(&value).unwrap(), "backup-pass-123").is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn restore_merges_local_only_accounts_instead_of_dropping_them() {
        let source = temp_base("merge-source");
        let secrets = seed(&source);
        let bytes = export_with_password(&source, settings(), Selection::all(), &secrets, "backup-pass-123").unwrap();

        let target = temp_base("merge-target");
        std::fs::create_dir_all(target.join("accounts")).unwrap();
        std::fs::write(
            target.join("accounts").join("workbuddy.json"),
            r#"[{"id":"workbuddy@local","label":"Local only"}]"#,
        )
        .unwrap();
        let mut target_secrets = MapSecrets(BTreeMap::new(), true);
        apply(&target, &bytes, "backup-pass-123", Selection::all(), &mut target_secrets).unwrap();

        let merged: Value = serde_json::from_str(
            &std::fs::read_to_string(target.join("accounts").join("workbuddy.json")).unwrap(),
        )
        .unwrap();
        let ids: Vec<&str> = merged
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str))
            .collect();
        assert!(ids.contains(&"workbuddy@local"));
        assert!(ids.contains(&"workbuddy@abc"));
        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn unsafe_paths_and_secret_keys_are_rejected() {
        let data = BackupData {
            format: PAYLOAD_FORMAT.into(),
            created_at: now_ms(),
            settings: None,
            accounts: Some(AccountBackup {
                files: BTreeMap::from([("accounts/../../config.json".into(), "{}".into())]),
                secrets: BTreeMap::new(),
            }),
            keys: None,
        };
        let bytes = encode_envelope(&data, "backup-pass-123").unwrap();
        let target = temp_base("unsafe-path");
        let mut secrets = MapSecrets(BTreeMap::new(), true);
        assert!(apply(&target, &bytes, "backup-pass-123", Selection { accounts: true, keys: false, settings: false }, &mut secrets).is_err());
        let _ = std::fs::remove_dir_all(&target);

        let data = BackupData {
            format: PAYLOAD_FORMAT.into(),
            created_at: now_ms(),
            settings: None,
            accounts: Some(AccountBackup {
                files: BTreeMap::new(),
                secrets: BTreeMap::from([("token:master".into(), "x".into())]),
            }),
            keys: None,
        };
        let bytes = encode_envelope(&data, "backup-pass-123").unwrap();
        let target = temp_base("bad-secret");
        let mut secrets = MapSecrets(BTreeMap::new(), true);
        assert!(apply(&target, &bytes, "backup-pass-123", Selection { accounts: true, keys: false, settings: false }, &mut secrets).is_err());
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn settings_failure_rolls_back_accounts_secrets_and_vault() {
        let source = temp_base("settings-rollback-source");
        let secrets = seed(&source);
        let bytes = export_with_password(&source, settings(), Selection::all(), &secrets, "backup-pass-123").unwrap();

        let target = temp_base("settings-rollback-target");
        std::fs::create_dir_all(target.join("accounts")).unwrap();
        std::fs::write(
            target.join("accounts").join("workbuddy.json"),
            r#"[{"id":"local","label":"Keep"}]"#,
        )
        .unwrap();
        std::fs::write(
            target.join("keyvault.json"),
            r#"{"version":2,"kdf":{},"canary_nonce_b64":"","canary_b64":"","entries":[]}"#,
        )
        .unwrap();
        let mut target_secrets = MapSecrets(
            BTreeMap::from([("account:workbuddy@abc".to_string(), "old".to_string())]),
            true,
        );
        let error = apply_with(
            &target,
            &bytes,
            "backup-pass-123",
            Selection::all(),
            &mut target_secrets,
            |_| Err("config write failed".into()),
        )
        .unwrap_err();
        assert!(error.contains("config write failed"));
        let restored: Value = serde_json::from_str(
            &std::fs::read_to_string(target.join("accounts").join("workbuddy.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(restored.as_array().unwrap().len(), 1);
        assert_eq!(restored[0]["id"], "local");
        assert_eq!(
            target_secrets.0.get("account:workbuddy@abc").map(String::as_str),
            Some("old")
        );
        let vault: Value = serde_json::from_str(
            &std::fs::read_to_string(target.join("keyvault.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(vault.get("version").and_then(Value::as_u64), Some(2));
        assert_eq!(vault.get("entries").and_then(Value::as_array).map(Vec::len), Some(0));
        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn corrupt_or_unknown_vault_payload_is_rejected() {
        let base = temp_base("vault-validation");
        let secrets = seed(&base);
        std::fs::write(base.join("keyvault.json"), r#"{"version":99}"#).unwrap();
        assert!(export_with_password(&base, settings(), Selection::all(), &secrets, "backup-pass-123").is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn selection_limits_sections_and_vault_write() {
        let base = temp_base("selection");
        let secrets = seed(&base);
        let selection = Selection { accounts: true, keys: false, settings: false };
        let bytes = export_with_password(&base, settings(), selection, &secrets, "backup-pass-123").unwrap();
        let data = open(&bytes, "backup-pass-123").unwrap();
        assert!(data.accounts.is_some());
        assert!(data.keys.is_none());
        assert!(data.settings.is_none());

        let target = temp_base("selection-target");
        let mut target_secrets = MapSecrets(BTreeMap::new(), true);
        let applied = apply(&target, &bytes, "backup-pass-123", selection, &mut target_secrets).unwrap();
        assert_eq!(applied.account_files, 2);
        assert!(!applied.keyvault_written);
        assert!(applied.settings.is_empty());
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&target);
    }

    #[test]
    fn unavailable_vault_aborts_export_instead_of_dropping_secrets() {
        let base = temp_base("unavailable");
        let mut secrets = seed(&base);
        secrets.1 = false;
        assert!(export_with_password(&base, settings(), Selection::all(), &secrets, "backup-pass-123").is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn import_preview_counts_existing_accounts_as_conflicts() {
        let base = temp_base("conflict");
        let secrets = seed(&base);
        let bytes = export_with_password(&base, settings(), Selection::all(), &secrets, "backup-pass-123").unwrap();
        let preview = preview(&base, &bytes, "backup-pass-123", Selection::all()).unwrap();
        assert_eq!(preview.conflicting_accounts, 2);
        assert_eq!(preview.added_accounts, 0);
        let _ = std::fs::remove_dir_all(&base);
    }
}
