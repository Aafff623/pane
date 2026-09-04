use super::fingerprint::{DisplayUnit, SiteFingerprint};
use super::ids::new_id_avoiding;
use super::url::NormalizedUrl;
use super::{CreateSiteResult, KeyDto, SiteDto};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoreFile {
    pub version: u32,
    pub sites: Vec<SiteRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SiteRecord {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub next_key_ordinal: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency_symbol: Option<String>,
    /// Raw-quota → display-value scale from the probe (`/api/status`).
    /// Absent in older files; consumers fall back to new-api defaults.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_unit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<f64>,
    /// Optional NewAPI dashboard access token (个人设置 → 系统访问令牌)
    /// plus the matching numeric user id (rc builds demand it as the
    /// `New-Api-User` header). Empty means not configured.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub access_token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user_id: String,
    pub keys: Vec<KeyRecord>,
}

impl SiteRecord {
    pub fn quota_display(&self) -> DisplayUnit {
        DisplayUnit::from_store(
            self.display_unit.as_deref(),
            self.currency_symbol.as_deref(),
        )
    }

    fn set_display_unit(&mut self, display: DisplayUnit) {
        let (unit, symbol) = display.to_store();
        self.display_unit = unit;
        self.currency_symbol = symbol;
    }

    fn set_fingerprint(&mut self, fp: &SiteFingerprint) {
        self.set_display_unit(fp.unit.clone());
        self.per_unit = Some(fp.per_unit);
        self.rate = Some(fp.rate);
    }

    /// (quota per display unit, USD-multiplier) with new-api defaults for
    /// files written before the scale was captured.
    pub fn quota_scale(&self) -> (f64, f64) {
        let per_unit = self.per_unit.filter(|n| n.is_finite() && *n > 0.0);
        let rate = self.rate.filter(|n| n.is_finite() && *n >= 0.0);
        (per_unit.unwrap_or(500_000.0), rate.unwrap_or(1.0))
    }

    pub fn has_access_token(&self) -> bool {
        !self.access_token.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyRecord {
    pub id: String,
    pub label: String,
    pub api_key: String,
}

impl SiteRecord {
    pub fn to_dto(&self) -> SiteDto {
        SiteDto {
            id: self.id.clone(),
            name: self.name.clone(),
            base_url: self.base_url.clone(),
            has_access_token: self.has_access_token(),
            // The dashboard user id is not a credential (it rides along on
            // every dashboard request anyway), so the UI can pre-fill it.
            user_id: self.user_id.clone(),
            keys: self.keys.iter().map(KeyRecord::to_dto).collect(),
        }
    }
}

impl KeyRecord {
    fn to_dto(&self) -> KeyDto {
        KeyDto {
            id: self.id.clone(),
            label: self.label.clone(),
            has_api_key: !self.api_key.is_empty(),
        }
    }
}

pub fn load(path: &Path) -> Result<StoreFile, String> {
    if !path.exists() {
        return Ok(StoreFile {
            version: 1,
            sites: Vec::new(),
        });
    }
    if path.is_file() {
        let _ = restrict_owner_only(path);
    }
    let raw = super::super::read_small_text(path, 1_048_576, "onenewapi.json")?;
    let raw = raw.trim_start_matches('\u{feff}');
    let doc: StoreFile =
        serde_json::from_str(raw).map_err(|e| format!("onenewapi.json is unreadable: {e}"))?;
    if doc.version != 1 {
        return Err(format!(
            "onenewapi.json has unsupported version {}",
            doc.version
        ));
    }
    Ok(doc)
}

pub fn save(path: &Path, doc: &StoreFile) -> Result<(), String> {
    if path.exists() {
        load(path)?;
    }
    let raw =
        serde_json::to_string_pretty(doc).map_err(|e| format!("serialize onenewapi.json: {e}"))?;
    atomic_write(path, &raw)
}

fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let dir = path
        .parent()
        .ok_or_else(|| "onenewapi.json path is missing a directory".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| format!("create config dir: {e}"))?;
    let tmp = dir.join(format!(
        "onenewapi.{}.tmp",
        super::ids::new_id().unwrap_or_else(|_| "tmp".into())
    ));
    let result = write_private_then_replace(path, &tmp, contents);
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn write_private_then_replace(path: &Path, tmp: &Path, contents: &str) -> Result<(), String> {
    create_empty(tmp)?;
    restrict_owner_only(tmp)?;
    std::fs::write(tmp, contents).map_err(|e| format!("write onenewapi.json: {e}"))?;
    if path.exists() {
        replace_existing(tmp, path).map_err(|e| format!("replace onenewapi.json: {e}"))?;
    } else {
        std::fs::rename(tmp, path).map_err(|e| format!("replace onenewapi.json: {e}"))?;
    }
    restrict_owner_only(path)
}

fn create_empty(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        drop(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .map_err(|e| format!("write onenewapi.json: {e}"))?,
        );
        Ok(())
    }
    #[cfg(not(unix))]
    {
        drop(std::fs::File::create_new(path).map_err(|e| format!("write onenewapi.json: {e}"))?);
        Ok(())
    }
}

fn restrict_owner_only(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let sid = current_user_sid()?;
        set_protected_dacl(path, &format!("D:P(A;;FA;;;SY)(A;;FA;;;{sid})"))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("restrict onenewapi.json: {e}"))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(windows)]
fn current_user_sid() -> Result<String, String> {
    static SID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    if let Some(s) = SID.get() {
        return Ok(s.clone());
    }
    let s = query_current_user_sid()?;
    Ok(SID.get_or_init(|| s).clone())
}

#[cfg(windows)]
struct LocalAlloc(*mut core::ffi::c_void);

#[cfg(windows)]
impl Drop for LocalAlloc {
    fn drop(&mut self) {
        if !self.0.is_null() {
            use windows::Win32::Foundation::{LocalFree, HLOCAL};
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0)));
            }
        }
    }
}

#[cfg(windows)]
fn query_current_user_sid() -> Result<String, String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
    use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)
            .map_err(|e| format!("restrict onenewapi.json: {e}"))?;
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        let mut buf = vec![0u8; needed as usize];
        let info = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            needed,
            &mut needed,
        );
        let _ = CloseHandle(token);
        info.map_err(|e| format!("restrict onenewapi.json: {e}"))?;
        let user = &*(buf.as_ptr() as *const TOKEN_USER);
        let mut sid = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut sid)
            .map_err(|e| format!("restrict onenewapi.json: {e}"))?;
        let _free = LocalAlloc(sid.0.cast());
        sid.to_string()
            .map_err(|e| format!("restrict onenewapi.json: {e}"))
    }
}

