use serde_json::Value;

const MAX_STATUS_BYTES: usize = 64 * 1024;

/// Site-level quota display unit from `/api/status`. Billing numbers stay in
/// the OpenAI cents convention for every unit; this only chooses formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisplayUnit {
    Usd,
    Cny,
    Tokens,
    Custom(String),
}

impl DisplayUnit {
    pub fn to_store(&self) -> (Option<String>, Option<String>) {
        match self {
            Self::Usd => (Some("usd".into()), None),
            Self::Cny => (Some("cny".into()), None),
            Self::Tokens => (Some("tokens".into()), None),
            Self::Custom(sym) => {
                let symbol = if sym.trim().is_empty() {
                    "¤".to_string()
                } else {
                    sym.clone()
                };
                (Some("custom".into()), Some(symbol))
            }
        }
    }

    pub fn from_store(unit: Option<&str>, symbol: Option<&str>) -> Self {
        match unit.map(str::trim) {
            Some("cny") => Self::Cny,
            Some("tokens") => Self::Tokens,
            Some("custom") => {
                let symbol = symbol
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or("¤");
                Self::Custom(symbol.to_string())
            }
            _ => Self::Usd,
        }
    }
}

/// Structural OneAPI / NewAPI check. Does not require branding text or a
/// particular quota display unit. On success, parse the unit for formatting.
pub fn fingerprint_payload(v: &Value) -> Result<DisplayUnit, String> {
    if v.get("success") != Some(&Value::Bool(true)) {
        return Err("status fingerprint mismatch".into());
    }
    let Some(data) = v.get("data") else {
        return Err("status fingerprint mismatch".into());
    };
    let Some(obj) = data.as_object() else {
        return Err("status fingerprint mismatch".into());
    };
    let named = ["version", "system_name"].iter().any(|key| {
        obj.get(*key)
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
    });
    if !named {
        return Err("status fingerprint mismatch".into());
    }
    Ok(parse_display_unit(data))
}

pub fn parse_display_unit(data: &Value) -> DisplayUnit {
    let ty = data
        .get("quota_display_type")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_ascii_uppercase());
    match ty.as_deref() {
        Some("CNY") => DisplayUnit::Cny,
        Some("TOKENS") => DisplayUnit::Tokens,
        Some("CUSTOM") => {
            let symbol = data
                .get("custom_currency_symbol")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("¤");
            DisplayUnit::Custom(symbol.to_string())
        }
        Some("USD") => DisplayUnit::Usd,
        _ => {
            if data.get("display_in_currency") == Some(&Value::Bool(false)) {
                DisplayUnit::Tokens
            } else {
                DisplayUnit::Usd
            }
        }
    }
}

pub async fn probe(origin: &str) -> Result<DisplayUnit, String> {
    let origin = super::url::normalize_base_url(origin)?.origin;
    let url = format!("{origin}/api/status");
    let resp = super::super::http_no_redirect()
        .get(&url)
        .send()
        .await
        .map_err(|_| "status transport".to_string())?;
    if resp.status().as_u16() == 404 {
        // Missing /api/status means this origin is not a One/New API panel.
        return Err("status fingerprint mismatch".into());
    }
    if !resp.status().is_success() {
        return Err(format!("status endpoint: HTTP {}", resp.status()));
    }
    let body = super::super::json_body(resp, MAX_STATUS_BYTES, "status")
        .await
        .map_err(|e| {
            if e.contains("too large") {
                "status too large".to_string()
            } else {
                "status parse".to_string()
            }
        })?;
    fingerprint_payload(&body)
}

#[cfg(test)]
mod tests {
    use super::{fingerprint_payload, parse_display_unit, probe, DisplayUnit};
    use serde_json::{json, Value};
    use std::io::ErrorKind;
    use std::time::Duration;

    fn ok_payload() -> Value {
        json!({
            "success": true,
            "data": {
                "version": "v0.0-test",
                "system_name": "New API",
                "quota_display_type": "USD"
            }
        })
    }

    #[test]
    fn accepts_structural_payload_regardless_of_branding_or_unit() {
        assert_eq!(
            fingerprint_payload(&ok_payload()).unwrap(),
            DisplayUnit::Usd
        );
        assert_eq!(
            fingerprint_payload(&json!({
                "success": true,
                "data": {
                    "version": "1.0",
                    "system_name": "Totally Custom Panel",
                    "quota_display_type": "usd"
                }
            }))
            .unwrap(),
            DisplayUnit::Usd
        );
        assert_eq!(
            fingerprint_payload(&json!({
                "success": true,
                "data": {
                    "system_name": "One API",
                    "display_in_currency": true
                }
            }))
            .unwrap(),
            DisplayUnit::Usd
        );
        assert_eq!(
            fingerprint_payload(&json!({
                "success": true,
                "data": {
                    "version": "build",
                    "quota_display_type": "USD",
                    "display_in_currency": false
                }
            }))
            .unwrap(),
            DisplayUnit::Usd
        );
        assert_eq!(
            fingerprint_payload(&json!({
                "success": true,
                "data": {"version": "1"}
            }))
            .unwrap(),
            DisplayUnit::Usd
        );
        assert_eq!(
            fingerprint_payload(&json!({
                "success": true,
                "data": {"version": "1", "quota_display_type": "CNY"}
            }))
            .unwrap(),
            DisplayUnit::Cny
        );
        assert_eq!(
            fingerprint_payload(&json!({
                "success": true,
                "data": {"version": "1", "quota_display_type": "TOKENS"}
            }))
            .unwrap(),
            DisplayUnit::Tokens
        );
        assert_eq!(
            fingerprint_payload(&json!({
                "success": true,
                "data": {"version": "1", "display_in_currency": false}
            }))
            .unwrap(),
            DisplayUnit::Tokens
        );
        assert_eq!(
            fingerprint_payload(&json!({
                "success": true,
                "data": {
                    "version": "",
                    "system_name": "国创Token运营平台",
                    "quota_display_type": "CNY",
                    "display_in_currency": true
                }
            }))
            .unwrap(),
            DisplayUnit::Cny
        );
    }

