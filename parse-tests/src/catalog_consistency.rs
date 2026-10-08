//! Cross-surface consistency gate for the provider catalog.
//!
//! The Rust catalog, the frontend mirror and the icon assets are three
//! hand-maintained copies of the same truth (see the 2026-10 architecture
//! handoff). These tests read the frontend sources straight from disk so a
//! drift between the surfaces fails `cargo test` instead of shipping.

#[cfg(test)]
mod tests {
    use crate::provider_catalog;

    const TS_CATALOG: &str = include_str!("../../src/providerCatalog.ts");
    const TS_VISUALS: &str = include_str!("../../src/providerVisuals.ts");

    /// Families that intentionally live only in the frontend mirror
    /// (spend-only cards with no Rust quota provider).
    const TS_ONLY_WHITELIST: &[&str] = &["mcode", "qoder"];

    fn ts_family_ids() -> Vec<String> {
        let mut out = vec![];
        for line in TS_CATALOG.lines() {
            if let Some(rest) = line.split("familyId: \"").nth(1) {
                if let Some(end) = rest.find('"') {
                    out.push(rest[..end].to_string());
                }
            }
        }
        out
    }

    fn ts_icon_keys() -> Vec<(String, String)> {
        let mut out = vec![];
        for line in TS_CATALOG.lines() {
            let fam = line
                .split("familyId: \"")
                .nth(1)
                .and_then(|r| r.find('"').map(|e| r[..e].to_string()));
            let key = line
                .split("iconKey: \"")
                .nth(1)
                .and_then(|r| r.find('"').map(|e| r[..e].to_string()));
            if let (Some(f), Some(k)) = (fam, key) {
                out.push((f, k));
            }
        }
        assert!(
            out.len() >= 40,
            "iconKey parser found only {} entries — the TS catalog shape changed",
            out.len()
        );
        out
    }

    #[test]
    fn rust_and_ts_catalogs_agree_on_families() {
        let rust: Vec<&str> = provider_catalog::provider_definitions()
            .iter()
            .map(|d| d.family_id)
            .collect();
        let ts = ts_family_ids();
        for fam in &ts {
            if TS_ONLY_WHITELIST.contains(&fam.as_str()) {
                continue;
            }
            assert!(
                rust.contains(&fam.as_str()),
                "frontend family `{fam}` has no Rust catalog entry"
            );
        }
        for fam in &rust {
            assert!(
                ts.iter().any(|t| t == fam),
                "Rust family `{fam}` missing from src/providerCatalog.ts"
            );
        }
    }

    #[test]
    fn every_icon_key_resolves_to_a_registered_visual() {
        for (fam, key) in ts_icon_keys() {
            let imported = TS_VISUALS.contains(&format!("{key}Icon"))
                || TS_VISUALS.contains(&format!("\"{key}\""));
            assert!(
                imported,
                "family `{fam}` iconKey `{key}` is not registered in src/providerVisuals.ts"
            );
        }
    }
}
