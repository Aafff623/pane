import antigravityIcon from "./assets/providers/antigravity.svg?raw";
import aihubmixIcon from "./assets/providers/aihubmix.svg?raw";
import claudeIcon from "./assets/providers/claude.svg?raw";
import clawsgoIcon from "./assets/providers/clawsgo.svg?raw";
import codebuffIcon from "./assets/providers/codebuff.svg?raw";
import codebuddyIcon from "./assets/providers/codebuddy.svg?raw";
import codexIcon from "./assets/providers/codex.svg?raw";
import windsurfIcon from "./assets/providers/windsurf.svg?raw";
import commandcodeIcon from "./assets/providers/commandcode.svg?raw";
import clinepassIcon from "./assets/providers/clinepass.svg?raw";
import copilotIcon from "./assets/providers/copilot.svg?raw";
import cursorIcon from "./assets/providers/cursor.svg?raw";
import deepseekIcon from "./assets/providers/deepseek.svg?raw";
import devinIcon from "./assets/providers/devin.svg?raw";
import doubaoIcon from "./assets/providers/doubao.svg?raw";
import elevenlabsIcon from "./assets/providers/elevenlabs.svg?raw";
import grokIcon from "./assets/providers/grok.svg?raw";
import hermesIcon from "./assets/providers/hermes.svg?raw";
import kiloIcon from "./assets/providers/kilo.svg?raw";
import kimiIcon from "./assets/providers/kimi.svg?raw";
import minimaxIcon from "./assets/providers/minimax.svg?raw";
import mcodeIcon from "./assets/providers/mcode.svg?raw";
import zcodeIcon from "./assets/providers/zcode.svg?raw";
import novitaIcon from "./assets/providers/novita.svg?raw";
import ollamaIcon from "./assets/providers/ollama.svg?raw";
import onenewapiIcon from "./assets/providers/onenewapi.svg?raw";
import opencodeIcon from "./assets/providers/opencode.svg?raw";
import openrouterIcon from "./assets/providers/openrouter.svg?raw";
import qwenIcon from "./assets/providers/qwen.svg?raw";
import qodercnIcon from "./assets/providers/qodercn.svg?raw";
import relaybalanceIcon from "./assets/providers/relaybalance.svg?raw";
import sensenovaIcon from "./assets/providers/sensenova.svg?raw";
import traecnIcon from "./assets/providers/traecn.svg?raw";
import sharkaiIcon from "./assets/providers/sharkai.svg?raw";
import siliconflowIcon from "./assets/providers/siliconflow.svg?raw";
import stepfunIcon from "./assets/providers/stepfun.svg?raw";
import zaiIcon from "./assets/providers/zai.svg?raw";
import bochaIcon from "./assets/providers/bocha.svg?raw";
import tavilyIcon from "./assets/providers/tavily.svg?raw";
import firecrawlIcon from "./assets/providers/firecrawl.svg?raw";
import braveIcon from "./assets/providers/brave.svg?raw";
import apigotoIcon from "./assets/providers/apigoto.svg?raw";
import shandianshuoIcon from "./assets/providers/shandianshuo.svg?raw";
import { providerDefinition, providerFamily } from "./providerCatalog";
import ampIcon from "./assets/providers/amp.svg?raw";
import bedrockIcon from "./assets/providers/bedrock.svg?raw";
import chutesIcon from "./assets/providers/chutes.svg?raw";
import deepgramIcon from "./assets/providers/deepgram.svg?raw";
import kiroIcon from "./assets/providers/kiro.svg?raw";
import openaiIcon from "./assets/providers/openai.svg?raw";
import poeIcon from "./assets/providers/poe.svg?raw";
import veniceIcon from "./assets/providers/venice.svg?raw";
import vertexaiIcon from "./assets/providers/vertexai.svg?raw";
import warpIcon from "./assets/providers/warp.svg?raw";
import mimoIcon from "./assets/providers/mimo.svg?raw";
import traeIcon from "./assets/providers/trae.svg?raw";
import qoderIcon from "./assets/providers/qoder.svg?raw";
import zedIcon from "./assets/providers/zed.svg?raw";
import factoryIcon from "./assets/providers/factory.svg?raw";
import jetbrainsIcon from "./assets/providers/jetbrains.svg?raw";
import groqIcon from "./assets/providers/groq.svg?raw";
import huggingfaceIcon from "./assets/providers/huggingface.svg?raw";
import longcatIcon from "./assets/providers/longcat.svg?raw";
import sub2apiIcon from "./assets/providers/sub2api.svg?raw";
import mistralIcon from "./assets/providers/mistral.svg?raw";
import perplexityIcon from "./assets/providers/perplexity.svg?raw";
import volcengineIcon from "./assets/providers/volcengine.svg?raw";

export interface ProviderVisual {
  iconKey: string;
  iconSvg: string;
  iconColor?: string;
  invertOnDarkTray?: boolean;
  recolorOnTray?: boolean;
}

/// Near-black brand marks become white silhouettes on the dark theme
/// (brightness(0) invert(1) via CSS); Groq's white mark becomes a black
/// silhouette on the light theme instead. Class is baked into the raw svg
/// so every render site (settings rows, customize, tray) gets it for free.
const darkSilhouette = (svg: string): string => svg.replace(/<svg\b/i, '<svg class="pv-silhouette-dark"');
const lightSilhouette = (svg: string): string => svg.replace(/<svg\b/i, '<svg class="pv-silhouette-light"');


