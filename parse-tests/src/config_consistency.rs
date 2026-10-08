//! Cross-surface consistency gate for the persisted config keys.
//!
//! The frontend sends every key in `FRONTEND_CONFIG_KEYS` (src/main.ts) to
//! `set_config`; Rust silently drops anything missing from `CONFIG_KEYS`
//! (src-tauri/src/lib.rs) with only a log line. That drift made the
//! "5 小时窗口即将重置" toggle a no-op — the switch rendered, the save was
//! discarded, and nothing failed. This gate reads both sources from disk so
//! the next drift fails `cargo test` instead of shipping.

#[cfg(test)]
mod tests {
    const TS_MAIN: &str = include_str!("../../src/main.ts");
    const RS_LIB: &str = include_str!("../../src-tauri/src/lib.rs");

    /// Frontend keys Rust intentionally refuses: `set_config` is not their
    /// write path (a dedicated command or a Rust-side seed owns them).
    const ALLOWED_TS_ONLY: &[&str] = &[];

    /// Pull the quoted, identifier-shaped strings out of a JS/Rust array
    /// literal that starts at `marker` and ends at the first `\n]`.
    /// Non-identifier fragments (comment prose) are dropped so a stray quote
    /// in a comment cannot shift the pairing.
    fn parse_key_list(source: &str, marker: &str) -> Vec<String> {
        let start = source.find(marker).unwrap_or_else(|| panic!("marker not found: {marker}"));
        let rest = &source[start..];
        let end = rest.find("\n]").expect("array end not found");
        rest[..end]
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|s| {
                !s.is_empty() && s.len() < 40 && s.chars().all(|c| c.is_ascii_alphanumeric())
            })
            .map(str::to_string)
            .collect()
    }

    fn frontend_keys() -> Vec<String> {
        parse_key_list(TS_MAIN, "const FRONTEND_CONFIG_KEYS = [")
    }

    fn rust_keys() -> Vec<String> {
        parse_key_list(RS_LIB, "const CONFIG_KEYS: &[&str] = &[")
    }

    fn missing_keys(frontend: &[String], rust: &[String]) -> Vec<String> {
        frontend
            .iter()
            .filter(|k| !rust.contains(k) && !ALLOWED_TS_ONLY.contains(&k.as_str()))
            .cloned()
            .collect()
    }

    #[test]
    fn both_key_lists_parse() {
        let fe = frontend_keys();
        let be = rust_keys();
        assert!(fe.len() > 20, "frontend list parsed too small: {fe:?}");
        assert!(be.len() > 20, "rust list parsed too small: {be:?}");
    }

    /// The gate is only worth having if it can fail: a key present on one
    /// side must be reported (this is the notifyResetSoon shape).
    #[test]
    fn missing_key_is_reported() {
        let frontend = vec!["notifyResetSoon".to_string(), "locale".to_string()];
        let rust = vec!["locale".to_string()];
        assert_eq!(missing_keys(&frontend, &rust), vec!["notifyResetSoon".to_string()]);
        assert!(missing_keys(&rust, &rust).is_empty());
    }

    #[test]
    fn frontend_config_keys_survive_rust_set_config() {
        let missing = missing_keys(&frontend_keys(), &rust_keys());
        assert!(
            missing.is_empty(),
            "set_config would silently drop these frontend-persisted keys: {missing:?}\n\
             add them to CONFIG_KEYS in src-tauri/src/lib.rs, or list them in \
             ALLOWED_TS_ONLY with the reason"
        );
    }
}
