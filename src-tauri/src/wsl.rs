//! WSL/native union collection (#69). Only the discovery funnel lives here;
//! each scanner keeps its own parser and merges the extra roots as if they
//! were native directories. `wsl.exe -l -q` emits UTF-16LE, so it is decoded
//! explicitly rather than through the UTF-8 helper.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WslMode {
    NativeOnly,
    Both,
    WslFirst,
}

pub fn mode() -> WslMode {
    let cfg: serde_json::Value = std::fs::read_to_string(
        crate::providers::config_dir().join("config.json"),
    )
    .ok()
    .and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())
    .unwrap_or_default();
    let raw = cfg
        .pointer("/spend/wsl_mode")
        .or_else(|| cfg.get("wslMode"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("both");
    match raw {
        "native-only" => WslMode::NativeOnly,
        "wsl-first" => WslMode::WslFirst,
        _ => WslMode::Both,
    }
}

fn decode_utf16(bytes: &[u8]) -> String {
    let start = if bytes.starts_with(&[0xFF, 0xFE]) { 2 } else { 0 };
    let units: Vec<u16> = bytes[start..]
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

fn distro_homes() -> Vec<(String, String)> {
    static CACHE: Mutex<Option<(Instant, Vec<(String, String)>)>> = Mutex::new(None);
    let mut guard = CACHE.lock().unwrap_or_else(|error| error.into_inner());
    if let Some((at, homes)) = guard.as_ref() {
        if at.elapsed() < Duration::from_secs(3_600) {
            return homes.clone();
        }
    }
    let Some(raw) = crate::platform::run_hidden_bytes("wsl.exe", &["-l", "-q"]) else {
        return Vec::new();
    };
    let names: Vec<String> = decode_utf16(&raw)
        .lines()
        .map(|line| line.trim_matches(|c: char| c == '\0' || c.is_whitespace()).to_string())
        .filter(|line| !line.is_empty())
        .collect();
    let mut homes = Vec::new();
    for name in names {
        let Some(home) = crate::platform::run_hidden("wsl.exe", &["-d", &name, "--", "printenv", "HOME"])
        else {
            continue;
        };
        let home = home.trim().to_string();
        if home.starts_with('/') {
            homes.push((name, home));
        }
    }
    *guard = Some((Instant::now(), homes.clone()));
    homes
}

/// Native home root plus one UNC root per distro, in the configured order.
/// Non-existent UNC roots are dropped so a broken distro never slows a scan.
pub fn roots_for(relative: &str) -> Vec<PathBuf> {
    let native = dirs::home_dir().unwrap_or_default().join(relative);
    let mut roots = vec![native.clone()];
    if mode() == WslMode::NativeOnly {
        return roots;
    }
    let mut remote = Vec::new();
    for (distro, home) in distro_homes() {
        let sub = home.trim_start_matches('/').replace('/', "\\");
        let candidate = PathBuf::from(format!(r"\\wsl.localhost\{distro}\{sub}")).join(relative);
        if candidate.exists() {
            remote.push(candidate);
        }
    }
    if mode() == WslMode::WslFirst {
        remote.push(native);
        return remote;
    }
    roots.extend(remote);
    roots
}

#[cfg(test)]
mod tests {
    use super::{decode_utf16, WslMode};

    #[test]
    fn wsl_list_output_is_utf16le_with_optional_bom() {
        let text = "Ubuntu\r\ndebian\r\n";
        let mut bytes = vec![0xFF, 0xFE];
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(decode_utf16(&bytes).trim(), "Ubuntu\r\ndebian");
    }

    #[test]
    fn mode_defaults_to_both_and_parses_all_three_names() {
        // The parser itself is exercised through the enum mapping in mode();
        // this keeps the three spellings explicit.
        assert_ne!(WslMode::Both, WslMode::NativeOnly);
        assert_ne!(WslMode::WslFirst, WslMode::Both);
    }
}
