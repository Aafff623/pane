mod billing;
mod fingerprint;
mod ids;
mod snapshot;
mod store;
mod url;

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SiteDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
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

pub fn list_sites() -> Result<Vec<SiteDto>, String> {
    store::list_sites(&store_path())
}

pub async fn probe_site(base_url: String) -> Result<ProbeDto, String> {
    probe_site_url(&base_url).await
}

async fn probe_site_url(base_url: &str) -> Result<ProbeDto, String> {
    let normalized = url::normalize_base_url(base_url)?;
    fingerprint::probe(&normalized.origin).await?;
    Ok(ProbeDto {
        base_url: normalized.origin,
        hostname: normalized.hostname,
        http_plaintext: normalized.http_plaintext,
    })
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
    fingerprint::probe(&normalized.origin).await?;
    store::insert_site(path, &name, &normalized)
}

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

/// Commits a site edit after the caller has fingerprinted a changed origin.
/// Keeping this separate lets the app clear derived caches after validation
/// but before the credential document changes.
pub(crate) fn update_site_after_probe(
    id: String,
    name: Option<String>,
    base_url: Option<String>,
) -> Result<SiteDto, String> {
    let normalized = base_url
        .as_deref()
        .map(url::normalize_base_url)
        .transpose()?;
    store::update_site(&store_path(), &id, name, normalized)
}

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
    let new_url = match base_url {
        Some(raw) => {
            let normalized = url::normalize_base_url(&raw)?;
            if normalized.origin != site.base_url {
                fingerprint::probe(&normalized.origin).await?;
                Some(normalized)
            } else {
                None
            }
        }
        None => None,
    };
    store::update_site(path, id, name, new_url)
}

pub fn delete_site(id: String) -> Result<(), String> {
    store::delete_site(&store_path(), &id)
}

#[derive(Debug, Serialize)]
pub struct CreatedKey {
    pub site: SiteDto,
    pub key_id: String,
    pub first_key: bool,
}

pub fn create_key(site_id: String, label: String, api_key: String) -> Result<CreatedKey, String> {
    store::create_key(&store_path(), &site_id, &label, &api_key)
}

pub fn update_key(
    site_id: String,
    key_id: String,
    label: Option<String>,
    api_key: Option<String>,
) -> Result<SiteDto, String> {
    store::update_key(&store_path(), &site_id, &key_id, label, api_key)
}

pub fn delete_key(site_id: String, key_id: String) -> Result<SiteDto, String> {
    store::delete_key(&store_path(), &site_id, &key_id)
}

#[allow(dead_code)]
pub fn configured_key_count() -> usize {
    store::configured_key_count_at(&store_path())
}

pub use snapshot::{refresh_clients, snapshot_key_with_client, KeyCard};

pub fn key_cards() -> Result<Vec<KeyCard>, String> {
    snapshot::key_cards_at(&store_path())
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
    }

    #[test]
    fn probe_site_returns_plaintext_flag_without_saving() {
        let tmp = TempStore::new();
        let (origin, join) = spawn_ok_server(1);
        let dto = tauri::async_runtime::block_on(probe_site_url(&origin)).unwrap();
        assert_eq!(dto.base_url, origin);
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
    fn create_result_serialization_is_internally_tagged() {
        let created = serde_json::to_value(CreateSiteResult::Created {
            site: SiteDto {
                id: "abc".into(),
                name: "N".into(),
                base_url: "https://n.example".into(),
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