#[cfg(windows)]
fn set_protected_dacl(path: &Path, sddl: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SetNamedSecurityInfoW,
        SDDL_REVISION_1, SE_FILE_OBJECT,
    };
    use windows::Win32::Security::{
        GetSecurityDescriptorDacl, ACL, DACL_SECURITY_INFORMATION,
        PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR,
    };

    let path_w = path_wide(path);
    let sddl_w: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
    let mut sd = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl_w.as_ptr()),
            SDDL_REVISION_1,
            &mut sd,
            None,
        )
        .map_err(|e| format!("restrict onenewapi.json: {e}"))?;
        let _free = LocalAlloc(sd.0);
        let mut present = windows::core::BOOL::default();
        let mut defaulted = windows::core::BOOL::default();
        let mut dacl: *mut ACL = std::ptr::null_mut();
        GetSecurityDescriptorDacl(sd, &mut present, &mut dacl, &mut defaulted)
            .map_err(|e| format!("restrict onenewapi.json: {e}"))?;
        if !present.as_bool() || dacl.is_null() {
            return Err("restrict onenewapi.json: missing DACL".into());
        }
        let err = SetNamedSecurityInfoW(
            PCWSTR(path_w.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(dacl),
            None,
        );
        if err != ERROR_SUCCESS {
            return Err(format!("restrict onenewapi.json: {err:?}"));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn path_wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(all(windows, test))]
fn dacl_sddl(path: &Path) -> Result<String, String> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::Security::Authorization::{
        ConvertSecurityDescriptorToStringSecurityDescriptorW, GetNamedSecurityInfoW,
        SDDL_REVISION_1, SE_FILE_OBJECT,
    };
    use windows::Win32::Security::{DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR};

    let path_w = path_wide(path);
    let mut sd = PSECURITY_DESCRIPTOR::default();
    unsafe {
        let err = GetNamedSecurityInfoW(
            PCWSTR(path_w.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            None,
            None,
            &mut sd,
        );
        if err != ERROR_SUCCESS {
            return Err(format!("read DACL: {err:?}"));
        }
        let _free_sd = LocalAlloc(sd.0);
        let mut sddl = PWSTR::null();
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            sd,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut sddl,
            None,
        )
        .map_err(|e| format!("read DACL: {e}"))?;
        let _free_sddl = LocalAlloc(sddl.0.cast());
        sddl.to_string().map_err(|e| format!("read DACL: {e}"))
    }
}

#[cfg(windows)]
fn replace_existing(replacement: &Path, destination: &Path) -> std::io::Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let destination = path_wide(destination);
    let replacement = path_wide(replacement);
    unsafe {
        MoveFileExW(
            PCWSTR(replacement.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(std::io::Error::other)
}

#[cfg(not(windows))]
fn replace_existing(replacement: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(replacement, destination)
}

fn occupied_ids(doc: &StoreFile) -> HashSet<String> {
    let mut ids = HashSet::new();
    for site in &doc.sites {
        ids.insert(site.id.clone());
        for key in &site.keys {
            ids.insert(key.id.clone());
        }
    }
    ids
}

fn display_name(name: &str, hostname: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        hostname.to_string()
    } else {
        name.to_string()
    }
}

pub fn list_sites(path: &Path) -> Result<Vec<SiteDto>, String> {
    Ok(load(path)?.sites.iter().map(SiteRecord::to_dto).collect())
}

pub fn insert_site(
    path: &Path,
    name: &str,
    normalized: &NormalizedUrl,
    fingerprint: SiteFingerprint,
) -> Result<CreateSiteResult, String> {
    let mut doc = load(path)?;
    if let Some(existing) = doc.sites.iter().find(|s| s.base_url == normalized.origin) {
        return Ok(CreateSiteResult::Duplicate {
            site_id: existing.id.clone(),
        });
    }
    let id = new_id_avoiding(&occupied_ids(&doc))?;
    let (display_unit, currency_symbol) = fingerprint.unit.to_store();
    let per_unit = Some(fingerprint.per_unit);
    let rate = Some(fingerprint.rate);
    let site = SiteRecord {
        id,
        name: display_name(name, &normalized.hostname),
        base_url: normalized.origin.clone(),
        next_key_ordinal: 1,
        display_unit,
        currency_symbol,
        per_unit,
        rate,
        access_token: String::new(),
        user_id: String::new(),
        keys: Vec::new(),
    };
    let dto = site.to_dto();
    doc.sites.push(site);
    save(path, &doc)?;
    Ok(CreateSiteResult::Created { site: dto })
}

pub fn update_site(
    path: &Path,
    id: &str,
    name: Option<String>,
    new_url: Option<NormalizedUrl>,
    fingerprint: Option<SiteFingerprint>,
) -> Result<SiteDto, String> {
    let mut doc = load(path)?;
    if let Some(ref n) = new_url {
        if doc
            .sites
            .iter()
            .any(|s| s.id != id && s.base_url == n.origin)
        {
            return Err("a site with this URL already exists".into());
        }
    }
    let site = doc
        .sites
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| "site not found".to_string())?;
    if let Some(n) = new_url {
        site.base_url = n.origin;
        if let Some(ref name) = name {
            site.name = display_name(name, &n.hostname);
        }
        if let Some(fp) = fingerprint {
            site.set_fingerprint(&fp);
        }
    } else if let Some(ref name) = name {
        let hostname = reqwest::Url::parse(&site.base_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| site.base_url.clone());
        site.name = display_name(name, &hostname);
    }
    let dto = site.to_dto();
    save(path, &doc)?;
    Ok(dto)
}

/// Backfill write: no-op unless this site is still at `origin` and still
/// missing a display unit or a quota scale (pre-scale sites must keep
/// receiving the scale once the panel exposes it). Returns whether the file
/// was written.
pub fn set_fingerprint_if_still_missing_at_origin(
    path: &Path,
    id: &str,
    origin: &str,
    fingerprint: SiteFingerprint,
) -> Result<bool, String> {
    let mut doc = load(path)?;
    let Some(site) = doc.sites.iter_mut().find(|s| s.id == id) else {
        return Ok(false);
    };
    if site.base_url != origin || (site.display_unit.is_some() && site.per_unit.is_some()) {
        return Ok(false);
    }
    site.set_fingerprint(&fingerprint);
    save(path, &doc)?;
    Ok(true)
}

