//! Windows implementation of the platform seam. See `platform/mod.rs` for
//! the contract each function has to honor.

use std::collections::BTreeMap;

use super::{run_hidden, ProcessInfo};

/// Reads a generic credential's blob from Windows Credential Manager.
pub fn secret_blob(target: &str) -> Option<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };
    let wide: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    let mut pcred: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        if CredReadW(PCWSTR(wide.as_ptr()), CRED_TYPE_GENERIC, None, &mut pcred).is_err() {
            return None;
        }
        let cred = &*pcred;
        let blob =
            std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize)
                .to_vec();
        CredFree(pcred as *mut std::ffi::c_void);
        Some(blob)
    }
}

/// Windows DPAPI per-user decryption (`CryptUnprotectData`). Chromium's
/// `os_crypt` (and therefore Qoder CN's `auth.v1.dat` master key) hands out
/// blobs encrypted this way under the current user.
pub fn dpapi_unprotect(blob: &[u8]) -> Option<Vec<u8>> {
    use windows::Win32::Security::Cryptography::CryptUnprotectData;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB;
    let mut in_blob = CRYPT_INTEGER_BLOB {
        cbData: blob.len() as u32,
        pbData: blob.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB::default();
    unsafe {
        if CryptUnprotectData(
            &mut in_blob,
            None,
            None,
            None,
            None,
            0, // no UI prompt — batch decryption must stay silent
            &mut out_blob,
        )
        .is_err()
        {
            return None;
        }
        let plain =
            std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize).to_vec();
        LocalFree(Some(HLOCAL(out_blob.pbData as *mut _)));
        Some(plain)
    }
}

/// Primary language 0x04 = Chinese (zh-CN, zh-TW, zh-HK, …).
fn langid_is_zh(langid: u16) -> bool {
    const LANG_CHINESE: u16 = 0x04;
    langid & 0x03FF == LANG_CHINESE
}

/// Primary language 0x19 = Russian (ru-RU, ru-MD, …).
fn langid_is_ru(langid: u16) -> bool {
    const LANG_RUSSIAN: u16 = 0x19;
    langid & 0x03FF == LANG_RUSSIAN
}

/// The Windows *display* language, not the regional-format locale.
pub fn system_ui_language() -> &'static str {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    let langid = unsafe { GetUserDefaultUILanguage() };
    if langid_is_zh(langid) {
        "zh"
    } else if langid_is_ru(langid) {
        "ru"
    } else {
        "en"
    }
}

pub fn screen_is_being_shared() -> bool {
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTECONTROL};

    // Someone is remotely controlling this session (Quick Assist, etc.).
    if unsafe { GetSystemMetrics(SM_REMOTECONTROL) } != 0 {
        return true;
    }
    if let Ok(state) = unsafe { SHQueryUserNotificationState() } {
        // Presentation Settings / exclusive fullscreen — the closest
        // public Windows equivalent of macOS's screen-watcher flag.
        // QUNS_BUSY is skipped: a fullscreen YouTube tab would hide
        // numbers all evening.
        if state == QUNS_PRESENTATION_MODE || state == QUNS_RUNNING_D3D_FULL_SCREEN {
            return true;
        }
    }
    false
}

