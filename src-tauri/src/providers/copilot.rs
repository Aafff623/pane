use super::{http, Metric, Snapshot};
use serde_json::Value;
use std::path::PathBuf;

const ID: &str = "copilot";
const NAME: &str = "Copilot";

/// GitHub tokens can come from Copilot's editor config or the GitHub CLI.
/// Every source is scoped to github.com: this snapshot only ever talks to
/// api.github.com, so a token owned by any other host (a GitHub Enterprise
/// login sharing the same file) must never be selected.
fn find_token() -> Option<String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(&local).join("github-copilot").join("apps.json"));
        candidates.push(PathBuf::from(&local).join("github-copilot").join("hosts.json"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".config").join("github-copilot").join("apps.json"));
        candidates.push(home.join(".config").join("github-copilot").join("hosts.json"));
    }
    for path in candidates {
        let Ok(raw) = std::fs::read_to_string(&path) else { continue };
        if let Some(tok) = copilot_json_token(&raw) {
            return Some(tok);
        }
    }
    // GitHub CLI. Older versions kept oauth_token in hosts.yml; modern gh
    // (which the new Copilot CLI piggybacks on) stores it in Windows
    // Credential Manager under gh:github.com[:username].
    let mut usernames: Vec<String> = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        let hosts = PathBuf::from(appdata).join("GitHub CLI").join("hosts.yml");
        if let Ok(raw) = std::fs::read_to_string(&hosts) {
            if let Some(tok) = hosts_yml_token(&raw, &mut usernames) {
                return Some(tok);
            }
        }
    }
    let mut targets: Vec<String> =
        usernames.iter().map(|u| format!("gh:github.com:{u}")).collect();
    targets.push("gh:github.com:".into());
    targets.push("gh:github.com".into());
    for target in targets {
        if let Some(token) = super::credential_string(&target) {
            let token = token.trim().to_string();
            if !token.is_empty() {
                return Some(token);
            }
        }
    }
    None
}

/// Pure local probe for the Customize gear panel (existence only, no
/// parsing, no network): the same sources find_token reads.
pub fn local_credential_hint() -> Option<String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(&local).join("github-copilot").join("apps.json"));
        candidates.push(PathBuf::from(&local).join("github-copilot").join("hosts.json"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".config").join("github-copilot").join("apps.json"));
        candidates.push(home.join(".config").join("github-copilot").join("hosts.json"));
    }
    if candidates.iter().any(|p| p.exists()) {
        return Some("GitHub Copilot sign-in (editor config)".into());
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        if PathBuf::from(appdata).join("GitHub CLI").join("hosts.yml").exists() {
            return Some("GitHub CLI sign-in (gh)".into());
        }
    }
    if super::credential_string("gh:github.com").is_some() {
        return Some("GitHub CLI sign-in (Credential Manager)".into());
    }
    None
}

/// The github.com entry's oauth_token from a Copilot apps.json/hosts.json.
/// apps.json keys carry a client-id suffix ("github.com:Iv1.…"); hosts.json
/// keys are bare hostnames. Entries under any other host are ignored.
fn copilot_json_token(raw: &str) -> Option<String> {
    let doc = serde_json::from_str::<Value>(raw).ok()?;
    for (host, entry) in doc.as_object()? {
        if host != "github.com" && !host.starts_with("github.com:") {
            continue;
        }
        if let Some(tok) = entry.get("oauth_token").and_then(Value::as_str) {
            if !tok.is_empty() {
                return Some(tok.to_string());
            }
        }
    }
    None
}

/// The github.com section's oauth_token from the gh CLI's hosts.yml, plus
/// its usernames for the Credential Manager lookup. Lines under any other
/// top-level host section are skipped entirely.
fn hosts_yml_token(raw: &str, usernames: &mut Vec<String>) -> Option<String> {
    let mut in_github = false;
    let mut in_users = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 {
            in_github = trimmed == "github.com:";
            in_users = false;
            continue;
        }
        if !in_github {
            continue;
        }
        if let Some(tok) = trimmed.strip_prefix("oauth_token:") {
            let tok = tok.trim();
            if !tok.is_empty() {
                return Some(tok.to_string());
            }
        }
        if trimmed == "users:" {
            in_users = true;
        } else if in_users {
            if indent >= 8 && trimmed.ends_with(':') {
                usernames.push(trimmed.trim_end_matches(':').to_string());
            } else if indent <= 4 {
                in_users = false;
            }
        }
    }
    None
}

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

