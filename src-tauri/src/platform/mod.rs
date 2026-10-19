//! The platform seam.
//!
//! Everything Pane needs from the host OS that differs between Windows,
//! macOS, and Linux lives behind this module: the credential store, the
//! display language, process/port discovery, screen-share detection, and
//! the WebView memory hint. Callers elsewhere in the crate stay
//! platform-agnostic — no `#[cfg]`, no Win32 imports, no `ps` parsing.
//!
//! Discovery functions degrade: a platform that cannot answer returns
//! `None`, an empty list, or `false`, and the caller tries its next source.
//! Private-file and randomness operations return errors rather than
//! silently dropping security guarantees. That keeps a provider card honest
//! ("not found on this PC") rather than broken on a platform whose
//! credential store Pane cannot read.

#[cfg_attr(windows, path = "windows.rs")]
#[cfg_attr(not(windows), path = "unix.rs")]
mod imp;

use std::collections::BTreeMap;
use std::path::Path;

#[cfg(all(windows, test))]
pub(crate) use imp::{current_user_sid, dacl_sddl, set_protected_dacl};

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

/// Windows DPAPI per-user decryption (`CryptUnprotectData`), as used by
/// Chromium's `os_crypt` master key inside `Local State`. Non-Windows
/// platforms have no DPAPI — `None` and the caller falls through.
pub fn dpapi_unprotect(blob: &[u8]) -> Option<Vec<u8>> {
    imp::dpapi_unprotect(blob)
}

