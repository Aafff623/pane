#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryKind {
    /// A provider-specific adapter produces a complete Snapshot.
    NativeSnapshot,
    /// A provider-specific API-key adapter reads a balance or quota endpoint.
    NativeBalance,
    /// A provider-specific coding-plan adapter reads plan windows.
    NativeCodingPlan,
    /// More than one credential/query path contributes to the Snapshot.
    Composite,
    /// The provider is local-only and does not query a remote quota endpoint.
    LocalOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderDefinition {
    pub family_id: &'static str,
    pub display_name: &'static str,
    pub query_kind: QueryKind,
    pub supports_api_key: bool,
    pub supports_extra_accounts: bool,
    pub icon_key: &'static str,
}

// Keep this list in the same stable order as the dashboard's provider list.
// This is intentionally a capability catalog, not a second query dispatcher.
const PROVIDER_DEFINITIONS: &[ProviderDefinition] = &[
    ProviderDefinition {
        family_id: "claude",
        display_name: "Claude",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "claude",
    },
    ProviderDefinition {
        family_id: "codex",
        display_name: "Codex",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "codex",
    },
    ProviderDefinition {
        family_id: "cursor",
        display_name: "Cursor",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        // Cursor accounts are imported token pairs / OAuth logins (its own
        // store lives in cursor-accounts.json), same multi-account UI.
        supports_extra_accounts: true,
        icon_key: "cursor",
    },
    ProviderDefinition {
        family_id: "opencode",
        display_name: "OpenCode",
        query_kind: QueryKind::Composite,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "opencode",
    },
    ProviderDefinition {
        family_id: "copilot",
        display_name: "Copilot",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "copilot",
    },
    ProviderDefinition {
        family_id: "grok",
        display_name: "Grok",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "grok",
    },
    ProviderDefinition {
        family_id: "devin",
        display_name: "Devin",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "devin",
    },
    ProviderDefinition {
        family_id: "minimax",
        display_name: "MiniMax",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "minimax",
    },
    ProviderDefinition {
        family_id: "openrouter",
        display_name: "OpenRouter",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "openrouter",
    },
    ProviderDefinition {
        family_id: "zai",
        display_name: "Z.ai",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "zai",
    },
    ProviderDefinition {
        family_id: "antigravity",
        display_name: "Antigravity",
        query_kind: QueryKind::Composite,
        supports_api_key: false,
        // Antigravity "accounts" are captured Google OAuth slots (its own
        // storage lives in antigravity-accounts.json, not accounts/), but
        // the multi-account UI contract is the same.
        supports_extra_accounts: true,
        icon_key: "antigravity",
    },
    ProviderDefinition {
        family_id: "deepseek",
        display_name: "DeepSeek",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "deepseek",
    },
    ProviderDefinition {
        family_id: "moonshot",
        display_name: "Kimi API",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "kimi",
    },
    ProviderDefinition {
        family_id: "elevenlabs",
        display_name: "ElevenLabs",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "elevenlabs",
    },
    ProviderDefinition {
        family_id: "ollama",
        display_name: "Ollama",
        query_kind: QueryKind::LocalOnly,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "ollama",
    },
    ProviderDefinition {
        family_id: "codebuff",
        display_name: "Codebuff",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "codebuff",
    },
    ProviderDefinition {
        family_id: "kilo",
        display_name: "Kilo",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "kilo",
    },
    ProviderDefinition {
        family_id: "aihubmix",
        display_name: "AihubMix",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "aihubmix",
    },
    ProviderDefinition {
        family_id: "onenewapi",
        display_name: "One/New API",
        query_kind: QueryKind::Composite,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "onenewapi",
    },
    ProviderDefinition {
        family_id: "qwen",
        display_name: "Qwen Code",
        query_kind: QueryKind::Composite,
        supports_api_key: true,
        supports_extra_accounts: false,
        icon_key: "qwen",
    },
    ProviderDefinition {
        family_id: "hermes",
        display_name: "Hermes",
        query_kind: QueryKind::LocalOnly,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "hermes",
    },
    ProviderDefinition {
        family_id: "kimi",
        display_name: "Kimi Code",
        query_kind: QueryKind::Composite,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "kimi",
    },
    ProviderDefinition {
        family_id: "stepfun",
        display_name: "StepFun",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "stepfun",
    },
    ProviderDefinition {
        family_id: "siliconflow",
        display_name: "SiliconFlow",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "siliconflow",
    },
    ProviderDefinition {
        family_id: "novita",
        display_name: "Novita AI",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "novita",
    },
    ProviderDefinition {
        family_id: "relaybalance",
        display_name: "Custom Balance",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "relaybalance",
    },
];

