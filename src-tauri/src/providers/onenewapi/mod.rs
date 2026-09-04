mod billing;
mod fingerprint;
mod ids;
mod snapshot;
mod store;
mod url;

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SiteDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    /// Whether a NewAPI dashboard access token is saved (value never leaves
    /// the store).
    pub has_access_token: bool,
    /// The dashboard user id paired with the access token (`New-Api-User`
    /// header). Not a credential — exposed so the UI can pre-fill it.
    pub user_id: String,
    pub keys: Vec<KeyDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KeyDto {
    pub id: String,
    pub label: String,
    pub has_api_key: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProbeDto {
    pub base_url: String,
    pub hostname: String,
    pub http_plaintext: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status")]
pub enum CreateSiteResult {
    #[serde(rename = "created")]
    Created { site: SiteDto },
    #[serde(rename = "duplicate")]
    Duplicate { site_id: String },
}

fn store_path() -> PathBuf {
    super::config_dir().join("onenewapi.json")
}

fn store_mutation_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_store_mutation() -> Result<MutexGuard<'static, ()>, String> {
    store_mutation_lock()
        .lock()
        .map_err(|_| "onenewapi store mutation lock is poisoned".to_string())
}

/// Serializes a primary store write with its derived-state cleanup. Cleanup
/// runs only after the atomic store write succeeds; if cleanup fails, the
/// complete credential document is restored before the error is returned.
/// The cleanup closure must not re-enter this module's store interface.
fn coordinate_store_mutation_at<T, Mutate, Cleanup>(
    path: &Path,
    mutate: Mutate,
    cleanup: Cleanup,
) -> Result<T, String>
where
    Mutate: FnOnce(&Path) -> Result<T, String>,
    Cleanup: FnOnce(&T) -> Result<(), String>,
{
    let _lock = lock_store_mutation()?;
    let checkpoint = store::load(path)?;
    let result = mutate(path)?;
    if let Err(cleanup_error) = cleanup(&result) {
        return match store::save(path, &checkpoint) {
            Ok(()) => Err(cleanup_error),
            Err(rollback_error) => Err(format!(
                "{cleanup_error}; rollback onenewapi.json failed: {rollback_error}"
            )),
        };
    }
    Ok(result)
}

pub fn list_sites() -> Result<Vec<SiteDto>, String> {
    let _lock = lock_store_mutation()?;
    store::list_sites(&store_path())
}

pub async fn probe_site(base_url: String) -> Result<ProbeDto, String> {
    Ok(probe_site_url(&base_url).await?.0)
}

pub(crate) async fn probe_site_display(
    base_url: String,
) -> Result<(ProbeDto, fingerprint::SiteFingerprint), String> {
    probe_site_url(&base_url).await
}

async fn probe_site_url(base_url: &str) -> Result<(ProbeDto, fingerprint::SiteFingerprint), String> {
    let normalized = url::normalize_base_url(base_url)?;
    let display = fingerprint::probe(&normalized.origin).await?;
    Ok((
        ProbeDto {
            base_url: normalized.origin,
            hostname: normalized.hostname,
            http_plaintext: normalized.http_plaintext,
        },
        display,
    ))
}

pub async fn create_site(name: String, base_url: String) -> Result<CreateSiteResult, String> {
    create_site_at(&store_path(), name, base_url).await
}

pub async fn create_site_at(
    path: &Path,
    name: String,
    base_url: String,
) -> Result<CreateSiteResult, String> {
    let normalized = url::normalize_base_url(&base_url)?;
    let existing = store::load(path)?;
    if let Some(site) = existing
        .sites
        .iter()
        .find(|s| s.base_url == normalized.origin)
    {
        return Ok(CreateSiteResult::Duplicate {
            site_id: site.id.clone(),
        });
    }
    let display = fingerprint::probe(&normalized.origin).await?;
    let _lock = lock_store_mutation()?;
    store::insert_site(path, &name, &normalized, display)
}

#[allow(dead_code)]
pub async fn update_site(
    id: String,
    name: Option<String>,
    base_url: Option<String>,
) -> Result<SiteDto, String> {
    update_site_at(&store_path(), &id, name, base_url).await
}

pub(crate) fn normalize_site_url(base_url: &str) -> Result<String, String> {
    Ok(url::normalize_base_url(base_url)?.origin)
}

/// Commits a site edit after the caller has fingerprinted a changed origin,
/// then applies its derived-state cleanup before exposing success.
pub(crate) fn update_site_consistently<Cleanup>(
    id: String,
    name: Option<String>,
    base_url: Option<String>,
    fingerprint: Option<fingerprint::SiteFingerprint>,
    cleanup: Cleanup,
) -> Result<SiteDto, String>
where
    Cleanup: FnOnce(&SiteDto) -> Result<(), String>,
{
    let normalized = base_url
        .as_deref()
        .map(url::normalize_base_url)
        .transpose()?;
    coordinate_store_mutation_at(
        &store_path(),
        |path| store::update_site(path, &id, name, normalized, fingerprint),
        cleanup,
    )
}

#[allow(dead_code)]
pub async fn update_site_at(
    path: &Path,
    id: &str,
    name: Option<String>,
    base_url: Option<String>,
) -> Result<SiteDto, String> {
    let doc = store::load(path)?;
    let site = doc
        .sites
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "site not found".to_string())?;
    let (new_url, fp) = match base_url {
        Some(raw) => {
            let normalized = url::normalize_base_url(&raw)?;
            if normalized.origin != site.base_url {
                let fp = fingerprint::probe(&normalized.origin).await?;
                (Some(normalized), Some(fp))
            } else {
                (None, None)
            }
        }
        None => (None, None),
    };
    let _lock = lock_store_mutation()?;
    store::update_site(path, id, name, new_url, fp)
}

