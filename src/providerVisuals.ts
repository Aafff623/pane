import antigravityIcon from "./assets/providers/antigravity.svg?raw";
import aihubmixIcon from "./assets/providers/aihubmix.svg?raw";
import claudeIcon from "./assets/providers/claude.svg?raw";
import codexIcon from "./assets/providers/codex.svg?raw";
import copilotIcon from "./assets/providers/copilot.svg?raw";
import cursorIcon from "./assets/providers/cursor.svg?raw";
import deepseekIcon from "./assets/providers/deepseek.svg?raw";
import devinIcon from "./assets/providers/devin.svg?raw";
import grokIcon from "./assets/providers/grok.svg?raw";
import hermesIcon from "./assets/providers/hermes.svg?raw";
import kimiIcon from "./assets/providers/kimi.svg?raw";
import minimaxIcon from "./assets/providers/minimax.svg?raw";
import novitaIcon from "./assets/providers/novita.svg?raw";
import ollamaIcon from "./assets/providers/ollama.svg?raw";
import onenewapiIcon from "./assets/providers/onenewapi.svg?raw";
import opencodeIcon from "./assets/providers/opencode.svg?raw";
import openrouterIcon from "./assets/providers/openrouter.svg?raw";
import qwenIcon from "./assets/providers/qwen.svg?raw";
import sharkaiIcon from "./assets/providers/sharkai.svg?raw";
import siliconflowIcon from "./assets/providers/siliconflow.svg?raw";
import stepfunIcon from "./assets/providers/stepfun.svg?raw";
import zaiIcon from "./assets/providers/zai.svg?raw";
import { providerDefinition, providerFamily } from "./providerCatalog";

export interface ProviderVisual {
  iconKey: string;
  iconSvg: string;
  iconColor?: string;
  invertOnDarkTray?: boolean;
  recolorOnTray?: boolean;
}

const VISUALS: Readonly<Record<string, ProviderVisual>> = {
  antigravity: { iconKey: "antigravity", iconSvg: antigravityIcon },
  aihubmix: { iconKey: "aihubmix", iconSvg: aihubmixIcon },
  claude: { iconKey: "claude", iconSvg: claudeIcon },
  codex: { iconKey: "codex", iconSvg: codexIcon },
  copilot: { iconKey: "copilot", iconSvg: copilotIcon },
  cursor: { iconKey: "cursor", iconSvg: cursorIcon },
  deepseek: { iconKey: "deepseek", iconSvg: deepseekIcon },
  devin: { iconKey: "devin", iconSvg: devinIcon },
  grok: { iconKey: "grok", iconSvg: grokIcon },
  hermes: { iconKey: "hermes", iconSvg: hermesIcon },
  kimi: { iconKey: "kimi", iconSvg: kimiIcon },
  minimax: { iconKey: "minimax", iconSvg: minimaxIcon },
  novita: { iconKey: "novita", iconSvg: novitaIcon, invertOnDarkTray: true },
  ollama: { iconKey: "ollama", iconSvg: ollamaIcon },
  onenewapi: { iconKey: "onenewapi", iconSvg: onenewapiIcon },
  sharkai: { iconKey: "sharkai", iconSvg: sharkaiIcon },
  opencode: { iconKey: "opencode", iconSvg: opencodeIcon },
  openrouter: { iconKey: "openrouter", iconSvg: openrouterIcon },
  qwen: { iconKey: "qwen", iconSvg: qwenIcon },
  siliconflow: { iconKey: "siliconflow", iconSvg: siliconflowIcon },
  stepfun: { iconKey: "stepfun", iconSvg: stepfunIcon },
  zai: { iconKey: "zai", iconSvg: zaiIcon },
};

/// Known One/New API hosts that ship their own colorful mark.  Keyed by hostname
/// (lower-case) so the site-owner SharkAI instance gets its own card icon.
const ONENEWSITE_ICONS: Readonly<Record<string, keyof typeof VISUALS>> = {
  "api2.sharkai.cc": "sharkai",
};

function snapshotIconKey(id: string, origin?: string): string | null {
  if (providerFamily(id) !== "onenewapi") return null;
  if (!origin) return null;
  try {
    const host = new URL(origin).hostname.toLowerCase();
    return ONENEWSITE_ICONS[host] ?? null;
  } catch {
    return null;
  }
}

/** Resolve the ProviderVisual for a snapshot.  Pass `origin` (from the snapshot)
 * so onenewapi sites with a known brand get their branded icon. */
export function providerVisual(id: string, origin?: string): ProviderVisual | undefined {
  const override = snapshotIconKey(id, origin);
  if (override) return VISUALS[override];
  const family = providerFamily(id);
  const key = providerDefinition(family)?.iconKey ?? family;
  return VISUALS[key] ?? VISUALS[family];
}

// Kept as a small compatibility projection for the existing render helpers.
export const PROVIDER_ICONS: Readonly<Record<string, string>> = Object.fromEntries(
  Object.entries(VISUALS).map(([key, visual]) => [key, visual.iconSvg]),
);

export const TRAIL_RECOLOR_ICONS = new Set(
  Object.entries(VISUALS)
    .filter(([, visual]) => visual.recolorOnTray)
    .map(([key]) => key),
);

export const TRAIL_INVERT_DARK_ICONS = new Set(
  Object.entries(VISUALS)
    .filter(([, visual]) => visual.invertOnDarkTray)
    .map(([key]) => key),
);
