//! Qoder — the international edition of the Qoder IDE (qoder.com; the
//! China edition has its own card, `qodercn`).
//!
//! Same Chromium os_crypt storage as the CN edition: sign-in token in
//! `%APPDATA%\com.qoder.app.stable\auth.v1.dat`, AES-GCM key in the sibling
//! `Local State` (see `qodercn.rs` for the crypto). Credits come from
//! openapi.qoder.sh — each site rejects the other's tokens, which is why
//! the editions are separate cards.

use super::qodercn;
use super::Snapshot;
use std::path::{Path, PathBuf};

const ID: &str = "qoder";
const NAME: &str = "Qoder";
/// The international app's config dirs, in priority order. The first
/// mirrors the CN naming (`com.qodercn.app.stable`); the second covers a
/// build that ships without the reverse-DNS prefix. 待确认：真实安装目录
/// 名以用户装机实测为准（CONTEXT.md 有记录）。
const APP_DIRS: [&str; 2] = ["com.qoder.app.stable", "Qoder"];
const OPENAPI_BASE: &str = "https://openapi.qoder.sh";
const PLAN_PATH: &str = "/api/v2/user/plan";
const USAGE_PATH: &str = "/api/v2/quota/usage";

pub async fn snapshot() -> Snapshot {
    match fetch().await {
        Ok(s) => s,
        Err(e) => Snapshot::error(ID, NAME, e),
    }
}

/// Pure local probe for the Customize gear panel (no network): the Qoder
/// app's sign-in blob exists on this machine.
pub fn local_credential_hint() -> Option<String> {
    auth_file_path().map(|_| "Qoder app sign-in".to_string())
}

async fn fetch() -> Result<Snapshot, String> {
    let Some(auth_path) = auth_file_path() else {
        return Ok(Snapshot::no_credentials(
            ID,
            NAME,
            "Qoder sign-in not found. Open Qoder and sign in once, then refresh.",
        ));
    };
    let token = qodercn::load_token(&auth_path)?;
    let (plan, usage) = tokio::join!(
        qodercn::fetch_api(OPENAPI_BASE, &token, PLAN_PATH, "plan"),
        qodercn::fetch_api(OPENAPI_BASE, &token, USAGE_PATH, "usage")
    );
    let usage = usage?;
    let (plan, metrics) = qodercn::credit_metrics(plan.as_ref().ok(), &usage)?;
    Ok(Snapshot::ok(ID, NAME, plan, metrics))
}

fn auth_file_path() -> Option<PathBuf> {
    let cfg = dirs::config_dir()?;
    auth_file_candidates(&cfg).into_iter().find(|p| p.is_file())
}

/// Config dir → auth.v1.dat candidates in priority order (pure, testable).
fn auth_file_candidates(config_dir: &Path) -> Vec<PathBuf> {
    APP_DIRS.iter().map(|dir| config_dir.join(dir).join("auth.v1.dat")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_candidates_prefer_the_reverse_dns_dir() {
        let base = Path::new("C:/Users/x/AppData/Roaming");
        let candidates = auth_file_candidates(base);
        assert_eq!(candidates.len(), 2);
        let first = candidates[0].to_string_lossy();
        let second = candidates[1].to_string_lossy();
        assert!(first.contains("com.qoder.app.stable"), "first = {first}");
        assert!(first.ends_with("auth.v1.dat") || first.ends_with("auth.v1.dat\\"));
        assert!(second.contains("Qoder"), "second = {second}");
        // The CN directory must never leak into the international card.
        assert!(!first.contains("qodercn") && !second.contains("qodercn"));
    }
}