#[allow(dead_code)]
pub fn delete_site(id: String) -> Result<(), String> {
    let _lock = lock_store_mutation()?;
    store::delete_site(&store_path(), &id)
}

pub(crate) fn delete_site_consistently<Cleanup>(id: String, cleanup: Cleanup) -> Result<(), String>
where
    Cleanup: FnOnce() -> Result<(), String>,
{
    coordinate_store_mutation_at(
        &store_path(),
        |path| store::delete_site(path, &id),
        |_| cleanup(),
    )
}

#[derive(Debug, Serialize)]
pub struct CreatedKey {
    pub site: SiteDto,
    pub key_id: String,
    pub first_key: bool,
}

pub fn create_key(site_id: String, label: String, api_key: String) -> Result<CreatedKey, String> {
    let _lock = lock_store_mutation()?;
    store::create_key(&store_path(), &site_id, &label, &api_key)
}

#[allow(dead_code)]
pub fn update_key(
    site_id: String,
    key_id: String,
    label: Option<String>,
    api_key: Option<String>,
) -> Result<SiteDto, String> {
    let _lock = lock_store_mutation()?;
    store::update_key(&store_path(), &site_id, &key_id, label, api_key)
}

pub(crate) fn update_key_consistently<Cleanup>(
    site_id: String,
    key_id: String,
    label: Option<String>,
    api_key: Option<String>,
    cleanup: Cleanup,
) -> Result<SiteDto, String>
where
    Cleanup: FnOnce(&SiteDto) -> Result<(), String>,
{
    coordinate_store_mutation_at(
        &store_path(),
        |path| store::update_key(path, &site_id, &key_id, label, api_key),
        cleanup,
    )
}

#[allow(dead_code)]
pub fn delete_key(site_id: String, key_id: String) -> Result<SiteDto, String> {
    let _lock = lock_store_mutation()?;
    store::delete_key(&store_path(), &site_id, &key_id)
}

pub(crate) fn delete_key_consistently<Cleanup>(
    site_id: String,
    key_id: String,
    cleanup: Cleanup,
) -> Result<SiteDto, String>
where
    Cleanup: FnOnce() -> Result<(), String>,
{
    coordinate_store_mutation_at(
        &store_path(),
        |path| store::delete_key(path, &site_id, &key_id),
        |_| cleanup(),
    )
}

#[allow(dead_code)]
pub fn configured_key_count() -> usize {
    let Ok(_lock) = lock_store_mutation() else {
        return 0;
    };
    store::configured_key_count_at(&store_path())
}

pub use snapshot::{refresh_clients, snapshot_key_with_client, KeyCard};

pub fn key_cards() -> Result<Vec<KeyCard>, String> {
    let _lock = lock_store_mutation()?;
    snapshot::key_cards_at(&store_path())
}

pub async fn prepare_key_cards() -> Result<Vec<KeyCard>, String> {
    snapshot::schedule_backfill_missing_display_units(store_path());
    key_cards()
}

