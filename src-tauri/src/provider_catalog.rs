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

/// Dashboard grouping: coding agents are the historical default; anything
/// that is not an AI coding tool (voice assistants, document/MCP/search
/// utilities) lands in Productivity, and quota-metered MCP/search services
/// land in Mcp. Keep in sync with `src/providerCatalog.ts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderCategory {
    Coding,
    Productivity,
    Mcp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderDefinition {
    pub family_id: &'static str,
    pub display_name: &'static str,
    pub query_kind: QueryKind,
    pub supports_api_key: bool,
    pub supports_extra_accounts: bool,
    pub icon_key: &'static str,
    pub category: ProviderCategory,
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
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "codex",
        display_name: "Codex",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        // Pane-managed logins (auth center) are independent accounts, so the
        // family is account-aware; the frontend renders them as parallel
        // cards like Antigravity's slots rather than merged tabs.
        supports_extra_accounts: true,
        icon_key: "codex",
        category: ProviderCategory::Coding,
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
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "opencode",
        display_name: "OpenCode",
        query_kind: QueryKind::Composite,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "opencode",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "copilot",
        display_name: "Copilot",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        // Pane-managed GitHub logins (auth center) are independent
        // accounts — same parallel-card treatment as Codex's logins.
        supports_extra_accounts: true,
        icon_key: "copilot",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "grok",
        display_name: "Grok",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        // Pane-managed xAI logins (auth center) are independent accounts —
        // same parallel-card treatment as Codex's logins.
        supports_extra_accounts: true,
        icon_key: "grok",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "devin",
        display_name: "Devin",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "devin",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "minimax",
        display_name: "MiniMax",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "minimax",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "openrouter",
        display_name: "OpenRouter",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "openrouter",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "zai",
        display_name: "Z.ai",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "zai",
        category: ProviderCategory::Coding,
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
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "deepseek",
        display_name: "DeepSeek",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "deepseek",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "moonshot",
        display_name: "Kimi API",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "kimi",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "elevenlabs",
        display_name: "ElevenLabs",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "elevenlabs",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "ollama",
        display_name: "Ollama",
        query_kind: QueryKind::LocalOnly,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "ollama",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "codebuff",
        display_name: "Codebuff",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "codebuff",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "kilo",
        display_name: "Kilo",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "kilo",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "aihubmix",
        display_name: "AihubMix",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "aihubmix",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "onenewapi",
        display_name: "One/New API",
        query_kind: QueryKind::Composite,
        supports_api_key: false,
        // Relay sites are accounts: one merged family card with a tab per
        // site key (or per token-only site), like the Kimi account model.
        supports_extra_accounts: true,
        icon_key: "onenewapi",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "qwen",
        display_name: "Qwen Code",
        query_kind: QueryKind::Composite,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "qwen",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "hermes",
        display_name: "Hermes",
        query_kind: QueryKind::LocalOnly,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "hermes",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "kimi",
        display_name: "Kimi Code",
        query_kind: QueryKind::Composite,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "kimi",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "stepfun",
        display_name: "StepFun",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "stepfun",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "stepfun-plan",
        display_name: "StepFun Step Plan",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "stepfun",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "shandianshuo",
        display_name: "闪电说",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "shandianshuo",
        category: ProviderCategory::Productivity,
    },
    ProviderDefinition {
        family_id: "siliconflow",
        display_name: "SiliconFlow",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "siliconflow",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "novita",
        display_name: "Novita AI",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "novita",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "relaybalance",
        display_name: "Custom Relay",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "relaybalance",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "qodercn",
        display_name: "Qoder CN",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "qodercn",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "traecn",
        display_name: "Trae CN",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "traecn",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "commandcode",
        display_name: "Command Code",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        // Several GOAT subscriptions side by side (commandcode@fp cards).
        supports_extra_accounts: true,
        icon_key: "commandcode",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "doubao",
        display_name: "Doubao",
        query_kind: QueryKind::NativeCodingPlan,
        // Web-session provider: sign-in cookies from the desktop app, no
        // API key exists.
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "doubao",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "clawsgo",
        display_name: "ClawsGO",
        query_kind: QueryKind::NativeCodingPlan,
        // Web-session token from the browser's localStorage — pasted as
        // the "API key".
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "clawsgo",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "bocha",
        display_name: "BochaAI",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "bocha",
        category: ProviderCategory::Mcp,
    },
    ProviderDefinition {
        family_id: "tavily",
        display_name: "Tavily",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "tavily",
        category: ProviderCategory::Mcp,
    },
    ProviderDefinition {
        family_id: "firecrawl",
        display_name: "Firecrawl",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "firecrawl",
        category: ProviderCategory::Mcp,
    },
    ProviderDefinition {
        family_id: "clinepass",
        display_name: "ClinePass",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "clinepass",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "sensenova",
        display_name: "SenseNova",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "sensenova",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "apigoto",
        display_name: "APIGOTO",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "apigoto",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "brave",
        display_name: "Brave Search",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "brave",
        category: ProviderCategory::Mcp,
    },
    ProviderDefinition {
        family_id: "amp",
        display_name: "Amp",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "amp",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "bedrock",
        display_name: "AWS Bedrock",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "bedrock",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "chutes",
        display_name: "Chutes",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "chutes",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "deepgram",
        display_name: "Deepgram",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "deepgram",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "kiro",
        display_name: "Kiro",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: false,
        // Pane-managed Kiro logins (auth center, browser PKCE) are
        // independent accounts — same parallel-card treatment as Codex.
        supports_extra_accounts: true,
        icon_key: "kiro",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "openai-api",
        display_name: "OpenAI API",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "openai",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "poe",
        display_name: "Poe",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "poe",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "venice",
        display_name: "Venice",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "venice",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "vertexai",
        display_name: "Vertex AI",
        query_kind: QueryKind::LocalOnly,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "vertexai",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "warp",
        display_name: "Warp",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "warp",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "mimo",
        display_name: "MiMo",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "mimo",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "trae",
        display_name: "Trae",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "trae",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "qoder",
        display_name: "Qoder",
        query_kind: QueryKind::NativeSnapshot,
        supports_api_key: false,
        // Pane-managed Qoder logins (auth center, device login) are
        // independent accounts — same parallel-card treatment as Codex.
        supports_extra_accounts: true,
        icon_key: "qoder",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "zed",
        display_name: "Zed",
        query_kind: QueryKind::LocalOnly,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "zed",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "factory",
        display_name: "Droid",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "factory",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "jetbrains",
        display_name: "JetBrains AI",
        query_kind: QueryKind::LocalOnly,
        supports_api_key: false,
        supports_extra_accounts: false,
        icon_key: "jetbrains",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "groq",
        display_name: "Groq",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "groq",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "huggingface",
        display_name: "Hugging Face",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "huggingface",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "longcat",
        display_name: "LongCat",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "longcat",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "sub2api",
        display_name: "sub2api",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "sub2api",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "mistral",
        display_name: "Mistral",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "mistral",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "perplexity",
        display_name: "Perplexity",
        query_kind: QueryKind::NativeBalance,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "perplexity",
        category: ProviderCategory::Coding,
    },
    ProviderDefinition {
        family_id: "volcengine",
        display_name: "Volcengine Ark",
        query_kind: QueryKind::NativeCodingPlan,
        supports_api_key: true,
        supports_extra_accounts: true,
        icon_key: "volcengine",
        category: ProviderCategory::Coding,
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

/// Families whose saved credential also carries a user-chosen relay base
/// URL (stored alongside the key, hashed into account card ids). Shared by
/// lib.rs's save paths and accounts.rs so the two can't drift apart.
pub fn takes_base_url(family_id: &str) -> bool {
    matches!(family_id, "relaybalance" | "sub2api")
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
        supports_extra_accounts, takes_base_url, QueryKind,
    };

    #[test]
    fn catalog_marks_the_multi_account_families() {
        // Every API-key family (sub-accounts = separate keys), plus
        // Antigravity's captured OAuth slots, Cursor's imported logins,
        // the One/New API relay sites (site = account), and the
        // Pane-managed login families (copilot/grok).
        let expected = [
            "codex",
            "cursor",
            "opencode",
            "copilot",
            "grok",
            "minimax",
            "openrouter",
            "zai",
            "antigravity",
            "deepseek",
            "moonshot",
            "elevenlabs",
            "codebuff",
            "kilo",
            "aihubmix",
            "onenewapi",
            "qwen",
            "kimi",
            "stepfun",
            "stepfun-plan",
            "siliconflow",
            "novita",
            "relaybalance",
            "commandcode",
            "clawsgo",
            "clinepass",
            "apigoto",
            "amp",
            "chutes",
            "deepgram",
            "kiro",
            "openai-api",
            "poe",
            "venice",
            "warp",
            "mimo",
            "qoder",
            "factory",
            "groq",
            "huggingface",
            "longcat",
            "sub2api",
            "mistral",
            "perplexity",
            "volcengine",
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
    fn relay_base_url_families_are_flagged() {
        assert!(takes_base_url("relaybalance"));
        assert!(takes_base_url("sub2api"));
        assert!(!takes_base_url("deepseek"));
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
            "stepfun-plan",
            "siliconflow",
            "novita",
            "relaybalance",
            "commandcode",
            "clawsgo",
            "clinepass",
            "apigoto",
            "amp",
            "chutes",
            "deepgram",
            "openai-api",
            "poe",
            "venice",
            "warp",
            "mimo",
            "factory",
            "groq",
            "huggingface",
            "longcat",
            "sub2api",
            "mistral",
            "perplexity",
            "volcengine",
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
            ("stepfun-plan", QueryKind::NativeCodingPlan),
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