/// Seconds-resolution unix timestamp of the current OS boot. Stable for the
/// whole boot session, so the startup animation can tell "already played
/// this boot" from "first launch after this boot". `None` where the OS does
/// not answer — the caller then plays once per app launch instead.
pub fn boot_id() -> Option<u64> {
    imp::boot_id()
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

/// Cookies a webview holds for `url`, read through the browser's own cookie
/// manager. Empty on platforms that can't read them (the caller then has no
/// web-session credential to work with).
pub async fn read_webview_cookies(
    window: &tauri::WebviewWindow,
    url: &str,
) -> Vec<(String, String)> {
    imp::read_webview_cookies(window, url).await
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
    hide_window(&mut cmd);
    let out = cmd.output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The async twin of [`run_hidden`], for callers inside a tokio task (the
/// provider probes). Same obligation: every console child must go through
/// one of these two, never a bare `Command::new`.
///
/// This matters because `pane.exe` is a GUI-subsystem binary in release
/// builds (`main.rs`: `windows_subsystem = "windows"` when not
/// `debug_assertions`), so it has **no console to hand down**. An unflagged
/// console child therefore makes Windows allocate a fresh one, and on
/// Windows 11 with Windows Terminal as the default terminal that is a
/// visible black window flashing on every probe — reported as issue #2 and
/// invisible in debug builds, which do own a console.
pub(crate) fn hidden_command(program: &str) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    {
        // tokio's own `creation_flags`; no `CommandExt` import needed.
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// `CREATE_NO_WINDOW` — the child gets no console at all.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn hide_window(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_cmd: &mut std::process::Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The crate's source root, however this test was built: `src-tauri/src`
    /// from the crate itself, `../src-tauri/src` from the parse-tests
    /// harness. Panics rather than skipping — a guard that can silently not
    /// run is worse than no guard.
    fn source_root() -> std::path::PathBuf {
        let mut dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for _ in 0..3 {
            for cand in [dir.join("src-tauri").join("src"), dir.join("src")] {
                if cand.join("platform").join("mod.rs").is_file() {
                    return cand;
                }
            }
            match dir.parent() {
                Some(p) => dir = p.to_path_buf(),
                None => break,
            }
        }
        panic!("could not locate src-tauri/src from {}", env!("CARGO_MANIFEST_DIR"));
    }

    fn collect_rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect_rust_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    /// Every console child must suppress its window (issue #2: a bare
    /// `Command::new` inherits nothing from the GUI-subsystem `pane.exe`, so
    /// Windows opens a console that flashes on screen). The sanctioned entry
    /// points are `run_hidden` and `hidden_command` in this file; this test
    /// fails the moment anyone spawns a tokio child elsewhere.
    ///
    /// Only the tokio constructor is policed: every provider probe is async,
    /// so that is where the defect lived and where a regression would land.
    /// The sync path is already funnelled through `run_hidden`, and its only
    /// other caller is a `#[cfg(test)]` mklink probe.
    #[test]
    fn no_bare_tokio_command_spawns_outside_this_module() {
        let root = source_root();
        let mut files = Vec::new();
        collect_rust_files(&root, &mut files);
        assert!(
            files.len() > 50,
            "source scan found only {} files under {} — wrong root?",
            files.len(),
            root.display()
        );
        let sanctioned = std::path::Path::new("platform").join("mod.rs");
        let offenders: Vec<String> = files
            .iter()
            .filter(|f| !f.ends_with(&sanctioned))
            .filter(|f| {
                std::fs::read_to_string(f)
                    .is_ok_and(|t| t.contains("tokio::process::Command::new"))
            })
            .map(|f| f.strip_prefix(&root).unwrap_or(f).display().to_string())
            .collect();
        assert!(
            offenders.is_empty(),
            "these spawn a console child with no CREATE_NO_WINDOW (use \
             platform::hidden_command): {offenders:?}"
        );
    }

    #[test]
    fn provider_modules_do_not_implement_os_calls() {
        let root = source_root();
        let mut files = Vec::new();
        collect_rust_files(&root.join("providers"), &mut files);
        let offenders: Vec<String> = files.iter().filter(|file| {
            std::fs::read_to_string(file).is_ok_and(|source| {
                source.contains("use windows::")
                    || source.contains("extern \"system\"")
                    || source.contains("std::os::windows::")
                    || source.contains("Component::Prefix")
            })
        }).map(|file| file.strip_prefix(&root).unwrap().display().to_string()).collect();
        assert!(offenders.is_empty(), "OS implementation must use the platform seam: {offenders:?}");
    }

    /// Both spawn helpers must still run a real console child and capture its
    /// stdout: the window-suppression flag must not break the spawn itself.
    #[cfg(windows)]
    #[test]
    fn hidden_spawns_still_run_and_capture_stdout() {
        let sync = run_hidden("cmd", &["/C", "echo", "pane-probe"]);
        assert!(
            sync.as_ref().is_some_and(|s| s.contains("pane-probe")),
            "run_hidden lost its output: {sync:?}"
        );
        let out = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                hidden_command("cmd")
                    .args(["/C", "echo", "pane-probe"])
                    .output()
                    .await
                    .ok()
            })
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
        assert!(
            out.as_ref().is_some_and(|s| s.contains("pane-probe")),
            "hidden_command lost its output: {out:?}"
        );
    }

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

/// "127.0.0.1:7897" or "http=127.0.0.1:7897;https=127.0.0.1:7897" → one URL.
/// https wins when both are listed (every vendor we talk to is https).
pub(crate) fn parse_win_proxy(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let pick = |scheme: &str| {
        raw.split(';')
            .filter_map(|part| part.trim().split_once('='))
            .find(|(key, _)| key.trim().eq_ignore_ascii_case(scheme))
            .map(|(_, value)| value.trim().to_string())
            .filter(|value| !value.is_empty())
    };
    let host = pick("https")
        .or_else(|| pick("http"))
        .or_else(|| (!raw.contains('=')).then(|| raw.to_string()))?;
    if host.is_empty() {
        return None;
    }
    Some(if host.contains("://") {
        host
    } else {
        format!("http://{host}")
    })
}


/// Enabled system proxy, absent on hosts without a supported system setting.
pub fn system_proxy() -> Option<String> { imp::system_proxy() }

/// Owner/SYSTEM access on Windows, mode 0600 on Unix. Failures stay visible.
pub(crate) fn restrict_owner_only(path: &Path) -> Result<(), String> { imp::restrict_owner_only(path) }

/// Create a new private staging file without truncating an existing file.
pub(crate) fn create_private_file(path: &Path) -> Result<(), String> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    drop(options.open(path).map_err(|error| format!("create private file: {error}"))?);
    Ok(())
}

/// Atomically replace an existing destination; Windows requests write-through.
pub(crate) fn replace_file(replacement: &Path, destination: &Path) -> std::io::Result<()> {
    imp::replace_file(replacement, destination)
}

/// Reject relative and Windows current-drive-root paths for credential homes.
pub(crate) fn path_is_fully_absolute(path: &Path) -> bool { imp::path_is_fully_absolute(path) }

/// Fallible OS randomness; never replace an unavailable CSPRNG with predictable ids.
pub(crate) fn fill_random(bytes: &mut [u8]) -> Result<(), String> {
    use rand_core::{OsRng, RngCore};
    OsRng.try_fill_bytes(bytes).map_err(|error| format!("OS RNG failed: {error}"))
}