fn set_fingerprint_at(
    path: &Path,
    id: &str,
    origin: &str,
    fingerprint: fingerprint::SiteFingerprint,
) -> Result<bool, String> {
    let _lock = lock_store_mutation()?;
    store::set_fingerprint_if_still_missing_at_origin(path, id, origin, fingerprint)
}

/// Saves (or clears) the site's NewAPI dashboard access token / user id
/// pair that enables the subscription display. `None` keeps a value.
pub fn set_site_access_token(
    site_id: String,
    access_token: Option<String>,
    user_id: Option<String>,
) -> Result<SiteDto, String> {
    set_site_access_token_at(&store_path(), &site_id, access_token, user_id)
}

fn set_site_access_token_at(
    path: &Path,
    site_id: &str,
    access_token: Option<String>,
    user_id: Option<String>,
) -> Result<SiteDto, String> {
    let _lock = lock_store_mutation()?;
    store::set_site_auth(path, site_id, access_token.as_deref(), user_id.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
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
            let dir = std::env::temp_dir()
                .join(format!("pane-onenewapi-mod-{}-{stamp}", std::process::id()));
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

    fn ok_status_body() -> String {
        json!({
            "success": true,
            "data": {
                "version": "v0.0-test",
                "system_name": "Custom Brand",
                "quota_display_type": "USD"
            }
        })
        .to_string()
    }

    fn spawn_ok_server(n: usize) -> (String, std::thread::JoinHandle<Vec<Option<String>>>) {
        spawn_status_server(n, 200, ok_status_body())
    }

    fn spawn_fail_server(n: usize) -> (String, std::thread::JoinHandle<Vec<Option<String>>>) {
        spawn_status_server(n, 500, "nope".into())
    }

    fn spawn_status_server(
        n: usize,
        status: u16,
        body: String,
    ) -> (String, std::thread::JoinHandle<Vec<Option<String>>>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_ip().unwrap();
        let origin = format!("http://127.0.0.1:{}", addr.port());
        let join = std::thread::spawn(move || {
            let mut auths = Vec::new();
            for _ in 0..n {
                match server.recv_timeout(std::time::Duration::from_secs(3)) {
                    Ok(Some(req)) => {
                        let auth = req
                            .headers()
                            .iter()
                            .find(|h| h.field.equiv("Authorization"))
                            .map(|h| h.value.as_str().to_string());
                        auths.push(auth);
                        let _ = req.respond(
                            tiny_http::Response::from_string(body.clone()).with_status_code(status),
                        );
                    }
                    _ => break,
                }
            }
            auths
        });
        (origin, join)
    }

    #[test]
    fn create_site_probes_then_saves_and_duplicate_returns_id() {
        let tmp = TempStore::new();
        let (origin, join) = spawn_ok_server(1);
        let created = tauri::async_runtime::block_on(create_site_at(
            &tmp.path,
            "  ".into(),
            format!("{origin}/v1"),
        ))
        .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        assert_eq!(site.base_url, origin);
        assert_eq!(site.name, "127.0.0.1");
        assert!(site.keys.is_empty());
        let dup = tauri::async_runtime::block_on(create_site_at(
            &tmp.path,
            "Ignored".into(),
            format!("{origin}/"),
        ))
        .unwrap();
        match dup {
            CreateSiteResult::Duplicate { site_id } => assert_eq!(site_id, site.id),
            CreateSiteResult::Created { .. } => panic!("expected duplicate"),
        }
        let auths = join.join().unwrap();
        assert_eq!(auths, vec![None]);
        assert_eq!(store::list_sites(&tmp.path).unwrap().len(), 1);
        assert_eq!(
            store::load(&tmp.path).unwrap().sites[0].quota_display(),
            fingerprint::DisplayUnit::Usd
        );
    }

    #[test]
    fn create_site_persists_cny_display_unit() {
        let tmp = TempStore::new();
        let body = json!({
            "success": true,
            "data": {"version": "1", "quota_display_type": "CNY"}
        })
        .to_string();
        let (origin, join) = spawn_status_server(1, 200, body);
        let created =
            tauri::async_runtime::block_on(create_site_at(&tmp.path, "Panel".into(), origin))
                .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        let _ = join.join();
        assert_eq!(
            store::load(&tmp.path).unwrap().sites[0].quota_display(),
            fingerprint::DisplayUnit::Cny
        );
        assert_eq!(site.name, "Panel");
    }

    #[test]
    fn probe_site_returns_plaintext_flag_without_saving() {
        let tmp = TempStore::new();
        let (origin, join) = spawn_ok_server(1);
        let (dto, fp) = tauri::async_runtime::block_on(probe_site_url(&origin)).unwrap();
        assert_eq!(dto.base_url, origin);
        assert_eq!(
            fp,
            fingerprint::SiteFingerprint::from(fingerprint::DisplayUnit::Usd)
        );
        assert_eq!(dto.hostname, "127.0.0.1");
        assert!(dto.http_plaintext);
        assert!(!tmp.path.exists());
        let _ = join.join();
    }

    #[test]
    fn url_change_fingerprints_then_saves() {
        let tmp = TempStore::new();
        let (origin_a, join_a) = spawn_ok_server(1);
        let created =
            tauri::async_runtime::block_on(create_site_at(&tmp.path, "A".into(), origin_a.clone()))
                .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        let _ = join_a.join();
        let (origin_b, join_b) = spawn_ok_server(1);
        let updated = tauri::async_runtime::block_on(update_site_at(
            &tmp.path,
            &site.id,
            None,
            Some(origin_b.clone()),
        ))
        .unwrap();
        assert_eq!(updated.id, site.id);
        assert_eq!(updated.base_url, origin_b);
        assert_eq!(updated.name, "A");
        let auths = join_b.join().unwrap();
        assert_eq!(auths, vec![None]);
    }

    #[test]
    fn url_change_with_two_keys_keeps_ids() {
        let tmp = TempStore::new();
        let (origin_a, join_a) = spawn_ok_server(1);
        let created =
            tauri::async_runtime::block_on(create_site_at(&tmp.path, "A".into(), origin_a.clone()))
                .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        let _ = join_a.join();
        let k1 = store::create_key(&tmp.path, &site.id, "One", "sk-1").unwrap();
        let k2 = store::create_key(&tmp.path, &site.id, "Two", "sk-2").unwrap();
        let other = store::insert_site(
            &tmp.path,
            "Other",
            &url::normalize_base_url("https://other.example.com").unwrap(),
            fingerprint::SiteFingerprint::from(fingerprint::DisplayUnit::Usd),
        )
        .unwrap();
        let CreateSiteResult::Created { site: other_site } = other else {
            panic!("expected created");
        };
        let other_key = store::create_key(&tmp.path, &other_site.id, "One", "sk-other").unwrap();
        let (origin_b, join_b) = spawn_ok_server(1);
        let updated = tauri::async_runtime::block_on(update_site_at(
            &tmp.path,
            &site.id,
            None,
            Some(origin_b.clone()),
        ))
        .unwrap();
        assert_eq!(updated.id, site.id);
        assert_eq!(updated.base_url, origin_b);
        assert_eq!(updated.keys.len(), 2);
        assert_eq!(updated.keys[0].id, k1.key_id);
        assert_eq!(updated.keys[1].id, k2.key_id);
        let listed = store::list_sites(&tmp.path).unwrap();
        let other_listed = listed.iter().find(|s| s.id == other_site.id).unwrap();
        assert_eq!(other_listed.base_url, "https://other.example.com");
        assert_eq!(other_listed.keys[0].id, other_key.key_id);
        let auths = join_b.join().unwrap();
        assert_eq!(auths, vec![None]);
    }

    #[test]
    fn url_change_probe_failure_changes_nothing() {
        let tmp = TempStore::new();
        let (origin, join) = spawn_ok_server(1);
        let created =
            tauri::async_runtime::block_on(create_site_at(&tmp.path, "A".into(), origin.clone()))
                .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        let _ = join.join();
        let k1 = store::create_key(&tmp.path, &site.id, "One", "sk-1").unwrap();
        let (bad, join_bad) = spawn_fail_server(1);
        let err = tauri::async_runtime::block_on(update_site_at(
            &tmp.path,
            &site.id,
            Some("Renamed".into()),
            Some(bad),
        ))
        .unwrap_err();
        assert!(!err.is_empty());
        let _ = join_bad.join();
        let listed = store::list_sites(&tmp.path).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, site.id);
        assert_eq!(listed[0].name, "A");
        assert_eq!(listed[0].base_url, origin);
        assert_eq!(listed[0].keys.len(), 1);
        assert_eq!(listed[0].keys[0].id, k1.key_id);
    }

    #[test]
    fn name_only_update_skips_probe() {
        let tmp = TempStore::new();
        let (origin, join) = spawn_ok_server(1);
        let created =
            tauri::async_runtime::block_on(create_site_at(&tmp.path, "Old".into(), origin.clone()))
                .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        let updated = tauri::async_runtime::block_on(update_site_at(
            &tmp.path,
            &site.id,
            Some("New".into()),
            None,
        ))
        .unwrap();
        assert_eq!(updated.name, "New");
        assert_eq!(updated.base_url, origin);
        let auths = join.join().unwrap();
        assert_eq!(auths.len(), 1);
    }

    #[test]
    fn set_site_access_token_saves_trims_and_clears() {
        let tmp = TempStore::new();
        let (origin, join) = spawn_ok_server(1);
        let created =
            tauri::async_runtime::block_on(create_site_at(&tmp.path, "Panel".into(), origin))
                .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created");
        };
        let _ = join.join();
        assert!(!site.has_access_token);
        assert_eq!(site.user_id, "");

        let saved =
            super::set_site_access_token_at(&tmp.path, &site.id, Some("  at-7 ".into()), None)
                .unwrap();
        assert!(saved.has_access_token);
        let listed = store::load(&tmp.path).unwrap();
        assert_eq!(listed.sites[0].access_token, "at-7");

        let paired = super::set_site_access_token_at(
            &tmp.path,
            &site.id,
            None,
            Some(" 42 ".into()),
        )
        .unwrap();
        assert!(paired.has_access_token);
        assert_eq!(paired.user_id, "42");

        let cleared = super::set_site_access_token_at(&tmp.path, &site.id, Some("".into()), None)
            .unwrap();
        assert!(!cleared.has_access_token);
        assert_eq!(cleared.user_id, "42");
        assert!(
            super::set_site_access_token_at(&tmp.path, "missing", Some("x".into()), None)
                .is_err()
        );
    }

    fn seed_site_with_key(tmp: &TempStore) -> (SiteDto, CreatedKey) {
        let normalized = url::normalize_base_url("https://panel.example.com").unwrap();
        let created = store::insert_site(
            &tmp.path,
            "Panel",
            &normalized,
            fingerprint::SiteFingerprint::from(fingerprint::DisplayUnit::Usd),
        )
        .unwrap();
        let CreateSiteResult::Created { site } = created else {
            panic!("expected created site");
        };
        let key = store::create_key(&tmp.path, &site.id, "Primary", "sk-primary").unwrap();
        (site, key)
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct DerivedStateFixture {
        cache: Vec<&'static str>,
        failure_state: Vec<&'static str>,
        alerts: Vec<&'static str>,
        disabled: Vec<&'static str>,
        layout: Vec<&'static str>,
        pinned: Option<&'static str>,
        tray: Vec<&'static str>,
    }

    impl DerivedStateFixture {
        fn seeded() -> Self {
            Self {
                cache: vec!["onenewapi@primary"],
                failure_state: vec!["onenewapi@primary"],
                alerts: vec!["onenewapi@primary:Usage"],
                disabled: vec!["onenewapi@primary"],
                layout: vec!["onenewapi@primary"],
                pinned: Some("onenewapi@primary"),
                tray: vec!["onenewapi@primary"],
            }
        }

        fn clear(&mut self) {
            self.cache.clear();
            self.failure_state.clear();
            self.alerts.clear();
            self.disabled.clear();
            self.layout.clear();
            self.pinned = None;
            self.tray.clear();
        }
    }

    fn assert_store_failure_kept_derived_state<T>(
        result: Result<T, String>,
        derived: &std::cell::RefCell<DerivedStateFixture>,
        before: &DerivedStateFixture,
    ) {
        assert!(result.is_err());
        assert_eq!(
            &*derived.borrow(),
            before,
            "failed store mutation must not run derived cleanup"
        );
    }

    #[test]
    fn update_site_store_failure_skips_cleanup() {
        let tmp = TempStore::new();
        let _ = seed_site_with_key(&tmp);
        let derived = std::cell::RefCell::new(DerivedStateFixture::seeded());
        let before = derived.borrow().clone();
        let result = coordinate_store_mutation_at(
            &tmp.path,
            |path| store::update_site(path, "missing-site", Some("New".into()), None, None),
            |_| {
                derived.borrow_mut().clear();
                Ok(())
            },
        );
        assert_store_failure_kept_derived_state(result, &derived, &before);
    }

    #[test]
    fn update_key_store_failure_skips_cleanup() {
        let tmp = TempStore::new();
        let (site, _) = seed_site_with_key(&tmp);
        let derived = std::cell::RefCell::new(DerivedStateFixture::seeded());
        let before = derived.borrow().clone();
        let result = coordinate_store_mutation_at(
            &tmp.path,
            |path| {
                store::update_key(
                    path,
                    &site.id,
                    "missing-key",
                    Some("New".into()),
                    Some("sk-new".into()),
                )
            },
            |_| {
                derived.borrow_mut().clear();
                Ok(())
            },
        );
        assert_store_failure_kept_derived_state(result, &derived, &before);
    }

    #[test]
    fn delete_site_store_failure_skips_cleanup() {
        let tmp = TempStore::new();
        let _ = seed_site_with_key(&tmp);
        let derived = std::cell::RefCell::new(DerivedStateFixture::seeded());
        let before = derived.borrow().clone();
        let result = coordinate_store_mutation_at(
            &tmp.path,
            |path| store::delete_site(path, "missing-site"),
            |_| {
                derived.borrow_mut().clear();
                Ok(())
            },
        );
        assert_store_failure_kept_derived_state(result, &derived, &before);
    }

    #[test]
    fn delete_key_store_failure_skips_cleanup() {
        let tmp = TempStore::new();
        let (site, _) = seed_site_with_key(&tmp);
        let derived = std::cell::RefCell::new(DerivedStateFixture::seeded());
        let before = derived.borrow().clone();
        let result = coordinate_store_mutation_at(
            &tmp.path,
            |path| store::delete_key(path, &site.id, "missing-key"),
            |_| {
                derived.borrow_mut().clear();
                Ok(())
            },
        );
        assert_store_failure_kept_derived_state(result, &derived, &before);
    }

    #[test]
    fn successful_store_mutation_cleans_before_returning() {
        let tmp = TempStore::new();
        let (site, key) = seed_site_with_key(&tmp);
        let cleaned = std::cell::Cell::new(false);
        let updated = coordinate_store_mutation_at(
            &tmp.path,
            |path| store::update_key(path, &site.id, &key.key_id, Some("Renamed".into()), None),
            |saved| {
                assert_eq!(
                    saved
                        .keys
                        .iter()
                        .find(|item| item.id == key.key_id)
                        .unwrap()
                        .label,
                    "Renamed"
                );
                cleaned.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert!(cleaned.get());
        assert_eq!(
            updated
                .keys
                .iter()
                .find(|item| item.id == key.key_id)
                .unwrap()
                .label,
            "Renamed"
        );
    }

    #[test]
    fn cleanup_failure_restores_primary_store() {
        let tmp = TempStore::new();
        let (site, _) = seed_site_with_key(&tmp);
        let before = fs::read_to_string(&tmp.path).unwrap();
        let result = coordinate_store_mutation_at(
            &tmp.path,
            |path| store::delete_site(path, &site.id),
            |_| Err("config locked".into()),
        );
        assert_eq!(result.unwrap_err(), "config locked");
        assert_eq!(fs::read_to_string(&tmp.path).unwrap(), before);
    }

    #[test]
    fn cleanup_and_rollback_failures_are_both_reported() {
        let tmp = TempStore::new();
        let (site, _) = seed_site_with_key(&tmp);
        let result = coordinate_store_mutation_at(
            &tmp.path,
            |path| store::delete_site(path, &site.id),
            |_| {
                fs::remove_file(&tmp.path).unwrap();
                fs::create_dir(&tmp.path).unwrap();
                Err("config locked".into())
            },
        );
        let error = result.unwrap_err();
        assert!(error.contains("config locked"), "{error}");
        assert!(error.contains("rollback onenewapi.json failed"), "{error}");
    }

    #[test]
    fn create_result_serialization_is_internally_tagged() {
        let created = serde_json::to_value(CreateSiteResult::Created {
            site: SiteDto {
                id: "abc".into(),
                name: "N".into(),
                base_url: "https://n.example".into(),
                has_access_token: false,
                user_id: String::new(),
                keys: vec![],
            },
        })
        .unwrap();
        assert_eq!(created["status"], "created");
        assert_eq!(created["site"]["base_url"], "https://n.example");
        let dup = serde_json::to_value(CreateSiteResult::Duplicate {
            site_id: "abc".into(),
        })
        .unwrap();
        assert_eq!(dup, json!({"status": "duplicate", "site_id": "abc"}));
    }
}
