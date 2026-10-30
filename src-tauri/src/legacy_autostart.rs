//! Retire the autostart entry left behind by the pre-identity app.
//!
//! Before #105 the product registered itself as `Pane`; the current build
//! uses `Pane-Aafff623`. The identity migration enabled the new entry but
//! never deleted the old `HKCU\...\Run\Pane` value, so a machine that kept
//! an older install booted it every time — and that old build honestly saw
//! the latest manifest and kept offering the update the user had already
//! installed. This module removes only that stale value, only when it points
//! at a different `pane.exe`; the current autostart entry is untouched.
//!
//! The old build re-asserts its own autostart at the end of its setup, so a
//! single early delete is not enough when both builds launch at logon; the
//! caller retries after the startup wave.

/// Pull the executable path out of a Run value. Handles quoted paths with
/// arguments (`"C:\Tools\Pane\pane.exe" --silent`), unquoted paths with
/// trailing switches (`C:\Tools\Pane\pane.exe --silent`), and a launchable
/// value whose closing quote is missing.
pub fn run_value_executable(value: &str) -> &str {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return &rest[..end];
        }
    }
    let lower = value.to_ascii_lowercase();
    match lower.find(".exe") {
        Some(index) => value[..index + 4].trim(),
        None => value,
    }
}

pub fn is_pane_executable(value: &str) -> bool {
    let exe = run_value_executable(value);
    let file = exe
        .rsplit(|c| c == '\\' || c == '/')
        .next()
        .unwrap_or(exe);
    file.eq_ignore_ascii_case("pane.exe")
}

fn normalized_path(path: &str) -> String {
    let path = path.trim().trim_matches('"').replace('/', "\\");
    let path = path.strip_prefix(r"\\?\").unwrap_or(&path);
    path.to_ascii_lowercase()
}

/// Junctions, 8.3 short names and symlinks can name the current executable
/// without matching its display string. Only trust canonicalization when
/// both files still exist; the legacy target may already be uninstalled and
/// must still be retired.
fn same_existing_file(left: &str, right: &str) -> bool {
    let (Ok(left), Ok(right)) = (std::fs::canonicalize(left), std::fs::canonicalize(right)) else {
        return false;
    };
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

/// True only for a legacy `Pane` value that points at some *other* pane.exe.
/// A value already pointing at this executable may be the user's working
/// autostart (or a duplicate), so it is left alone. Values using `%VAR%`
/// expansion are skipped: they may resolve to the current executable and we
/// cannot expand them without the original registry type.
pub fn should_retire(value: &str, current_exe: &str) -> bool {
    if !is_pane_executable(value) {
        return false;
    }
    let target = run_value_executable(value);
    if target.contains('%') {
        return false;
    }
    if normalized_path(target) == normalized_path(current_exe) {
        return false;
    }
    !same_existing_file(target.trim_matches('"'), current_exe)
}

#[cfg(windows)]
fn remove_startup_approved() {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run",
        KEY_READ | KEY_WRITE,
    ) else {
        return;
    };
    match key.delete_value("Pane") {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => eprintln!("[pane] legacy StartupApproved cleanup: {error}"),
    }
}

#[cfg(windows)]
pub fn retire_legacy_autostart() -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    let current_exe = std::env::current_exe().ok()?.display().to_string();
    let run = match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_READ | KEY_WRITE,
    ) {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            eprintln!("[pane] legacy autostart key open: {error}");
            return None;
        }
    };
    let value: String = match run.get_value("Pane") {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            eprintln!("[pane] legacy autostart read: {error}");
            return None;
        }
    };
    if !should_retire(&value, &current_exe) {
        return None;
    }
    match run.delete_value("Pane") {
        Ok(()) => {
            remove_startup_approved();
            Some(value)
        }
        Err(error) => {
            eprintln!("[pane] legacy autostart delete: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_and_unquoted_commands() {
        assert_eq!(
            run_value_executable(r#""D:\Tools\Pane\pane.exe" --silent"#),
            r"D:\Tools\Pane\pane.exe"
        );
        assert_eq!(
            run_value_executable(r"D:\Tools\Pane\pane.exe --silent"),
            r"D:\Tools\Pane\pane.exe"
        );
        assert_eq!(
            run_value_executable(r#""C:\Program Files\Pane\pane.exe""#),
            r"C:\Program Files\Pane\pane.exe"
        );
        // A launchable value with a missing closing quote still identifies
        // the executable; Windows itself tolerates this.
        assert!(is_pane_executable(r#""D:\Tools\Pane\pane.exe --silent"#));
    }

    #[test]
    fn identifies_only_pane_executables() {
        assert!(is_pane_executable(r#"D:\Tools\Pane\pane.exe"#));
        assert!(is_pane_executable(r#""D:\Tools\Pane\PANE.EXE" --x"#));
        assert!(!is_pane_executable(r"D:\Tools\Other\notpane.exe"));
        assert!(!is_pane_executable("not-a-path"));
    }

    #[test]
    fn retires_a_different_install_but_keeps_the_current_one() {
        let current = r"C:\Users\Me\AppData\Local\Pane-Aafff623\pane.exe";
        assert!(should_retire(r"D:\Tools\Pane\pane.exe", current));
        assert!(should_retire(r#""D:\Tools\Pane\pane.exe" --silent"#, current));
        assert!(!should_retire(
            r"c:/users/me/appdata/local/pane-aafff623/pane.exe",
            current
        ));
        assert!(!should_retire(current, current));
        assert!(!should_retire(&format!(r"\\?\{}", current), current));
        assert!(!should_retire(&format!("\"{}\"", current), current));
        assert!(!should_retire(
            r"%LOCALAPPDATA%\Pane-Aafff623\pane.exe",
            current
        ));
        assert!(!should_retire(r"D:\Tools\Other\notpane.exe", current));
        assert!(!should_retire("", current));
    }
}