/// Saves (or clears, with an empty string) the site's NewAPI dashboard
/// access token / matching user id used for the subscription endpoint.
/// `None` keeps the stored value untouched.
pub fn set_site_auth(
    path: &Path,
    id: &str,
    access_token: Option<&str>,
    user_id: Option<&str>,
) -> Result<SiteDto, String> {
    let mut doc = load(path)?;
    let site = doc
        .sites
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| "site not found".to_string())?;
    if let Some(token) = access_token {
        site.access_token = token.trim().to_string();
    }
    if let Some(user_id) = user_id {
        site.user_id = user_id.trim().to_string();
    }
    let dto = site.to_dto();
    save(path, &doc)?;
    Ok(dto)
}

pub fn delete_site(path: &Path, id: &str) -> Result<(), String> {
    let mut doc = load(path)?;
    let before = doc.sites.len();
    doc.sites.retain(|s| s.id != id);
    if doc.sites.len() == before {
        return Err("site not found".into());
    }
    save(path, &doc)
}

pub fn configured_key_count_at(path: &Path) -> usize {
    load(path)
        .map(|doc| doc.sites.iter().map(|s| s.keys.len()).sum())
        .unwrap_or(0)
}

fn total_keys(doc: &StoreFile) -> usize {
    doc.sites.iter().map(|s| s.keys.len()).sum()
}

fn assign_label(
    site: &mut SiteRecord,
    label: &str,
    exclude_id: Option<&str>,
) -> Result<String, String> {
    let taken = |site: &SiteRecord, candidate: &str| {
        site.keys
            .iter()
            .any(|k| exclude_id != Some(k.id.as_str()) && k.label == candidate)
    };
    let trimmed = label.trim();
    if !trimmed.is_empty() {
        if taken(site, trimmed) {
            return Err("a key with this label already exists on this site".into());
        }
        return Ok(trimmed.to_string());
    }
    loop {
        let n = site.next_key_ordinal;
        let candidate = format!("Key {n}");
        site.next_key_ordinal = n
            .checked_add(1)
            .ok_or_else(|| "could not allocate a key label".to_string())?;
        if !taken(site, &candidate) {
            return Ok(candidate);
        }
    }
}

pub fn create_key(
    path: &Path,
    site_id: &str,
    label: &str,
    api_key: &str,
) -> Result<super::CreatedKey, String> {
    let secret = api_key.trim();
    if secret.is_empty() {
        return Err("API key is required".into());
    }
    let mut doc = load(path)?;
    let first_key = total_keys(&doc) == 0;
    let occupied = occupied_ids(&doc);
    let site = doc
        .sites
        .iter_mut()
        .find(|s| s.id == site_id)
        .ok_or_else(|| "site not found".to_string())?;
    if site.keys.iter().any(|k| k.api_key == secret) {
        return Err("this API key is already saved on this site".into());
    }
    let id = new_id_avoiding(&occupied)?;
    let label = assign_label(site, label, None)?;
    site.keys.push(KeyRecord {
        id: id.clone(),
        label,
        api_key: secret.to_string(),
    });
    let dto = site.to_dto();
    save(path, &doc)?;
    Ok(super::CreatedKey {
        site: dto,
        key_id: id,
        first_key,
    })
}

pub fn update_key(
    path: &Path,
    site_id: &str,
    key_id: &str,
    label: Option<String>,
    api_key: Option<String>,
) -> Result<super::SiteDto, String> {
    let mut doc = load(path)?;
    let site = doc
        .sites
        .iter_mut()
        .find(|s| s.id == site_id)
        .ok_or_else(|| "site not found".to_string())?;
    let idx = site
        .keys
        .iter()
        .position(|k| k.id == key_id)
        .ok_or_else(|| "key not found".to_string())?;
    if let Some(ref raw) = api_key {
        let secret = raw.trim();
        if !secret.is_empty() {
            if site
                .keys
                .iter()
                .any(|k| k.id != key_id && k.api_key == secret)
            {
                return Err("this API key is already saved on this site".into());
            }
            site.keys[idx].api_key = secret.to_string();
        }
    }
    if let Some(ref raw) = label {
        let assigned = assign_label(site, raw, Some(key_id))?;
        site.keys[idx].label = assigned;
    }
    let dto = site.to_dto();
    save(path, &doc)?;
    Ok(dto)
}

