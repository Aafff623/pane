use super::{http, Metric, Snapshot};
use serde_json::Value;
use std::path::PathBuf;

const ID: &str = "vertexai";
const NAME: &str = "Vertex AI";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// gcloud Application Default Credentials, Windows edition:
/// %APPDATA%\gcloud\application_default_credentials.json (or the env overrides).
fn adc_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    let base = std::env::var("CLOUDSDK_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::config_dir().unwrap_or_default().join("gcloud"));
    let p = base.join("application_default_credentials.json");
    p.exists().then_some(p)
}

fn gcloud_dir() -> PathBuf {
    std::env::var("CLOUDSDK_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::config_dir().unwrap_or_default().join("gcloud"))
}

fn project_id() -> Option<String> {
    let cfg = gcloud_dir().join("configurations").join("config_default");
    if let Ok(text) = std::fs::read_to_string(cfg) {
        for line in text.lines() {
            if let Some(rest) = line.trim().strip_prefix("project") {
                if let Some(v) = rest.trim().strip_prefix('=') {
                    let v = v.trim();
                    if !v.is_empty() {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }
    ["GOOGLE_CLOUD_PROJECT", "GCLOUD_PROJECT", "CLOUDSDK_CORE_PROJECT"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
}

/// JWT id_token → email claim (base64url middle segment, no verification —
/// we only display it).
fn email_from_id_token(token: &str) -> Option<String> {
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload).ok()?;
    let doc: Value = serde_json::from_slice(&bytes).ok()?;
    doc.get("email").and_then(Value::as_str).map(str::to_string)
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(path) = adc_path() else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Run `gcloud auth application-default login` to connect Vertex AI.",
        ));
    };
    let doc: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .ok_or("could not read gcloud credentials")?;

    let refresh_token = doc.get("refresh_token").and_then(Value::as_str);
    let client_id = doc.get("client_id").and_then(Value::as_str);
    let client_secret = doc.get("client_secret").and_then(Value::as_str);

    // Verify the credential still works by refreshing it (kept in memory
    // only — gcloud's file is never touched, same as the Mac app).
    let email: Option<String>;
    if let (Some(rt), Some(cid), Some(cs)) = (refresh_token, client_id, client_secret) {
        let resp = http()
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", cid),
                ("client_secret", cs),
                ("refresh_token", rt),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| format!("token refresh: {e}"))?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| format!("token parse: {e}"))?;
        if !status.is_success() {
            let err = body.get("error").and_then(Value::as_str).unwrap_or("unknown");
            return Err(match err {
                "invalid_grant" => "gcloud login expired — run `gcloud auth application-default login`".into(),
                other => format!("token refresh failed: {other}"),
            });
        }
        email = body
            .get("id_token")
            .and_then(Value::as_str)
            .and_then(email_from_id_token)
            .or_else(|| doc.get("id_token").and_then(Value::as_str).and_then(email_from_id_token));
    } else {
        // Service-account ADC — presence is all we can show without signing.
        if doc.get("client_email").is_none() {
            return Err("unrecognized gcloud credential format".into());
        }
        email = doc.get("client_email").and_then(Value::as_str).map(str::to_string);
    }

    let mut metrics = Vec::new();
    metrics.push(Metric::text("Account", email.unwrap_or_else(|| "signed in".into())));
    if let Some(project) = project_id() {
        metrics.push(Metric::text("Project", project));
    }
    Ok(Snapshot::ok(ID, NAME, Some("gcloud".into()), metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_token_email_comes_from_the_unverified_jwt_payload() {
        use base64::Engine;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"email":"user@example.com","exp":123}"#);
        let token = format!("eyJhbGciOiJub25lIn0.{payload}.sig");
        assert_eq!(email_from_id_token(&token).as_deref(), Some("user@example.com"));
        // Malformed segments never surface as an account row.
        assert_eq!(email_from_id_token("not-a-jwt"), None);
        assert_eq!(email_from_id_token("h.!!!.s"), None);
    }
}
