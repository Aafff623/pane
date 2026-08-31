use super::super::{http_no_redirect, json_body, Snapshot};
use super::billing;
use super::fingerprint::DisplayUnit;
use super::store;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

const MAX_BILLING_BYTES: usize = 64 * 1024;
const MAX_IN_FLIGHT: usize = 8;

fn billing_sema() -> &'static tokio::sync::Semaphore {
    static SEMA: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    SEMA.get_or_init(|| tokio::sync::Semaphore::new(MAX_IN_FLIGHT))
}

/// One shared client per origin for a refresh. Keys of the same site reuse it.
pub fn refresh_clients(cards: &[KeyCard]) -> HashMap<String, reqwest::Client> {
    let mut map = HashMap::new();
    for card in cards {
        map.entry(card.origin.clone())
            .or_insert_with(http_no_redirect);
    }
    map
}

async fn billing_get(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    what: &str,
) -> Result<(reqwest::StatusCode, Result<serde_json::Value, String>), reqwest::Error> {
    let _permit = billing_sema().acquire().await.expect("billing semaphore");
    let resp = client.get(url).bearer_auth(api_key).send().await?;
    let status = resp.status();
    let json = json_body(resp, MAX_BILLING_BYTES, what).await;
    Ok((status, json))
}

pub struct KeyCard {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub api_key: String,
    pub display: DisplayUnit,
}

pub fn key_cards_at(path: &Path) -> Result<Vec<KeyCard>, String> {
    let doc = store::load(path)?;
    Ok(doc
        .sites
        .iter()
        .flat_map(|site| {
            site.keys
                .iter()
                .filter(|k| !k.api_key.is_empty())
                .map(|key| KeyCard {
                    id: format!("onenewapi@{}", key.id),
                    name: format!("{} · {}", site.name, key.label),
                    origin: site.base_url.clone(),
                    api_key: key.api_key.clone(),
                    display: site.quota_display(),
                })
        })
        .collect())
}

#[cfg_attr(not(test), allow(dead_code))]
pub async fn snapshot_key(card: KeyCard) -> Snapshot {
    snapshot_key_with_client(http_no_redirect(), card).await
}

pub async fn snapshot_key_with_client(client: reqwest::Client, card: KeyCard) -> Snapshot {
    match fetch_key(&client, &card).await {
        Ok(metrics) => with_dashboard(Snapshot::ok(&card.id, &card.name, None, metrics), &card),
        Err(e) => with_dashboard(Snapshot::error(&card.id, &card.name, e), &card),
    }
}

fn with_dashboard(mut snap: Snapshot, card: &KeyCard) -> Snapshot {
    snap.dashboard_url = Some(card.origin.clone());
    snap
}

async fn fetch_key(
    client: &reqwest::Client,
    card: &KeyCard,
) -> Result<Vec<super::super::Metric>, String> {
    let origin = super::url::normalize_base_url(&card.origin)?.origin;
    let sub_url = format!("{origin}/v1/dashboard/billing/subscription");
    let usage_url = format!("{origin}/v1/dashboard/billing/usage");
    let (sub_resp, usage_resp) = tokio::join!(
        billing_get(client, &sub_url, &card.api_key, "subscription"),
        billing_get(client, &usage_url, &card.api_key, "usage"),
    );

    let (status, sub_json) = sub_resp.map_err(|_| "subscription transport".to_string())?;
    if status.as_u16() == 401 {
        return Err("key may be invalid, expired, disabled, or out of quota".into());
    }
    if !status.is_success() {
        return Err(format!("subscription HTTP {status}"));
    }
    let sub = sub_json.map_err(|e| billing_error_category("subscription", &e))?;

    let usage = match usage_resp {
        Ok((status, json)) if status.is_success() => match json {
            Ok(body) => Some(body),
            Err(e) => return Err(billing_error_category("usage", &e)),
        },
        Ok((status, _)) if status.as_u16() == 404 => None,
        Ok((status, _)) => return Err(format!("usage HTTP {status}")),
        Err(_) => return Err("usage transport".to_string()),
    };

    billing::metrics_from(&sub, usage.as_ref(), &card.display)
}