pub fn delete_key(path: &Path, site_id: &str, key_id: &str) -> Result<super::SiteDto, String> {
    let mut doc = load(path)?;
    let site = doc
        .sites
        .iter_mut()
        .find(|s| s.id == site_id)
        .ok_or_else(|| "site not found".to_string())?;
    let before = site.keys.len();
    site.keys.retain(|k| k.id != key_id);
    if site.keys.len() == before {
        return Err("key not found".into());
    }
    let dto = site.to_dto();
    save(path, &doc)?;
    Ok(dto)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::onenewapi::url::normalize_base_url;
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempStore {
        dir: PathBuf,
        path: PathBuf,
    }

    impl TempStore {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let dir =
                std::env::temp_dir().join(format!("pane-onenewapi-{}-{stamp}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("onenewapi.json");
            Self { dir, path }
        }
    }

    impl Drop for TempStore {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn https(host: &str) -> NormalizedUrl {
        normalize_base_url(&format!("https://{host}")).unwrap()
    }

    fn fp(unit: DisplayUnit) -> SiteFingerprint {
        SiteFingerprint::from(unit)
    }

    #[test]
    fn missing_file_is_empty_sites() {
        let tmp = TempStore::new();
        let doc = load(&tmp.path).unwrap();
        assert_eq!(doc.version, 1);
        assert!(doc.sites.is_empty());
        assert!(list_sites(&tmp.path).unwrap().is_empty());
    }

    #[test]
    fn save_then_load_round_trip() {
        let tmp = TempStore::new();
        let created = insert_site(
            &tmp.path,
            "Panel",
            &https("one.example.com"),
            fp(DisplayUnit::Usd),
        )
        .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        assert_eq!(site.name, "Panel");
        assert_eq!(site.base_url, "https://one.example.com");
        assert!(site.keys.is_empty());
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.sites.len(), 1);
        assert_eq!(loaded.sites[0].next_key_ordinal, 1);
        assert_eq!(loaded.sites[0].keys.len(), 0);
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Usd);
        let raw = fs::read_to_string(&tmp.path).unwrap();
        assert!(raw.contains("\"baseUrl\""));
        assert!(raw.contains("\"nextKeyOrdinal\""));
        assert!(raw.contains("\"displayUnit\""));
        assert!(!raw.contains("\"base_url\""));
    }

    #[test]
    fn atomic_write_replaces_existing() {
        let tmp = TempStore::new();
        insert_site(&tmp.path, "A", &https("a.example.com"), fp(DisplayUnit::Usd)).unwrap();
        insert_site(&tmp.path, "B", &https("b.example.com"), fp(DisplayUnit::Usd)).unwrap();
        let listed = list_sites(&tmp.path).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].name, "A");
        assert_eq!(listed[1].name, "B");
        let leftovers: Vec<_> = fs::read_dir(&tmp.dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
        assert!(is_owner_only(&tmp.path), "{}", owner_only_debug(&tmp.path));
    }

    #[test]
    fn atomic_write_does_not_inherit_permissive_parent() {
        let tmp = TempStore::new();
        make_inheritable_world_readable(&tmp.dir);
        let control = tmp.dir.join("control.txt");
        fs::write(&control, "x").unwrap();
        assert!(
            !is_owner_only(&control),
            "parent should leak to ordinary files: {}",
            owner_only_debug(&control)
        );
        insert_site(&tmp.path, "A", &https("acl.example.com"), fp(DisplayUnit::Usd)).unwrap();
        assert!(is_owner_only(&tmp.path), "{}", owner_only_debug(&tmp.path));
        insert_site(&tmp.path, "B", &https("acl2.example.com"), fp(DisplayUnit::Usd)).unwrap();
        assert!(is_owner_only(&tmp.path), "{}", owner_only_debug(&tmp.path));
        let leftovers: Vec<_> = fs::read_dir(&tmp.dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn load_tightens_existing_world_readable_file() {
        let tmp = TempStore::new();
        let raw = json!({ "version": 1, "sites": [] }).to_string();
        fs::write(&tmp.path, &raw).unwrap();
        make_world_readable_file(&tmp.path);
        assert!(
            !is_owner_only(&tmp.path),
            "precondition: {}",
            owner_only_debug(&tmp.path)
        );
        let doc = load(&tmp.path).unwrap();
        assert!(doc.sites.is_empty());
        assert_eq!(fs::read_to_string(&tmp.path).unwrap(), raw);
        assert!(is_owner_only(&tmp.path), "{}", owner_only_debug(&tmp.path));
    }

    fn is_owner_only(path: &Path) -> bool {
        #[cfg(windows)]
        {
            let Ok(sddl) = super::dacl_sddl(path) else {
                return false;
            };
            let Ok(sid) = super::current_user_sid() else {
                return false;
            };
            sddl.contains("D:P")
                && sddl.contains(&sid)
                && sddl.contains(";SY)")
                && !sddl.contains(";WD)")
                && !sddl.contains(";BU)")
                && !sddl.contains(";AU)")
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(path).unwrap().permissions().mode() & 0o777 == 0o600
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = path;
            true
        }
    }

    fn owner_only_debug(path: &Path) -> String {
        #[cfg(windows)]
        {
            super::dacl_sddl(path).unwrap_or_else(|e| e)
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            format!(
                "{:o}",
                fs::metadata(path).unwrap().permissions().mode() & 0o777
            )
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = path;
            "n/a".into()
        }
    }

    fn make_inheritable_world_readable(dir: &Path) {
        #[cfg(windows)]
        {
            let sid = super::current_user_sid().unwrap();
            super::set_protected_dacl(
                dir,
                &format!("D:P(A;OICI;FA;;;WD)(A;OICI;FA;;;SY)(A;OICI;FA;;;{sid})"),
            )
            .unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dir, fs::Permissions::from_mode(0o777)).unwrap();
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = dir;
        }
    }

    fn make_world_readable_file(path: &Path) {
        #[cfg(windows)]
        {
            let sid = super::current_user_sid().unwrap();
            super::set_protected_dacl(path, &format!("D:P(A;;FA;;;WD)(A;;FA;;;SY)(A;;FA;;;{sid})"))
                .unwrap();
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = path;
        }
    }

    #[test]
    fn corrupt_file_fail_closed_and_save_does_not_clobber() {
        let tmp = TempStore::new();
        fs::write(&tmp.path, "{not json").unwrap();
        assert!(load(&tmp.path).is_err());
        let garbage = fs::read_to_string(&tmp.path).unwrap();
        let err =
            insert_site(&tmp.path, "X", &https("x.example.com"), fp(DisplayUnit::Usd)).unwrap_err();
        assert!(!err.is_empty());
        assert_eq!(fs::read_to_string(&tmp.path).unwrap(), garbage);
        let empty = StoreFile {
            version: 1,
            sites: vec![],
        };
        assert!(save(&tmp.path, &empty).is_err());
        assert_eq!(fs::read_to_string(&tmp.path).unwrap(), garbage);
    }

    #[test]
    fn unreadable_directory_fail_closed() {
        let tmp = TempStore::new();
        fs::create_dir(&tmp.path).unwrap();
        assert!(load(&tmp.path).is_err());
        assert!(insert_site(&tmp.path, "X", &https("x.example.com"), fp(DisplayUnit::Usd)).is_err());
        assert!(tmp.path.is_dir());
    }

    #[test]
    fn unique_origin_and_duplicate_returns_existing_id() {
        let tmp = TempStore::new();
        let first = insert_site(
            &tmp.path,
            "One",
            &https("dup.example.com"),
            fp(DisplayUnit::Usd),
        )
        .unwrap();
        let CreateSiteResult::Created { site } = first else {
            panic!("expected created");
        };
        let again = insert_site(
            &tmp.path,
            "Other",
            &normalize_base_url("https://dup.example.com/v1/").unwrap(),
            fp(DisplayUnit::Usd),
        )
        .unwrap();
        match again {
            CreateSiteResult::Duplicate { site_id } => assert_eq!(site_id, site.id),
            CreateSiteResult::Created { .. } => panic!("expected duplicate"),
        }
        assert_eq!(list_sites(&tmp.path).unwrap().len(), 1);
    }

    #[test]
    fn missing_display_unit_defaults_to_usd_and_name_edit_keeps_it() {
        let tmp = TempStore::new();
        fs::write(
            &tmp.path,
            json!({
                "version": 1,
                "sites": [{
                    "id": "siteidabcdefghijkAAA",
                    "name": "Panel",
                    "baseUrl": "https://cny.example.com",
                    "nextKeyOrdinal": 1,
                    "keys": []
                }]
            })
            .to_string(),
        )
        .unwrap();
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].display_unit, None);
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Usd);
        assert!(set_fingerprint_if_still_missing_at_origin(
            &tmp.path,
            "siteidabcdefghijkAAA",
            "https://cny.example.com",
            fp(DisplayUnit::Cny),
        )
        .unwrap());
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Cny);
        update_site(
            &tmp.path,
            "siteidabcdefghijkAAA",
            Some("Renamed".into()),
            None,
            None,
        )
        .unwrap();
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].name, "Renamed");
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Cny);
        update_site(
            &tmp.path,
            "siteidabcdefghijkAAA",
            None,
            Some(https("tokens.example.com")),
            Some(fp(DisplayUnit::Tokens)),
        )
        .unwrap();
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].base_url, "https://tokens.example.com");
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Tokens);
    }

    #[test]
    fn display_unit_cas_skips_changed_origin_or_existing_unit() {
        let tmp = TempStore::new();
        fs::write(
            &tmp.path,
            json!({
                "version": 1,
                "sites": [{
                    "id": "siteidabcdefghijkAAA",
                    "name": "Panel",
                    "baseUrl": "https://old.example.com",
                    "nextKeyOrdinal": 1,
                    "keys": []
                }]
            })
            .to_string(),
        )
        .unwrap();
        assert!(set_fingerprint_if_still_missing_at_origin(
            &tmp.path,
            "siteidabcdefghijkAAA",
            "https://old.example.com",
            fp(DisplayUnit::Cny),
        )
        .unwrap());
        assert_eq!(
            load(&tmp.path).unwrap().sites[0].quota_display(),
            DisplayUnit::Cny
        );
        assert!(!set_fingerprint_if_still_missing_at_origin(
            &tmp.path,
            "siteidabcdefghijkAAA",
            "https://old.example.com",
            fp(DisplayUnit::Usd),
        )
        .unwrap());
        assert_eq!(
            load(&tmp.path).unwrap().sites[0].quota_display(),
            DisplayUnit::Cny
        );
        update_site(
            &tmp.path,
            "siteidabcdefghijkAAA",
            None,
            Some(https("new.example.com")),
            Some(fp(DisplayUnit::Tokens)),
        )
        .unwrap();
        assert!(!set_fingerprint_if_still_missing_at_origin(
            &tmp.path,
            "siteidabcdefghijkAAA",
            "https://old.example.com",
            fp(DisplayUnit::Cny),
        )
        .unwrap());
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].base_url, "https://new.example.com");
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Tokens);
        assert!(!set_fingerprint_if_still_missing_at_origin(
            &tmp.path,
            "missing-site",
            "https://new.example.com",
            fp(DisplayUnit::Usd),
        )
        .unwrap());
    }

    #[test]
    fn access_token_roundtrip_clear_and_scale_defaults() {
        let tmp = TempStore::new();
        let created = insert_site(
            &tmp.path,
            "Panel",
            &https("sub.example.com"),
            fp(DisplayUnit::Usd),
        )
        .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        assert!(!site.has_access_token);
        assert_eq!(site.user_id, "");
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].quota_scale(), (500_000.0, 1.0));
        assert!(!loaded.sites[0].has_access_token());

        let updated = set_site_auth(
            &tmp.path,
            &site.id,
            Some("  at-123  "),
            Some(" 42 "),
        )
        .unwrap();
        assert!(updated.has_access_token);
        assert_eq!(updated.user_id, "42");
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].access_token, "at-123");
        assert_eq!(loaded.sites[0].user_id, "42");

        // None keeps, Some("") clears — token and id move independently.
        let kept = set_site_auth(&tmp.path, &site.id, None, Some("")).unwrap();
        assert!(kept.has_access_token);
        assert_eq!(kept.user_id, "");
        let cleared = set_site_auth(&tmp.path, &site.id, Some(""), None).unwrap();
        assert!(!cleared.has_access_token);
        assert_eq!(cleared.user_id, "");
        assert!(set_site_auth(&tmp.path, "missing-site", Some("x"), None).is_err());
    }

    #[test]
    fn legacy_file_without_scale_and_token_uses_defaults() {
        let tmp = TempStore::new();
        fs::write(
            &tmp.path,
            json!({
                "version": 1,
                "sites": [{
                    "id": "siteidabcdefghijkAAA",
                    "name": "Panel",
                    "baseUrl": "https://old.example.com",
                    "nextKeyOrdinal": 1,
                    "keys": []
                }]
            })
            .to_string(),
        )
        .unwrap();
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].quota_scale(), (500_000.0, 1.0));
        assert!(!loaded.sites[0].has_access_token());
        let dto = loaded.sites[0].to_dto();
        assert!(!dto.has_access_token);
        // A CNY site that stored a rate keeps it across the round trip.
        fs::write(
            &tmp.path,
            json!({
                "version": 1,
                "sites": [{
                    "id": "siteidabcdefghijkAAB",
                    "name": "CNY",
                    "baseUrl": "https://cny.example.com",
                    "nextKeyOrdinal": 1,
                    "displayUnit": "cny",
                    "perUnit": 600_000.0,
                    "rate": 7.25,
                    "accessToken": "at-secret",
                    "userId": "7",
                    "keys": []
                }]
            })
            .to_string(),
        )
        .unwrap();
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].quota_scale(), (600_000.0, 7.25));
        assert!(loaded.sites[0].has_access_token());
        assert_eq!(loaded.sites[0].user_id, "7");
    }

    #[test]
    fn backfill_adds_missing_scale_to_legacy_unit_only_site() {
        let tmp = TempStore::new();
        fs::write(
            &tmp.path,
            json!({
                "version": 1,
                "sites": [{
                    "id": "siteidabcdefghijkAAA",
                    "name": "CNY",
                    "baseUrl": "https://cny.example.com",
                    "nextKeyOrdinal": 1,
                    "displayUnit": "cny",
                    "keys": []
                }]
            })
            .to_string(),
        )
        .unwrap();
        // The site has a display unit but no scale: the CAS write must still
        // apply so pre-scale files receive the subscription conversion data.
        assert!(set_fingerprint_if_still_missing_at_origin(
            &tmp.path,
            "siteidabcdefghijkAAA",
            "https://cny.example.com",
            fp(DisplayUnit::Cny),
        )
        .unwrap());
        let loaded = load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Cny);
        assert_eq!(loaded.sites[0].quota_scale(), (500_000.0, 1.0));
        // Now that both are present, the same-origin write is skipped again.
        assert!(!set_fingerprint_if_still_missing_at_origin(
            &tmp.path,
            "siteidabcdefghijkAAA",
            "https://cny.example.com",
            fp(DisplayUnit::Usd),
        )
        .unwrap());
    }

    #[test]
    fn blank_name_falls_back_to_hostname() {
        let tmp = TempStore::new();
        let created = insert_site(
            &tmp.path,
            "  ",
            &https("panel.example.com"),
            fp(DisplayUnit::Usd),
        )
        .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        assert_eq!(site.name, "panel.example.com");
    }

    #[test]
    fn empty_site_crud() {
        let tmp = TempStore::new();
        let created = insert_site(
            &tmp.path,
            "Alpha",
            &https("alpha.example.com"),
            fp(DisplayUnit::Usd),
        )
        .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        let renamed = update_site(&tmp.path, &site.id, Some("Beta".into()), None, None).unwrap();
        assert_eq!(renamed.name, "Beta");
        assert_eq!(renamed.base_url, "https://alpha.example.com");
        let moved = update_site(
            &tmp.path,
            &site.id,
            None,
            Some(https("beta.example.com")),
            None,
        )
        .unwrap();
        assert_eq!(moved.id, site.id);
        assert_eq!(moved.name, "Beta");
        assert_eq!(moved.base_url, "https://beta.example.com");
        delete_site(&tmp.path, &site.id).unwrap();
        assert!(list_sites(&tmp.path).unwrap().is_empty());
    }

    #[test]
    fn name_edit_keeps_key_ids() {
        let tmp = TempStore::new();
        let site = created_site(&tmp, "Alpha", "alpha.example.com");
        let k1 = create_key(&tmp.path, &site.id, "One", "sk-1").unwrap();
        let k2 = create_key(&tmp.path, &site.id, "Two", "sk-2").unwrap();
        let other = created_site(&tmp, "Other", "other.example.com");
        let other_key = create_key(&tmp.path, &other.id, "One", "sk-other").unwrap();
        let renamed = update_site(&tmp.path, &site.id, Some("Beta".into()), None, None).unwrap();
        assert_eq!(renamed.id, site.id);
        assert_eq!(renamed.name, "Beta");
        assert_eq!(renamed.base_url, "https://alpha.example.com");
        assert_eq!(renamed.keys.len(), 2);
        assert_eq!(renamed.keys[0].id, k1.key_id);
        assert_eq!(renamed.keys[1].id, k2.key_id);
        assert_eq!(renamed.keys[0].label, "One");
        assert_eq!(renamed.keys[1].label, "Two");
        assert_eq!(stored_secret(&tmp.path, &site.id, &k1.key_id), "sk-1");
        let listed = list_sites(&tmp.path).unwrap();
        let other_listed = listed.iter().find(|s| s.id == other.id).unwrap();
        assert_eq!(other_listed.name, "Other");
        assert_eq!(other_listed.keys[0].id, other_key.key_id);
    }

    #[test]
    fn url_change_with_two_keys_keeps_ids() {
        let tmp = TempStore::new();
        let site = created_site(&tmp, "Panel", "old.example.com");
        let k1 = create_key(&tmp.path, &site.id, "One", "sk-1").unwrap();
        let k2 = create_key(&tmp.path, &site.id, "Two", "sk-2").unwrap();
        let other = created_site(&tmp, "Other", "other.example.com");
        let other_key = create_key(&tmp.path, &other.id, "One", "sk-other").unwrap();
        let moved = update_site(
            &tmp.path,
            &site.id,
            None,
            Some(https("new.example.com")),
            None,
        )
        .unwrap();
        assert_eq!(moved.id, site.id);
        assert_eq!(moved.name, "Panel");
        assert_eq!(moved.base_url, "https://new.example.com");
        assert_eq!(moved.keys.len(), 2);
        assert_eq!(moved.keys[0].id, k1.key_id);
        assert_eq!(moved.keys[1].id, k2.key_id);
        assert_eq!(stored_secret(&tmp.path, &site.id, &k1.key_id), "sk-1");
        assert_eq!(stored_secret(&tmp.path, &site.id, &k2.key_id), "sk-2");
        let listed = list_sites(&tmp.path).unwrap();
        let other_listed = listed.iter().find(|s| s.id == other.id).unwrap();
        assert_eq!(other_listed.base_url, "https://other.example.com");
        assert_eq!(other_listed.keys[0].id, other_key.key_id);
        assert_eq!(
            stored_secret(&tmp.path, &other.id, &other_key.key_id),
            "sk-other"
        );
    }

    #[test]
    fn dto_redacts_api_key_and_secret_fragments() {
        let tmp = TempStore::new();
        let secret = "sk-live-super-secret-value-9f3a";
        fs::write(
            &tmp.path,
            json!({
                "version": 1,
                "sites": [{
                    "id": "siteidabcdefghijkAAA",
                    "name": "Panel",
                    "baseUrl": "https://secret.example.com",
                    "nextKeyOrdinal": 2,
                    "keys": [{
                        "id": "keyidabcdefghijkAAA",
                        "label": "Key 1",
                        "apiKey": secret
                    }]
                }]
            })
            .to_string(),
        )
        .unwrap();
        let listed = list_sites(&tmp.path).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].keys.len(), 1);
        assert!(listed[0].keys[0].has_api_key);
        assert_eq!(configured_key_count_at(&tmp.path), 1);
        let encoded = serde_json::to_string(&listed).unwrap();
        assert!(!encoded.contains("apiKey"), "{encoded}");
        assert!(!encoded.contains(secret), "{encoded}");
        assert!(!encoded.contains("sk-live-super-secret"), "{encoded}");
    }

    #[test]
    fn delete_on_corrupt_file_does_not_overwrite() {
        let tmp = TempStore::new();
        fs::write(&tmp.path, "corrupt").unwrap();
        assert!(delete_site(&tmp.path, "anything").is_err());
        assert!(delete_key(&tmp.path, "site", "key").is_err());
        assert_eq!(fs::read_to_string(&tmp.path).unwrap(), "corrupt");
    }

    #[test]
    fn delete_site_removes_only_that_site() {
        let tmp = TempStore::new();
        let a = created_site(&tmp, "Alpha", "alpha-del.example.com");
        let b = created_site(&tmp, "Beta", "beta-del.example.com");
        let a1 = create_key(&tmp.path, &a.id, "A1", "sk-a1").unwrap();
        let a2 = create_key(&tmp.path, &a.id, "A2", "sk-a2").unwrap();
        let b1 = create_key(&tmp.path, &b.id, "B1", "sk-b1").unwrap();
        delete_site(&tmp.path, &a.id).unwrap();
        let listed = list_sites(&tmp.path).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, b.id);
        assert_eq!(listed[0].keys.len(), 1);
        assert_eq!(listed[0].keys[0].id, b1.key_id);
        assert_eq!(stored_secret(&tmp.path, &b.id, &b1.key_id), "sk-b1");
        let raw = fs::read_to_string(&tmp.path).unwrap();
        assert!(!raw.contains(&a1.key_id), "{raw}");
        assert!(!raw.contains(&a2.key_id), "{raw}");
        assert!(raw.contains(&b1.key_id), "{raw}");
    }

    fn created_site(tmp: &TempStore, name: &str, host: &str) -> SiteDto {
        match insert_site(&tmp.path, name, &https(host), fp(DisplayUnit::Usd)).unwrap() {
            CreateSiteResult::Created { site } => site,
            CreateSiteResult::Duplicate { .. } => panic!("expected created"),
        }
    }

    fn stored_secret(path: &Path, site_id: &str, key_id: &str) -> String {
        let doc = load(path).unwrap();
        doc.sites
            .iter()
            .find(|s| s.id == site_id)
            .unwrap()
            .keys
            .iter()
            .find(|k| k.id == key_id)
            .unwrap()
            .api_key
            .clone()
    }

    #[test]
    fn create_key_trims_secret_rejects_blank_and_redacts_dto() {
        let tmp = TempStore::new();
        let site = created_site(&tmp, "Panel", "keys.example.com");
        let err = create_key(&tmp.path, &site.id, "Prod", "   ").unwrap_err();
        assert!(!err.is_empty());
        let err = create_key(&tmp.path, &site.id, "Prod", "").unwrap_err();
        assert!(!err.is_empty());
        let created = create_key(&tmp.path, &site.id, "  Prod  ", "  sk-live-one  ").unwrap();
        assert!(created.first_key);
        assert_eq!(created.site.keys.len(), 1);
        assert_eq!(created.site.keys[0].id, created.key_id);
        assert_eq!(created.site.keys[0].label, "Prod");
        assert!(created.site.keys[0].has_api_key);
        let encoded = serde_json::to_string(&created.site).unwrap();
        assert!(!encoded.contains("sk-live-one"), "{encoded}");
        assert!(!encoded.contains("apiKey"), "{encoded}");
        assert_eq!(
            stored_secret(&tmp.path, &site.id, &created.key_id),
            "sk-live-one"
        );
    }

    #[test]
    fn blank_label_assigns_key_n_skips_occupied_never_reuses() {
        let tmp = TempStore::new();
        let site = created_site(&tmp, "Panel", "n.example.com");
        let named = create_key(&tmp.path, &site.id, "Key 3", "sk-a").unwrap();
        assert_eq!(named.site.keys[0].label, "Key 3");
        assert_eq!(load(&tmp.path).unwrap().sites[0].next_key_ordinal, 1);

        let k1 = create_key(&tmp.path, &site.id, "  ", "sk-b").unwrap();
        assert!(!k1.first_key);
        assert_eq!(
            k1.site
                .keys
                .iter()
                .find(|k| k.id == k1.key_id)
                .unwrap()
                .label,
            "Key 1"
        );

        let k2 = create_key(&tmp.path, &site.id, "", "sk-c").unwrap();
        assert_eq!(
            k2.site
                .keys
                .iter()
                .find(|k| k.id == k2.key_id)
                .unwrap()
                .label,
            "Key 2"
        );

        let k4 = create_key(&tmp.path, &site.id, "", "sk-d").unwrap();
        assert_eq!(
            k4.site
                .keys
                .iter()
                .find(|k| k.id == k4.key_id)
                .unwrap()
                .label,
            "Key 4"
        );
        assert_eq!(load(&tmp.path).unwrap().sites[0].next_key_ordinal, 5);

        delete_key(&tmp.path, &site.id, &k1.key_id).unwrap();
        let after_delete = load(&tmp.path).unwrap();
        assert_eq!(after_delete.sites[0].next_key_ordinal, 5);
        assert_eq!(after_delete.sites[0].keys.len(), 3);

        let k5 = create_key(&tmp.path, &site.id, "", "sk-e").unwrap();
        assert_eq!(
            k5.site
                .keys
                .iter()
                .find(|k| k.id == k5.key_id)
                .unwrap()
                .label,
            "Key 5"
        );
    }

    #[test]
    fn duplicate_secret_and_label_rejected_within_site_ordinal_unchanged() {
        let tmp = TempStore::new();
        let site = created_site(&tmp, "Panel", "dup.example.com");
        create_key(&tmp.path, &site.id, "Prod", "sk-same").unwrap();
        assert_eq!(load(&tmp.path).unwrap().sites[0].next_key_ordinal, 1);
        let err = create_key(&tmp.path, &site.id, "", "  sk-same  ").unwrap_err();
        assert!(!err.is_empty());
        assert_eq!(load(&tmp.path).unwrap().sites[0].next_key_ordinal, 1);
        assert_eq!(load(&tmp.path).unwrap().sites[0].keys.len(), 1);

        create_key(&tmp.path, &site.id, "Other", "sk-other").unwrap();
        let err = create_key(&tmp.path, &site.id, " Prod ", "sk-third").unwrap_err();
        assert!(!err.is_empty());
        assert_eq!(load(&tmp.path).unwrap().sites[0].keys.len(), 2);
    }

    #[test]
    fn cross_site_same_secret_allowed() {
        let tmp = TempStore::new();
        let a = created_site(&tmp, "A", "a.example.com");
        let b = created_site(&tmp, "B", "b.example.com");
        let ka = create_key(&tmp.path, &a.id, "One", "sk-shared").unwrap();
        let kb = create_key(&tmp.path, &b.id, "One", "sk-shared").unwrap();
        assert_eq!(stored_secret(&tmp.path, &a.id, &ka.key_id), "sk-shared");
        assert_eq!(stored_secret(&tmp.path, &b.id, &kb.key_id), "sk-shared");
        assert_ne!(ka.key_id, kb.key_id);
    }

    #[test]
    fn update_key_keeps_id_and_empty_secret_and_uniqueness() {
        let tmp = TempStore::new();
        let site = created_site(&tmp, "Panel", "up.example.com");
        let first = create_key(&tmp.path, &site.id, "Alpha", "sk-old").unwrap();
        let second = create_key(&tmp.path, &site.id, "Beta", "sk-other").unwrap();
        let key_id = first.key_id.clone();

        let renamed = update_key(
            &tmp.path,
            &site.id,
            &key_id,
            Some("Gamma".into()),
            Some("".into()),
        )
        .unwrap();
        let updated = renamed.keys.iter().find(|k| k.id == key_id).unwrap();
        assert_eq!(updated.id, key_id);
        assert_eq!(updated.label, "Gamma");
        assert_eq!(stored_secret(&tmp.path, &site.id, &key_id), "sk-old");

        let rotated = update_key(
            &tmp.path,
            &site.id,
            &key_id,
            None,
            Some("  sk-new  ".into()),
        )
        .unwrap();
        assert_eq!(
            rotated.keys.iter().find(|k| k.id == key_id).unwrap().id,
            key_id
        );
        assert_eq!(stored_secret(&tmp.path, &site.id, &key_id), "sk-new");

        let err = update_key(&tmp.path, &site.id, &key_id, Some("Beta".into()), None).unwrap_err();
        assert!(!err.is_empty());
        let err =
            update_key(&tmp.path, &site.id, &key_id, None, Some("sk-other".into())).unwrap_err();
        assert!(!err.is_empty());
        assert_eq!(
            load(&tmp.path).unwrap().sites[0]
                .keys
                .iter()
                .find(|k| k.id == second.key_id)
                .unwrap()
                .label,
            "Beta"
        );
    }

    #[test]
    fn rotation_preserves_key_id() {
        let tmp = TempStore::new();
        let site = created_site(&tmp, "Panel", "rot.example.com");
        let created = create_key(&tmp.path, &site.id, "Prod", "sk-old").unwrap();
        let key_id = created.key_id.clone();
        let rotated = update_key(
            &tmp.path,
            &site.id,
            &key_id,
            None,
            Some("sk-rotated".into()),
        )
        .unwrap();
        assert_eq!(rotated.keys.len(), 1);
        assert_eq!(rotated.keys[0].id, key_id);
        assert_eq!(rotated.keys[0].label, "Prod");
        assert_eq!(stored_secret(&tmp.path, &site.id, &key_id), "sk-rotated");
        assert_eq!(load(&tmp.path).unwrap().sites[0].keys[0].id, key_id);
    }

    #[test]
    fn later_key_is_not_first_key() {
        let tmp = TempStore::new();
        let site = created_site(&tmp, "Panel", "later.example.com");
        let first = create_key(&tmp.path, &site.id, "A", "sk-1").unwrap();
        assert!(first.first_key);
        let second = create_key(&tmp.path, &site.id, "B", "sk-2").unwrap();
        assert!(!second.first_key);
        assert_ne!(first.key_id, second.key_id);
        assert_eq!(second.site.keys.len(), 2);
    }

    #[test]
    fn two_sites_multiple_keys_stay_independent() {
        let tmp = TempStore::new();
        let a = created_site(&tmp, "A", "a.example.com");
        let b = created_site(&tmp, "B", "b.example.com");
        let a1 = create_key(&tmp.path, &a.id, "One", "sk-a1").unwrap();
        let a2 = create_key(&tmp.path, &a.id, "Two", "sk-a2").unwrap();
        let b1 = create_key(&tmp.path, &b.id, "One", "sk-a1").unwrap();
        assert!(a1.first_key);
        assert!(!a2.first_key);
        assert!(!b1.first_key);
        assert_ne!(a1.key_id, a2.key_id);
        assert_ne!(a1.key_id, b1.key_id);
        assert_eq!(list_sites(&tmp.path).unwrap().len(), 2);
        assert_eq!(
            list_sites(&tmp.path)
                .unwrap()
                .iter()
                .find(|s| s.id == a.id)
                .unwrap()
                .keys
                .len(),
            2
        );
        assert_eq!(stored_secret(&tmp.path, &a.id, &a1.key_id), "sk-a1");
        assert_eq!(stored_secret(&tmp.path, &b.id, &b1.key_id), "sk-a1");
    }

    #[test]
    fn delete_key_leaves_empty_site() {
        let tmp = TempStore::new();
        let site = created_site(&tmp, "Panel", "del.example.com");
        let k1 = create_key(&tmp.path, &site.id, "A", "sk-1").unwrap();
        let k2 = create_key(&tmp.path, &site.id, "B", "sk-2").unwrap();
        let after = delete_key(&tmp.path, &site.id, &k1.key_id).unwrap();
        assert_eq!(after.id, site.id);
        assert_eq!(after.keys.len(), 1);
        assert_eq!(after.keys[0].id, k2.key_id);

        let empty = delete_key(&tmp.path, &site.id, &k2.key_id).unwrap();
        assert_eq!(empty.id, site.id);
        assert!(empty.keys.is_empty());
        assert_eq!(list_sites(&tmp.path).unwrap().len(), 1);
        assert_eq!(list_sites(&tmp.path).unwrap()[0].keys.len(), 0);
    }
}
