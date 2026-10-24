//! Cross-surface consistency gate for the provider catalog.
//!
//! The Rust catalog, the frontend mirror and the icon assets are three
//! hand-maintained copies of the same truth (see the 2026-10 architecture
//! handoff). These tests read the frontend sources straight from disk so a
//! drift between the surfaces fails `cargo test` instead of shipping.

#[cfg(test)]
mod tests {
    use crate::provider_catalog;
    use provider_catalog::{ProviderCategory, QueryKind};
    use std::collections::HashSet;

    const TS_CATALOG: &str = include_str!("../../src/providerCatalog.ts");
    const TS_VISUALS: &str = include_str!("../../src/providerVisuals.ts");

    /// Families that intentionally live only in the frontend mirror
    /// (spend-only cards with no Rust quota provider).
    const TS_ONLY_WHITELIST: &[&str] = &["mcode"];

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FrontendDefinition {
        family_id: String,
        display_name: String,
        query_kind: String,
        supports_api_key: bool,
        supports_extra_accounts: bool,
        #[serde(rename = "supportsOAuth", default)]
        supports_oauth: Option<bool>,
        #[serde(rename = "webSession", default)]
        web_session: Option<bool>,
        icon_key: String,
        category: Option<String>,
        primary_unit: crate::usage_units::UsageUnit,
    }

