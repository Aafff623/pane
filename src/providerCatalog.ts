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
}

export const providerCatalog: readonly ProviderDefinition[] = [
  { familyId: "claude", displayName: "Claude", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "claude" },
  { familyId: "codex", displayName: "Codex", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: true, iconKey: "codex" },
  { familyId: "cursor", displayName: "Cursor", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "cursor" },
  { familyId: "opencode", displayName: "OpenCode", queryKind: "composite", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "opencode" },
  { familyId: "copilot", displayName: "Copilot", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: true, iconKey: "copilot" },
  { familyId: "grok", displayName: "Grok", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: true, iconKey: "grok" },
  { familyId: "devin", displayName: "Devin", queryKind: "nativeSnapshot", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "devin" },
  { familyId: "minimax", displayName: "MiniMax", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "minimax" },
  { familyId: "openrouter", displayName: "OpenRouter", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "openrouter" },
  { familyId: "zai", displayName: "Z.ai", queryKind: "nativeCodingPlan", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "zai" },
  { familyId: "antigravity", displayName: "Antigravity", queryKind: "composite", supportsApiKey: false, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "antigravity" },
  { familyId: "deepseek", displayName: "DeepSeek", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "deepseek" },
  { familyId: "moonshot", displayName: "Kimi API", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "kimi" },
  { familyId: "elevenlabs", displayName: "ElevenLabs", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "elevenlabs" },
  { familyId: "ollama", displayName: "Ollama", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "ollama" },
  { familyId: "codebuff", displayName: "Codebuff", queryKind: "nativeSnapshot", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "codebuff" },
  { familyId: "kilo", displayName: "Kilo", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "kilo" },
  { familyId: "aihubmix", displayName: "AihubMix", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "aihubmix" },
  { familyId: "onenewapi", displayName: "One/New API", queryKind: "composite", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "onenewapi" },
  { familyId: "qwen", displayName: "Qwen Code", queryKind: "composite", supportsApiKey: true, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "qwen" },
  { familyId: "hermes", displayName: "Hermes", queryKind: "localOnly", supportsApiKey: false, supportsExtraAccounts: false, supportsOAuth: false, iconKey: "hermes" },
  { familyId: "kimi", displayName: "Kimi Code", queryKind: "composite", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "kimi" },
  { familyId: "stepfun", displayName: "StepFun", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "stepfun" },
  { familyId: "siliconflow", displayName: "SiliconFlow", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "siliconflow" },
  { familyId: "novita", displayName: "Novita AI", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "novita" },
  { familyId: "relaybalance", displayName: "Custom Balance", queryKind: "nativeBalance", supportsApiKey: true, supportsExtraAccounts: true, supportsOAuth: false, iconKey: "relaybalance" },
];

export function providerFamily(id: string): string {
  return id.split("@")[0];
}

export function providerDefinition(familyId: string): ProviderDefinition | undefined {
  return providerCatalog.find((definition) => definition.familyId === familyId);
}

export function supportsApiKey(familyId: string): boolean {
  return providerDefinition(familyId)?.supportsApiKey ?? false;
}

export function supportsExtraAccounts(familyId: string): boolean {
  return providerDefinition(familyId)?.supportsExtraAccounts ?? false;
}