const VISUALS: Readonly<Record<string, ProviderVisual>> = {
  antigravity: { iconKey: "antigravity", iconSvg: antigravityIcon },
  aihubmix: { iconKey: "aihubmix", iconSvg: aihubmixIcon },
  claude: { iconKey: "claude", iconSvg: claudeIcon },
  clinepass: { iconKey: "clinepass", iconSvg: clinepassIcon },
  codebuff: { iconKey: "codebuff", iconSvg: codebuffIcon },
  codebuddy: { iconKey: "codebuddy", iconSvg: codebuddyIcon },
  codex: { iconKey: "codex", iconSvg: codexIcon },
  windsurf: { iconKey: "windsurf", iconSvg: windsurfIcon },
  copilot: { iconKey: "copilot", iconSvg: copilotIcon },
  cursor: { iconKey: "cursor", iconSvg: cursorIcon },
  deepseek: { iconKey: "deepseek", iconSvg: deepseekIcon },
  devin: { iconKey: "devin", iconSvg: devinIcon },
  grok: { iconKey: "grok", iconSvg: grokIcon },
  hermes: { iconKey: "hermes", iconSvg: hermesIcon },
  kilo: { iconKey: "kilo", iconSvg: kiloIcon },
  kimi: { iconKey: "kimi", iconSvg: kimiIcon },
  minimax: { iconKey: "minimax", iconSvg: minimaxIcon },
  mcode: { iconKey: "mcode", iconSvg: mcodeIcon },
  zcode: { iconKey: "zcode", iconSvg: zcodeIcon },
  novita: { iconKey: "novita", iconSvg: darkSilhouette(novitaIcon), invertOnDarkTray: true },
  ollama: { iconKey: "ollama", iconSvg: ollamaIcon },
  onenewapi: { iconKey: "onenewapi", iconSvg: onenewapiIcon },
  sharkai: { iconKey: "sharkai", iconSvg: sharkaiIcon },
  opencode: { iconKey: "opencode", iconSvg: opencodeIcon },
  openrouter: { iconKey: "openrouter", iconSvg: openrouterIcon },
  qwen: { iconKey: "qwen", iconSvg: qwenIcon },
  siliconflow: { iconKey: "siliconflow", iconSvg: siliconflowIcon },
  stepfun: { iconKey: "stepfun", iconSvg: stepfunIcon },
  zai: { iconKey: "zai", iconSvg: zaiIcon },
  qodercn: { iconKey: "qodercn", iconSvg: qodercnIcon },
  traecn: { iconKey: "traecn", iconSvg: traecnIcon },
  commandcode: { iconKey: "commandcode", iconSvg: commandcodeIcon },
  doubao: { iconKey: "doubao", iconSvg: doubaoIcon },
  elevenlabs: { iconKey: "elevenlabs", iconSvg: elevenlabsIcon },
  relaybalance: { iconKey: "relaybalance", iconSvg: relaybalanceIcon },
  sensenova: { iconKey: "sensenova", iconSvg: sensenovaIcon },
  clawsgo: { iconKey: "clawsgo", iconSvg: clawsgoIcon },
  shandianshuo: { iconKey: "shandianshuo", iconSvg: shandianshuoIcon },
  bocha: { iconKey: "bocha", iconSvg: bochaIcon },
  tavily: { iconKey: "tavily", iconSvg: tavilyIcon },
  firecrawl: { iconKey: "firecrawl", iconSvg: firecrawlIcon },
  brave: { iconKey: "brave", iconSvg: braveIcon },
  apigoto: { iconKey: "apigoto", iconSvg: apigotoIcon },
  amp: { iconKey: "amp", iconSvg: ampIcon },
  bedrock: { iconKey: "bedrock", iconSvg: bedrockIcon },
  chutes: { iconKey: "chutes", iconSvg: chutesIcon },
  deepgram: { iconKey: "deepgram", iconSvg: deepgramIcon },
  kiro: { iconKey: "kiro", iconSvg: kiroIcon },
  "openai-api": { iconKey: "openai", iconSvg: openaiIcon },
  poe: { iconKey: "poe", iconSvg: poeIcon },
  venice: { iconKey: "venice", iconSvg: veniceIcon },
  vertexai: { iconKey: "vertexai", iconSvg: vertexaiIcon },
  warp: { iconKey: "warp", iconSvg: warpIcon },
  mimo: { iconKey: "mimo", iconSvg: mimoIcon },
  trae: { iconKey: "trae", iconSvg: traeIcon },
  qoder: { iconKey: "qoder", iconSvg: darkSilhouette(qoderIcon) },
  zed: { iconKey: "zed", iconSvg: zedIcon },
  factory: { iconKey: "factory", iconSvg: darkSilhouette(factoryIcon) },
  jetbrains: { iconKey: "jetbrains", iconSvg: jetbrainsIcon },
  groq: { iconKey: "groq", iconSvg: lightSilhouette(groqIcon) },
  huggingface: { iconKey: "huggingface", iconSvg: huggingfaceIcon },
  longcat: { iconKey: "longcat", iconSvg: longcatIcon },
  sub2api: { iconKey: "sub2api", iconSvg: sub2apiIcon },
  mistral: { iconKey: "mistral", iconSvg: mistralIcon },
  perplexity: { iconKey: "perplexity", iconSvg: perplexityIcon },
  volcengine: { iconKey: "volcengine", iconSvg: volcengineIcon },
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