    fn ts_definitions() -> Vec<FrontendDefinition> {
        let property = regex::Regex::new(r"([,{]\s*)([A-Za-z]\w*)\s*:").unwrap();
        let out: Vec<FrontendDefinition> = TS_CATALOG
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("{ familyId:"))
            .map(|line| {
                let json = property.replace_all(line.trim_end_matches(','), "$1\"$2\":");
                serde_json::from_str(&json).expect("frontend catalog row must be readable")
            })
            .collect();
        assert!(
            out.len() >= provider_catalog::provider_definitions().len(),
            "frontend catalog parser stopped finding all registrations"
        );
        out
    }

    #[test]
    fn rust_and_ts_catalogs_agree_on_families() {
        let rust: Vec<&str> = provider_catalog::provider_definitions()
            .iter()
            .map(|d| d.family_id)
            .collect();
        let definitions = ts_definitions();
        let ts: Vec<&str> = definitions
            .iter()
            .map(|row| row.family_id.as_str())
            .collect();
        assert_eq!(
            ts.iter().copied().collect::<HashSet<_>>().len(),
            ts.len(),
            "duplicate frontend family"
        );
        assert_eq!(
            rust.iter().copied().collect::<HashSet<_>>().len(),
            rust.len(),
            "duplicate Rust family"
        );
        for fam in &ts {
            if TS_ONLY_WHITELIST.contains(fam) {
                continue;
            }
            assert!(
                rust.contains(fam),
                "frontend family `{fam}` has no Rust catalog entry"
            );
        }
        for fam in &rust {
            assert!(
                ts.contains(fam),
                "Rust family `{fam}` missing from src/providerCatalog.ts"
            );
        }
    }

    #[test]
    fn every_icon_key_resolves_to_a_registered_visual() {
        for row in ts_definitions() {
            let (fam, key) = (row.family_id, row.icon_key);
            let imported = TS_VISUALS.contains(&format!("{key}Icon"))
                || TS_VISUALS.contains(&format!("\"{key}\""));
            assert!(
                imported,
                "family `{fam}` iconKey `{key}` is not registered in src/providerVisuals.ts"
            );
        }
    }

    #[test]
    fn frontend_mirrors_every_shared_catalog_field() {
        let frontend = ts_definitions();
        for rust in provider_catalog::provider_definitions() {
            let ts = frontend
                .iter()
                .find(|row| row.family_id == rust.family_id)
                .unwrap();
            let kind = match rust.query_kind {
                QueryKind::NativeSnapshot => "nativeSnapshot",
                QueryKind::NativeBalance => "nativeBalance",
                QueryKind::NativeCodingPlan => "nativeCodingPlan",
                QueryKind::Composite => "composite",
                QueryKind::LocalOnly => "localOnly",
            };
            let category = match rust.category {
                ProviderCategory::Coding => "coding",
                ProviderCategory::Mcp => "mcp",
                ProviderCategory::Productivity => "productivity",
            };
            assert_eq!(ts.primary_unit, rust.primary_unit, "primary usage unit differs for {}", rust.family_id);
            assert_eq!(
                (
                    ts.display_name.as_str(),
                    ts.query_kind.as_str(),
                    ts.supports_api_key,
                    ts.supports_extra_accounts,
                    ts.icon_key.as_str(),
                    ts.category.as_deref().unwrap_or("coding"),
                ),
                (
                    rust.display_name,
                    kind,
                    rust.supports_api_key,
                    rust.supports_extra_accounts,
                    rust.icon_key,
                    category,
                ),
                "shared catalog fields differ for `{}`",
                rust.family_id
            );
        }
    }

    #[test]
    fn descriptor_contract_matches_frontend_capabilities() {
        let frontend = ts_definitions();
        for rust in provider_catalog::provider_definitions() {
            let ts = frontend
                .iter()
                .find(|row| row.family_id == rust.family_id)
                .unwrap();
            let d = rust.descriptor();
            let auth = if ts.supports_api_key {
                "api_key"
            } else if ts.supports_oauth == Some(true) {
                "oauth"
            } else if ts.web_session == Some(true) {
                "web_session"
            } else if ts.query_kind == "localOnly" {
                "local"
            } else {
                "none"
            };
            assert_eq!(d.auth_kind.as_str(), auth, "auth_kind differs for {}", rust.family_id);

            let counter = ts.query_kind == "localOnly";
            assert_eq!(
                d.probe_kind.as_str(),
                if counter { "local_counter" } else { "active_get" },
                "probe_kind differs for {}",
                rust.family_id
            );

            let scope = if ts.supports_api_key {
                "key"
            } else if auth == "oauth" || auth == "web_session" || ts.supports_extra_accounts {
                "account"
            } else {
                "unknown"
            };
            assert_eq!(d.quota_scope.as_str(), scope, "quota_scope differs for {}", rust.family_id);

            let timestamped = matches!(
                ts.query_kind.as_str(),
                "nativeCodingPlan" | "nativeSnapshot" | "composite"
            );
            assert_eq!(
                d.reset_semantics.as_str(),
                if timestamped { "api_timestamp" } else { "none" },
                "reset_semantics differs for {}",
                rust.family_id
            );
            assert_eq!(
                d.pace_capability, timestamped,
                "pace_capability differs for {}",
                rust.family_id
            );
        }
        for ts in &frontend {
            if ts.supports_oauth == Some(true) {
                assert!(
                    provider_catalog::OAUTH_FAMILIES.contains(&ts.family_id.as_str()),
                    "frontend OAuth family `{}` missing from Rust OAUTH_FAMILIES",
                    ts.family_id
                );
            }
        }
    }

    #[test]
    fn single_provider_refresh_covers_every_global_native_registration() {
        let mut families = HashSet::new();
        for definition in crate::provider_dispatch::registrations() {
            assert!(
                families.insert(definition.family_id),
                "duplicate executable family"
            );
            // Construct and drop the lazy production future without any network or file I/O.
            assert!(
                crate::provider_dispatch::native_snapshot(definition.family_id).is_some(),
                "global family {} has no single-provider adapter",
                definition.family_id
            );
        }
        let expected: HashSet<_> = provider_catalog::provider_definitions()
            .iter()
            .filter(|definition| definition.family_id != "onenewapi")
            .map(|definition| definition.family_id)
            .collect();
        assert_eq!(
            families, expected,
            "One/New API alone uses the composite-site path"
        );
    }

    #[test]
    fn every_registered_family_can_be_pinned_without_accepting_malformed_card_ids() {
        for definition in provider_catalog::provider_definitions() {
            assert!(provider_catalog::valid_card_id(definition.family_id));
            assert!(provider_catalog::valid_card_id(&format!(
                "{}@work-123",
                definition.family_id
            )));
        }
        for invalid in [
            "unknown",
            "giphy@",
            "claude@a@b",
            "codex@../../x",
            "codex@work account",
        ] {
            assert!(!provider_catalog::valid_card_id(invalid), "{invalid}");
        }
    }

    #[test]
    fn fresh_install_defaults_keep_mainstream_families_and_park_optional_ones() {
        let disabled = provider_catalog::default_disabled_families();
        assert_eq!(
            disabled.len(),
            64,
            "preserve the existing fresh-install defaults (+ tokenrhythm, 2026-10-19; + xfyun, 2026-10-23; + api-key pilots, 2026-10-23; + xfyun-speech, zcode and orcaterm, 2026-10-24)"
        );
        for family in [
            "claude",
            "codex",
            "kimi",
            "codebuddy",
            "codebuddycn",
            "workbuddy",
        ] {
            assert!(!disabled.contains(&family), "{family}");
        }
        for family in ["giphy", "stepfun-plan", "onenewapi", "deepseek", "orcaterm"] {
            assert!(disabled.contains(&family), "{family}");
        }
    }
}