pub fn provider_definitions() -> &'static [ProviderDefinition] {
    PROVIDER_DEFINITIONS
}

pub fn provider_definition(family_id: &str) -> Option<&'static ProviderDefinition> {
    PROVIDER_DEFINITIONS
        .iter()
        .find(|definition| definition.family_id == family_id)
}

pub fn supports_extra_accounts(family_id: &str) -> bool {
    provider_definition(family_id).is_some_and(|definition| definition.supports_extra_accounts)
}

pub fn supports_api_key(family_id: &str) -> bool {
    provider_definition(family_id).is_some_and(|definition| definition.supports_api_key)
}

/// Returns the family part of a card id. Account fingerprints and One/New API
/// key ids use the same separator.
pub fn family_of(id: &str) -> String {
    id.split_once('@')
        .map_or_else(|| id.to_string(), |(family, _)| family.to_string())
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn query_kind_for_instance(id: &str) -> Option<QueryKind> {
    provider_definition(&family_of(id)).map(|definition| definition.query_kind)
}

#[cfg(test)]
mod tests {
    use super::{
        family_of, provider_definition, provider_definitions, query_kind_for_instance,
        supports_extra_accounts, QueryKind,
    };

    #[test]
    fn catalog_marks_the_multi_account_families() {
        // The six API-key families plus Antigravity's captured OAuth slots.
        let expected = [
            "cursor",
            "antigravity",
            "deepseek",
            "kimi",
            "stepfun",
            "siliconflow",
            "novita",
            "relaybalance",
        ];
        let actual: Vec<&str> = provider_definitions()
            .iter()
            .filter(|definition| definition.supports_extra_accounts)
            .map(|definition| definition.family_id)
            .collect();
        assert_eq!(actual, expected.to_vec());
        for family in expected {
            assert!(
                supports_extra_accounts(family),
                "{family} should support extra accounts"
            );
        }
        assert!(!supports_extra_accounts("claude"));
    }

    #[test]
    fn catalog_lists_every_api_key_settings_provider() {
        let expected = [
            "opencode",
            "minimax",
            "openrouter",
            "zai",
            "deepseek",
            "moonshot",
            "elevenlabs",
            "codebuff",
            "kilo",
            "aihubmix",
            "qwen",
            "kimi",
            "stepfun",
            "siliconflow",
            "novita",
            "relaybalance",
        ];
        let actual: Vec<&str> = provider_definitions()
            .iter()
            .filter(|definition| definition.supports_api_key)
            .map(|definition| definition.family_id)
            .collect();
        assert_eq!(actual, expected.to_vec());
    }

    #[test]
    fn catalog_exposes_query_kind_and_unknowns_are_absent() {
        assert_eq!(
            provider_definition("deepseek").map(|definition| definition.query_kind),
            Some(QueryKind::NativeBalance)
        );
        assert!(provider_definition("unknown").is_none());
        assert!(provider_definitions()
            .iter()
            .any(|definition| definition.family_id == "kimi"));
    }

    #[test]
    fn catalog_keeps_the_mandatory_api_key_query_families() {
        let expected = [
            ("kimi", QueryKind::Composite),
            ("stepfun", QueryKind::NativeBalance),
            ("siliconflow", QueryKind::NativeBalance),
            ("opencode", QueryKind::Composite),
            ("novita", QueryKind::NativeBalance),
            ("relaybalance", QueryKind::NativeBalance),
        ];
        for (family, query_kind) in expected {
            let definition = provider_definition(family)
                .unwrap_or_else(|| panic!("mandatory API-key family {family} is missing"));
            assert_eq!(
                definition.query_kind, query_kind,
                "unexpected route for {family}"
            );
        }
    }

    #[test]
    fn instance_routes_resolve_through_their_family() {
        assert_eq!(family_of("deepseek@1"), "deepseek");
        assert_eq!(family_of("relaybalance@1"), "relaybalance");
        assert_eq!(family_of("onenewapi@key-7"), "onenewapi");
        assert_eq!(family_of("kimi"), "kimi");
        assert_eq!(
            query_kind_for_instance("deepseek@1"),
            Some(QueryKind::NativeBalance)
        );
        assert_eq!(query_kind_for_instance("kimi"), Some(QueryKind::Composite));
        assert_eq!(
            query_kind_for_instance("onenewapi@key-7"),
            Some(QueryKind::Composite)
        );
        assert_eq!(query_kind_for_instance("unknown@1"), None);
    }
}