pub fn list_processes(name_prefixes: &[&str]) -> Vec<ProcessInfo> {
    // The prefixes are compiled-in constants, but they land inside a
    // PowerShell regex literal — keep anything that could close the quote
    // or alter the pattern out of it.
    let safe: Vec<String> = name_prefixes
        .iter()
        .filter(|p| p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .map(|p| (*p).to_string())
        .collect();
    if safe.is_empty() {
        return Vec::new();
    }
    let script = format!(
        "Get-CimInstance Win32_Process | Where-Object {{ $_.Name -match '^({})' }} \
         | Select-Object ProcessId, Name, CommandLine | ConvertTo-Json -Compress",
        safe.join("|")
    );
    let raw = match run_hidden(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", &script],
    ) {
        Some(r) if !r.trim().is_empty() => r,
        _ => return Vec::new(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(raw.trim()) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    // ConvertTo-Json collapses a one-element result to a bare object.
    let procs = match parsed {
        serde_json::Value::Array(a) => a,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => Vec::new(),
    };
    procs
        .iter()
        .filter_map(|p| {
            let pid = p.get("ProcessId").and_then(serde_json::Value::as_u64)? as u32;
            let command_line = p.get("CommandLine").and_then(serde_json::Value::as_str)?;
            if pid == 0 || command_line.is_empty() {
                return None;
            }
            Some(ProcessInfo { pid, command_line: command_line.to_string() })
        })
        .collect()
}

pub fn listening_loopback_ports() -> BTreeMap<u32, Vec<u16>> {
    let raw = run_hidden("netstat", &["-ano", "-p", "TCP"]).unwrap_or_default();
    let mut map: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    for line in raw.lines() {
        if !line.contains("LISTENING") {
            continue;
        }
        let mut cols = line.split_whitespace();
        let (_proto, local) = (cols.next(), cols.next());
        let Some(local) = local else { continue };
        let Some(pid) = cols.last().and_then(|p| p.parse::<u32>().ok()) else { continue };
        let Some((addr, port)) = local.rsplit_once(':') else { continue };
        if addr != "127.0.0.1" && addr != "0.0.0.0" {
            continue;
        }
        let Ok(port) = port.parse::<u16>() else { continue };
        map.entry(pid).or_default().push(port);
    }
    for ports in map.values_mut() {
        ports.sort_unstable();
        ports.dedup();
    }
    map
}

/// Tells WebView2 to release memory while the popover is hidden and return
/// to normal when it shows. Tauri doesn't expose wry's setter for this, so
/// we make the same COM calls wry does (SetMemoryUsageTargetLevel).
pub fn set_webview_memory_level(window: &tauri::WebviewWindow, low: bool) {
    // The parse-tests harness compiles this file against a Tauri stub and
    // has no WebView2 headers — the COM call is the real app's concern.
    #[cfg(not(feature = "harness"))]
    {
        let _ = window.with_webview(move |webview| unsafe {
            use webview2_com::Microsoft::Web::WebView2::Win32::{
                ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL,
            };
            use windows_core::Interface;
            if let Ok(core) = webview.controller().CoreWebView2() {
                if let Ok(wv19) = core.cast::<ICoreWebView2_19>() {
                    let level = COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL(if low { 1 } else { 0 });
                    let _ = wv19.SetMemoryUsageTargetLevel(level);
                }
            }
        });
    }
    #[cfg(feature = "harness")]
    {
        let _ = (window, low);
    }
}

/// Windows 11 draws a light focus stroke around a frameless HWND
/// (`DWMWA_BORDER_COLOR` default). On this dark popover it reads as a
/// thick white card frame. COLOR_NONE removes the stroke; the drop
/// shadow stays. Caption color is pinned to the zinc background so any
/// leftover DWM chrome blends into the card instead of flashing white.
pub fn hide_window_border(window: &tauri::WebviewWindow) {
    #[cfg(not(feature = "harness"))]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR,
            DWMWA_COLOR_NONE, DWMWA_USE_IMMERSIVE_DARK_MODE,
        };
        let Ok(hwnd) = window.hwnd() else { return };
        let hwnd = HWND(hwnd.0 as *mut core::ffi::c_void);
        // COLORREF 0x00BBGGRR of --background (#09090b).
        const ZINC_BG: u32 = 0x000B_0909;
        let none: u32 = DWMWA_COLOR_NONE;
        let dark: i32 = 1;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark as *const i32 as *const core::ffi::c_void,
                std::mem::size_of::<i32>() as u32,
            );
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                &none as *const u32 as *const core::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_CAPTION_COLOR,
                &ZINC_BG as *const u32 as *const core::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
    #[cfg(feature = "harness")]
    {
        let _ = window;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_langids_match() {
        assert!(langid_is_zh(0x0804)); // zh-CN
        assert!(langid_is_zh(0x0404)); // zh-TW
        assert!(langid_is_zh(0x0C04)); // zh-HK
        assert!(!langid_is_zh(0x0409)); // en-US
        assert!(!langid_is_zh(0x0411)); // ja
        assert!(!langid_is_zh(0x0419)); // ru-RU
    }

    #[test]
    fn russian_langids_match() {
        assert!(langid_is_ru(0x0419)); // ru-RU
        assert!(langid_is_ru(0x0819)); // ru-MD
        assert!(!langid_is_ru(0x0409)); // en-US
        assert!(!langid_is_ru(0x0804)); // zh-CN
    }

    #[test]
    fn process_name_prefixes_cannot_break_out_of_the_regex() {
        // A prefix carrying a quote would close the PowerShell string; the
        // filter drops it rather than building the command anyway.
        assert!(list_processes(&["'; Remove-Item C:\\ #"]).is_empty());
    }
}
