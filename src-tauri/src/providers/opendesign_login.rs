//! OpenDesign (Vela) CLI device-code login.
//!
//! Deliberately NOT in `oauth.rs`: this is not RFC 8628. The vendor invents its
//! own request body (`cliVersion`/`host`/`profile`), returns a `deviceSecret`
//! instead of a device code, and the token response carries `controlKey`
//! rather than `access_token`. Folding it into the shared RFC flow would add
//! per-vendor branches to three providers that do not need them.
//!
//! Verified live 2026-11-24:
//! - `POST /cli/device-authorizations` → 201 with `pollIntervalSeconds: 2`
//!   and a 10-minute `expiresAt`.
//! - `POST …/{deviceId}/token` → `{"status":"pending"}` until the browser
//!   approves, then `approved` with `controlKey` + `runtimeKey`.
//! - The activation page may display "登录中…" while the browser session
//!   settles; the poll flips on its own, so polling must outlast the page.

use serde_json::{json, Value};
use std::time::Duration;

const API_BASE: &str = "https://amr-api.open-design.ai/api/v1";
const CLI_VERSION: &str = "0.0.33";
const MAX_BODY_BYTES: usize = 64 * 1024;
const TOKEN_TARGET: &str = "Pane:opendesign:control-key";
/// The vendor's own code lives 10 minutes; give the reader a little of our own
/// slack before declaring the attempt dead.
const LOGIN_TIMEOUT_SECS: u64 = 660;

pub struct DeviceCode {
    pub device_id: String,
    pub user_code: String,
    pub device_secret: String,
    pub activation_url: String,
    pub interval: Duration,
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "pane".into())
}

pub async fn start() -> Result<DeviceCode, String> {
    let body = json!({
        "cliVersion": CLI_VERSION,
        "host": hostname(),
        "profile": "prod",
    });
    let resp = super::send_with_direct_fallback(|http| {
        http.post(format!("{API_BASE}/cli/device-authorizations"))
            .header("accept", "application/json")
            .json(&body)
            .timeout(Duration::from_secs(15))
    })
    .await
    .map_err(|e| format!("OpenDesign 设备码请求失败：{e}"))?;

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("OpenDesign 设备码：HTTP {status}"));
    }
    let doc = super::json_body(resp, MAX_BODY_BYTES, "OpenDesign 设备码").await?;
    let text = |key: &str| {
        doc.get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let interval = doc
        .get("pollIntervalSeconds")
        .and_then(Value::as_u64)
        .filter(|s| *s >= 1)
        .unwrap_or(2);

    Ok(DeviceCode {
        device_id: text("deviceId").ok_or("OpenDesign 设备码响应缺少 deviceId")?,
        user_code: text("userCode").ok_or("OpenDesign 设备码响应缺少 userCode")?,
        device_secret: text("deviceSecret").ok_or("OpenDesign 设备码响应缺少 deviceSecret")?,
        activation_url: text("activationUrl").ok_or("OpenDesign 设备码响应缺少 activationUrl")?,
        interval: Duration::from_secs(interval),
    })
}

enum PollState {
    Pending,
    Approved(String),
}

async fn poll(device: &DeviceCode) -> Result<PollState, String> {
    let body = json!({ "deviceSecret": device.device_secret });
    let resp = super::send_with_direct_fallback(|http| {
        http.post(format!(
            "{API_BASE}/cli/device-authorizations/{}/token",
            device.device_id
        ))
        .header("accept", "application/json")
        .json(&body)
        .timeout(Duration::from_secs(15))
    })
    .await
    .map_err(|e| format!("OpenDesign 授权轮询失败：{e}"))?;

    let status = resp.status().as_u16();
    if status == 410 || status == 404 {
        return Err("OpenDesign 授权码已过期，请重新登录。".into());
    }
    if !(200..300).contains(&status) {
        return Err(format!("OpenDesign 授权轮询：HTTP {status}"));
    }
    let doc = super::json_body(resp, MAX_BODY_BYTES, "OpenDesign 授权轮询").await?;
    match doc.get("status").and_then(Value::as_str) {
        Some("approved") => doc
            .get("controlKey")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(|k| PollState::Approved(k.to_string()))
            .ok_or_else(|| "OpenDesign 已批准但没有返回控制面凭据".to_string()),
        Some("pending") => Ok(PollState::Pending),
        Some("denied") => Err("OpenDesign 授权被拒绝，请重新登录。".into()),
        Some("expired") => Err("OpenDesign 授权码已过期，请重新登录。".into()),
        Some(other) => Err(format!("OpenDesign 授权状态异常：{other}")),
        None => Err("OpenDesign 授权响应缺少 status".into()),
    }
}

/// Opens the vendor's activation page in the user's own browser and waits for
/// approval, then stores the control key. The approval happens on the vendor's
/// side, so nothing from the page is scraped; Pane only watches the device-code
/// poll. An embedded WebView is deliberately NOT used: the page is a
/// client-rendered SPA that paints nothing in one (verified 2026-11-25 in a
/// plain Chrome window too, so the page is at fault, not the container).
#[cfg(not(feature = "harness"))]
pub async fn run_approval_window(app: &tauri::AppHandle, device: DeviceCode) -> Result<(), String> {
    static LOGIN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = LOGIN_LOCK
        .try_lock()
        .map_err(|_| "OpenDesign 登录已在进行中".to_string())?;

    // The activation page is a client-rendered SPA that paints nothing in an
    // embedded WebView2 (verified 2026-11-25: a plain Chrome window renders the
    // same blank surface, so it is the page, not our container). Hand the URL
    // to the user's own browser instead — it usually already holds the console
    // session, which is what the approval step needs — and let the poll below
    // decide when the approval landed.
    let url = tauri::Url::parse(&device.activation_url)
        .map_err(|e| format!("OpenDesign 激活地址无效：{e}"))?;
    {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|e| format!("OpenDesign 无法打开浏览器：{e}"))?;
    }

    // Nothing to close afterwards: the page lives in the user's browser, whose
    // lifetime we do not own. The device code itself expires on the vendor's
    // clock, so a forgotten tab cannot hold us open longer than that.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(LOGIN_TIMEOUT_SECS);
    loop {
        tokio::time::sleep(device.interval).await;
        if tokio::time::Instant::now() >= deadline {
            return Err("OpenDesign 授权超时，请重新登录。".to_string());
        }
        match poll(&device).await {
            Ok(PollState::Approved(key)) => {
                crate::secretstore::put(TOKEN_TARGET, &key)?;
                return Ok(());
            }
            Ok(PollState::Pending) => continue,
            Err(error) => return Err(error),
        }
    }
}

#[cfg(feature = "harness")]
pub async fn run_approval_window(_app: &tauri::AppHandle, _device: DeviceCode) -> Result<(), String> {
    Err("OpenDesign 登录在测试环境中不可用".into())
}