async fn fetch() -> Result<Snapshot, String> {
    // Pane's own OAuth login (oauth.rs) takes precedence over the editor's
    // or the CLI's — the user explicitly signed in here, and "Is there a
    // local login?" must not shadow it. The stored GitHub OAuth token is
    // what `copilot_internal/user` accepts (the Copilot token is only for
    // the `copilot_internal/v2/token` endpoints).
    let token = match crate::oauth::github_oauth_token() {
        Ok(Some(tok)) => tok,
        Ok(None) => {
            let Some(token) = find_token() else {
                return Ok(Snapshot::no_credentials(
                    ID,
                    NAME,
                    "No GitHub sign-in found (Copilot in an editor, or `gh auth login`).",
                ));
            };
            token
        }
        Err(e) => return Err(e),
    };

    let resp = http()
        .get("https://api.github.com/copilot_internal/user")
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/json")
        .header("Editor-Version", "vscode/1.101.0")
        .header("Editor-Plugin-Version", "copilot-chat/0.27.0")
        .header("X-GitHub-Api-Version", "2025-04-01")
        .send()
        .await
        .map_err(|e| format!("usage request: {e}"))?;
    if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 {
        return Err("GitHub token was rejected — sign in to Copilot again".into());
    }
    if !resp.status().is_success() {
        return Err(format!("usage endpoint: HTTP {}", resp.status()));
    }
    let user: Value = resp.json().await.map_err(|e| format!("usage parse: {e}"))?;

    let plan = user
        .get("copilot_plan")
        .and_then(Value::as_str)
        .map(str::to_string);
    // Monthly quotas with a known reset date, e.g. "2026-08-01".
    let resets_at = user
        .get("quota_reset_date")
        .and_then(Value::as_str)
        .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp_millis());

    let mut metrics = Vec::new();
    if let Some(snapshots) = user.get("quota_snapshots") {
        push_quota(&mut metrics, snapshots.get("premium_interactions"), "Credits", resets_at);
        push_quota(&mut metrics, snapshots.get("chat"), "Chat", resets_at);
        push_quota(&mut metrics, snapshots.get("completions"), "Completions", resets_at);
    }
    if metrics.is_empty() {
        return Err("no quota data in response (plan may not expose quotas)".into());
    }
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

#[cfg(test)]
mod tests {
    use super::{copilot_json_token, hosts_yml_token};

    #[test]
    fn copilot_json_is_host_scoped() {
        // An enterprise entry listed first must not win over github.com.
        let raw = r#"{
            "github.enterprise.example:Iv1.aaa": {"oauth_token": "enterprise-secret"},
            "github.com:Iv1.bbb": {"oauth_token": "dotcom-secret"}
        }"#;
        assert_eq!(copilot_json_token(raw), Some("dotcom-secret".into()));
        // hosts.json style: bare hostname keys.
        let raw = r#"{"ghe.internal": {"oauth_token": "x"}, "github.com": {"oauth_token": "y"}}"#;
        assert_eq!(copilot_json_token(raw), Some("y".into()));
        // Enterprise-only file yields nothing rather than the wrong token.
        let raw = r#"{"ghe.internal:Iv1.ccc": {"oauth_token": "enterprise-secret"}}"#;
        assert_eq!(copilot_json_token(raw), None);
        // A hostname merely *containing* github.com must not match.
        let raw = r#"{"github.com.evil.example": {"oauth_token": "z"}}"#;
        assert_eq!(copilot_json_token(raw), None);
    }

    #[test]
    fn hosts_yml_is_host_scoped() {
        let raw = "github.enterprise.example:\n    oauth_token: enterprise-secret\n    user: boss\ngithub.com:\n    oauth_token: dotcom-secret\n    user: me\n";
        let mut users = Vec::new();
        assert_eq!(hosts_yml_token(raw, &mut users), Some("dotcom-secret".into()));

        // Enterprise-only file: no token, no usernames.
        let raw = "github.enterprise.example:\n    users:\n        boss:\n            oauth_token: enterprise-secret\n";
        let mut users = Vec::new();
        assert_eq!(hosts_yml_token(raw, &mut users), None);
        assert!(users.is_empty());

        // Modern gh layout: usernames collected from github.com only.
        let raw = "ghe.internal:\n    users:\n        boss:\ngithub.com:\n    users:\n        me:\n        alt:\n    git_protocol: https\n";
        let mut users = Vec::new();
        assert_eq!(hosts_yml_token(raw, &mut users), None);
        assert_eq!(users, vec!["me".to_string(), "alt".to_string()]);
    }

    /// Live probe with this machine's real GitHub login — run manually via
    /// `cargo test --lib copilot -- --ignored --nocapture`. Prints statuses
    /// and counts only, never token values.
    #[test]
    #[ignore]
    fn live_probe() {
        let snap = tauri::async_runtime::block_on(super::snapshot());
        eprintln!(
            "copilot: status={} plan={:?} error={:?} metrics={}",
            snap.status,
            snap.plan,
            snap.error,
            snap.metrics.len()
        );
        for m in &snap.metrics {
            eprintln!("  {}: used={:?} value={:?}", m.label, m.used_percent, m.value);
        }
    }
}

fn push_quota(metrics: &mut Vec<Metric>, node: Option<&Value>, label: &str, resets_at: Option<i64>) {
    const MONTH_MS: i64 = 30 * 86_400_000;
    let Some(node) = node else { return };
    if node.get("unlimited").and_then(Value::as_bool) == Some(true) {
        metrics.push(Metric::text(label, "Unlimited".into()));
        return;
    }
    let Some(percent_remaining) = node.get("percent_remaining").and_then(Value::as_f64) else {
        return;
    };
    let detail = (|| {
        let remaining = node.get("remaining").and_then(Value::as_f64)?;
        let entitlement = node.get("entitlement").and_then(Value::as_f64)?;
        Some(format!("{remaining:.0} of {entitlement:.0} left"))
    })();
    metrics.push(
        Metric::progress(label, 100.0 - percent_remaining, detail)
            .with_reset(resets_at, Some(MONTH_MS)),
    );
}
