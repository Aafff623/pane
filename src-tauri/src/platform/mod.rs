//! The platform seam.
//!
//! Everything Pane needs from the host OS that differs between Windows,
//! macOS, and Linux lives behind this module: the credential store, the
//! display language, process/port discovery, screen-share detection, and
//! the WebView memory hint. Callers elsewhere in the crate stay
//! platform-agnostic — no `#[cfg]`, no Win32 imports, no `ps` parsing.
//!
//! Every function degrades instead of failing: a platform that cannot
//! answer returns `None`, an empty list, or `false`, and the caller falls
//! through to its next source. That is what keeps a provider card honest
//! ("not found on this PC") rather than broken on a platform whose
//! credential store Pane cannot read.

#[cfg_attr(windows, path = "windows.rs")]
#[cfg_attr(not(windows), path = "unix.rs")]
mod imp;

use std::collections::BTreeMap;

/// One running process, with the full command line callers parse flags out
/// of (Antigravity's language server publishes its port and CSRF token
/// there).
pub struct ProcessInfo {
    pub pid: u32,
    pub command_line: String,
}

/// Reads a secret out of the OS credential store — Windows Credential
/// Manager, the macOS keychain, or the freedesktop Secret Service.
///
/// `target` is the service name Go's `keyring` library writes under, which
/// is what every CLI Pane reads through here (gh, Antigravity) uses on all
/// three platforms, so one target string works everywhere.
pub fn secret(target: &str) -> Option<String> {
    decode_secret(&imp::secret_blob(target)?)
}

/// Credential blob → text: UTF-8 or UTF-16 LE, unwrapping go-keyring's
/// `go-keyring-base64:` prefix (used by Go CLIs like gh and Antigravity).
fn decode_secret(blob: &[u8]) -> Option<String> {
    let utf8 = String::from_utf8(blob.to_vec()).ok();
    // Credential Manager often stores generic secrets as UTF-16 LE. Those
    // blobs are also valid UTF-8 (NUL every other byte), so a UTF-8-first
    // parse would keep the interior NULs ("t\0o\0k") instead of "tok".
    let looks_utf16 = blob.len() >= 2
        && blob.len() % 2 == 0
        && blob.chunks_exact(2).any(|c| c[1] == 0);
    let text = if looks_utf16 {
        utf16_le(blob).or(utf8)?
    } else {
        utf8.or_else(|| utf16_le(blob))?
    };
    let text = text.trim().trim_matches('\0').to_string();
    if let Some(b64) = text.strip_prefix("go-keyring-base64:") {
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .ok()?;
        return String::from_utf8(decoded).ok();
    }
    Some(text)
}

fn utf16_le(blob: &[u8]) -> Option<String> {
    if blob.len() % 2 != 0 {
        return None;
    }
    let units: Vec<u16> = blob
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16(&units).ok()
}

/// The OS *display* language reduced to a locale Pane ships strings for.
/// Not the regional-format locale — a US-formatted machine with a Chinese
/// UI should read Chinese.
pub fn system_ui_language() -> &'static str {
    imp::system_ui_language()
}

/// Maps an OS language tag onto one of Pane's locales. Handles every shape
/// the three platforms hand out: `zh_CN.UTF-8` (POSIX `LANG`),
/// `zh-Hans-CN` (macOS `AppleLanguages`), and bare `ru`.
#[cfg_attr(windows, allow(dead_code))] // unix.rs is the only non-test caller
fn language_tag_to_locale(tag: &str) -> &'static str {
    let primary = tag
        .trim()
        .trim_matches('"')
        .split(['.', '@'])
        .next()
        .unwrap_or("")
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match primary.as_str() {
        "zh" => "zh",
        "ru" => "ru",
        _ => "en",
    }
}

