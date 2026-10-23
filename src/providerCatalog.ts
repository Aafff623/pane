import type { UsageUnit } from "./usageUnits.ts";

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
  primaryUnit: UsageUnit;
  supportsApiKey: boolean;
  supportsExtraAccounts: boolean;
  /** Pane's own device-flow OAuth sign-in (gear panel "Sign in with browser"). */
  supportsOAuth: boolean;
  /** Credentials come from Pane's hosted web-login window, not an API key. */
  webSession?: boolean;
  iconKey: string;
  /** Dashboard category. Absent = coding agent (the historical default). */
  category?: "coding" | "mcp" | "productivity";
}

/** Every category the catalog can name, in stable chip order — the two
 *  built-ins first, extras after. Sections render an extra only when a
 *  visible provider carries it (闪电说's productivity), so a fresh install
 *  still shows just the two built-in groups. */
export const CATALOG_CATEGORIES = ["coding", "mcp", "productivity"] as const;

export const providerCatalog: readonly ProviderDefinition[] = [
  { familyId: "claude", displayName: "Claude", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "claude" },
  { familyId: "codex", displayName: "Codex", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "codex" },
  { familyId: "codebuddy", displayName: "CodeBuddy", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "codebuddy" },
  { familyId: "codebuddycn", displayName: "CodeBuddy CN", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "codebuddy" },
  { familyId: "workbuddy", displayName: "WorkBuddy", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "workbuddy" },
  { familyId: "workbuddyai", displayName: "WorkBuddy AI", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "workbuddy" },
  { familyId: "cursor", displayName: "Cursor", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "cursor" },
  { familyId: "opencode", displayName: "OpenCode", primaryUnit: "tokens", queryKind: "composite", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "opencode" },
  { familyId: "copilot", displayName: "Copilot", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "copilot" },
  { familyId: "grok", displayName: "Grok", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "grok" },
  { familyId: "devin", displayName: "Devin", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "devin" },
  { familyId: "minimax", displayName: "MiniMax", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "minimax" },
  { familyId: "openrouter", displayName: "OpenRouter", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "openrouter" },
  { familyId: "zai", displayName: "Z.ai", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "zai" },
  { familyId: "antigravity", displayName: "Antigravity", primaryUnit: "tokens", queryKind: "composite", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "antigravity" },
  { familyId: "deepseek", displayName: "DeepSeek", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "deepseek" },
  { familyId: "moonshot", displayName: "Kimi API", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "kimi" },
  { familyId: "elevenlabs", displayName: "ElevenLabs", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "elevenlabs" },
  { familyId: "ollama", displayName: "Ollama", primaryUnit: "tokens", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "ollama" },
  { familyId: "codebuff", displayName: "Codebuff", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "codebuff" },
  { familyId: "kilo", displayName: "Kilo", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "kilo" },
  { familyId: "aihubmix", displayName: "AihubMix", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "aihubmix" },
  { familyId: "onenewapi", displayName: "One/New API", primaryUnit: "tokens", queryKind: "composite", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "onenewapi" },
  { familyId: "qwen", displayName: "Qwen Code", primaryUnit: "tokens", queryKind: "composite", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "qwen" },
  { familyId: "hermes", displayName: "Hermes", primaryUnit: "tokens", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "hermes" },
  { familyId: "kimi", displayName: "Kimi Code", primaryUnit: "tokens", queryKind: "composite", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "kimi" },
  { familyId: "stepfun", displayName: "StepFun", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "stepfun" },
  { familyId: "stepfun-plan", displayName: "StepFun Step Plan", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "stepfun" },
  { familyId: "shandianshuo", displayName: "闪电说", primaryUnit: "minutes", queryKind: "nativeCodingPlan", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "shandianshuo", category: "productivity" },
  { familyId: "siliconflow", displayName: "SiliconFlow", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "siliconflow" },
  { familyId: "novita", displayName: "Novita AI", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "novita" },
  { familyId: "relaybalance", displayName: "Custom Relay", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "relaybalance" },
  { familyId: "qodercn", displayName: "Qoder CN", primaryUnit: "credits", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "qodercn" },
  { familyId: "traecn", displayName: "Trae CN", primaryUnit: "credits", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "traecn" },
  { familyId: "commandcode", displayName: "Command Code", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "commandcode" },
  { familyId: "doubao", displayName: "Doubao", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "doubao" },
  { familyId: "clawsgo", displayName: "ClawsGO", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "clawsgo" },
  { familyId: "bocha", displayName: "BochaAI", primaryUnit: "credits", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "bocha", category: "mcp" },
  { familyId: "tavily", displayName: "Tavily", primaryUnit: "credits", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "tavily", category: "mcp" },
  { familyId: "firecrawl", displayName: "Firecrawl", primaryUnit: "credits", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "firecrawl", category: "mcp" },
  { familyId: "brave", displayName: "Brave Search", primaryUnit: "credits", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "brave", category: "mcp" },
  { familyId: "giphy", displayName: "GIPHY", primaryUnit: "requests", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "giphy", category: "mcp" },
  { familyId: "clinepass", displayName: "ClinePass", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "clinepass" },
  { familyId: "sensenova", displayName: "SenseNova", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "sensenova" },
  { familyId: "apigoto", displayName: "APIGOTO", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "apigoto" },
  { familyId: "mcode", displayName: "MiniMax Code", primaryUnit: "tokens", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "mcode" },
  { familyId: "amp", displayName: "Amp", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "amp" },
  { familyId: "bedrock", displayName: "AWS Bedrock", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "bedrock" },
  { familyId: "chutes", displayName: "Chutes", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "chutes" },
  { familyId: "deepgram", displayName: "Deepgram", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "deepgram" },
  { familyId: "kiro", displayName: "Kiro", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "kiro" },
  { familyId: "openai-api", displayName: "OpenAI API", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "openai" },
  { familyId: "poe", displayName: "Poe", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "poe" },
  { familyId: "tokenrhythm", displayName: "基元律动", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, webSession: true, iconKey: "tokenrhythm" },
  { familyId: "xfyun", displayName: "讯飞星火", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, webSession: true, iconKey: "xfyun" },
  { familyId: "venice", displayName: "Venice", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "venice" },
  { familyId: "vertexai", displayName: "Vertex AI", primaryUnit: "tokens", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "vertexai" },
  { familyId: "warp", displayName: "Warp", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "warp" },
  { familyId: "windsurf", displayName: "Windsurf", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "windsurf" },
  { familyId: "mimo", displayName: "MiMo", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "mimo" },
  { familyId: "trae", displayName: "Trae", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "trae" },
  { familyId: "qoder", displayName: "Qoder", primaryUnit: "tokens", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "qoder" },
  { familyId: "zed", displayName: "Zed", primaryUnit: "tokens", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: true, iconKey: "zed" },
  { familyId: "factory", displayName: "Droid", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "factory" },
  { familyId: "jetbrains", displayName: "JetBrains AI", primaryUnit: "tokens", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "jetbrains" },
  { familyId: "groq", displayName: "Groq", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "groq" },
  { familyId: "huggingface", displayName: "Hugging Face", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "huggingface" },
  { familyId: "longcat", displayName: "LongCat", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "longcat" },
  { familyId: "sub2api", displayName: "sub2api", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "sub2api" },
  { familyId: "mistral", displayName: "Mistral", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "mistral" },
  { familyId: "perplexity", displayName: "Perplexity", primaryUnit: "tokens", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "perplexity" },
  { familyId: "volcengine", displayName: "Volcengine Ark", primaryUnit: "tokens", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "volcengine" },
];

export function providerFamily(id: string): string {
  return id.split("@")[0];
}

export function providerDefinition(familyId: string): ProviderDefinition | undefined {
  return providerCatalog.find((definition) => definition.familyId === familyId);
}

export type ProviderCategory = "coding" | "mcp" | "productivity";

export function providerCategory(familyId: string): ProviderCategory {
  return providerDefinition(familyId)?.category ?? "coding";
}

export function supportsApiKey(familyId: string): boolean {
  return providerDefinition(familyId)?.supportsApiKey ?? false;
}

export function supportsExtraAccounts(familyId: string): boolean {
  return providerDefinition(familyId)?.supportsExtraAccounts ?? false;
}

export function isWebSessionFamily(familyId: string): boolean {
  return providerDefinition(familyId)?.webSession === true;
}
