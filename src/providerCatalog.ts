export type QueryKind =
  | "nativeSnapshot"
  | "nativeBalance"
  | "nativeCodingPlan"
  | "composite"
  | "localOnly";

export interface ProviderDefinition {
  familyId: string;
  displayName: string;
  queryKind: QueryKind;
  supportsApiKey: boolean;
  supportsExtraAccounts: boolean;
  /** Pane's own device-flow OAuth sign-in (gear panel "Sign in with browser"). */
  supportsOAuth: boolean;
  iconKey: string;
  /** Dashboard category. Absent = coding agent (the historical default). */
  category?: "coding" | "productivity" | "mcp";
}

export const providerCatalog: readonly ProviderDefinition[] = [
  { familyId: "claude", displayName: "Claude", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "claude" },
  { familyId: "codex", displayName: "Codex", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: true, iconKey: "codex" },
  { familyId: "cursor", displayName: "Cursor", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "cursor" },
  { familyId: "opencode", displayName: "OpenCode", queryKind: "composite", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "opencode" },
  { familyId: "copilot", displayName: "Copilot", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: true, iconKey: "copilot" },
  { familyId: "grok", displayName: "Grok", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: true, iconKey: "grok" },
  { familyId: "devin", displayName: "Devin", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "devin" },
  { familyId: "minimax", displayName: "MiniMax", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "minimax" },
  { familyId: "openrouter", displayName: "OpenRouter", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "openrouter" },
  { familyId: "zai", displayName: "Z.ai", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "zai" },
  { familyId: "antigravity", displayName: "Antigravity", queryKind: "composite", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "antigravity" },
  { familyId: "deepseek", displayName: "DeepSeek", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "deepseek" },
  { familyId: "moonshot", displayName: "Kimi API", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "kimi" },
  { familyId: "elevenlabs", displayName: "ElevenLabs", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "elevenlabs" },
  { familyId: "ollama", displayName: "Ollama", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "ollama" },
  { familyId: "codebuff", displayName: "Codebuff", queryKind: "nativeSnapshot", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "codebuff" },
  { familyId: "kilo", displayName: "Kilo", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "kilo" },
  { familyId: "aihubmix", displayName: "AihubMix", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "aihubmix" },
  { familyId: "onenewapi", displayName: "One/New API", queryKind: "composite", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "onenewapi" },
  { familyId: "qwen", displayName: "Qwen Code", queryKind: "composite", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "qwen" },
  { familyId: "hermes", displayName: "Hermes", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "hermes" },
  { familyId: "kimi", displayName: "Kimi Code", queryKind: "composite", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "kimi" },
  { familyId: "stepfun", displayName: "StepFun", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "stepfun" },
  { familyId: "stepfun-plan", displayName: "StepFun Step Plan", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "stepfun" },
  { familyId: "shandianshuo", displayName: "闪电说", queryKind: "nativeCodingPlan", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "shandianshuo", category: "productivity" },
  { familyId: "siliconflow", displayName: "SiliconFlow", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "siliconflow" },
  { familyId: "novita", displayName: "Novita AI", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "novita" },
  { familyId: "relaybalance", displayName: "Custom Relay", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "relaybalance" },
  { familyId: "qodercn", displayName: "Qoder CN", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "qodercn" },
  { familyId: "traecn", displayName: "Trae CN", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "traecn" },
  { familyId: "commandcode", displayName: "Command Code", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "commandcode" },
  { familyId: "doubao", displayName: "Doubao", queryKind: "nativeCodingPlan", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "doubao" },
  { familyId: "clawsgo", displayName: "ClawsGO", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "clawsgo" },
  { familyId: "bocha", displayName: "BochaAI", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "bocha", category: "mcp" },
  { familyId: "tavily", displayName: "Tavily", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "tavily", category: "mcp" },
  { familyId: "firecrawl", displayName: "Firecrawl", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "firecrawl", category: "mcp" },
  { familyId: "brave", displayName: "Brave Search", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "brave", category: "mcp" },
  { familyId: "clinepass", displayName: "ClinePass", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "clinepass" },
  { familyId: "sensenova", displayName: "SenseNova", queryKind: "nativeCodingPlan", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "sensenova" },
  { familyId: "apigoto", displayName: "APIGOTO", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "apigoto" },
  { familyId: "mcode", displayName: "MaxCode", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "mcode" },
  { familyId: "amp", displayName: "Amp", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "amp" },
  { familyId: "bedrock", displayName: "AWS Bedrock", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "bedrock" },
  { familyId: "chutes", displayName: "Chutes", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "chutes" },
  { familyId: "deepgram", displayName: "Deepgram", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "deepgram" },
  { familyId: "kiro", displayName: "Kiro", queryKind: "nativeCodingPlan", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "kiro" },
  { familyId: "openai-api", displayName: "OpenAI API", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "openai" },
  { familyId: "poe", displayName: "Poe", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "poe" },
  { familyId: "venice", displayName: "Venice", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "venice" },
  { familyId: "vertexai", displayName: "Vertex AI", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "vertexai" },
  { familyId: "warp", displayName: "Warp", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "warp" },
  { familyId: "mimo", displayName: "MiMo", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "mimo" },
  { familyId: "trae", displayName: "Trae", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "trae" },
  { familyId: "qoder", displayName: "Qoder", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "qoder" },
  { familyId: "zed", displayName: "Zed", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "zed" },
  { familyId: "factory", displayName: "Droid", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "factory" },
  { familyId: "jetbrains", displayName: "JetBrains AI", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "jetbrains" },
  { familyId: "groq", displayName: "Groq", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "groq" },
  { familyId: "huggingface", displayName: "Hugging Face", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "huggingface" },
  { familyId: "longcat", displayName: "LongCat", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "longcat" },
  { familyId: "sub2api", displayName: "sub2api", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "sub2api" },
  { familyId: "mistral", displayName: "Mistral", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "mistral" },
  { familyId: "perplexity", displayName: "Perplexity", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "perplexity" },
  { familyId: "volcengine", displayName: "Volcengine Ark", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "volcengine" },
];

export function providerFamily(id: string): string {
  return id.split("@")[0];
}

export function providerDefinition(familyId: string): ProviderDefinition | undefined {
  return providerCatalog.find((definition) => definition.familyId === familyId);
}

export type ProviderCategory = "coding" | "productivity" | "mcp";

export function providerCategory(familyId: string): ProviderCategory {
  return providerDefinition(familyId)?.category ?? "coding";
}

export function supportsApiKey(familyId: string): boolean {
  return providerDefinition(familyId)?.supportsApiKey ?? false;
}

export function supportsExtraAccounts(familyId: string): boolean {
  return providerDefinition(familyId)?.supportsExtraAccounts ?? false;
}
