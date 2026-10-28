use std::path::{Path, PathBuf};

/// Windows- and cross-platform-safe file name for an exported share PNG.
/// Path components are dropped, invalid characters collapse to '-', and the
/// result always ends in `.png`. The dialog is not involved; this is only the
/// default name used when writing to the Downloads folder.
pub fn sanitize_file_name(input: &str) -> String {
    let base = Path::new(input)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let mut out = String::new();
    for ch in base.chars() {
        if out.chars().count() >= 76 {
            break;
        }
        let invalid = ch.is_control()
            || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*');
        if invalid || ch.is_whitespace() {
            if !out.ends_with('-') {
                out.push('-');
            }
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim_matches(|ch| ch == '-' || ch == '.' || ch == ' ');
    let stem = if trimmed.to_ascii_lowercase().ends_with(".png") {
        trimmed[..trimmed.len() - 4].trim_matches(|ch| ch == '-' || ch == '.' || ch == ' ')
    } else {
        trimmed
    };
    let stem = if stem.is_empty() { "pane-share" } else { stem };
    let upper = stem.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = if reserved.contains(&upper.as_str()) {
        format!("pane-{stem}")
    } else {
        stem.to_string()
    };
    format!("{stem}.png")
}

/// Never overwrite an existing share image. `pane.png` -> `pane-2.png` ->
/// `pane-3.png`; the fallback timestamp only matters if 10k files exist.
pub fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let first = dir.join(file_name);
    if !first.exists() {
        return first;
    }
    let stem = file_name.strip_suffix(".png").unwrap_or(file_name);
    for index in 2..10_000 {
        let candidate = dir.join(format!("{stem}-{index}.png"));
        if !candidate.exists() {
            return candidate;
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    dir.join(format!("{stem}-{stamp}.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_drops_paths_and_reserved_characters() {
        assert_eq!(sanitize_file_name("../../etc/passwd"), "passwd.png");
        let name = sanitize_file_name("a/b*c?.png");
        assert_eq!(name, "b-c.png");
    }

    #[test]
    fn sanitize_keeps_unicode_and_falls_back_when_empty() {
        let name = sanitize_file_name("报告 2026.png");
        assert_eq!(name, "报告-2026.png");
        assert_eq!(sanitize_file_name(""), "pane-share.png");
        assert_eq!(sanitize_file_name(".png"), "png.png");
        assert_eq!(sanitize_file_name("CON.png"), "pane-CON.png");
    }

    #[test]
    fn unique_path_never_overwrites() {
        let dir = std::env::temp_dir().join(format!(
            "pane-share-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let first = unique_path(&dir, "pane.png");
        assert_eq!(first, dir.join("pane.png"));
        std::fs::write(&first, b"x").expect("first file");
        let second = unique_path(&dir, "pane.png");
        assert_eq!(second, dir.join("pane-2.png"));
        std::fs::write(&second, b"x").expect("second file");
        let third = unique_path(&dir, "pane.png");
        assert_eq!(third, dir.join("pane-3.png"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
