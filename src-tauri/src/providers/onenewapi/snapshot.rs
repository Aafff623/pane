use super::super::{http_no_redirect, json_body, Snapshot};
use super::billing;
use super::fingerprint::DisplayUnit;
use super::store;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

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
    dashboard_get(client, url, api_key, None, what).await
}

/// GET with a bearer credential; dashboard endpoints on rc builds also
/// demand the matching `New-Api-User` header alongside the access token.
async fn dashboard_get(
    client: &reqwest::Client,
    url: &str,
    bearer: &str,
    user_id: Option<&str>,
    what: &str,
) -> Result<(reqwest::StatusCode, Result<serde_json::Value, String>), reqwest::Error> {
    let _permit = billing_sema().acquire().await.expect("billing semaphore");
    let mut request = client.get(url).bearer_auth(bearer);
    if let Some(uid) = user_id {
        request = request.header("New-Api-User", uid);
    }
    let resp = request.send().await?;
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
    /// Raw-quota → display-value scale for the subscription endpoint.
    pub per_unit: f64,
    pub rate: f64,
    /// Site's NewAPI dashboard access token; `None` = subscription display
    /// off, billing only.
    pub access_token: Option<String>,
    /// Dashboard user id paired with the token (`New-Api-User` header on
    /// rc builds).
    pub user_id: Option<String>,
}