pub async fn backfill_missing_display_units(path: &Path) {
    let Ok(doc) = store::load(path) else {
        return;
    };
    let missing: Vec<(String, String)> = doc
        .sites
        .iter()
        .filter(|site| site.display_unit.is_none())
        .map(|site| (site.id.clone(), site.base_url.clone()))
        .collect();
    for (id, origin) in missing {
        if let Ok(unit) = super::fingerprint::probe(&origin).await {
            let _ = super::set_display_unit_at(path, &id, &origin, unit);
        }
    }
}

fn billing_error_category(what: &str, err: &str) -> String {
    if err.contains("too large") {
        format!("{what} too large")
    } else {
        format!("{what} parse")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        backfill_missing_display_units, key_cards_at, refresh_clients, snapshot_key, DisplayUnit,
        KeyCard,
    };
    use crate::providers::onenewapi::store;
    use crate::providers::onenewapi::url::normalize_base_url;
    use crate::providers::onenewapi::CreateSiteResult;
    use serde_json::json;
    use std::fs;
    use std::io::{ErrorKind, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
            let dir = std::env::temp_dir().join(format!(
                "pane-onenewapi-snap-{}-{stamp}",
                std::process::id()
            ));
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

    struct Captured {
        url: String,
        authorization: Option<String>,
    }

    fn authorization(req: &tiny_http::Request) -> Option<String> {
        req.headers()
            .iter()
            .find(|h| h.field.equiv("Authorization"))
            .map(|h| h.value.as_str().to_string())
    }

    fn spawn_billing_server(
        n: usize,
        respond: impl Fn(&str, &tiny_http::Request) -> tiny_http::Response<std::io::Cursor<Vec<u8>>>
            + Send
            + 'static,
    ) -> (String, std::thread::JoinHandle<Vec<Captured>>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_ip().unwrap();
        let origin = format!("http://127.0.0.1:{}", addr.port());
        let origin_for_handler = origin.clone();
        let join = std::thread::spawn(move || {
            let mut out = Vec::new();
            for _ in 0..n {
                match server.recv_timeout(Duration::from_secs(3)) {
                    Ok(Some(req)) => {
                        out.push(Captured {
                            url: req.url().to_string(),
                            authorization: authorization(&req),
                        });
                        let resp = respond(&origin_for_handler, &req);
                        let _ = req.respond(resp);
                    }
                    Ok(None) => break,
                    Err(e) if e.kind() == ErrorKind::TimedOut => break,
                    Err(_) => break,
                }
            }
            out
        });
        (origin, join)
    }

    fn path_of(req: &tiny_http::Request) -> &str {
        req.url().split('?').next().unwrap_or(req.url())
    }

    fn sub_body() -> String {
        json!({
            "hard_limit_usd": 1217.81744,
            "system_hard_limit_usd": 1217.81744,
            "access_until": 1790479748
        })
        .to_string()
    }

    fn usage_body() -> String {
        json!({"total_usage": 59218.42860000001}).to_string()
    }

    fn card(origin: &str, key: &str) -> KeyCard {
        card_with_display(origin, key, DisplayUnit::Usd)
    }

    fn card_with_display(origin: &str, key: &str, display: DisplayUnit) -> KeyCard {
        KeyCard {
            id: "onenewapi@keyidabcdefghijkAAA".into(),
            name: "Panel · Key 1".into(),
            origin: origin.into(),
            api_key: key.into(),
            display,
        }
    }

    fn ok_billing(
        origin: &str,
        req: &tiny_http::Request,
        sub: &str,
        usage: &str,
    ) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
        match path_of(req) {
            "/v1/dashboard/billing/subscription" => {
                tiny_http::Response::from_string(sub.to_string()).with_status_code(200)
            }
            "/v1/dashboard/billing/usage" => {
                tiny_http::Response::from_string(usage.to_string()).with_status_code(200)
            }
            _ => tiny_http::Response::from_string(format!("unexpected {origin}"))
                .with_status_code(404),
        }
    }

    #[test]
    fn key_cards_names_every_key_and_corrupt_fails_closed() {
        let tmp = TempStore::new();
        let url = normalize_base_url("https://panel.example.com").unwrap();
        let CreateSiteResult::Created { site } =
            store::insert_site(&tmp.path, "Panel", &url, DisplayUnit::Usd).unwrap()
        else {
            panic!("expected created");
        };
        assert!(key_cards_at(&tmp.path).unwrap().is_empty());
        let created = store::create_key(&tmp.path, &site.id, "", "sk-one").unwrap();
        let cards = key_cards_at(&tmp.path).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, format!("onenewapi@{}", created.key_id));
        assert_eq!(cards[0].name, "Panel · Key 1");
        assert_eq!(cards[0].origin, "https://panel.example.com");
        assert_eq!(cards[0].api_key, "sk-one");
        assert_eq!(cards[0].display, DisplayUnit::Usd);
        let second = store::create_key(&tmp.path, &site.id, "Prod", "sk-two").unwrap();
        let cards = key_cards_at(&tmp.path).unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].origin, cards[1].origin);
        assert_ne!(cards[0].id, cards[1].id);
        assert_eq!(cards[1].id, format!("onenewapi@{}", second.key_id));
        assert_eq!(cards[1].name, "Panel · Prod");
        let pool = refresh_clients(&cards);
        assert_eq!(pool.len(), 1);
        assert!(pool.contains_key("https://panel.example.com"));

        fs::write(&tmp.path, "{nope").unwrap();
        assert!(key_cards_at(&tmp.path).is_err());
    }

    #[test]
    fn snapshot_happy_path_paths_bearer_and_naming() {
        let sub = sub_body();
        let usage = usage_body();
        let key = "sk-live-quota";
        let (origin, join) =
            spawn_billing_server(2, move |origin, req| ok_billing(origin, req, &sub, &usage));
        let snap = tauri::async_runtime::block_on(snapshot_key(card(&origin, key)));
        let captured = join.join().unwrap();
        assert_eq!(snap.id, "onenewapi@keyidabcdefghijkAAA");
        assert_eq!(snap.name, "Panel · Key 1");
        assert_eq!(snap.plan, None);
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.dashboard_url.as_deref(), Some(origin.as_str()));
        let usage = snap.metrics.iter().find(|m| m.label == "Usage").unwrap();
        assert_eq!(usage.detail.as_deref(), Some("$592.18 of $1217.82"));
        assert_eq!(usage.resets_at, None);
        let expiry = snap.metrics.iter().find(|m| m.label == "Expiry").unwrap();
        assert_eq!(expiry.kind, "action");
        assert_eq!(expiry.resets_at, Some(1_790_479_748_000));

        let mut urls: Vec<_> = captured.iter().map(|c| c.url.as_str()).collect();
        urls.sort_unstable();
        assert_eq!(
            urls,
            [
                "/v1/dashboard/billing/subscription",
                "/v1/dashboard/billing/usage"
            ]
        );
        assert!(captured
            .iter()
            .all(|c| c.authorization.as_deref() == Some("Bearer sk-live-quota")));
        assert!(!snap.error.clone().unwrap_or_default().contains(key));
    }

    #[test]
    fn public_http_is_rejected_before_bearer_request() {
        let snap = tauri::async_runtime::block_on(snapshot_key(card(
            "http://example.com",
            "sk-must-not-send",
        )));
        assert_eq!(snap.status, "error");
        assert_eq!(
            snap.error.as_deref(),
            Some("plain HTTP is only allowed for private, loopback, or link-local IP addresses")
        );
    }

    #[test]
    fn snapshot_does_not_follow_redirects() {
        let usage = usage_body();
        let (origin, join) = spawn_billing_server(3, move |origin, req| match path_of(req) {
            "/v1/dashboard/billing/subscription" => {
                let loc = format!("{origin}/ok");
                let header = tiny_http::Header::from_bytes(&b"Location"[..], loc.as_bytes())
                    .expect("location header");
                tiny_http::Response::from_string(String::new())
                    .with_status_code(302)
                    .with_header(header)
            }
            "/v1/dashboard/billing/usage" => {
                tiny_http::Response::from_string(usage.clone()).with_status_code(200)
            }
            _ => tiny_http::Response::from_string("followed").with_status_code(200),
        });
        let snap = tauri::async_runtime::block_on(snapshot_key(card(&origin, "sk-r")));
        let captured = join.join().unwrap();
        assert_eq!(snap.status, "error");
        assert!(
            snap.error.as_deref().unwrap_or("").contains("HTTP"),
            "{:?}",
            snap.error
        );
        assert!(captured
            .iter()
            .all(|c| c.url != "/ok" && !c.url.starts_with("/ok?")));
        assert!(captured
            .iter()
            .any(|c| c.url == "/v1/dashboard/billing/subscription"));
    }

    #[test]
    fn snapshot_401_is_key_scoped_error() {
        let usage = usage_body();
        let key = "sk-bad";
        let (origin, join) = spawn_billing_server(2, move |_origin, req| match path_of(req) {
            "/v1/dashboard/billing/subscription" => {
                tiny_http::Response::from_string("denied").with_status_code(401)
            }
            _ => tiny_http::Response::from_string(usage.clone()).with_status_code(200),
        });
        let snap = tauri::async_runtime::block_on(snapshot_key(card(&origin, key)));
        let _ = join.join();
        assert_eq!(snap.status, "error");
        assert_eq!(snap.id, "onenewapi@keyidabcdefghijkAAA");
        let err = snap.error.unwrap();
        for needle in ["invalid", "expired", "disabled", "out of quota"] {
            assert!(err.contains(needle), "{err}");
        }
        assert!(!err.contains(key));
        assert_eq!(snap.dashboard_url.as_deref(), Some(origin.as_str()));
    }

    #[test]
    fn usage_404_keeps_limit() {
        let sub = json!({"hard_limit_usd": 40}).to_string();
        let (origin, join) = spawn_billing_server(2, move |_origin, req| match path_of(req) {
            "/v1/dashboard/billing/subscription" => {
                tiny_http::Response::from_string(sub.clone()).with_status_code(200)
            }
            _ => tiny_http::Response::from_string("missing").with_status_code(404),
        });
        let snap = tauri::async_runtime::block_on(snapshot_key(card(&origin, "sk-ok")));
        let _ = join.join();
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan, None);
        let limit = snap.metrics.iter().find(|m| m.label == "Limit").unwrap();
        assert_eq!(limit.value.as_deref(), Some("$40.00"));
        assert!(snap.metrics.iter().all(|m| m.label != "Usage"));
    }

    #[test]
    fn usage_http_failure_is_snapshot_error() {
        let sub = json!({"hard_limit_usd": 40}).to_string();
        let (origin, join) = spawn_billing_server(2, move |_origin, req| match path_of(req) {
            "/v1/dashboard/billing/subscription" => {
                tiny_http::Response::from_string(sub.clone()).with_status_code(200)
            }
            _ => tiny_http::Response::from_string("nope").with_status_code(500),
        });
        let snap = tauri::async_runtime::block_on(snapshot_key(card(&origin, "sk-ok")));
        let _ = join.join();
        assert_eq!(snap.status, "error");
        assert!(
            snap.error.as_deref().unwrap_or("").contains("usage HTTP"),
            "{:?}",
            snap.error
        );
        assert!(snap.metrics.iter().all(|m| m.label != "Limit"));
        assert!(snap.metrics.iter().all(|m| m.label != "Usage"));
    }

    #[test]
    fn usage_parse_failure_is_snapshot_error() {
        let sub = json!({"hard_limit_usd": 40}).to_string();
        let (origin, join) = spawn_billing_server(2, move |_origin, req| match path_of(req) {
            "/v1/dashboard/billing/subscription" => {
                tiny_http::Response::from_string(sub.clone()).with_status_code(200)
            }
            _ => tiny_http::Response::from_string("{not json").with_status_code(200),
        });
        let snap = tauri::async_runtime::block_on(snapshot_key(card(&origin, "sk-ok")));
        let _ = join.join();
        assert_eq!(snap.status, "error");
        assert_eq!(snap.error.as_deref(), Some("usage parse"));
        assert!(snap.metrics.iter().all(|m| m.label != "Usage"));
    }

    fn card_at(origin: &str, id: &str, name: &str, key: &str) -> KeyCard {
        KeyCard {
            id: id.into(),
            name: name.into(),
            origin: origin.into(),
            api_key: key.into(),
            display: DisplayUnit::Usd,
        }
    }

    #[test]
    fn two_keys_bearer_isolation_401_does_not_pollute() {
        let usage = usage_body();
        let sub = sub_body();
        let (origin, join) = spawn_billing_server(4, move |_origin, req| {
            let auth = authorization(req);
            match path_of(req) {
                "/v1/dashboard/billing/subscription" if auth.as_deref() == Some("Bearer sk-a") => {
                    tiny_http::Response::from_string("denied").with_status_code(401)
                }
                "/v1/dashboard/billing/subscription" => {
                    tiny_http::Response::from_string(sub.clone()).with_status_code(200)
                }
                _ => tiny_http::Response::from_string(usage.clone()).with_status_code(200),
            }
        });
        let a = card_at(&origin, "onenewapi@keyA", "Panel · A", "sk-a");
        let b = card_at(&origin, "onenewapi@keyB", "Panel · B", "sk-b");
        let (snap_a, snap_b) = tauri::async_runtime::block_on(async {
            tokio::join!(snapshot_key(a), snapshot_key(b))
        });
        let captured = join.join().unwrap();
        assert_eq!(snap_a.status, "error");
        assert_eq!(snap_a.id, "onenewapi@keyA");
        let err = snap_a.error.unwrap();
        assert!(err.contains("invalid"));
        assert!(!err.contains("sk-a"));
        assert!(!err.contains("sk-b"));
        assert_eq!(snap_b.status, "ok");
        assert_eq!(snap_b.id, "onenewapi@keyB");
        assert_eq!(snap_b.name, "Panel · B");
        let usage = snap_b.metrics.iter().find(|m| m.label == "Usage").unwrap();
        assert_eq!(usage.detail.as_deref(), Some("$592.18 of $1217.82"));
        let mut auths: Vec<_> = captured
            .iter()
            .filter_map(|c| c.authorization.as_deref())
            .collect();
        auths.sort_unstable();
        assert!(auths.iter().any(|a| *a == "Bearer sk-a"));
        assert!(auths.iter().any(|a| *a == "Bearer sk-b"));
        assert!(captured.iter().all(|c| {
            c.authorization.as_deref() == Some("Bearer sk-a")
                || c.authorization.as_deref() == Some("Bearer sk-b")
        }));
        assert!(captured.iter().all(|c| {
            let mixed = c.authorization.as_deref() == Some("Bearer sk-a, Bearer sk-b");
            !mixed
        }));
    }

    #[test]
    fn identical_quota_stays_independent_cards() {
        let sub = sub_body();
        let usage = usage_body();
        let (origin, join) =
            spawn_billing_server(4, move |origin, req| ok_billing(origin, req, &sub, &usage));
        let a = card_at(&origin, "onenewapi@keyA", "Panel · A", "sk-a");
        let b = card_at(&origin, "onenewapi@keyB", "Panel · B", "sk-b");
        let (snap_a, snap_b) = tauri::async_runtime::block_on(async {
            tokio::join!(snapshot_key(a), snapshot_key(b))
        });
        let _ = join.join();
        assert_eq!(snap_a.status, "ok");
        assert_eq!(snap_b.status, "ok");
        assert_ne!(snap_a.id, snap_b.id);
        assert_ne!(snap_a.name, snap_b.name);
        let ua = snap_a.metrics.iter().find(|m| m.label == "Usage").unwrap();
        let ub = snap_b.metrics.iter().find(|m| m.label == "Usage").unwrap();
        assert_eq!(ua.detail, ub.detail);
        assert_eq!(ua.used_percent, ub.used_percent);
        assert_eq!(ua.detail.as_deref(), Some("$592.18 of $1217.82"));
        let summed = ua
            .used_percent
            .zip(ub.used_percent)
            .map(|(x, y)| x + y)
            .unwrap_or(0.0);
        assert!(summed > 90.0, "must not merge two ~48% bars into one");
        assert_eq!(snap_a.metrics.len(), snap_b.metrics.len());
    }

    #[test]
    fn billing_in_flight_capped_at_eight() {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));
        let n_keys = 9;
        let n_requests = n_keys * 2;
        let (origin, join) = spawn_slow_billing_server(
            n_requests,
            Duration::from_millis(200),
            Arc::clone(&in_flight),
            Arc::clone(&max_in_flight),
        );
        let cards: Vec<KeyCard> = (0..n_keys)
            .map(|i| {
                card_at(
                    &origin,
                    &format!("onenewapi@cap{i:02}"),
                    &format!("Panel · Key {i}"),
                    &format!("sk-{i}"),
                )
            })
            .collect();
        let snaps = tauri::async_runtime::block_on(async {
            let handles: Vec<_> = cards
                .into_iter()
                .map(|card| tauri::async_runtime::spawn(snapshot_key(card)))
                .collect();
            let mut out = Vec::new();
            for h in handles {
                out.push(h.await.expect("snapshot join"));
            }
            out
        });
        join.join().unwrap();
        let max = max_in_flight.load(Ordering::SeqCst);
        assert!(max <= 8, "in-flight billing GETs peaked at {max}, cap is 8");
        assert_eq!(snaps.len(), n_keys);
        assert!(
            snaps.iter().all(|s| s.status == "ok"),
            "{:?}",
            snaps
                .iter()
                .map(|s| (s.id.as_str(), s.status.as_str(), s.error.as_deref()))
                .collect::<Vec<_>>()
        );
        assert_eq!(in_flight.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn billing_in_flight_cap_holds_through_chunked_body() {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_in_flight = Arc::new(AtomicUsize::new(0));
        let n_keys = 9;
        let n_requests = n_keys * 2;
        let (origin, join) = spawn_chunked_slow_billing_server(
            n_requests,
            Duration::from_millis(200),
            Arc::clone(&in_flight),
            Arc::clone(&max_in_flight),
        );
        let cards: Vec<KeyCard> = (0..n_keys)
            .map(|i| {
                card_at(
                    &origin,
                    &format!("onenewapi@chunk{i:02}"),
                    &format!("Panel · Key {i}"),
                    &format!("sk-{i}"),
                )
            })
            .collect();
        let snaps = tauri::async_runtime::block_on(async {
            let handles: Vec<_> = cards
                .into_iter()
                .map(|card| tauri::async_runtime::spawn(snapshot_key(card)))
                .collect();
            let mut out = Vec::new();
            for h in handles {
                out.push(h.await.expect("snapshot join"));
            }
            out
        });
        join.join().unwrap();
        let max = max_in_flight.load(Ordering::SeqCst);
        assert!(
            max <= 8,
            "in-flight billing GETs peaked at {max} during chunked bodies, cap is 8"
        );
        assert_eq!(snaps.len(), n_keys);
        assert!(
            snaps.iter().all(|s| s.status == "ok"),
            "{:?}",
            snaps
                .iter()
                .map(|s| (s.id.as_str(), s.status.as_str(), s.error.as_deref()))
                .collect::<Vec<_>>()
        );
        assert_eq!(in_flight.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn snapshot_cny_card_formats_yen() {
        let sub = sub_body();
        let usage = usage_body();
        let (origin, join) =
            spawn_billing_server(2, move |origin, req| ok_billing(origin, req, &sub, &usage));
        let snap = tauri::async_runtime::block_on(snapshot_key(card_with_display(
            &origin,
            "sk-cny",
            DisplayUnit::Cny,
        )));
        let _ = join.join();
        assert_eq!(snap.status, "ok");
        let usage = snap.metrics.iter().find(|m| m.label == "Usage").unwrap();
        assert_eq!(usage.detail.as_deref(), Some("¥592.18 of ¥1217.82"));
    }

    #[test]
    fn backfill_persists_missing_unit_then_skips_status() {
        let tmp = TempStore::new();
        let status_hits = Arc::new(AtomicUsize::new(0));
        let hits = Arc::clone(&status_hits);
        let body = json!({
            "success": true,
            "data": {"version": "1", "quota_display_type": "CNY"}
        })
        .to_string();
        let (origin, join) = spawn_billing_server(1, move |_origin, req| {
            if path_of(req) == "/api/status" {
                hits.fetch_add(1, Ordering::SeqCst);
                return tiny_http::Response::from_string(body.clone()).with_status_code(200);
            }
            tiny_http::Response::from_string("nope").with_status_code(404)
        });
        fs::write(
            &tmp.path,
            json!({
                "version": 1,
                "sites": [{
                    "id": "siteidabcdefghijkAAA",
                    "name": "Panel",
                    "baseUrl": origin,
                    "nextKeyOrdinal": 2,
                    "keys": [{
                        "id": "keyidabcdefghijkAAA",
                        "label": "Key 1",
                        "apiKey": "sk-one"
                    }]
                }]
            })
            .to_string(),
        )
        .unwrap();
        let cards = key_cards_at(&tmp.path).unwrap();
        assert_eq!(cards[0].display, DisplayUnit::Usd);
        tauri::async_runtime::block_on(backfill_missing_display_units(&tmp.path));
        let cards = key_cards_at(&tmp.path).unwrap();
        assert_eq!(cards[0].display, DisplayUnit::Cny);
        tauri::async_runtime::block_on(backfill_missing_display_units(&tmp.path));
        let captured = join.join().unwrap();
        assert_eq!(status_hits.load(Ordering::SeqCst), 1);
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].url, "/api/status");
        assert_eq!(captured[0].authorization, None);
        let loaded = store::load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Cny);
    }

    #[test]
    fn backfill_skips_write_after_origin_change() {
        let tmp = TempStore::new();
        let release = Arc::new(AtomicBool::new(false));
        let status_hits = Arc::new(AtomicUsize::new(0));
        let hits = Arc::clone(&status_hits);
        let gate = Arc::clone(&release);
        let body = json!({
            "success": true,
            "data": {"version": "1", "quota_display_type": "CNY"}
        })
        .to_string();
        let (origin, join) = spawn_billing_server(1, move |_origin, req| {
            if path_of(req) == "/api/status" {
                hits.fetch_add(1, Ordering::SeqCst);
                let deadline = Instant::now() + Duration::from_secs(5);
                while !gate.load(Ordering::SeqCst) {
                    if Instant::now() > deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                return tiny_http::Response::from_string(body.clone()).with_status_code(200);
            }
            tiny_http::Response::from_string("nope").with_status_code(404)
        });
        fs::write(
            &tmp.path,
            json!({
                "version": 1,
                "sites": [{
                    "id": "siteidabcdefghijkAAA",
                    "name": "Panel",
                    "baseUrl": origin,
                    "nextKeyOrdinal": 1,
                    "keys": []
                }]
            })
            .to_string(),
        )
        .unwrap();
        let path = tmp.path.clone();
        let backfill = std::thread::spawn(move || {
            tauri::async_runtime::block_on(backfill_missing_display_units(&path));
        });
        let deadline = Instant::now() + Duration::from_secs(3);
        while status_hits.load(Ordering::SeqCst) == 0 {
            assert!(Instant::now() < deadline, "status probe never arrived");
            std::thread::sleep(Duration::from_millis(10));
        }
        store::update_site(
            &tmp.path,
            "siteidabcdefghijkAAA",
            None,
            Some(normalize_base_url("https://tokens.example.com").unwrap()),
            Some(DisplayUnit::Tokens),
        )
        .unwrap();
        release.store(true, Ordering::SeqCst);
        backfill.join().unwrap();
        let _ = join.join();
        let loaded = store::load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].base_url, "https://tokens.example.com");
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Tokens);
    }

    fn spawn_slow_billing_server(
        n: usize,
        delay: Duration,
        in_flight: Arc<AtomicUsize>,
        max_in_flight: Arc<AtomicUsize>,
    ) -> (String, std::thread::JoinHandle<()>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_ip().unwrap();
        let origin = format!("http://127.0.0.1:{}", addr.port());
        let sub = sub_body();
        let usage = usage_body();
        let join = std::thread::spawn(move || {
            let mut joins = Vec::with_capacity(n);
            for _ in 0..n {
                match server.recv_timeout(Duration::from_secs(5)) {
                    Ok(Some(req)) => {
                        let in_flight = Arc::clone(&in_flight);
                        let max_in_flight = Arc::clone(&max_in_flight);
                        let sub = sub.clone();
                        let usage = usage.clone();
                        joins.push(std::thread::spawn(move || {
                            let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                            max_in_flight.fetch_max(now, Ordering::SeqCst);
                            std::thread::sleep(delay);
                            let resp = match path_of(&req) {
                                "/v1/dashboard/billing/subscription" => {
                                    tiny_http::Response::from_string(sub).with_status_code(200)
                                }
                                _ => tiny_http::Response::from_string(usage).with_status_code(200),
                            };
                            let _ = req.respond(resp);
                            in_flight.fetch_sub(1, Ordering::SeqCst);
                        }));
                    }
                    _ => break,
                }
            }
            for j in joins {
                let _ = j.join();
            }
        });
        (origin, join)
    }

    fn spawn_chunked_slow_billing_server(
        n: usize,
        delay: Duration,
        in_flight: Arc<AtomicUsize>,
        max_in_flight: Arc<AtomicUsize>,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let origin = format!("http://127.0.0.1:{}", addr.port());
        listener.set_nonblocking(true).unwrap();
        let sub = sub_body();
        let usage = usage_body();
        let join = std::thread::spawn(move || {
            let mut joins = Vec::with_capacity(n);
            let deadline = Instant::now() + Duration::from_secs(15);
            while joins.len() < n {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let in_flight = Arc::clone(&in_flight);
                        let max_in_flight = Arc::clone(&max_in_flight);
                        let sub = sub.clone();
                        let usage = usage.clone();
                        joins.push(std::thread::spawn(move || {
                            serve_chunked_billing(
                                stream,
                                delay,
                                &sub,
                                &usage,
                                &in_flight,
                                &max_in_flight,
                            );
                        }));
                    }
                    Err(e)
                        if e.kind() == ErrorKind::WouldBlock
                            || e.kind() == ErrorKind::TimedOut
                            || e.kind() == ErrorKind::Interrupted =>
                    {
                        if Instant::now() > deadline {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
            for j in joins {
                let _ = j.join();
            }
        });
        (origin, join)
    }

    fn serve_chunked_billing(
        mut stream: TcpStream,
        delay: Duration,
        sub: &str,
        usage: &str,
        in_flight: &AtomicUsize,
        max_in_flight: &AtomicUsize,
    ) {
        let _ = stream.set_nodelay(true);
        let req = match read_http_head(&mut stream) {
            Ok(req) => req,
            Err(_) => return,
        };
        let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        max_in_flight.fetch_max(now, Ordering::SeqCst);
        let body = if req.contains("/v1/dashboard/billing/subscription") {
            sub
        } else {
            usage
        };
        let headers = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: application/json\r\n",
            "Transfer-Encoding: chunked\r\n",
            "Connection: close\r\n",
            "\r\n",
        );
        if stream.write_all(headers.as_bytes()).is_err() || stream.flush().is_err() {
            in_flight.fetch_sub(1, Ordering::SeqCst);
            return;
        }
        std::thread::sleep(delay);
        let chunk = format!("{:x}\r\n{body}\r\n0\r\n\r\n", body.len());
        let _ = stream.write_all(chunk.as_bytes());
        let _ = stream.flush();
        in_flight.fetch_sub(1, Ordering::SeqCst);
    }

    fn read_http_head(stream: &mut TcpStream) -> std::io::Result<String> {
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut buf = Vec::new();
        let mut tmp = [0u8; 512];
        loop {
            let n = stream.read(&mut tmp)?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 16 * 1024 {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }
}
