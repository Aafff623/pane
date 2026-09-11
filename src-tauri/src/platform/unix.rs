//! macOS and Linux implementation of the platform seam. See
//! `platform/mod.rs` for the contract each function has to honor.
//!
//! The two share every tool-shaped answer (`ps`, `lsof`) and differ only in
//! where secrets and the display language live, so they stay in one file
//! rather than duplicating the process plumbing twice.

use std::collections::BTreeMap;

use super::{language_tag_to_locale, run_hidden, ProcessInfo};

/// No DPAPI off Windows — Chromium-derived apps there encrypt their
/// `os_crypt` keys with an OS keyring/agent instead, which Pane does not read.
pub fn dpapi_unprotect(_blob: &[u8]) -> Option<Vec<u8>> {
    None
}

/// macOS keychain. `security(1)` is part of the base system, so this needs
/// no extra crate or entitlement — the same generic-password item Go's
/// keyring library (and therefore gh and Antigravity) writes.
#[cfg(target_os = "macos")]
pub fn secret_blob(target: &str) -> Option<Vec<u8>> {
    let out = run_hidden("security", &["find-generic-password", "-s", target, "-w"])?;
    let text = out.trim_end_matches(['\n', '\r']);
    if text.is_empty() {
        return None;
    }
    Some(text.as_bytes().to_vec())
}

/// freedesktop Secret Service through `secret-tool`. Absent on a headless
/// box or a desktop without a keyring daemon, in which case this returns
/// None and the caller falls through to its file-based sources.
#[cfg(not(target_os = "macos"))]
pub fn secret_blob(target: &str) -> Option<Vec<u8>> {
    let out = run_hidden("secret-tool", &["lookup", "service", target])?;
    if out.is_empty() {
        return None;
    }
    Some(out.into_bytes())
}

/// macOS keeps the UI language list in global preferences; `LANG` is often
/// unset for GUI apps, so it is only the fallback here.
#[cfg(target_os = "macos")]
pub fn system_ui_language() -> &'static str {
    if let Some(raw) = run_hidden("defaults", &["read", "-g", "AppleLanguages"]) {
        // `("zh-Hans-CN",\n "en-US"\n)` — the first entry is the choice.
        if let Some(first) = raw
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && *l != "(" && *l != ")")
        {
            return language_tag_to_locale(first.trim_end_matches(','));
        }
    }
    posix_lang().unwrap_or("en")
}

#[cfg(not(target_os = "macos"))]
pub fn system_ui_language() -> &'static str {
    posix_lang().unwrap_or("en")
}

/// POSIX locale precedence: LC_ALL overrides everything, then the messages
/// category, then LANG.
fn posix_lang() -> Option<&'static str> {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(key) {
            if !v.is_empty() && v != "C" && v != "POSIX" {
                return Some(language_tag_to_locale(&v));
            }
        }
    }
    None
}

/// No portable signal for "the screen is being presented or shared" exists
/// on either platform without private APIs, so the tray strip keeps showing
/// its numbers. Reporting `false` is the honest answer, not a stub.
pub fn screen_is_being_shared() -> bool {
    false
}

pub fn list_processes(name_prefixes: &[&str]) -> Vec<ProcessInfo> {
    let Some(raw) = run_hidden("ps", &["-axo", "pid=,args="]) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let (pid, rest) = line.split_once(char::is_whitespace)?;
            let pid = pid.parse::<u32>().ok()?;
            let command_line = rest.trim();
            if command_line.is_empty() {
                return None;
            }
            // Match on the executable's own name, not anywhere in the
            // command line — otherwise every process that merely mentions
            // the binary (a grep, an editor, this very scan) would match.
            let exe = command_line.split_whitespace().next()?;
            let name = exe.rsplit('/').next().unwrap_or(exe);
            if !name_prefixes.iter().any(|p| name.starts_with(p)) {
                return None;
            }
            Some(ProcessInfo { pid, command_line: command_line.to_string() })
        })
        .collect()
}

pub fn listening_loopback_ports() -> BTreeMap<u32, Vec<u16>> {
    let mut map = lsof_listeners();
    if map.is_empty() {
        // lsof is standard on macOS but optional on Linux distributions.
        map = ss_listeners();
    }
    for ports in map.values_mut() {
        ports.sort_unstable();
        ports.dedup();
    }
    map
}

/// `lsof -F` emits one field per line, tagged by its first character: `p`
/// opens a process block, every `n` after it is one of that process's
/// addresses.
fn lsof_listeners() -> BTreeMap<u32, Vec<u16>> {
    let Some(raw) = run_hidden("lsof", &["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpn"]) else {
        return BTreeMap::new();
    };
    let mut map: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    let mut pid = None;
    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix('p') {
            pid = rest.parse::<u32>().ok();
        } else if let Some(rest) = line.strip_prefix('n') {
            if let (Some(pid), Some(port)) = (pid, loopback_port(rest)) {
                map.entry(pid).or_default().push(port);
            }
        }
    }
    map
}

/// `ss -lntp` on Linux: the local address is the fourth column and the pid
/// hides inside the trailing `users:(("name",pid=123,fd=8))` field.
fn ss_listeners() -> BTreeMap<u32, Vec<u16>> {
    let Some(raw) = run_hidden("ss", &["-lntpH"]) else {
        return BTreeMap::new();
    };
    let mut map: BTreeMap<u32, Vec<u16>> = BTreeMap::new();
    for line in raw.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        let Some(local) = cols.get(3) else { continue };
        let Some(port) = loopback_port(local) else { continue };
        let Some(pid) = line
            .split("pid=")
            .nth(1)
            .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|d| d.parse::<u32>().ok())
        else {
            continue;
        };
        map.entry(pid).or_default().push(port);
    }
    map
}

/// Port of a listening address, but only when it is reachable over
/// loopback: `127.0.0.1:x`, a wildcard bind, or IPv6 localhost.
fn loopback_port(addr: &str) -> Option<u16> {
    let (host, port) = addr.rsplit_once(':')?;
    let host = host.trim_matches(['[', ']']);
    let loopback = matches!(host, "127.0.0.1" | "::1" | "localhost" | "*" | "0.0.0.0" | "::" | "");
    if !loopback {
        return None;
    }
    port.parse::<u16>().ok()
}

/// WebView2's memory-target hint has no WKWebView or WebKitGTK equivalent;
/// both manage their own footprint when the window is hidden.
pub fn set_webview_memory_level(_window: &tauri::WebviewWindow, _low: bool) {}

pub fn hide_window_border(_window: &tauri::WebviewWindow) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_addresses_are_recognized_across_tools() {
        assert_eq!(loopback_port("127.0.0.1:6736"), Some(6736));
        assert_eq!(loopback_port("*:8080"), Some(8080)); // lsof wildcard
        assert_eq!(loopback_port("0.0.0.0:8080"), Some(8080)); // ss wildcard
        assert_eq!(loopback_port("[::1]:443"), Some(443));
        assert_eq!(loopback_port("[::]:443"), Some(443));
    }

    #[test]
    fn routable_addresses_are_not_loopback() {
        assert_eq!(loopback_port("192.168.1.10:8080"), None);
        assert_eq!(loopback_port("10.0.0.1:80"), None);
        assert_eq!(loopback_port("no-port-here"), None);
    }
}