pub fn key_cards_at(path: &Path) -> Result<Vec<KeyCard>, String> {
    let doc = store::load(path)?;
    Ok(doc
        .sites
        .iter()
        .flat_map(|site| {
            let (per_unit, rate) = site.quota_scale();
            let access_token = site
                .has_access_token()
                .then(|| site.access_token.clone());
            let user_id = (!site.user_id.is_empty()).then(|| site.user_id.clone());
            site.keys
                .iter()
                .filter(|k| !k.api_key.is_empty())
                .map(move |key| KeyCard {
                    id: format!("onenewapi@{}", key.id),
                    name: format!("{} · {}", site.name, key.label),
                    origin: site.base_url.clone(),
                    api_key: key.api_key.clone(),
                    display: site.quota_display(),
                    per_unit,
                    rate,
                    access_token: access_token.clone(),
                    user_id: user_id.clone(),
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
        Ok(usage) => with_dashboard(
            Snapshot::ok(&card.id, &card.name, usage.plan, usage.metrics),
            &card,
        ),
        Err(e) => with_dashboard(Snapshot::error(&card.id, &card.name, e), &card),
    }
}

fn with_dashboard(mut snap: Snapshot, card: &KeyCard) -> Snapshot {
    snap.dashboard_url = Some(card.origin.clone());
    snap
}

/// One key card's numbers: optional plan tag (subscription mode) + metrics.
struct KeyUsage {
    plan: Option<String>,
    metrics: Vec<super::super::Metric>,
}

async fn fetch_key(
    client: &reqwest::Client,
    card: &KeyCard,
) -> Result<KeyUsage, String> {
    let origin = super::url::normalize_base_url(&card.origin)?.origin;

    // Subscription display first: a configured dashboard access token turns
    // the card into the NewAPI subscription plan (daily-reset allowance,
    // next-reset countdown, expiry). Anything that fails to produce an
    // active subscription — missing endpoint, rejected token, parse —
    // degrades silently to the wallet billing display below.
    if let Some(token) = card.access_token.as_deref().filter(|t| !t.trim().is_empty()) {
        let url = format!("{origin}/api/subscription/self");
        let fetched = dashboard_get(
            client,
            &url,
            token,
            card.user_id.as_deref(),
            "subscription",
        )
        .await;
        if let Ok((status, body)) = fetched {
            if status.is_success() {
                if let Ok(value) = body {
                    if let Some(view) = billing::subscription_view(
                        &value,
                        &card.display,
                        card.per_unit,
                        card.rate,
                    ) {
                        return Ok(KeyUsage {
                            plan: Some(view.plan_label),
                            metrics: view.metrics,
                        });
                    }
                }
            }
        }
    }

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

    let metrics = billing::metrics_from(&sub, usage.as_ref(), &card.display)?;
    Ok(KeyUsage {
        plan: None,
        metrics,
    })
}

fn backfill_claimed() -> &'static Mutex<HashSet<(String, String)>> {
    static CLAIMED: OnceLock<Mutex<HashSet<(String, String)>>> = OnceLock::new();
    CLAIMED.get_or_init(Default::default)
}

/// One unauthenticated status backfill per stored site, off the refresh path.
pub fn schedule_backfill_missing_display_units(path: PathBuf) {
    tauri::async_runtime::spawn(async move {
        backfill_missing_display_units(&path).await;
    });
}

pub async fn backfill_missing_display_units(path: &Path) {
    let Ok(doc) = store::load(path) else {
        return;
    };
    let missing: Vec<(String, String)> = {
        let mut claimed = backfill_claimed().lock().unwrap();
        doc.sites
            .iter()
            .filter(|site| site.display_unit.is_none() || site.per_unit.is_none())
            .filter(|site| claimed.insert((site.id.clone(), site.base_url.clone())))
            .map(|site| (site.id.clone(), site.base_url.clone()))
            .collect()
    };
    for (id, origin) in missing {
        let Ok(fp) = super::fingerprint::probe(&origin).await else {
            continue;
        };
        let _ = super::set_fingerprint_at(path, &id, &origin, fp);
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
        backfill_missing_display_units, key_cards_at, refresh_clients,
        schedule_backfill_missing_display_units, snapshot_key, DisplayUnit, KeyCard,
    };
    use crate::providers::onenewapi::fingerprint::SiteFingerprint;
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
        new_api_user: Option<String>,
    }

    fn authorization(req: &tiny_http::Request) -> Option<String> {
        req.headers()
            .iter()
            .find(|h| h.field.equiv("Authorization"))
            .map(|h| h.value.as_str().to_string())
    }

    fn new_api_user(req: &tiny_http::Request) -> Option<String> {
        req.headers()
            .iter()
            .find(|h| h.field.equiv("New-Api-User"))
            .map(|h| h.value.as_str().to_string())
    }

    fn capture(req: &tiny_http::Request) -> Captured {
        Captured {
            url: req.url().to_string(),
            authorization: authorization(req),
            new_api_user: new_api_user(req),
        }
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
                        out.push(capture(&req));
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
        card_full(origin, key, display, 500_000.0, 1.0, None, None)
    }

    fn card_full(
        origin: &str,
        key: &str,
        display: DisplayUnit,
        per_unit: f64,
        rate: f64,
        access_token: Option<&str>,
        user_id: Option<&str>,
    ) -> KeyCard {
        KeyCard {
            id: "onenewapi@keyidabcdefghijkAAA".into(),
            name: "Panel · Key 1".into(),
            origin: origin.into(),
            api_key: key.into(),
            display,
            per_unit,
            rate,
            access_token: access_token.map(str::to_string),
            user_id: user_id.map(str::to_string),
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
            store::insert_site(&tmp.path, "Panel", &url, SiteFingerprint::from(DisplayUnit::Usd)).unwrap()
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

    // --- subscription display (site access token) --------------------------

    fn subscription_body() -> String {
        json!({
            "success": true,
            "message": "",
            "data": {
                "billing_preference": "subscription_first",
                "subscriptions": [{
                    "subscription": {
                        "id": 239,
                        "amount_total": 1_000_000_000i64,
                        "amount_used": 279_000_000i64,
                        "end_time": 1_791_164_506,
                        "next_reset_time": 1_791_155_200,
                        "status": "active"
                    }
                }],
                "all_subscriptions": []
            }
        })
        .to_string()
    }

    #[test]
    fn access_token_serves_subscription_and_skips_billing() {
        let body = subscription_body();
        let (origin, join) = spawn_billing_server(1, move |_origin, req| match path_of(req) {
            "/api/subscription/self" => {
                tiny_http::Response::from_string(body.clone()).with_status_code(200)
            }
            _ => tiny_http::Response::from_string("must not be hit").with_status_code(500),
        });
        let snap = tauri::async_runtime::block_on(snapshot_key(card_full(
            &origin,
            "sk-live-quota",
            DisplayUnit::Usd,
            500_000.0,
            1.0,
            Some("at-token"),
            Some("42"),
        )));
        let captured = join.join().unwrap();
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan.as_deref(), Some("Subscription #239"));
        let usage = snap.metrics.iter().find(|m| m.label == "Usage").unwrap();
        assert_eq!(usage.kind, "progress");
        // $558 used of $2 000 → 27.9% used, the card headline reads "72.1% left".
        assert!((usage.used_percent.unwrap() - 27.9).abs() < 1e-9);
        assert_eq!(usage.detail.as_deref(), Some("$558.00 of $2000.00"));
        assert_eq!(usage.resets_at, Some(1_791_155_200_000));
        assert_eq!(usage.period_ms, Some(24 * 60 * 60 * 1000));
        let expiry = snap.metrics.iter().find(|m| m.label == "Expiry").unwrap();
        assert_eq!(expiry.resets_at, Some(1_791_164_506_000));

        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].url, "/api/subscription/self");
        // The dashboard token is sent alone; the sk- key never leaks there.
        assert_eq!(
            captured[0].authorization.as_deref(),
            Some("Bearer at-token")
        );
        // rc builds require the matching New-Api-User header.
        assert_eq!(captured[0].new_api_user.as_deref(), Some("42"));
        assert_eq!(snap.dashboard_url.as_deref(), Some(origin.as_str()));
    }

    #[test]
    fn rejected_access_token_falls_back_to_billing() {
        let sub = sub_body();
        let usage = usage_body();
        let (origin, join) = spawn_billing_server(3, move |_origin, req| {
            match path_of(req) {
                "/api/subscription/self" => {
                    tiny_http::Response::from_string("denied").with_status_code(401)
                }
                "/v1/dashboard/billing/subscription" => {
                    tiny_http::Response::from_string(sub.clone()).with_status_code(200)
                }
                _ => tiny_http::Response::from_string(usage.clone()).with_status_code(200),
            }
        });
        let snap = tauri::async_runtime::block_on(snapshot_key(card_full(
            &origin,
            "sk-live-quota",
            DisplayUnit::Usd,
            500_000.0,
            1.0,
            Some("at-bad"),
            None,
        )));
        let captured = join.join().unwrap();
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan, None);
        let usage = snap.metrics.iter().find(|m| m.label == "Usage").unwrap();
        assert_eq!(usage.detail.as_deref(), Some("$592.18 of $1217.82"));
        assert_eq!(
            captured[0].authorization.as_deref(),
            Some("Bearer at-bad")
        );
        assert!(captured[1..]
            .iter()
            .all(|c| c.authorization.as_deref() == Some("Bearer sk-live-quota")));
    }

    #[test]
    fn subscription_without_active_plan_falls_back_to_billing() {
        let body = json!({
            "success": true,
            "data": {"subscriptions": [], "all_subscriptions": []}
        })
        .to_string();
        let sub = sub_body();
        let usage = usage_body();
        let (origin, join) = spawn_billing_server(3, move |_origin, req| match path_of(req) {
            "/api/subscription/self" => {
                tiny_http::Response::from_string(body.clone()).with_status_code(200)
            }
            "/v1/dashboard/billing/subscription" => {
                tiny_http::Response::from_string(sub.clone()).with_status_code(200)
            }
            _ => tiny_http::Response::from_string(usage.clone()).with_status_code(200),
        });
        let snap = tauri::async_runtime::block_on(snapshot_key(card_full(
            &origin,
            "sk-live-quota",
            DisplayUnit::Usd,
            500_000.0,
            1.0,
            Some("at-token"),
            None,
        )));
        let _ = join.join();
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.plan, None);
        let usage = snap.metrics.iter().find(|m| m.label == "Usage").unwrap();
        assert_eq!(usage.detail.as_deref(), Some("$592.18 of $1217.82"));
    }

    #[test]
    fn key_cards_carry_site_scale_and_token() {
        let tmp = TempStore::new();
        let url = normalize_base_url("https://panel.example.com").unwrap();
        let CreateSiteResult::Created { site } =
            store::insert_site(&tmp.path, "Panel", &url, SiteFingerprint {
                unit: DisplayUnit::Cny,
                per_unit: 600_000.0,
                rate: 7.25,
            })
            .unwrap()
        else {
            panic!("expected created");
        };
        store::create_key(&tmp.path, &site.id, "", "sk-one").unwrap();
        let cards = key_cards_at(&tmp.path).unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].display, DisplayUnit::Cny);
        assert_eq!(cards[0].per_unit, 600_000.0);
        assert_eq!(cards[0].rate, 7.25);
        assert_eq!(cards[0].access_token, None);
        assert_eq!(cards[0].user_id, None);

        store::set_site_auth(
            &tmp.path,
            &site.id,
            Some("at-9"),
            Some("42"),
        )
        .unwrap();
        let cards = key_cards_at(&tmp.path).unwrap();
        assert_eq!(cards[0].access_token.as_deref(), Some("at-9"));
        assert_eq!(cards[0].user_id.as_deref(), Some("42"));
        store::set_site_auth(&tmp.path, &site.id, Some(""), Some("")).unwrap();
        let cards = key_cards_at(&tmp.path).unwrap();
        assert_eq!(cards[0].access_token, None);
        assert_eq!(cards[0].user_id, None);
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
            per_unit: 500_000.0,
            rate: 1.0,
            access_token: None,
            user_id: None,
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
            Some(SiteFingerprint::from(DisplayUnit::Tokens)),
        )
        .unwrap();
        release.store(true, Ordering::SeqCst);
        backfill.join().unwrap();
        let _ = join.join();
        let loaded = store::load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].base_url, "https://tokens.example.com");
        assert_eq!(loaded.sites[0].quota_display(), DisplayUnit::Tokens);
    }

    #[test]
    fn backfill_failed_probe_leaves_unit_unset_and_does_not_retry() {
        let tmp = TempStore::new();
        let status_hits = Arc::new(AtomicUsize::new(0));
        let hits = Arc::clone(&status_hits);
        let (origin, join) = spawn_billing_server(1, move |_origin, req| {
            if path_of(req) == "/api/status" {
                hits.fetch_add(1, Ordering::SeqCst);
                return tiny_http::Response::from_string("down").with_status_code(500);
            }
            tiny_http::Response::from_string("nope").with_status_code(404)
        });
        fs::write(
            &tmp.path,
            json!({
                "version": 1,
                "sites": [{
                    "id": "siteidbackfillfail01A",
                    "name": "Panel",
                    "baseUrl": origin,
                    "nextKeyOrdinal": 1,
                    "keys": []
                }]
            })
            .to_string(),
        )
        .unwrap();
        tauri::async_runtime::block_on(backfill_missing_display_units(&tmp.path));
        tauri::async_runtime::block_on(backfill_missing_display_units(&tmp.path));
        let captured = join.join().unwrap();
        assert_eq!(status_hits.load(Ordering::SeqCst), 1);
        assert_eq!(captured.len(), 1);
        let loaded = store::load(&tmp.path).unwrap();
        assert_eq!(loaded.sites[0].display_unit, None);
    }

    #[test]
    fn scheduled_backfill_does_not_block_key_cards() {
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
                    "id": "siteidschedbackfillAA",
                    "name": "Panel",
                    "baseUrl": origin,
                    "nextKeyOrdinal": 2,
                    "keys": [{
                        "id": "keyidschedbackfillAA",
                        "label": "Key 1",
                        "apiKey": "sk-one"
                    }]
                }]
            })
            .to_string(),
        )
        .unwrap();
        let started = Instant::now();
        schedule_backfill_missing_display_units(tmp.path.clone());
        let cards = key_cards_at(&tmp.path).unwrap();
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "key card prepare waited on status backfill: {:?}",
            started.elapsed()
        );
        assert_eq!(cards[0].display, DisplayUnit::Usd);
        let deadline = Instant::now() + Duration::from_secs(3);
        while status_hits.load(Ordering::SeqCst) == 0 {
            assert!(Instant::now() < deadline, "status probe never arrived");
            std::thread::sleep(Duration::from_millis(10));
        }
        release.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let loaded = store::load(&tmp.path).unwrap();
            if loaded.sites[0].quota_display() == DisplayUnit::Cny {
                break;
            }
            assert!(Instant::now() < deadline, "background backfill never persisted");
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = join.join();
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