/// True while the user is presenting or being remote-controlled, so the
/// tray strip can hide dollar amounts and percentages from the audience.
/// Platforms without a public signal for this report `false` — the numbers
/// stay visible, which is the pre-existing behavior everywhere but Windows.
pub fn screen_is_being_shared() -> bool {
    imp::screen_is_being_shared()
}

/// Running processes whose executable name starts with one of `name_prefixes`.
/// Blocking (it shells out); callers run it on a blocking thread.
pub fn list_processes(name_prefixes: &[&str]) -> Vec<ProcessInfo> {
    imp::list_processes(name_prefixes)
}

/// pid → loopback TCP ports it is listening on. Queried once per scan and
/// shared across processes, since the underlying tool call is the expensive
/// part. Blocking, like `list_processes`.
pub fn listening_loopback_ports() -> BTreeMap<u32, Vec<u16>> {
    imp::listening_loopback_ports()
}

/// Asks the WebView to release memory while the popover is hidden. Only
/// WebView2 exposes this; elsewhere it is a no-op and the platform's own
/// memory management applies.
pub fn set_webview_memory_level(window: &tauri::WebviewWindow, low: bool) {
    imp::set_webview_memory_level(window, low);
}

/// Drops the Windows 11 focus hairline that DWM paints around a
/// frameless window. No-op on macOS/Linux — they don't draw that stroke.
pub fn hide_window_border(window: &tauri::WebviewWindow) {
    imp::hide_window_border(window);
}

/// Per-user config / roaming directory (`%APPDATA%` on Windows,
/// `~/Library/Application Support` on macOS, `~/.config` on Linux).
pub fn config_home() -> Option<std::path::PathBuf> {
    dirs::config_dir()
}

/// Per-user local data directory (`%LOCALAPPDATA%` on Windows, the same
/// Application Support folder on macOS, `~/.local/share` on Linux).
pub fn data_local_home() -> Option<std::path::PathBuf> {
    dirs::data_local_dir()
}

/// GitHub CLI `hosts.yml` locations — the Windows app writes
/// `<config>/GitHub CLI/hosts.yml`; the Unix `gh` binary writes
/// `<config>/gh/hosts.yml`.
pub fn github_cli_hosts() -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Some(cfg) = config_home() {
        out.push(cfg.join("GitHub CLI").join("hosts.yml"));
        out.push(cfg.join("gh").join("hosts.yml"));
    }
    out
}

/// Runs a console command without flashing a window, capturing stdout.
/// The window-suppression flag is Windows-only; on Unix this is a plain
/// child process.
pub(crate) fn run_hidden(program: &str, args: &[&str]) -> Option<String> {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_plain_utf8() {
        assert_eq!(decode_secret(b"gho_token").unwrap(), "gho_token");
    }

    #[test]
    fn decodes_utf16le_and_strips_nul() {
        let blob: Vec<u8> = "tok\0".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(decode_secret(&blob).unwrap(), "tok");
    }

    #[test]
    fn unwraps_go_keyring_base64() {
        // gh and Antigravity both store through Go's keyring wrapper.
        let blob = b"go-keyring-base64:Z2hvX3Rva2Vu";
        assert_eq!(decode_secret(blob).unwrap(), "gho_token");
    }

    #[test]
    fn language_tags_from_every_platform() {
        assert_eq!(language_tag_to_locale("zh_CN.UTF-8"), "zh"); // POSIX LANG
        assert_eq!(language_tag_to_locale("zh-Hans-CN"), "zh"); // macOS AppleLanguages
        assert_eq!(language_tag_to_locale("\"zh-Hant\""), "zh"); // defaults(1) quoting
        assert_eq!(language_tag_to_locale("ru_RU.UTF-8"), "ru");
        assert_eq!(language_tag_to_locale("ru"), "ru");
        assert_eq!(language_tag_to_locale("en_US.UTF-8"), "en");
        assert_eq!(language_tag_to_locale("C"), "en");
        assert_eq!(language_tag_to_locale(""), "en");
    }
}
