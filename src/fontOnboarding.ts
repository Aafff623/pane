export type FontOnboarding = "new" | "upgrade" | "skip";

export function fontOnboardingMode(state: {
  uiFont: string; uiFontPromptVersion: string; lastSeenVersion: string; welcomeDismissed: boolean;
}, fonts: readonly string[]): FontOnboarding {
  if (state.uiFontPromptVersion && !state.uiFontPromptVersion.startsWith("pending-")) return "skip";
  if (state.uiFontPromptVersion === "pending-new") return "new";
  if (state.uiFontPromptVersion === "pending-upgrade") {
    return state.uiFont.trim() || !fonts.some((font) => font.toLowerCase() === "mv boli") ? "skip" : "upgrade";
  }
  if (!state.lastSeenVersion && !state.welcomeDismissed) return "new";
  if (state.uiFont.trim() || !fonts.some((font) => font.toLowerCase() === "mv boli")) return "skip";
  return "upgrade";
}