    #[test]
    fn parse_display_unit_priority_and_fallbacks() {
        assert_eq!(
            parse_display_unit(&json!({"quota_display_type": "cny", "display_in_currency": true})),
            DisplayUnit::Cny
        );
        assert_eq!(
            parse_display_unit(&json!({"display_in_currency": false})),
            DisplayUnit::Tokens
        );
        assert_eq!(
            parse_display_unit(&json!({"display_in_currency": true})),
            DisplayUnit::Usd
        );
        assert_eq!(parse_display_unit(&json!({})), DisplayUnit::Usd);
        assert_eq!(
            parse_display_unit(&json!({
                "quota_display_type": "CUSTOM",
                "custom_currency_symbol": "€"
            })),
            DisplayUnit::Custom("€".into())
        );
        assert_eq!(
            parse_display_unit(&json!({"quota_display_type": "CUSTOM"})),
            DisplayUnit::Custom("¤".into())
        );
    }

    #[test]
    fn rejects_missing_or_false_structural_signals() {
        let cases = [
            json!({"success": false, "data": {"version": "1", "quota_display_type": "USD"}}),
            json!({"success": "true", "data": {"version": "1", "quota_display_type": "USD"}}),
            json!({"data": {"version": "1", "quota_display_type": "USD"}}),
            json!({"success": true}),
            json!({"success": true, "data": {}}),
            json!({"success": true, "data": []}),
            json!({"success": true, "data": null}),
            json!({"success": true, "data": {"version": ""}}),
            json!({"success": true, "data": {"version": "", "quota_display_type": "USD"}}),
        ];
        for case in cases {
            assert!(
                fingerprint_payload(&case).is_err(),
                "expected reject: {case}"
            );
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

    fn spawn_status_server(
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

    fn ok_body() -> String {
        ok_payload().to_string()
    }

    #[test]
    fn probe_sends_no_authorization_and_accepts_status() {
        let body = ok_body();
        let (origin, join) = spawn_status_server(1, move |_origin, _req| {
            tiny_http::Response::from_string(body.clone()).with_status_code(200)
        });
        tauri::async_runtime::block_on(probe(&origin)).unwrap();
        let captured = join.join().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].url, "/api/status");
        assert_eq!(captured[0].authorization, None);
    }

    #[test]
    fn probe_does_not_follow_redirects() {
        let body = ok_body();
        let (origin, join) = spawn_status_server(2, move |origin, req| {
            let path = req.url().split('?').next().unwrap_or(req.url());
            if path == "/api/status" {
                let loc = format!("{origin}/ok");
                let header = tiny_http::Header::from_bytes(&b"Location"[..], loc.as_bytes())
                    .expect("location header");
                tiny_http::Response::from_string(String::new())
                    .with_status_code(302)
                    .with_header(header)
            } else {
                tiny_http::Response::from_string(body.clone()).with_status_code(200)
            }
        });
        let result = tauri::async_runtime::block_on(probe(&origin));
        assert!(result.is_err(), "redirected status must fail: {result:?}");
        let captured = join.join().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].url, "/api/status");
        assert_eq!(captured[0].authorization, None);
    }

    #[test]
    fn probe_rejects_http_failure() {
        let (origin, join) = spawn_status_server(1, |_origin, _req| {
            tiny_http::Response::from_string("nope").with_status_code(500)
        });
        let err = tauri::async_runtime::block_on(probe(&origin)).unwrap_err();
        assert!(err.contains("HTTP 500"), "{err}");
        let _ = join.join();
    }

    #[test]
    fn probe_http_404_is_fingerprint_mismatch() {
        let (origin, join) = spawn_status_server(1, |_origin, _req| {
            tiny_http::Response::from_string("404 page not found").with_status_code(404)
        });
        let err = tauri::async_runtime::block_on(probe(&origin)).unwrap_err();
        assert_eq!(err, "status fingerprint mismatch");
        let _ = join.join();
    }

    #[test]
    fn probe_rejects_public_http_before_transport() {
        let err = tauri::async_runtime::block_on(probe("http://example.com")).unwrap_err();
        assert_eq!(
            err,
            "plain HTTP is only allowed for private, loopback, or link-local IP addresses"
        );
    }

    #[test]
    fn probe_accepts_cny_live_body() {
        let body = json!({
            "success": true,
            "data": {"version": "1", "quota_display_type": "CNY"}
        })
        .to_string();
        let (origin, join) = spawn_status_server(1, move |_origin, _req| {
            tiny_http::Response::from_string(body.clone()).with_status_code(200)
        });
        let unit = tauri::async_runtime::block_on(probe(&origin)).unwrap();
        assert_eq!(unit, DisplayUnit::Cny);
        let _ = join.join();
    }
}
