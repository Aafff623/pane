use serde_json::Value;

const MAX_STATUS_BYTES: usize = 64 * 1024;

/// Structural OneAPI / NewAPI check. Does not require branding text or a
/// particular quota display unit.
pub fn fingerprint_payload(v: &Value) -> Result<(), String> {
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
    Ok(())
}

pub async fn probe(origin: &str) -> Result<(), String> {
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
    use super::{fingerprint_payload, probe};
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
        fingerprint_payload(&ok_payload()).unwrap();
        fingerprint_payload(&json!({
            "success": true,
            "data": {
                "version": "1.0",
                "system_name": "Totally Custom Panel",
                "quota_display_type": "usd"
            }
        }))
        .unwrap();
        fingerprint_payload(&json!({
            "success": true,
            "data": {
                "system_name": "One API",
                "display_in_currency": true
            }
        }))
        .unwrap();
        fingerprint_payload(&json!({
            "success": true,
            "data": {
                "version": "build",
                "quota_display_type": "USD",
                "display_in_currency": false
            }
        }))
        .unwrap();
        fingerprint_payload(&json!({
            "success": true,
            "data": {"version": "1"}
        }))
        .unwrap();
        fingerprint_payload(&json!({
            "success": true,
            "data": {"version": "1", "quota_display_type": "CNY"}
        }))
        .unwrap();
        fingerprint_payload(&json!({
            "success": true,
            "data": {"version": "1", "quota_display_type": "TOKENS"}
        }))
        .unwrap();
        fingerprint_payload(&json!({
            "success": true,
            "data": {"version": "1", "display_in_currency": false}
        }))
        .unwrap();
        fingerprint_payload(&json!({
            "success": true,
            "data": {
                "version": "",
                "system_name": "国创Token运营平台",
                "quota_display_type": "CNY",
                "display_in_currency": true
            }
        }))
        .unwrap();
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
    fn probe_accepts_cny_live_body() {
        let body = json!({
            "success": true,
            "data": {"version": "1", "quota_display_type": "CNY"}
        })
        .to_string();
        let (origin, join) = spawn_status_server(1, move |_origin, _req| {
            tiny_http::Response::from_string(body.clone()).with_status_code(200)
        });
        tauri::async_runtime::block_on(probe(&origin)).unwrap();
        let _ = join.join();
    }
}
