//! System font family enumeration for the Settings font picker.

/// Family names of every font installed on the system, sorted
/// case-insensitively. Localized names (e.g. 微软雅黑 alongside Microsoft
/// YaHei UI) appear as separate entries — that is what DirectWrite reports
/// and both resolve to the same font at render time.
///
/// The first call pays the DirectWrite enumeration cost; the frontend caches
/// the result for the session, so no memoization lives here.
pub fn system_font_families() -> Vec<String> {
    let mut families = match font_kit::source::SystemSource::new().all_families() {
        Ok(families) => families,
        Err(e) => {
            eprintln!("[pane] font enumeration failed: {e}");
            return Vec::new();
        }
    };
    families.retain(|name| !name.trim().is_empty());
    families.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    families.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    families
}
