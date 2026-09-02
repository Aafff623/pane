import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import {
  applyStaticI18n,
  displayLinkLabel,
  displayMetricDetail,
  displayMetricLabel,
  localeTag,
  normalizeLocalePref,
  resolveLocale,
  setActiveLocale,
  setSystemLocale,
  t,
  type Locale,
  type LocalePref,
} from "./i18n";

// Injected by vite.config.ts at build time, e.g. "0707.1432".
declare const __BUILD_STAMP__: string;

// Official provider marks from the MIT-licensed macOS OpenUsage, rendered
// inline so CSS can recolor them like template icons.
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
import siliconflowIcon from "./assets/providers/siliconflow.svg?raw";
import stepfunIcon from "./assets/providers/stepfun.svg?raw";
// Inlined as data URIs (not URLs) so the share-card SVG snapshot can
// embed them — rasterized SVG images can't load external resources.
// The bare ring suits the sidebar; the footer uses the full rounded
// app icon, which stays legible at tiny sizes.
import paneLogo from "./assets/pane-logo.png?inline";
import paneIcon from "./assets/pane-icon.png?inline";
import zaiIcon from "./assets/providers/zai.svg?raw";
// The repo's changelog ships inside the bundle, so the "What's new" dialog
// and the Settings changelog viewer read the exact file releases maintain.
import changelogRaw from "../CHANGELOG.md?raw";

const PROVIDER_ICONS: Record<string, string> = {
  antigravity: antigravityIcon,
  aihubmix: aihubmixIcon,
  claude: claudeIcon,
  codex: codexIcon,
  copilot: copilotIcon,
  cursor: cursorIcon,
  deepseek: deepseekIcon,
  devin: devinIcon,
  grok: grokIcon,
  hermes: hermesIcon,
  kimi: kimiIcon,
  minimax: minimaxIcon,
  novita: novitaIcon,
  ollama: ollamaIcon,
  onenewapi: onenewapiIcon,
  opencode: opencodeIcon,
  openrouter: openrouterIcon,
  qwen: qwenIcon,
  siliconflow: siliconflowIcon,
  stepfun: stepfunIcon,
  zai: zaiIcon,
};

// Trail icons render each vendor's own mark (brand colors kept), so a few
// need per-icon care: copilot's glyph is authored white and must follow the
// theme color like the card icons do; minimax/novita paint solid black and
// disappear on the dark sidebar — they invert there.
const TRAIL_RECOLOR_ICONS = new Set(["copilot"]);
const TRAIL_INVERT_DARK_ICONS = new Set(["minimax", "novita"]);

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Metric {
  label: string;
  kind: string;
  used_percent: number | null;
  detail: string | null;
  value: string | null;
  resets_at: number | null;
  period_ms: number | null;
}

interface Snapshot {
  id: string;
  name: string;
  plan: string | null;
  status: string;
  error: string | null;
  metrics: Metric[];
  stale: boolean;
  warning: string | null;
  dashboard_url?: string | null;
}

interface ModelSpend {
  model: string;
  cost: number;
  tokens: number;
}

interface SpendWindow {
  cost: number;
  tokens: number;
  models: ModelSpend[];
}

interface ProviderSpend {
  id: string;
  name: string;
  today: SpendWindow;
  yesterday: SpendWindow;
  last30: SpendWindow;
  trend: number[];
  unpriced: number;
  unpriced_models: string[];
}

/// How to get each provider signed in again, for the ⚠ Outdated tooltip.
const RELOGIN_KEYS: Record<string, string> = {
  claude: "stale.relogin.claude",
  codex: "stale.relogin.codex",
  grok: "stale.relogin.grok",
  copilot: "stale.relogin.copilot",
  cursor: "stale.relogin.cursor",
  devin: "stale.relogin.devin",
  opencode: "stale.relogin.opencode",
  antigravity: "stale.relogin.antigravity",
  ollama: "stale.relogin.ollama",
  hermes: "stale.relogin.hermes",
  kimi: "stale.relogin.kimi",
};

/// The ⚠ Outdated tooltip: what went wrong, what fixes it, and the
/// reassurance that the visible numbers are the last good ones. Errors are
/// classified into sign-in / rate-limit / vendor-outage / connection
/// buckets so the fix is concrete instead of a bare HTTP code.
function staleHelp(s: Snapshot): string {
  const w = (s.warning ?? t("stale.lastFailed")).replace(/[.\s]+$/, "");
  const lw = w.toLowerCase();
  const relogin = RELOGIN_KEYS[s.id] ? t(RELOGIN_KEYS[s.id]) : t("stale.reloginDefault");
  let fix = t("stale.fixRetry");
  if (/run `|open the/.test(lw)) {
    // The provider's own message already says what to do.
    fix = t("stale.fixDone");
  } else if (/http 40[13]|invalid_grant|expired|no refresh token|sign[- ]?in|log ?in|credentials/.test(lw)) {
    fix = t("stale.fixRelogin", { how: relogin });
  } else if (/http 429|rate limit/.test(lw)) {
    fix = t("stale.fix429");
  } else if (/http 5\d\d/.test(lw)) {
    fix = t("stale.fix5xx");
  } else if (/error sending request|timed? ?out|connect|network|dns|proxy/.test(lw)) {
    fix = t("stale.fixNet");
  }
  return `${w}.\n${fix}\n${t("stale.tail")}`;
}

/// ⚠ shown when some events have no known model price — their tokens are
/// counted, but no dollars are guessed, so dollar totals under-report.
function unpricedWarn(sp: ProviderSpend | undefined): string {
  if (!sp || sp.unpriced <= 0) return "";
  const models = sp.unpriced_models.join(", ") || "unknown models";
  return `<span class="stale" title="${escapeHtml(
    t("unpriced.tip", { n: sp.unpriced, models }),
  )}">⚠</span>`;
}

type SpendTab = "today" | "yesterday" | "last30";

// Per-provider layout: which rows show, their order, which are tucked
// behind the caret ("On Demand"), and which are starred for the tray strip.
interface ProviderLayout {
  metricOrder: string[];
  onDemand: string[];
  hidden: string[];
  starred: string[];
  expanded: boolean;
  // One-shot: Bonus used to be a bar (always-visible). After the demotion
  // to a text row we tuck it once; later drags out of Show more stick.
  tuckedBonus?: boolean;
}

interface Layout {
  providerOrder: string[];
  providers: Record<string, ProviderLayout>;
}

interface Config {
  refreshMinutes: number;
  disabled: string[];
  pinned: { provider: string; label: string } | null;
  trayProviders: string[];
  pacingAlways: boolean;
  telemetry: boolean;
  notifyAlmostOut: boolean;
  notifyCuttingClose: boolean;
  notifyWillRunOut: boolean;
  spendTab: SpendTab;
  spendMetric: "cost" | "tokens" | "mtok";
  showUsed: boolean;
  resetExact: boolean;
  timeFormat: "auto" | "12" | "24";
  layout: Layout | null;
  appearance: "system" | "light" | "dark";
  density: "regular" | "compact";
  glassEffects: boolean;
  shortcut: string;
  proxy: { enabled: boolean; url: string };
  showTotalSpend: boolean;
  welcomeDismissed: boolean;
  lastSeenVersion: string;
  reduceAnimations: boolean;
  hideUsageWhileSharing: boolean;
  locale: LocalePref;
}

const FRONTEND_CONFIG_KEYS = [
  "refreshMinutes",
  "disabled",
  "pinned",
  "trayProviders",
  "pacingAlways",
  "telemetry",
  "notifyAlmostOut",
  "notifyCuttingClose",
  "notifyWillRunOut",
  "spendTab",
  "spendMetric",
  "showUsed",
  "resetExact",
  "timeFormat",
  "layout",
  "appearance",
  "density",
  "glassEffects",
  "shortcut",
  "proxy",
  "showTotalSpend",
  "welcomeDismissed",
  "lastSeenVersion",
  "reduceAnimations",
  "hideUsageWhileSharing",
  "locale",
] as const satisfies readonly (keyof Config)[];
type _AssertAllConfigKeys = Exclude<keyof Config, (typeof FRONTEND_CONFIG_KEYS)[number]> extends never
  ? true
  : Exclude<keyof Config, (typeof FRONTEND_CONFIG_KEYS)[number]>;
const _assertAllConfigKeys: _AssertAllConfigKeys = true;
void _assertAllConfigKeys;

interface TrayProjectionProvider {
  metricOrder: string[];
  hidden: string[];
  starred: string[];
}

interface TrayProjectionConfig {
  disabled: string[];
  providerOrder: string[];
  providers: Record<string, TrayProjectionProvider>;
  pinned: Config["pinned"];
  locale: Locale;
}

interface TrayStripEntry {
  id: string;
  logo: number[];
  values: number[];
  tooltip: string;
}

const ALL_PROVIDERS: [string, string][] = [
  ["claude", "Claude"],
  ["codex", "Codex"],
  ["cursor", "Cursor"],
  ["opencode", "OpenCode"],
  ["copilot", "Copilot"],
  ["grok", "Grok"],
  ["devin", "Devin"],
  ["minimax", "MiniMax"],
  ["openrouter", "OpenRouter"],
  ["zai", "Z.ai"],
  ["antigravity", "Antigravity"],
  ["deepseek", "DeepSeek"],
  // Internal id stays "moonshot" (config/layout/telemetry compatibility);
  // the toggle reads "Kimi API" because that's what it gates: the API bar
  // on the Kimi card (or the standalone wallet card without a CLI login).
  ["moonshot", "Kimi API"],
  ["elevenlabs", "ElevenLabs"],
  ["ollama", "Ollama"],
  ["codebuff", "Codebuff"],
  ["kilo", "Kilo"],
  ["aihubmix", "AihubMix"],
  ["onenewapi", "One/New API"],
  ["qwen", "Qwen Code"],
  ["hermes", "Hermes"],
  ["kimi", "Kimi Code"],
  ["stepfun", "StepFun"],
  ["siliconflow", "SiliconFlow"],
  ["novita", "Novita AI"],
  ["relaybalance", "Custom Balance"],
];

function providerDisplayName(id: string): string {
  return ALL_PROVIDERS.find(([pid]) => pid === id)?.[1] ?? id;
}

// Same quick links the Mac app ships (status pages + vendor dashboards).
const PROVIDER_LINKS: Record<string, { label: string; url: string }[]> = {
  claude: [
    { label: "Status", url: "https://status.anthropic.com/" },
    { label: "Dashboard", url: "https://claude.ai/settings/usage" },
  ],
  codex: [
    { label: "Status", url: "https://status.openai.com/" },
    { label: "Dashboard", url: "https://chatgpt.com/codex/settings/usage" },
  ],
  cursor: [
    { label: "Status", url: "https://status.cursor.com/" },
    { label: "Dashboard", url: "https://www.cursor.com/dashboard" },
  ],
  copilot: [
    { label: "Status", url: "https://www.githubstatus.com/" },
    { label: "Dashboard", url: "https://github.com/settings/billing" },
  ],
  grok: [
    { label: "Status", url: "https://status.x.ai" },
    { label: "Usage", url: "https://grok.com/?_s=usage" },
  ],
  devin: [{ label: "Dashboard", url: "https://app.devin.ai/settings/plans" }],
  minimax: [{ label: "Platform", url: "https://platform.minimax.io/" }],
  openrouter: [
    { label: "Activity", url: "https://openrouter.ai/activity" },
    { label: "Credits", url: "https://openrouter.ai/settings/credits" },
  ],
  zai: [
    { label: "Dashboard", url: "https://z.ai/manage-apikey/coding-plan/personal/my-plan" },
    { label: "API Keys", url: "https://z.ai/manage-apikey/apikey-list" },
  ],
  opencode: [{ label: "Console", url: "https://opencode.ai/console" }],
  aihubmix: [{ label: "Console", url: "https://console.aihubmix.com/" }],
  qwen: [
    { label: "Coding Plan", url: "https://modelstudio.console.alibabacloud.com/ap-southeast-1/?tab=globalset#/efm/coding_plan" },
  ],
  deepseek: [
    { label: "Status", url: "https://status.deepseek.com/" },
    { label: "Platform", url: "https://platform.deepseek.com/usage" },
  ],
  moonshot: [{ label: "Console", url: "https://platform.moonshot.ai/console" }],
  elevenlabs: [
    { label: "Status", url: "https://status.elevenlabs.io/" },
    { label: "Usage", url: "https://elevenlabs.io/app/usage" },
  ],
  ollama: [{ label: "Library", url: "https://ollama.com/library" }],
  codebuff: [{ label: "Dashboard", url: "https://www.codebuff.com/profile" }],
  kilo: [{ label: "Dashboard", url: "https://app.kilo.ai/" }],
  hermes: [{ label: "Site", url: "https://hermes-agent.com/" }],
  kimi: [
    { label: "Console", url: "https://www.kimi.com/code/console" },
    { label: "Quota", url: "https://www.kimi.com/membership/subscription?tab=quota" },
    { label: "API", url: "https://platform.moonshot.ai/console" },
  ],
};

// Brand palette for the Total Spend ring (Mac parity); unknown providers
// get a stable hue derived from their id.
const SPEND_COLORS: Record<string, string> = {
  claude: "#de7356",
  codex: "#3b82f6",
  openrouter: "#6467f2",
  antigravity: "#4285f4",
  copilot: "#a855f7",
  minimax: "#f5433c",
  grok: "#10a37f",
  opencode: "#b7b1b1",
  devin: "#38bdf8",
  cursor: "var(--spend-cursor)", // brand black, theme-flipped in CSS
  moonshot: "#e0b354", // moon gold
  kimi: "#ff8a4c", // Kimi Code peach
  hermes: "#c2a878", // Nous tan
  aihubmix: "#5eead4", // hub teal
  qwen: "#8b5cf6", // Qwen violet
  __others__: "#8b8b94", // the folded small-spenders wedge
};

function spendColor(id: string): string {
  const fixed = SPEND_COLORS[id];
  if (fixed) return fixed;
  let hash = 0;
  for (const ch of id) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
  return `hsl(${hash % 360} 62% 58%)`;
}

const SPEND_KEYS: [string, SpendTab][] = [
  ["Today", "today"],
  ["Yesterday", "yesterday"],
  ["Last 30 Days", "last30"],
];
const TREND_KEY = "Usage Trend";
const DIVIDER = "__ondemand__";

const STALE_MS = 60 * 1000;
let config: Config = {
  refreshMinutes: 5,
  disabled: [],
  pinned: null,
  trayProviders: [],
  pacingAlways: false,
  telemetry: true,
  notifyAlmostOut: false,
  notifyCuttingClose: false,
  notifyWillRunOut: false,
  spendTab: "today",
  spendMetric: "cost",
  showUsed: false,
  resetExact: false,
  timeFormat: "auto",
  layout: null,
  appearance: "system",
  density: "regular",
  glassEffects: true,
  shortcut: "",
  proxy: { enabled: false, url: "" },
  showTotalSpend: true,
  welcomeDismissed: false,
  lastSeenVersion: "",
  reduceAnimations: false,
  hideUsageWhileSharing: false,
  locale: "auto",
};
let lastFetch = 0;
let refreshing = false;
// A forced refresh requested while one was already in flight (saving an
// API key races the auto-refresh timer). Dropping it would leave the new
// state unfetched and the status line stuck on the save message.
let refreshQueued = false;
let refreshQueuedUsageOnly = true;
// A key saved while the first refresh is still in flight. First-run (and
// "new provider") auto-disable keys off that fetch's no_credentials list,
// which can predate the save and park the provider we just turned on.
// Value is the refresh generation that was in flight (or last completed)
// at save time — the exemption lasts through that pass plus one more.
const recentlyKeyed = new Map<string, number>();
// Newly enabled providers stay out of every Tray projection until their
// required forced usage attempt has completed. Value is the enable
// generation that must finish before this id may appear.
const pendingProviderEnables = new Map<string, number>();
let providerEnableGeneration = 0;

function markProviderEnablePending(id: string): number {
  const generation = ++providerEnableGeneration;
  pendingProviderEnables.set(id, generation);
  return generation;
}

function finishProviderEnable(id: string, generation: number): void {
  if (pendingProviderEnables.get(id) !== generation) return;
  pendingProviderEnables.delete(id);
  requestTraySync();
}

let refreshGeneration = 0;
let completedRefreshGeneration = 0;
const refreshAttemptWaiters: Array<{ generation: number; resolve: () => void }> = [];
let lastAppliedSpendGen = 0;
let refreshTimer: number | undefined;
let lastSnapshots: Snapshot[] = [];
let lastSpend: ProviderSpend[] = [];
let spendLoaded = false;
let spendTab: SpendTab = "today";
let customizeOpen = false;
let revealTimer = 0;
let animateExpandId: string | null = null;

/// One pass of entrance animations (cards slide in, bars fill) — played when
/// the popover opens or the first data lands, never on background re-renders.
function playReveal(): void {
  if (reduceMotion()) return;
  const el = document.querySelector<HTMLElement>("#providers");
  if (!el) return;
  el.classList.remove("reveal");
  void el.offsetWidth; // restart CSS animations
  el.classList.add("reveal");
  clearTimeout(revealTimer);
  revealTimer = window.setTimeout(() => el.classList.remove("reveal"), 950);
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (c) => {
    const map: Record<string, string> = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;",
    };
    return map[c];
  });
}

function clampPercent(value: number): number {
  return Math.min(100, Math.max(0, value));
}

function remainingPercent(metric: Metric): number {
  return Math.round(100 - clampPercent(metric.used_percent ?? 0));
}

function fmtMoney(v: number): string {
  if (v >= 1000) return `$${(v / 1000).toFixed(1)}K`;
  return `$${v.toFixed(2)}`;
}

function fmtTokens(v: number): string {
  if (v >= 1e9) return `${(v / 1e9).toFixed(1)}B`;
  if (v >= 1e6) return `${(v / 1e6).toFixed(1)}M`;
  if (v >= 1e3) return `${(v / 1e3).toFixed(1)}K`;
  return String(Math.round(v));
}

function fmtDuration(ms: number): string {
  const mins = Math.max(1, Math.round(ms / 60000));
  const days = Math.floor(mins / 1440);
  const hours = Math.floor((mins % 1440) / 60);
  const rem = mins % 60;
  if (days > 0) return t("time.daysHours", { d: days, h: hours });
  if (hours > 0) return t("time.hoursMins", { h: hours, m: String(rem).padStart(2, "0") });
  return t("time.mins", { m: rem });
}

// "today at 6:38 PM" / "tomorrow at 18:38" / "Sat, Jul 11 at 9:00 AM",
// honoring the Time Format setting.
function fmtExact(ts: number): string {
  const d = new Date(ts);
  const now = new Date();
  const hour12 =
    config.timeFormat === "12" ? true : config.timeFormat === "24" ? false : undefined;
  const tag = localeTag();
  const time = d.toLocaleTimeString(tag, { hour: "numeric", minute: "2-digit", hour12 });
  const dayStart = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diffDays = Math.round((dayStart(d) - dayStart(now)) / 86400000);
  if (diffDays === 0) return t("time.today", { time });
  if (diffDays === 1) return t("time.tomorrow", { time });
  const date = d.toLocaleDateString(tag, { weekday: "short", month: "short", day: "numeric" });
  return t("time.dateAt", { date, time });
}

let configSaveQueue: Promise<void> = Promise.resolve();
let configSaveError: string | null = null;

function snapshotConfig(): Config {
  const payload = {} as Record<string, unknown>;
  for (const key of FRONTEND_CONFIG_KEYS) {
    payload[key] = config[key];
  }
  return JSON.parse(JSON.stringify(payload)) as Config;
}

function applyConfigEcho(sent: Config, echoed: Config): void {
  // Keep newer in-memory fields. Only take server canonicalization for
  // frontend keys that still match the snapshot this save actually wrote.
  const current = config as unknown as Record<string, unknown>;
  const from = sent as unknown as Record<string, unknown>;
  const echo = echoed as unknown as Record<string, unknown>;
  for (const key of FRONTEND_CONFIG_KEYS) {
    if (JSON.stringify(current[key]) === JSON.stringify(from[key])) {
      current[key] = echo[key];
    }
  }
}

async function patchConfig(patch: Partial<Config>): Promise<void> {
  Object.assign(config, patch);
  // Send a full current snapshot. If an earlier serialized write failed,
  // the next save retries that still-live in-memory state as well.
  const payload = snapshotConfig();
  const save = configSaveQueue.then(async () => {
    const echoed = await invoke<Config>("set_config", { patch: payload });
    applyConfigEcho(payload, echoed);
    configSaveError = null;
  });
  configSaveQueue = save.catch(() => {});
  try {
    await save;
  } catch (err) {
    configSaveError = String(err);
    const status = document.querySelector("#status");
    if (status) status.textContent = t("footer.configSaveFailed", { err: configSaveError });
    throw err;
  }
}

// ---------------------------------------------------------------------------
// Layout: defaults, repair, persistence
// ---------------------------------------------------------------------------

function defaultProviderLayout(s: Snapshot | undefined, spend: ProviderSpend | undefined, migrateStar: boolean): ProviderLayout {
  const order: string[] = [];
  const onDemand: string[] = [];
  for (const m of s?.metrics ?? []) {
    if (order.includes(m.label)) continue; // one row per label
    order.push(m.label);
    // Used stays on the card: unlimited One/New API keys have no bar.
    if (m.kind !== "progress" && m.label !== "Used") onDemand.push(m.label);
  }
  // Balance-only providers (Moonshot, DeepSeek…) have no progress rows at
  // all — tucking everything would leave an empty card with a floating
  // caret, so their text rows stay visible.
  if (order.length > 0 && onDemand.length === order.length) onDemand.length = 0;
  if (spend) {
    order.push(TREND_KEY); // trend stays always-visible, like the Mac
    for (const [label] of SPEND_KEYS) {
      order.push(label);
      onDemand.push(label);
    }
  }
  const starred = migrateStar
    ? (s?.metrics ?? []).filter((m) => m.kind === "progress").slice(0, 2).map((m) => m.label)
    : [];
  return { metricOrder: order, onDemand, hidden: [], starred, expanded: false };
}

const ONA_QUOTA_LABELS = ["Usage", "Used", "Limit"] as const;

function liveOnaQuotaLabel(s: Snapshot): string | undefined {
  return s.metrics.find((m) => (ONA_QUOTA_LABELS as readonly string[]).includes(m.label))?.label;
}

/// One/New API emits one quota row: Usage (limited bar), Used (unlimited),
/// or Limit. Switching unlimited↔limited must replace that slot so the
/// card never shows both 用量 and 已用.
function migrateOnaQuotaLayout(s: Snapshot, L: ProviderLayout): boolean {
  if (providerFamily(s.id) !== "onenewapi") return false;
  const live = liveOnaQuotaLabel(s);
  if (!live) return false;
  let swapped = false;
  for (const old of ONA_QUOTA_LABELS) {
    if (old === live) continue;
    for (const list of [L.metricOrder, L.hidden, L.starred, L.onDemand]) {
      const at = list.indexOf(old);
      if (at < 0) continue;
      if (list.includes(live)) list.splice(at, 1);
      else list[at] = live;
      swapped = true;
    }
  }
  if (!swapped) return false;
  // The replacement inherits the old row's slot; unlimited Used and the
  // limited bar should land on the card, not behind Show more.
  if (live === "Usage" || live === "Used") {
    const at = L.onDemand.indexOf(live);
    if (at >= 0) L.onDemand.splice(at, 1);
  }
  if (live !== "Usage") {
    const starAt = L.starred.indexOf(live);
    if (starAt >= 0) L.starred.splice(starAt, 1);
  }
  return true;
}

function rankSnapshot(s: Snapshot): number {
  const FREE = /free|trial/i;
  if (s.status === "ok") {
    if (s.plan && !FREE.test(s.plan)) return 0;
    if (s.plan) return 2;
    return 1;
  }
  return s.status === "error" ? 3 : 4;
}

/// Builds the layout on first run and folds in newly-appeared providers or
/// metrics afterwards. Saves only when something actually changed.
function ensureLayout(): void {
  let changed = false;
  let layout = config.layout;

  if (!layout) {
    const orderedIds = [...lastSnapshots].sort((a, b) => rankSnapshot(a) - rankSnapshot(b)).map((s) => s.id);
    for (const [id] of ALL_PROVIDERS) if (!orderedIds.includes(id)) orderedIds.push(id);
    layout = { providerOrder: orderedIds, providers: {} };
    changed = true;
  }

  // A duplicated id in providerOrder renders the same provider as two
  // Customize rows and two dashboard cards — older builds could persist
  // one via an interrupted drag-reorder. First occurrence wins.
  const seenOrder = new Set<string>();
  const dedupedOrder = layout.providerOrder.filter((id) => {
    if (seenOrder.has(id)) return false;
    seenOrder.add(id);
    return true;
  });
  if (dedupedOrder.length !== layout.providerOrder.length) {
    layout.providerOrder = dedupedOrder;
    changed = true;
  }

  for (const [id] of ALL_PROVIDERS) {
    if (!layout.providerOrder.includes(id)) {
      layout.providerOrder.push(id);
      changed = true;
    }
  }
  // Configured One/New API keys keep an independent layout slot even
  // when the family is off (no snapshot). Append only — never regroup.
  if (foldOnaKeysIntoLayout(layout)) changed = true;

  // One-time label migration (Cursor bucket-era rename, 0.4.35): "Auto
  // usage" → "Cursor Models", "API usage" → "Other Models". Stars, pins,
  // hidden/on-demand flags and row order carry over — without this, a
  // starred/pinned old row silently loses its setting and the stale label
  // rots in metricOrder forever (no rename migration existed before).
  const CURSOR_RENAMES: Record<string, string> = {
    "Auto usage": "Cursor Models",
    "API usage": "Other Models",
  };
  for (const [pid, L] of Object.entries(layout.providers)) {
    if (providerFamily(pid) !== "cursor") continue;
    for (const list of [L.metricOrder, L.hidden, L.starred, L.onDemand]) {
      for (const [oldLabel, newLabel] of Object.entries(CURSOR_RENAMES)) {
        const at = list.indexOf(oldLabel);
        if (at < 0) continue;
        if (list.includes(newLabel)) list.splice(at, 1);
        else list[at] = newLabel;
        changed = true;
      }
    }
  }
  const hermesHasRecentModels = lastSnapshots.some(
    (s) => providerFamily(s.id) === "hermes" && s.metrics.some((m) => m.label === "Recent models"),
  );
  if (hermesHasRecentModels) {
    for (const [pid, L] of Object.entries(layout.providers)) {
      if (providerFamily(pid) !== "hermes") continue;
      for (const list of [L.metricOrder, L.hidden, L.starred, L.onDemand]) {
        const at = list.indexOf("Last used");
        if (at < 0) continue;
        if (list.includes("Recent models")) list.splice(at, 1);
        else list[at] = "Recent models";
        changed = true;
      }
    }
  }
  // On bucket-era accounts "Total usage" became a text row — the tray
  // strip and pinned tray number only accept progress metrics, so a
  // star/pin on it would silently vanish. Repoint both to the nearest
  // equivalent meter, "Cursor Models" (only when the live snapshot
  // confirms the row is text; pre-bucket accounts keep their bar).
  const cursorSnap = lastSnapshots.find((s) => providerFamily(s.id) === "cursor");
  const totalIsText =
    cursorSnap?.metrics.find((m) => m.label === "Total usage")?.kind === "text";
  if (totalIsText) {
    for (const [pid, L] of Object.entries(layout.providers)) {
      if (providerFamily(pid) !== "cursor") continue;
      const at = L.starred.indexOf("Total usage");
      if (at >= 0) {
        if (L.starred.includes("Cursor Models")) L.starred.splice(at, 1);
        else L.starred[at] = "Cursor Models";
        changed = true;
      }
    }
  }

    if (config.pinned && providerFamily(config.pinned.provider) === "cursor") {
    const renamed = CURSOR_RENAMES[config.pinned.label];
    const to = renamed ?? (totalIsText && config.pinned.label === "Total usage" ? "Cursor Models" : null);
    if (to) {
      config.pinned = { ...config.pinned, label: to };
      void patchConfig({ pinned: config.pinned }).catch(() => {});
    }
  }

  // "Bonus" briefly rendered as a bar and is now a text row (free
  // provider-sponsored usage — context, not a meter). Layouts saved in
  // that window placed it always-visible; tuck it behind Show more once,
  // then leave later Customize drags alone. Stars/pins on it still drop
  // every pass — the tray strip only accepts progress metrics.
  const bonusIsText =
    cursorSnap?.metrics.find((m) => m.label === "Bonus")?.kind === "text";
  if (bonusIsText) {
    for (const [pid, L] of Object.entries(layout.providers)) {
      if (providerFamily(pid) !== "cursor") continue;
      if (!L.tuckedBonus) {
        if (L.metricOrder.includes("Bonus") && !L.onDemand.includes("Bonus")) {
          L.onDemand.push("Bonus");
        }
        L.tuckedBonus = true;
        changed = true;
      }
      const starAt = L.starred.indexOf("Bonus");
      if (starAt >= 0) {
        L.starred.splice(starAt, 1);
        changed = true;
      }
    }
    if (
      config.pinned &&
      providerFamily(config.pinned.provider) === "cursor" &&
      config.pinned.label === "Bonus"
    ) {
      config.pinned = null;
      void patchConfig({ pinned: null }).catch(() => {});
    }
  }

  // Kimi Code folds the Moonshot wallet onto the plan card. Stars and the
  // tray pin on "Credits used" would otherwise vanish with that card —
  // but only migrate when the API bar is actually on that card, or we
  // plant a phantom star and the tray number goes blank.
  const kimiLive = lastSnapshots.some(
    (s) => s.id === "kimi" && s.status === "ok" && s.metrics.some((m) => m.label === "API"),
  );
  if (kimiLive) {
    const moonL = layout.providers.moonshot;
    const starAt = moonL?.starred.indexOf("Credits used") ?? -1;
    if (starAt >= 0 && moonL) {
      moonL.starred.splice(starAt, 1);
      let kimiL = layout.providers.kimi;
      if (!kimiL) {
        kimiL = defaultProviderLayout(
          lastSnapshots.find((s) => s.id === "kimi"),
          lastSpend.find((sp) => sp.id === "kimi"),
          false,
        );
        layout.providers.kimi = kimiL;
      }
      if (!kimiL.starred.includes("API")) {
        if (kimiL.starred.length >= 2) kimiL.starred.pop();
        kimiL.starred.push("API");
      }
      changed = true;
    }
    if (
      config.pinned?.provider === "moonshot" &&
      (config.pinned.label === "Credits used" || config.pinned.label === "API")
    ) {
      config.pinned = { provider: "kimi", label: "API" };
      void patchConfig({ pinned: config.pinned }).catch(() => {});
    }
  }

  for (const s of lastSnapshots) {
    if (!layout.providerOrder.includes(s.id)) {
      layout.providerOrder.push(s.id);
      changed = true;
    }
    const spend = lastSpend.find((sp) => sp.id === s.id);
    let L = layout.providers[s.id];
    if (!L) {
      // One-time migration: providers picked in the old tray-strip setting
      // become starred so the strip carries over.
      L = defaultProviderLayout(s, spend, config.trayProviders.includes(s.id));
      layout.providers[s.id] = L;
      changed = true;
      continue;
    }
    if (migrateOnaQuotaLayout(s, L)) changed = true;
    const liveQuota = liveOnaQuotaLabel(s);
    if (
      providerFamily(s.id) === "onenewapi" &&
      config.pinned?.provider === s.id &&
      (ONA_QUOTA_LABELS as readonly string[]).includes(config.pinned.label)
    ) {
      if (liveQuota === "Usage") {
        if (config.pinned.label !== "Usage") {
          config.pinned = { provider: s.id, label: "Usage" };
          void patchConfig({ pinned: config.pinned }).catch(() => {});
        }
      } else {
        config.pinned = null;
        void patchConfig({ pinned: null }).catch(() => {});
      }
    }
    // New metrics ship once; spend rows appear when spend data first exists.
    for (const m of s.metrics) {
      if (!L.metricOrder.includes(m.label)) {
        // Progress bars slot in above the Usage Trend (bars first, trend
        // after, like the Mac cards); everything else appends at the end.
        const trendAt = L.metricOrder.indexOf(TREND_KEY);
        if (m.kind === "progress" && trendAt >= 0) {
          L.metricOrder.splice(trendAt, 0, m.label);
        } else {
          L.metricOrder.push(m.label);
        }
        if (m.kind !== "progress" && m.label !== "Used") L.onDemand.push(m.label);
        changed = true;
      }
      // Do not yank an existing progress row out of Show more or shuffle
      // it above Usage Trend on later refreshes. Extra credits flips
      // text↔progress with balance; a Customize drag would otherwise
      // bounce back on the next snapshot (issue #166). New rows still
      // land always-visible above the trend via the first-seen branch.
    }
    if (spend) {
      if (!L.metricOrder.includes(TREND_KEY)) {
        L.metricOrder.push(TREND_KEY);
        changed = true;
      }
      for (const [label] of SPEND_KEYS) {
        if (!L.metricOrder.includes(label)) {
          L.metricOrder.push(label);
          L.onDemand.push(label);
          changed = true;
        }
      }
    }
    // Repair layouts saved while a provider emitted duplicate labels (old
    // Grok billing bug): the label landed in metricOrder twice and the
    // card rendered the same row twice.
    const seenKeys = new Set<string>();
    const dedupedOrder = L.metricOrder.filter((k) => !seenKeys.has(k) && (seenKeys.add(k), true));
    if (dedupedOrder.length !== L.metricOrder.length) {
      L.metricOrder = dedupedOrder;
      changed = true;
    }
    // Repair saved layouts where EVERY visible row sits behind the caret
    // (balance-only cards defaulted that way before this rule existed):
    // an all-tucked card renders as an empty panel with a floating ⌄, so
    // its own metric rows are promoted back to always-visible.
    const alwaysVisible = L.metricOrder.filter(
      (k) => !L.onDemand.includes(k) && !L.hidden.includes(k),
    );
    if (alwaysVisible.length === 0) {
      const own = new Set(s.metrics.map((m) => m.label));
      if (s.metrics.length > 0 && L.onDemand.some((k) => own.has(k))) {
        L.onDemand = L.onDemand.filter((k) => !own.has(k));
        changed = true;
      }
    }
  }

  config.layout = layout;
  if (changed) void patchConfig({ layout });
}

function providerLayout(id: string): ProviderLayout {
  return (
    config.layout?.providers[id] ?? {
      metricOrder: [],
      onDemand: [],
      hidden: [],
      starred: [],
      expanded: false,
    }
  );
}

function saveLayout(syncTray = true): void {
  if (!config.layout) return;
  // Undo history: remember the state we're moving away from.
  const next = JSON.stringify(config.layout);
  if (lastLayoutSnapshot && lastLayoutSnapshot !== next) {
    undoStack.push(lastLayoutSnapshot);
    if (undoStack.length > 50) undoStack.shift();
  }
  lastLayoutSnapshot = next;
  void patchConfig({ layout: config.layout });
  if (syncTray) requestTraySync();
}

// ---------------------------------------------------------------------------
// Pace engine (unchanged from Wave 1/4)
// ---------------------------------------------------------------------------

interface Pace {
  cls: string;
  note: string;
  noteClass: string;
  title: string;
  tick: number | null;
}

function computePace(m: Metric): Pace {
  const used = clampPercent(m.used_percent ?? 0);
  const left = 100 - used;
  const none: Pace = { cls: "", note: "", noteClass: "", title: "", tick: null };

  if (left < 0.5) {
    return { cls: "low", note: t("pace.limitReached"), noteClass: "danger", title: t("pace.limitReachedTitle"), tick: null };
  }

  const byLevel = (): Pace => {
    if (left <= 10) return { ...none, cls: "low", title: t("card.pctLeft", { n: Math.round(left) }) };
    if (used >= 80) return { ...none, cls: "warn", title: t("card.pctUsed", { n: Math.round(used) }) };
    return none;
  };
  if (!m.resets_at || !m.period_ms) return byLevel();

  const now = Date.now();
  const remainMs = Math.max(0, m.resets_at - now);
  const elapsedMs = m.period_ms - remainMs;
  const frac = elapsedMs / m.period_ms;
  if (frac < 0.05 || elapsedMs < 5 * 60000) return byLevel();
  // Near-empty windows stay calm: a floored 1% reading right at the
  // projection gate can land exactly on the limit and flash red (Mac
  // keeps the same 5% safeguard).
  if (used < 5) return byLevel();

  const projected = used / frac;
  const tick = clampPercent(frac * 100);

  if (projected >= 100) {
    const over = Math.round(projected - 100);
    const runOutAt = now + (left * elapsedMs) / used;
    if (runOutAt < m.resets_at - 60000) {
      const when = config.resetExact
        ? t("pace.limitAt", { when: fmtExact(runOutAt) })
        : t("pace.limitIn", { time: fmtDuration(runOutAt - now) });
      return { cls: "low", note: `🔥 ${when}`, noteClass: "danger", title: t("pace.overReset", { n: over }), tick };
    }
    return { cls: "low", note: "🔥", noteClass: "danger", title: t("pace.fullReset"), tick };
  }

  const spare = Math.max(1, Math.round(100 - projected));
  if (projected >= 90) {
    return {
      cls: "warn",
      note: t("pace.spare", { n: spare }),
      noteClass: "warn",
      title: t("pace.usedReset", { n: Math.round(projected) }),
      tick,
    };
  }
  return {
    cls: "",
    note: config.pacingAlways ? t("pace.leftReset", { n: spare }) : "",
    noteClass: "",
    title: t("pace.leftReset", { n: spare }),
    tick: config.pacingAlways ? tick : null,
  };
}

// ---------------------------------------------------------------------------
// Dashboard rendering
// ---------------------------------------------------------------------------

function renderMetric(m: Metric): string {
  if (m.kind === "progress" && m.used_percent !== null) {
    const used = clampPercent(m.used_percent);
    const left = Math.round(100 - used);
    const pace = computePace(m);
    const tick =
      pace.tick !== null && pace.tick > 1 && pace.tick < 99
        ? `<span class="tick" style="left:${pace.tick}%"></span>`
        : "";
    const note = pace.note
      ? `<span class="pace-note ${pace.noteClass}" title="${escapeHtml(pace.title)}">${escapeHtml(pace.note)}</span>`
      : "";
    const headline = config.showUsed ? t("card.pctUsed", { n: Math.round(used) }) : t("card.pctLeft", { n: left });
    const headlineAlt = config.showUsed ? t("card.pctLeft", { n: left }) : t("card.pctUsed", { n: Math.round(used) });

    let resetHtml = "";
    if (m.resets_at === null && m.period_ms !== null && m.period_ms <= 6 * 3_600_000 && used <= 1) {
      // GLM-style rolling session windows expose NO reset timestamp while
      // idle — the clock only starts on the first request after the last
      // window closed. An untouched ≤6h window with nothing to count down
      // to is "not started", not "missing data".
      resetHtml = `<span title="${escapeHtml(t("card.notStartedTip"))}">${escapeHtml(t("card.notStarted"))}</span>`;
    } else if (m.resets_at !== null && m.resets_at > Date.now()) {
      // A rolling session window (≤6h period) that is still full-length
      // hasn't begun — its clock starts on the first message, so a
      // countdown would lie. Codex floors percentages and reports 1% on an
      // untouched window, so the label keys on the window being fresh
      // (with a grace for server-side reset staleness), not on a zero the
      // backend no longer fabricates.
      let notStarted = false;
      if (m.period_ms !== null && m.period_ms <= 6 * 3_600_000 && used <= 1) {
        const grace = Math.max(60_000, m.period_ms / 100);
        notStarted = m.resets_at - Date.now() >= m.period_ms - grace;
      }
      if (notStarted) {
        resetHtml = `<span title="${escapeHtml(t("card.notStartedTip"))}">${escapeHtml(t("card.notStarted"))}</span>`;
      } else {
        const remain = m.resets_at - Date.now();
        const countdown = remain < 60_000 ? t("card.resetsSoon") : t("card.resetsIn", { time: fmtDuration(remain) });
        const exact = t("card.resetsAt", { when: fmtExact(m.resets_at) });
        const [text, alt] = config.resetExact ? [exact, countdown] : [countdown, exact];
        resetHtml = `<span class="clickable" data-flip="reset" title="${escapeHtml(alt)}">${escapeHtml(text)}</span>`;
      }
    }
    const detailHtml = [m.detail ? escapeHtml(displayMetricDetail(m.detail)) : "", resetHtml].filter(Boolean).join(" · ");
    return `
      <div class="metric">
        <div class="metric-head">
          <span class="metric-label">${escapeHtml(displayMetricLabel(m.label))}</span>
          ${note}
        </div>
        <div class="bar" title="${escapeHtml(pace.title)}">
          <div class="fill ${pace.cls}" style="width:${used}%"></div>
          ${tick}
        </div>
        <div class="metric-foot">
          <span class="left-val clickable" data-flip="usage" title="${escapeHtml(headlineAlt)}">${headline}</span>
          <span class="detail">${detailHtml}</span>
        </div>
      </div>`;
  }
  // Action row (reset credits): exact expiry, plus a Use button only when
  // the metric carries redeem detail. A credit dying within 24h gets an
  // amber dot so it isn't wasted.
  if (m.kind === "action") {
    const expiry =
      m.resets_at !== null
        ? t("card.expires", { when: fmtExact(m.resets_at) })
        : displayMetricDetail(m.value ?? t("card.available"));
    const remaining = m.resets_at === null ? null : m.resets_at - Date.now();
    const soon =
      remaining !== null && remaining > 0 && remaining < 86_400_000
        ? `<span class="warn-dot" title="${escapeHtml(t("card.creditDying", { time: fmtDuration(remaining) }))}">●</span> `
        : "";
    const useBtn = m.detail
      ? `<button class="redeem-btn" data-redeem="${escapeHtml(m.detail)}" title="${escapeHtml(t("card.useTip"))}">${escapeHtml(t("card.use"))}</button>`
      : "";
    return `
      <div class="metric-text action-row">
        <span>${soon}${escapeHtml(displayMetricLabel(m.label))}</span>
        <span class="action-right">
          <span class="detail">${escapeHtml(expiry)}</span>
          ${useBtn}
        </span>
      </div>`;
  }
  return `
    <div class="metric-text">
      <span>${escapeHtml(displayMetricLabel(m.label))}</span>
      <span class="detail">${escapeHtml(displayMetricDetail(m.value ?? ""))}</span>
    </div>`;
}

function renderTrend(spend: ProviderSpend): string {
  if (!spend.trend.some((v) => v > 0)) return "";
  const max = Math.max(...spend.trend);
  const peakIdx = spend.trend.indexOf(max);
  const dayMs = 86_400_000;
  const dateOf = (i: number) =>
    new Date(Date.now() - (29 - i) * dayMs).toLocaleDateString(localeTag(), { month: "short", day: "numeric" });
  // Each day is a group: the visible bar plus a full-height invisible hit
  // area so thin bars are easy to hover; [data-trend] drives the tooltip.
  const bars = spend.trend
    .map((v, i) => {
      const h = v > 0 ? Math.max(2, (v / max) * 30) : 1;
      return `<g class="trend-day">
        <rect class="${v > 0 ? "trend-bar" : "trend-zero"}" x="${i * 10}" y="${32 - h}" width="7" height="${h}" rx="1.5"/>
        <rect class="trend-hit" data-trend="${spend.id}|${i}" x="${i * 10 - 1.5}" y="0" width="10" height="32" fill="transparent"/>
      </g>`;
    })
    .join("");
  const title = t("spend.trendTip", {
    from: dateOf(0),
    to: dateOf(29),
    tokens: fmtTokens(max),
    peak: dateOf(peakIdx),
  });
  return `
    <div class="metric trend">
      <span class="metric-label" title="${escapeHtml(title)}">${escapeHtml(t("spend.trend"))}</span>
      <svg class="trend-chart" viewBox="0 0 297 32" preserveAspectRatio="none">${bars}</svg>
    </div>`;
}

function renderSpendRow(
  providerId: string,
  label: string,
  key: SpendTab,
  w: SpendWindow,
  sp?: ProviderSpend,
): string {
  // Cursor's CSV aggregates requests, so its dollars are honest estimates.
  const text =
    w.tokens > 0 || w.cost > 0.005
      ? providerId === "cursor"
        ? t("card.tokensEst", { cost: fmtMoney(w.cost), n: fmtTokens(w.tokens) })
        : t("card.tokensPlain", { cost: fmtMoney(w.cost), n: fmtTokens(w.tokens) })
      : t("card.noData");
  const warn = key === "last30" ? unpricedWarn(sp) : "";
  return `
    <div class="metric-text spend-row" data-spend="${providerId}|${key}">
      <span>${escapeHtml(displayMetricLabel(label))} ${warn}</span>
      <span class="detail">${text}</span>
    </div>`;
}

/// One card row addressed by its layout key.
function renderItem(s: Snapshot, spend: ProviderSpend | undefined, key: string): string {
  if (key === TREND_KEY) return spend ? renderTrend(spend) : "";
  const spendKey = SPEND_KEYS.find(([label]) => label === key);
  if (spendKey)
    return spend ? renderSpendRow(s.id, spendKey[0], spendKey[1], spend[spendKey[1]], spend) : "";
  const metric = s.metrics.find((m) => m.label === key);
  return metric ? renderMetric(metric) : "";
}

/// Account-scoped cards (claude@<hash>) inherit their family's chrome —
/// icon, quick links — while keeping their own identity everywhere else.
function providerFamily(id: string): string {
  return id.split("@")[0];
}

/// One/New API is two-level: family id `onenewapi` hides every key card.
/// Claude/Codex extra accounts stay independent of the bare family id.
function isCardDisabled(id: string, disabled: string[] = config.disabled): boolean {
  if (disabled.includes(id)) return true;
  const fam = providerFamily(id);
  return fam === "onenewapi" && disabled.includes("onenewapi");
}

function renderCard(s: Snapshot): string {
  const plan = s.plan ? `<span class="plan">${escapeHtml(s.plan)}</span>` : "";
  const icon = PROVIDER_ICONS[s.id] ?? PROVIDER_ICONS[providerFamily(s.id)] ?? "";
  const muted = s.status === "ok" ? "" : " muted";

  let body: string;
  let caret = "";
  if (s.status === "ok") {
    const L = providerLayout(s.id);
    const spend = lastSpend.find((sp) => sp.id === s.id);
    const visible = L.metricOrder.filter((k) => !L.hidden.includes(k));
    const always = visible.filter((k) => !L.onDemand.includes(k));
    const onDemand = visible.filter((k) => L.onDemand.includes(k));

    body = always.map((k) => renderItem(s, spend, k)).join("");
    const onDemandHtml = onDemand.map((k) => renderItem(s, spend, k)).join("");
    if (onDemandHtml.trim()) {
      const anim = L.expanded && animateExpandId === s.id ? " anim" : "";
      caret = `
        <button class="card-caret" data-caret="${s.id}" title="${L.expanded ? t("card.showLess") : t("card.showMore")}">${L.expanded ? "⌃" : "⌄"}</button>
        ${L.expanded ? `<div class="on-demand${anim}">${onDemandHtml}</div>` : ""}`;
    }
  } else {
    body = `<p class="placeholder">${escapeHtml(s.error ?? t("card.notConnected"))}</p>`;
  }

  const stale = s.stale
    ? `<span class="stale" title="${escapeHtml(staleHelp(s))}">${escapeHtml(t("card.outdated"))}</span>`
    : "";
  const family = providerFamily(s.id);
  const dashUrl = (s.dashboard_url ?? "").trim();
  const dashOk = /^https?:\/\//i.test(dashUrl);
  const staticLinks = PROVIDER_LINKS[s.id] ?? PROVIDER_LINKS[family] ?? [];
  const linkItems = dashOk
    ? [{ label: "Dashboard", url: dashUrl }, ...staticLinks.filter((l) => l.label !== "Dashboard")]
    : staticLinks;
  const links = linkItems
    .filter((l) => l.label !== "API" || s.metrics.some((m) => m.label === "API"))
    .map((l) => `<button class="quick-link" data-link="${escapeHtml(l.url)}">${escapeHtml(displayLinkLabel(l.label))}</button>`)
    .join("<span class='quick-sep'>·</span>");
  const linksRow = links ? `<div class="quick-links">${links}</div>` : "";
  const share =
    s.status === "ok"
      ? `<button class="share-btn" data-share="${s.id}" title="${escapeHtml(t("card.share"))}">⧉</button>`
      : "";
  return `
    <article class="provider${muted}" data-provider="${s.id}">
      <div class="provider-head">
        <span class="drag-grip" title="${escapeHtml(t("card.drag"))}">⠿</span>
        <span class="provider-name">${escapeHtml(s.name)}</span>
        ${plan}
        ${stale}
        <span class="spacer"></span>
        ${share}
        <span class="provider-icon">${icon}</span>
      </div>
      <div class="card-panel">
        ${body}
        ${linksRow}
        ${caret}
      </div>
    </article>`;
}

function orderedSnapshots(): Snapshot[] {
  const order = config.layout?.providerOrder ?? [];
  // Disabled providers disappear immediately — not on the next fetch.
  return lastSnapshots.filter((s) => !isCardDisabled(s.id)).sort((a, b) => {
    const ia = order.indexOf(a.id);
    const ib = order.indexOf(b.id);
    if (ia !== -1 && ib !== -1) return ia - ib;
    return rankSnapshot(a) - rankSnapshot(b);
  });
}

// The ring is built from annular wedges (like the Mac's SectorMark chart):
// radial-cut ends with softly rounded corners and angular gaps, so tiny
// spenders stay thin slivers instead of ballooning to a round-cap dot.
const TAU = Math.PI * 2;
const DONUT_OUT = 44; // outer radius
const DONUT_IN = 30; // inner radius — 14 thick, centered on r=37
const DONUT_PAD = 2.2 / 37; // angular gap between neighbors (~2px mid-ring)
const DONUT_MIN = 0.07; // slimmest visible sliver (~2.6px mid-ring)

type DonutEntry = {
  s: ProviderSpend;
  w: SpendWindow;
  /// Present on the synthetic "Others" entry: the folded-in providers,
  /// largest first, for the hover breakdown.
  parts?: { name: string; w: SpendWindow }[];
  /// The dollar bar the parts fell under (period-specific).
  foldLimit?: number;
};

const OTHERS_ID = "__others__";
/// Providers under this many dollars (in the visible window) fold into
/// one "Others" wedge; hovering it lists who spent what. The bar scales
/// with the period — a day's ring earns a slice at $5, a month's at $10.
function othersFoldUsd(tab: SpendTab): number {
  return tab === "last30" ? 10 : 5;
}

function donutEntries(tab: SpendTab): DonutEntry[] {
  const all: DonutEntry[] = lastSpend
    .filter((s) => !isCardDisabled(s.id)) // disabled = gone everywhere
    .map((s) => ({ s, w: s[tab] }))
    // Membership, order, and wedge share all follow the active metric so
    // the legend ranking always matches the ring (cost keeps a half-cent
    // noise floor).
    .filter((e) =>
      config.spendMetric === "tokens"
        ? e.w.tokens > 0
        : config.spendMetric === "mtok"
          ? e.w.tokens > 0 && e.w.cost > 0.005
          : e.w.cost > 0.005,
    )
    .sort((a, b) => spendVal(b.w) - spendVal(a.w));

  // Small spenders fold into a single "Others" wedge — even a lone one,
  // so under-threshold providers never claim their own legend row. Only
  // exception: at least one named provider must remain, because an
  // all-Others ring says nothing.
  const limit = othersFoldUsd(tab);
  const small = all.filter((e) => e.w.cost < limit);
  if (small.length === 0 || small.length === all.length) return all;

  const others: DonutEntry = {
    s: {
      id: OTHERS_ID,
      name: t("spend.others"),
    } as ProviderSpend,
    w: {
      cost: small.reduce((sum, e) => sum + e.w.cost, 0),
      tokens: small.reduce((sum, e) => sum + e.w.tokens, 0),
      models: [],
    },
    parts: small.map((e) => ({ name: e.s.name, w: e.w })),
    foldLimit: limit,
  };
  return [...all.filter((e) => e.w.cost >= limit), others].sort(
    (a, b) => spendVal(b.w) - spendVal(a.w),
  );
}

/// The donut meters dollars or raw tokens — a click on the ring toggles.
function spendVal(w: SpendWindow): number {
  if (config.spendMetric === "tokens") return w.tokens;
  if (config.spendMetric === "mtok") return w.tokens > 0 ? w.cost / (w.tokens / 1e6) : 0;
  return w.cost;
}

/// Dollar-rate figure: two decimals under $1k, abbreviated above.
function fmtRate(v: number): string {
  return v < 1000 ? `$${v.toFixed(2)}` : fmtMoney(v);
}

/// The ring's two-line center (and its hover text) for the active metric.
/// Cost/MTok is the overall average — total dollars over total megatokens —
/// not a sum of per-provider rates.
function spendCenter(entries: DonutEntry[]): { primary: string; sub: string; exact: string } {
  if (config.spendMetric === "mtok") {
    const cost = entries.reduce((s, e) => s + e.w.cost, 0);
    const mtok = entries.reduce((s, e) => s + e.w.tokens, 0) / 1e6;
    const rate = mtok > 0 ? cost / mtok : 0;
    return { primary: fmtRate(rate), sub: "$/MTok", exact: `${fmtRate(rate)}/MTok average` };
  }
  if (config.spendMetric === "tokens") {
    const tokens = entries.reduce((s, e) => s + e.w.tokens, 0);
    return { primary: fmtTokens(tokens), sub: t("spend.centerTokens"), exact: t("card.tokens", { n: fmtTokens(tokens) }) };
  }
  const c = entries.reduce((s, e) => s + e.w.cost, 0);
  return { primary: fmtMoney(c), sub: t("spend.metric.cost"), exact: `$${c.toFixed(2)}` };
}

/// The metric a click (or right-click, reversed) moves to next — the Mac
/// menu's order: Cost, Cost/MTok, Tokens.
function nextSpendMetric(back: boolean): "cost" | "tokens" | "mtok" {
  const order: ("cost" | "tokens" | "mtok")[] = ["cost", "mtok", "tokens"];
  const i = order.indexOf(config.spendMetric);
  return order[(i + (back ? order.length - 1 : 1)) % order.length];
}

const METRIC_NAMES = { cost: "spend.metric.cost", mtok: "spend.metric.mtok", tokens: "spend.metric.tokens" } as const;

function fmtSpendVal(w: SpendWindow): string {
  if (config.spendMetric === "tokens") return fmtTokens(w.tokens);
  if (config.spendMetric === "mtok") return `${fmtRate(spendVal(w))}/MTok`;
  return fmtMoney(w.cost);
}

/// Angular extent per provider (slivers lifted to stay visible), shared by
/// the initial render and the tab-switch morph. Angles run clockwise from
/// 12 o'clock; the first gap straddles the top like the Mac's ring.
function donutGeometry(entries: DonutEntry[]): { total: number; geo: Map<string, { a0: number; a1: number }> } {
  const total = entries.reduce((sum, e) => sum + spendVal(e.w), 0);
  const spenders = entries.filter((e) => spendVal(e.w) > 0);
  const geo = new Map<string, { a0: number; a1: number }>();
  if (spenders.length === 0 || total <= 0) return { total, geo };
  if (spenders.length === 1) {
    geo.set(spenders[0].s.id, { a0: 0, a1: TAU });
    return { total, geo };
  }
  const avail = TAU - spenders.length * DONUT_PAD;
  const spans = spenders.map((e) => (spendVal(e.w) / total) * avail);
  let excess = 0;
  for (let i = 0; i < spans.length; i++) {
    if (spans[i] < DONUT_MIN) {
      excess += DONUT_MIN - spans[i];
      spans[i] = DONUT_MIN;
    }
  }
  if (excess > 0) {
    const big = spans.indexOf(Math.max(...spans));
    spans[big] = Math.max(DONUT_MIN, spans[big] - excess);
  }
  let a = DONUT_PAD / 2;
  spenders.forEach((e, i) => {
    geo.set(e.s.id, { a0: a, a1: a + spans[i] });
    a += spans[i] + DONUT_PAD;
  });
  return { total, geo };
}

function donutPt(r: number, a: number): string {
  return `${(48 + r * Math.sin(a)).toFixed(2)} ${(48 - r * Math.cos(a)).toFixed(2)}`;
}

/// SVG path for one annular sector with rounded corners (d3-arc style).
/// A full-circle span comes back as a two-ring evenodd annulus instead.
function sectorPath(a0: number, a1: number): string {
  const span = a1 - a0;
  if (span >= TAU - 0.0001) {
    const ring = (r: number, sweep: number) =>
      `M ${donutPt(r, 0)} A ${r} ${r} 0 1 ${sweep} ${donutPt(r, Math.PI)} A ${r} ${r} 0 1 ${sweep} ${donutPt(r, TAU)} Z`;
    return `${ring(DONUT_OUT, 1)} ${ring(DONUT_IN, 0)}`;
  }
  // Corner radius shrinks on thin slivers so the roundings never overlap.
  const s = Math.sin(span / 2);
  const rc = Math.max(
    0.2,
    Math.min(3, (DONUT_OUT - DONUT_IN) / 2, (DONUT_IN * s) / (1 - s), (DONUT_OUT * s) / (1 + s)),
  );
  const f1 = Math.asin(rc / (DONUT_OUT - rc)); // angle eaten by an outer corner
  const f0 = Math.asin(rc / (DONUT_IN + rc)); // …and by an inner corner
  const d1 = Math.sqrt((DONUT_OUT - rc) ** 2 - rc * rc); // corner tangents on the radial cuts
  const d0 = Math.sqrt((DONUT_IN + rc) ** 2 - rc * rc);
  return [
    `M ${donutPt(d1, a0)}`,
    `A ${rc} ${rc} 0 0 1 ${donutPt(DONUT_OUT, a0 + f1)}`,
    `A ${DONUT_OUT} ${DONUT_OUT} 0 ${span - 2 * f1 > Math.PI ? 1 : 0} 1 ${donutPt(DONUT_OUT, a1 - f1)}`,
    `A ${rc} ${rc} 0 0 1 ${donutPt(d1, a1)}`,
    `L ${donutPt(d0, a1)}`,
    `A ${rc} ${rc} 0 0 1 ${donutPt(DONUT_IN, a1 - f0)}`,
    `A ${DONUT_IN} ${DONUT_IN} 0 ${span - 2 * f0 > Math.PI ? 1 : 0} 0 ${donutPt(DONUT_IN, a0 + f0)}`,
    `A ${rc} ${rc} 0 0 1 ${donutPt(d0, a0)}`,
    "Z",
  ].join(" ");
}

/// Hover nudges a wedge outward along its bisector, Mac-style.
function donutPop(g: { a0: number; a1: number }): { tx: string; ty: string } {
  const mid = (g.a0 + g.a1) / 2;
  return { tx: `${(2.5 * Math.sin(mid)).toFixed(2)}px`, ty: `${(-2.5 * Math.cos(mid)).toFixed(2)}px` };
}

/// Hover text for the "Others" wedge/row: who's inside and what each spent.
function othersBreakdown(e: DonutEntry): string {
  if (!e.parts) return "";
  return (
    `${t("spend.underEach", { limit: e.foldLimit ?? 1 })}\n` +
    e.parts.map((p) => `${p.name}  ${fmtSpendVal(p.w)}`).join("\n")
  );
}

function legendHtml(entries: DonutEntry[]): string {
  return entries
    .map(
      (e) => `
        <div class="legend-row" data-pid="${e.s.id}"${e.parts ? ` title="${escapeHtml(othersBreakdown(e))}"` : ""}>
          <span class="dot" style="background:${spendColor(e.s.id)}"></span>
          <span class="legend-name">${escapeHtml(e.s.name)}</span>
          <span class="legend-val">${fmtSpendVal(e.w)}</span>
        </div>`,
    )
    .join("");
}

/// Tab switch morphs the existing arcs in place (identity-keyed per
/// provider, CSS-transitioned) instead of rebuilding the card.
function switchSpendTab(tab: SpendTab): void {
  spendTab = tab;
  void patchConfig({ spendTab });
  const card = document.querySelector<HTMLElement>(".total-spend");
  const paths = card ? Array.from(card.querySelectorAll<SVGPathElement>("path.seg")) : [];
  const entries = donutEntries(tab);
  const { geo } = donutGeometry(entries);
  // Wedge paths share one command structure so CSS can tween `d`; a
  // full-circle annulus doesn't, so single-spender states rebuild instead.
  const morphable =
    card &&
    paths.length > 0 &&
    geo.size >= 2 &&
    paths.every((p) => !p.dataset.full) &&
    [...geo.keys()].every((id) => paths.some((p) => p.dataset.pid === id));
  if (!morphable) {
    renderAll();
    return;
  }
  const entryById = new Map(entries.map((en) => [en.s.id, en]));
  for (const p of paths) {
    const g = geo.get(p.dataset.pid ?? "");
    if (g) {
      const pop = donutPop(g);
      p.style.opacity = "1";
      p.style.setProperty("d", `path("${sectorPath(g.a0, g.a1)}")`);
      p.style.setProperty("--tx", pop.tx);
      p.style.setProperty("--ty", pop.ty);
    } else {
      p.style.opacity = "0";
    }
    // The Others wedge bakes its breakdown into an SVG <title>; the legend
    // rebuilds below but this child wouldn't, so sync it to the new period
    // (and drop it from any wedge that no longer carries a breakdown).
    const en = entryById.get(p.dataset.pid ?? "");
    const text = en?.parts ? othersBreakdown(en) : "";
    const t = p.querySelector("title");
    if (text) {
      if (t) {
        t.textContent = text;
      } else {
        const nt = document.createElementNS("http://www.w3.org/2000/svg", "title");
        nt.textContent = text;
        p.appendChild(nt);
      }
    } else if (t) {
      t.remove();
    }
  }
  const totalEl = card.querySelector(".donut-total");
  const center = spendCenter(entries);
  if (totalEl) totalEl.textContent = center.primary;
  const legend = card.querySelector(".legend");
  if (legend) legend.innerHTML = legendHtml(entries);
  card.querySelectorAll(".tab").forEach((t) => {
    t.classList.toggle("active", t.getAttribute("data-tab") === tab);
  });
  const wrap = card.querySelector<HTMLElement>(".donut-wrap");
  if (wrap) {
    wrap.title = t("spend.clickTip", {
      exact: center.exact,
      next: t(`spend.metric.${nextSpendMetric(false)}`),
    });
  }
}

function renderTotalSpend(): string {
  if (!config.showTotalSpend) return "";
  const entries = donutEntries(spendTab);
  if (lastSpend.length === 0) {
    // Quiet state instead of a missing card — on a fresh PC the donut only
    // appears after a CLI (Claude Code, Codex, Grok…) has logged some usage.
    const note = spendLoaded ? t("spend.emptyFirst") : t("spend.scanning");
    return `
      <article class="provider total-spend">
        <div class="provider-head">
          <span class="provider-name">${escapeHtml(t("spend.title"))}</span>
        </div>
        <div class="card-panel"><p class="placeholder" style="margin:4px 0">${note}</p></div>
      </article>`;
  }

  const { geo } = donutGeometry(entries);
  const segments = entries
    .filter((e) => geo.has(e.s.id))
    .map((e) => {
      const g = geo.get(e.s.id)!;
      const pop = donutPop(g);
      const full = g.a1 - g.a0 >= TAU - 0.0001 ? ` data-full="1"` : "";
      const hint = e.parts ? `<title>${escapeHtml(othersBreakdown(e))}</title>` : "";
      return `<path class="seg" data-pid="${e.s.id}"${full} fill-rule="evenodd"
        d="${sectorPath(g.a0, g.a1)}" style="fill:${spendColor(e.s.id)};--tx:${pop.tx};--ty:${pop.ty}">${hint}</path>`;
    })
    .join("");

  const legend = legendHtml(entries);

  const tab = (id: SpendTab, label: string) =>
    `<button class="tab${spendTab === id ? " active" : ""}" data-tab="${id}">${label}</button>`;

  const center = spendCenter(entries);
  const exact = t("spend.clickTip", {
    exact: center.exact,
    next: t(METRIC_NAMES[nextSpendMetric(false)]),
  });
  // An empty window still draws the ring — a zeroed track with $0.00 in the
  // center — so the card doesn't collapse to bare text between periods.
  const body = entries.length
    ? `
      <div class="donut-wrap" title="${escapeHtml(exact)}">
        <svg width="96" height="96" viewBox="0 0 96 96">
          ${segments}
          <text class="donut-total" x="48" y="50" text-anchor="middle" font-size="14" font-weight="600">${center.primary}</text>
          <text class="donut-sub" x="48" y="62" text-anchor="middle" font-size="8">${center.sub}</text>
        </svg>
        <div class="legend">${legend}</div>
      </div>`
    : `
      <div class="donut-wrap donut-empty" title="${escapeHtml(t("spend.emptyPeriodTip"))}">
        <svg width="96" height="96" viewBox="0 0 96 96">
          <path class="seg donut-zero" data-full="1" fill-rule="evenodd" d="${sectorPath(0, TAU)}"/>
          <text class="donut-total" x="48" y="50" text-anchor="middle" font-size="14" font-weight="600">${center.primary}</text>
          <text class="donut-sub" x="48" y="62" text-anchor="middle" font-size="8">${center.sub}</text>
        </svg>
        <div class="legend"><p class="placeholder" style="margin:0">${escapeHtml(t("spend.emptyPeriod"))}</p></div>
      </div>`;

  const contributors = lastSpend.map((s) => s.name).join(", ");
  return `
    <article class="provider total-spend">
      <div class="provider-head">
        <span class="provider-name">${escapeHtml(t("spend.title"))}</span>
        <span class="info" title="${escapeHtml(t("spend.info", { names: contributors }))}">&#9432;</span>
        <span class="spacer"></span>
        <button class="share-btn" data-share="__total__" title="${escapeHtml(t("card.share"))}">⧉</button>
      </div>
      <div class="card-panel">
        <div class="tabs">
          ${tab("today", t("spend.today"))}${tab("yesterday", t("spend.yesterday"))}${tab("last30", t("spend.days30"))}
        </div>
        ${body}
      </div>
    </article>`;
}

// ---------------------------------------------------------------------------
// Footer update flow — every popover open re-checks; the version stamp
// becomes "Checking for updates…" and then an Update button on a hit.
// ---------------------------------------------------------------------------

let buildText = "";
let updateVersion: string | null = null;
let checkingUpdate = false;

function renderBuildInfo(): void {
  const el = document.querySelector<HTMLElement>("#build-info");
  if (!el) return;
  if (updateVersion) {
    if (document.querySelector("#update-btn")) return;
    const version = updateVersion;
    const btn = document.createElement("button");
    btn.id = "update-btn";
    btn.textContent = t("update.to", { version });
    btn.addEventListener("click", () => {
      btn.textContent = t("update.installing");
      btn.disabled = true;
      // On success the app restarts, so only the failure path matters:
      // re-enable the button and surface the reason.
      invoke("install_update").catch((err) => {
        btn.textContent = t("update.retry", { version });
        btn.disabled = false;
        const status = document.querySelector("#status");
        if (status) status.textContent = t("footer.updateFailed", { err: String(err) });
      });
    });
    el.replaceChildren(btn);
  } else {
    el.textContent = checkingUpdate ? t("update.check") : buildText;
  }
}

async function checkForUpdate(): Promise<void> {
  if (checkingUpdate || updateVersion) return;
  checkingUpdate = true;
  renderBuildInfo();
  try {
    // Only ever upgrade knowledge: a null result must not erase a version
    // the background checker announced while this check was in flight.
    const v = await invoke<string | null>("check_update");
    if (v) updateVersion = v;
  } catch {
    // Offline or GitHub unreachable — the stamp just returns; the
    // 4-hourly background checker will try again anyway.
  }
  checkingUpdate = false;
  renderBuildInfo();
}

// ---------------------------------------------------------------------------
// Share cards — the live card element rasterized to PNG on the clipboard
// ---------------------------------------------------------------------------

/// Copy a card exactly as it appears on screen: serialize the live card
/// element plus the app stylesheet into an SVG <foreignObject> and
/// rasterize it at 2x. Whatever the card renders — donut, tabs, trend
/// bars, future rows — the copied image matches automatically, instead
/// of a hand-drawn approximation that drifts from the real UI.
/// In-app replacement for window.confirm: the native dialog renders as a
/// bare "localhost says" browser popup, which has no place in a glass UI.
/// Resolves true on confirm; Esc, the ✕, backdrop clicks, and Cancel all
/// resolve false. The keydown listener runs in the capture phase and stops
/// propagation so the app's global Esc (close panels) stays out of it.
/// Cancels the open appConfirm dialog, if any. The popover hides on focus
/// loss with the dialog still in the DOM — reopening must not resurface a
/// stale question, so the reopen routine dismisses it like Esc would.
let dismissConfirm: (() => void) | null = null;

function appConfirm(opts: {
  title: string;
  message: string;
  confirmLabel: string;
  danger?: boolean;
}): Promise<boolean> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.id = "confirm-overlay";
    overlay.innerHTML = `
      <div id="confirm-box" role="dialog" aria-modal="true">
        <h3>${escapeHtml(opts.title)}</h3>
        <p>${escapeHtml(opts.message)}</p>
        <div id="confirm-actions">
          <button id="confirm-cancel" type="button">${escapeHtml(t("dialog.cancel"))}</button>
          <button id="confirm-ok" type="button" class="${opts.danger ? "danger" : ""}">${escapeHtml(opts.confirmLabel)}</button>
        </div>
      </div>`;
    const done = (ok: boolean) => {
      dismissConfirm = null;
      document.removeEventListener("keydown", onKey, true);
      overlay.remove();
      resolve(ok);
    };
    dismissConfirm = () => done(false);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        done(false);
      }
    };
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) done(false);
    });
    overlay.querySelector("#confirm-cancel")!.addEventListener("click", () => done(false));
    overlay.querySelector("#confirm-ok")!.addEventListener("click", () => done(true));
    document.addEventListener("keydown", onKey, true);
    document.body.appendChild(overlay);
    overlay.querySelector<HTMLButtonElement>("#confirm-ok")!.focus();
  });
}

// ---------------------------------------------------------------------------
// Changelog — "What's new" after an update + the Settings viewer
// ---------------------------------------------------------------------------

interface ChangelogSection {
  version: string;
  date: string;
  body: string;
}

/// CHANGELOG.md split into per-version sections, newest first. The
/// "Unreleased" section is skipped — a shipped build's own notes carry its
/// version header (release retitles Unreleased), so users only ever see
/// released entries.
function parseChangelog(): ChangelogSection[] {
  const sections: ChangelogSection[] = [];
  for (const block of changelogRaw.split(/^## /m).slice(1)) {
    const nl = block.indexOf("\n");
    const header = block.slice(0, nl).trim();
    if (/^unreleased$/i.test(header)) continue;
    const m = header.match(/^([\d.]+)\s*—\s*(.+)$/);
    sections.push({
      version: m ? m[1] : header,
      date: m ? m[2] : "",
      body: block.slice(nl + 1).trim(),
    });
  }
  return sections;
}

/// Markdown-lite for changelog bodies: ### subheads, - bullets (with hanging
/// continuation lines), plain paragraphs, **bold**, `code`. Bullets and
/// paragraphs accumulate as raw markdown and are transformed only on flush,
/// so a bold/code span wrapped across the file's ~70-column lines still
/// matches. Input is escaped before any markup is applied, so the changelog
/// can never inject HTML.
function renderChangelogBody(md: string): string {
  const inline = (s: string) =>
    escapeHtml(s)
      .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
      .replace(/`([^`]+)`/g, "<code>$1</code>");
  let html = "";
  let items: string[] = [];
  let para = "";
  const flushItems = () => {
    if (items.length) html += `<ul>${items.map((i) => `<li>${inline(i)}</li>`).join("")}</ul>`;
    items = [];
  };
  const flushPara = () => {
    if (para) html += `<p>${inline(para)}</p>`;
    para = "";
  };
  for (const line of md.split("\n")) {
    if (line.startsWith("### ")) {
      flushItems();
      flushPara();
      html += `<h5>${escapeHtml(line.slice(4).trim())}</h5>`;
    } else if (line.startsWith("- ")) {
      flushPara();
      items.push(line.slice(2));
    } else if (/^\s+\S/.test(line) && items.length) {
      items[items.length - 1] += " " + line.trim();
    } else if (line.trim()) {
      flushItems();
      para += (para ? " " : "") + line.trim();
    } else {
      flushPara();
    }
  }
  flushItems();
  flushPara();
  return html;
}

/// Same lifecycle as dismissConfirm: the popover reopen routine clears a
/// stale dialog left behind by hide-on-focus-loss.
let dismissWhatsNew: (() => void) | null = null;

/// Card-styled scrollable dialog listing changelog sections. Esc, backdrop
/// clicks (anywhere outside the card), and the Got it button all dismiss.
function showChangelogDialog(title: string, sections: ChangelogSection[]): void {
  dismissWhatsNew?.();
  const overlay = document.createElement("div");
  overlay.id = "whatsnew-overlay";
  const list = sections
    .map(
      (s) =>
        `<section><h4>v${escapeHtml(s.version)}${
          s.date ? `<span>${escapeHtml(s.date)}</span>` : ""
        }</h4>${renderChangelogBody(s.body)}</section>`,
    )
    .join("");
  overlay.innerHTML = `
    <div id="whatsnew-box" role="dialog" aria-modal="true">
      <h3>${escapeHtml(title)}</h3>
      <div id="whatsnew-body">${list}</div>
      <div id="whatsnew-actions">
        <button id="whatsnew-ok" type="button">${escapeHtml(t("dialog.gotIt"))}</button>
      </div>
    </div>`;
  const done = () => {
    dismissWhatsNew = null;
    document.removeEventListener("keydown", onKey, true);
    overlay.remove();
  };
  dismissWhatsNew = done;
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      done();
    }
  };
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) done();
  });
  overlay.querySelector("#whatsnew-ok")!.addEventListener("click", done);
  document.addEventListener("keydown", onKey, true);
  document.body.appendChild(overlay);
}

/// The sections a just-updated install hasn't seen yet (newest first,
/// capped), or null when there's nothing to announce. Marks the current
/// version as seen immediately so the dialog can only ever appear once per
/// version, even if it's dismissed by closing the popover.
let appVersion = "";
let pendingWhatsNew: ChangelogSection[] | null = null;

function computeWhatsNew(version: string): ChangelogSection[] | null {
  const last = config.lastSeenVersion;
  if (last === version) return null;
  void patchConfig({ lastSeenVersion: version });
  const all = parseChangelog();
  if (!last) {
    // First run with this feature. An install that already dismissed the
    // welcome card is an *update* — show the new version's notes. A true
    // fresh install gets the welcome card instead, not two popups. Guard
    // the empty case (e.g. a build whose notes are still Unreleased) —
    // an empty array is truthy and would present a blank dialog.
    const own = config.welcomeDismissed ? all.filter((s) => s.version === version) : [];
    return own.length ? own : null;
  }
  const out: ChangelogSection[] = [];
  for (const s of all) {
    if (s.version === last || out.length >= 5) break;
    out.push(s);
  }
  return out.length ? out : null;
}

async function shareCard(id: string): Promise<void> {
  const status = document.querySelector("#status")!;
  try {
    const el =
      id === "__total__"
        ? document.querySelector<HTMLElement>("article.total-spend")
        : document.querySelector<HTMLElement>(`article.provider[data-provider="${id}"]`);
    if (!el) return;

    const rect = el.getBoundingClientRect();
    const W = Math.ceil(rect.width);
    const S = 2;
    const PAD = 20; // frame around the card, like the Mac share cards
    const FOOT = 30; // logo + tagline row

    let css = "";
    for (const sheet of Array.from(document.styleSheets)) {
      try {
        for (const rule of Array.from(sheet.cssRules)) css += rule.cssText + "\n";
      } catch {
        // Inaccessible sheet (shouldn't happen — all styles are bundled).
      }
    }
    // Static rasterization renders CSS animations at time zero, which for
    // the entrance animations means an invisible card. Freeze final state.
    // The body's inherited text styles are re-declared on the wrapper since
    // the snapshot document has no <body>.
    const bodyStyle = getComputedStyle(document.body);
    css +=
      "*{animation:none!important;transition:none!important}" +
      "#snap-root .share-btn{display:none!important}" +
      `#snap-foot{display:flex;align-items:center;justify-content:center;gap:6px;` +
      `height:${FOOT}px;color:var(--muted-foreground);font-size:12px}` +
      "#snap-foot img{width:16px;height:16px;border-radius:4px}";

    const clone = el.cloneNode(true) as HTMLElement;
    clone.style.margin = "0";
    clone.style.width = `${W}px`;
    clone.style.boxSizing = "border-box";

    // Shares are strictly what's on screen: everything the card currently
    // renders — bars, pace hints, the trend, and the On Demand section
    // when it's open — copies as-is. Only interactive chrome (buttons,
    // links, carets, grips) never belongs in an image. (The old "compact
    // composition" for collapsed cards is retired: it dropped the visible
    // trend and pace hints, which read as missing data in the copy.)
    // .snap-card restores the card surface the popover no longer draws
    // (cards sit flat on the background there, panels carry the chrome).
    clone.classList.add("snap-card");
    if (id !== "__total__") {
      clone
        .querySelectorAll(".share-btn, .card-caret, .quick-links, .action-row, .drag-grip")
        .forEach((n) => n.remove());
    }

    // The curated clone is shorter than the on-screen card (chrome
    // removed), so measure IT — briefly attached offscreen — instead of
    // sizing the canvas from the original and leaving dead space.
    clone.style.position = "fixed";
    clone.style.left = "-99999px";
    clone.style.top = "0";
    document.body.appendChild(clone);
    const H = Math.ceil(clone.getBoundingClientRect().height);
    clone.remove();
    clone.style.position = "";
    clone.style.left = "";
    clone.style.top = "";
    const W2 = W + PAD * 2;
    const H2 = H + PAD * 2 + FOOT;
    css +=
      `#snap-root{font-family:${bodyStyle.fontFamily};font-size:${bodyStyle.fontSize};` +
      `color:${bodyStyle.color};letter-spacing:${bodyStyle.letterSpacing};` +
      `background:var(--background);padding:${PAD}px;box-sizing:border-box;` +
      `width:${W2}px;height:${H2}px}`;

    // data-theme / data-density live on <html>; :root of the snapshot
    // document is the <svg>, so the attributes are mirrored there for the
    // :root[data-…] rules to keep matching.
    const root = document.documentElement;
    const svgMarkup =
      `<svg xmlns="http://www.w3.org/2000/svg" width="${W2 * S}" height="${H2 * S}" ` +
      `viewBox="0 0 ${W2} ${H2}" data-theme="${root.dataset.theme ?? ""}" ` +
      `data-density="${root.dataset.density ?? ""}">` +
      `<foreignObject width="${W2}" height="${H2}">` +
      `<div xmlns="http://www.w3.org/1999/xhtml" id="snap-root">` +
      // CDATA so CSS containing XML-special characters (`<`, `&` — e.g. in
      // a content: string) can never malform the snapshot document. A
      // literal "]]>" inside CSS would end the section early, so split it.
      `<style><![CDATA[${css.split("]]>").join("]]]]><![CDATA[>")}]]></style>` +
      new XMLSerializer().serializeToString(clone) +
      `<div id="snap-foot"><img src="${paneIcon}" alt="" /><span>${escapeHtml(t("share.tagline"))}</span></div>` +
      `</div></foreignObject></svg>`;

    const img = new Image();
    img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svgMarkup)}`;
    await img.decode();

    const canvas = document.createElement("canvas");
    canvas.width = W2 * S;
    canvas.height = H2 * S;
    const ctx = canvas.getContext("2d")!;
    ctx.drawImage(img, 0, 0);

    const dataUrl = canvas.toDataURL("image/png");
    const pngBase64 = dataUrl.slice(dataUrl.indexOf(",") + 1);
    await invoke("copy_share_image", { pngBase64 });
    status.textContent = t("footer.copied");
  } catch (err) {
    status.textContent = t("footer.shareFailed", { err: String(err) });
  }
}

// ---------------------------------------------------------------------------
// Liquid glass lens (prasen.dev original). A rounded-rect signed-distance
// field drives the displacement map, so refraction is concentrated at the
// rim while the center stays optically flat — like iOS Liquid Glass.
// ---------------------------------------------------------------------------

function generateLensMap(w: number, h: number): string | null {
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;
  const img = ctx.createImageData(w, h);
  const data = img.data;
  const cx = w / 2;
  const cy = h / 2;
  const radius = Math.min(w, h) / 2;
  const halfW = Math.max(w / 2 - radius, 0);
  const halfH = Math.max(h / 2 - radius, 0);
  const rim = 1.1 * radius; // bend zone width, measured inward from the edge
  let i = 0;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const ax = x + 0.5 - cx;
      const ay = y + 0.5 - cy;
      const px = Math.abs(ax) - halfW;
      const py = Math.abs(ay) - halfH;
      const sdf =
        Math.min(Math.max(px, py), 0) + Math.hypot(Math.max(px, 0), Math.max(py, 0)) - radius;
      let g = 0;
      if (sdf > -rim) {
        const e = Math.min(Math.max(1 + sdf / rim, 0), 1);
        g = e * e * (3 - 2 * e); // smoothstep toward the edge
      }
      data[i++] = Math.round(128 + (ax / (w / 2)) * g * 110);
      data[i++] = Math.round(128 + (ay / (h / 2)) * g * 110);
      data[i++] = 128;
      data[i++] = 255;
    }
  }
  ctx.putImageData(img, 0, 0);
  return canvas.toDataURL();
}

function applyLens(el: HTMLElement | null, filterId: string, imgId: string): void {
  if (!el) return;
  const w = 4 * Math.round(el.offsetWidth / 4);
  const h = 4 * Math.round(el.offsetHeight / 4);
  if (w < 8 || h < 8) return;
  const filter = document.getElementById(filterId);
  const img = document.getElementById(imgId);
  const map = generateLensMap(w, h);
  if (!filter || !img || !map) return;
  filter.setAttribute("width", String(w));
  filter.setAttribute("height", String(h));
  img.setAttribute("width", String(w));
  img.setAttribute("height", String(h));
  img.setAttribute("href", map);
  const f = `url(#${filterId}) blur(2px) saturate(1.8) brightness(1.04)`;
  el.style.backdropFilter = f;
  (el.style as unknown as Record<string, string>).webkitBackdropFilter = f;
}

/// "Liquid glass effects" off swaps the SDF refraction + backdrop blurs
/// for flat surfaces (body.no-glass CSS overrides win over the inline
/// styles applyLens sets). The expensive displacement filters then never
/// run — the fix for laptops where the popover animates below 60 fps.
function applyGlass(): void {
  document.body.classList.toggle("no-glass", config.glassEffects === false);
  // Lens init is skipped entirely while glass is off — build the maps the
  // first time the user turns it on.
  if (config.glassEffects !== false && !lensReady) initLiquidLens();
}

function reduceMotion(): boolean {
  return (
    config.reduceAnimations === true ||
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function applyReduceMotion(): void {
  document.body.classList.toggle("reduce-anim", config.reduceAnimations === true);
}

let lensReady = false;

function initLiquidLens(): void {
  if (config.glassEffects === false || lensReady) return;
  lensReady = true;
  const surfaces: [string, string, HTMLElement | null][] = [
    ["lens-side", "lens-map-side", document.querySelector(".sidebar")],
    ["lens-footer", "lens-map-footer", document.querySelector(".main-col footer")],
  ];
  for (const [filterId, imgId, el] of surfaces) {
    if (!el) continue;
    applyLens(el, filterId, imgId);
    new ResizeObserver(() => applyLens(el, filterId, imgId)).observe(el);
  }

  // Panel header bars (Customize / Settings) share one lens sized to the
  // window width. Applied through a CSS variable so re-rendered bars keep
  // the effect without JS re-application.
  const w = 4 * Math.round(window.innerWidth / 4);
  const h = 44;
  const filter = document.getElementById("lens-bar");
  const img = document.getElementById("lens-map-bar");
  const map = generateLensMap(w, h);
  if (filter && img && map) {
    filter.setAttribute("width", String(w));
    filter.setAttribute("height", String(h));
    img.setAttribute("width", String(w));
    img.setAttribute("height", String(h));
    img.setAttribute("href", map);
    document.documentElement.style.setProperty(
      "--bar-filter",
      "url(#lens-bar) blur(2px) saturate(1.8) brightness(1.04)",
    );
  }
}

// ---------------------------------------------------------------------------
// Appearance (System / Light / Dark) + density (Regular / Compact)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tooltip bubbles: every `title` attribute is silently upgraded to a custom
// bubble — 400ms deliberate dwell, balanced wrapping, anchored to the item.
// ---------------------------------------------------------------------------

function setupTooltips(): void {
  const tip = document.createElement("div");
  tip.id = "hover-tip";
  tip.hidden = true;
  document.body.appendChild(tip);
  let timer = 0;
  let anchor: HTMLElement | null = null;

  const hide = () => {
    clearTimeout(timer);
    tip.hidden = true;
    anchor = null;
  };

  document.addEventListener("mouseover", (e) => {
    const el = (e.target as HTMLElement).closest<HTMLElement>("[title], [data-tip]");
    if (!el) return;
    const title = el.getAttribute("title");
    if (title) {
      el.dataset.tip = title;
      el.removeAttribute("title"); // suppress the native tooltip
    }
    if (!el.dataset.tip || el === anchor) return;
    anchor = el;
    clearTimeout(timer);
    timer = window.setTimeout(() => {
      if (anchor !== el || !document.contains(el)) return;
      tip.textContent = el.dataset.tip ?? "";
      tip.hidden = false;
      const r = el.getBoundingClientRect();
      const w = tip.offsetWidth;
      const h = tip.offsetHeight;
      const x = Math.max(6, Math.min(r.left + r.width / 2 - w / 2, window.innerWidth - w - 6));
      let y = r.top - h - 8;
      if (y < 6) y = r.bottom + 8;
      tip.style.left = `${x}px`;
      tip.style.top = `${y}px`;
    }, 400);
  });
  document.addEventListener("mouseout", (e) => {
    const el = (e.target as HTMLElement).closest<HTMLElement>("[data-tip]");
    const to = e.relatedTarget as HTMLElement | null;
    if (el && (!to || !el.contains(to))) hide();
  });
  document.addEventListener("scroll", hide, true);
  document.addEventListener("mousedown", hide, true);
}

// ---------------------------------------------------------------------------
// Customize undo — whole-layout snapshots, Ctrl+Z restores.
// ---------------------------------------------------------------------------

const undoStack: string[] = [];
let lastLayoutSnapshot = "";

function undoLayout(): void {
  const prev = undoStack.pop();
  if (!prev) return;
  config.layout = JSON.parse(prev) as Layout;
  lastLayoutSnapshot = prev;
  void patchConfig({ layout: config.layout });
  renderAll();
  requestTraySync();
  document.querySelector("#status")!.textContent = "Layout change undone";
}

// ---------------------------------------------------------------------------
// Party mode 🎉 — ↑↑↓↓←→←→BA. Purely cosmetic, never persisted.
// ---------------------------------------------------------------------------

const KONAMI = [
  "ArrowUp", "ArrowUp", "ArrowDown", "ArrowDown",
  "ArrowLeft", "ArrowRight", "ArrowLeft", "ArrowRight", "b", "a",
];
let konamiAt = 0;

function toggleParty(): void {
  const on = document.body.classList.toggle("party");
  document.querySelector("#status")!.textContent = on ? "🎉 Party mode!" : "Party's over.";
}

function konamiListen(e: KeyboardEvent): void {
  const key = e.key.length === 1 ? e.key.toLowerCase() : e.key;
  konamiAt = key === KONAMI[konamiAt] ? konamiAt + 1 : key === KONAMI[0] ? 1 : 0;
  if (konamiAt === KONAMI.length) {
    konamiAt = 0;
    toggleParty();
  }
}

const systemLight = window.matchMedia("(prefers-color-scheme: light)");

function applyAppearance(): void {
  const mode =
    config.appearance === "system" ? (systemLight.matches ? "light" : "dark") : config.appearance;
  document.documentElement.dataset.theme = mode;
  document.documentElement.dataset.density = config.density;
  const btn = document.querySelector<HTMLElement>("#theme-btn");
  if (btn) {
    btn.textContent = mode === "light" ? "☾" : "☀";
    btn.title = mode === "light" ? t("sidebar.themeToDark") : t("sidebar.themeToLight");
    delete btn.dataset.tip;
  }
}

/// Day/night toggle with the circular wipe from jazii.dev: the new theme
/// expands as a clip-path circle from the button via the View Transitions
/// API. Falls back to an instant switch where unsupported.
function toggleTheme(e: Event): void {
  const next = document.documentElement.dataset.theme === "light" ? "dark" : "light";
  const apply = () => {
    config.appearance = next;
    applyAppearance();
    const select = document.querySelector<HTMLSelectElement>("#appearance");
    if (select) select.value = next;
  };

  const btn = e.currentTarget as HTMLElement;
  const rect = btn.getBoundingClientRect();
  const x = rect.left + rect.width / 2;
  const y = rect.top + rect.height / 2;
  const maxRadius = Math.hypot(
    Math.max(x, window.innerWidth - x),
    Math.max(y, window.innerHeight - y),
  );

  const doc = document as Document & { startViewTransition?: (cb: () => void) => { ready: Promise<void> } };
  if (!reduceMotion() && doc.startViewTransition) {
    const transition = doc.startViewTransition(apply);
    transition.ready
      .then(() => {
        document.documentElement.animate(
          [
            { clipPath: `circle(0px at ${x}px ${y}px)` },
            { clipPath: `circle(${maxRadius}px at ${x}px ${y}px)` },
          ],
          {
            duration: 500,
            easing: "cubic-bezier(0.4, 0, 0.2, 1)",
            pseudoElement: "::view-transition-new(root)",
          },
        );
      })
      .catch(() => {});
  } else {
    apply();
  }
  void patchConfig({ appearance: next });
}

systemLight.addEventListener("change", () => {
  if (config.appearance === "system") applyAppearance();
});

// ---------------------------------------------------------------------------
// Customize screen
// ---------------------------------------------------------------------------

function isStarrable(s: Snapshot | undefined, key: string): boolean {
  return s?.metrics.some((m) => m.label === key && m.kind === "progress") ?? false;
}

// Providers start collapsed in Customize; only what you're editing unfolds.
// Session-only — collapsing again on reopen keeps the list scannable.
const custExpanded = new Set<string>();

// Which provider's inline config panel is open (one at a time) and the
// live search text — session-only, like custExpanded.
let custConfigOpen: string | null = null;
let custSearchQuery = "";

// Which provider's read-only credential-info panel ("?" button) is open.
// One panel at a time, and opening one side closes the other.
let custInfoOpen: string | null = null;

// "Stored key / env key / local sign-in" answers from get_credential_status,
// cached so a background re-render of the drawer doesn't blank the panels.
interface CredStatus {
  storedKey: boolean;
  envKey: boolean;
  localCli: string | null;
  // Account label of Pane's own OAuth login (codex/grok), null when none.
  oauth: string | null;
}
const credStatusCache = new Map<string, CredStatus>();

function fetchCredStatus(id: string): void {
  if (credStatusCache.has(id)) return;
  refreshCredStatus(id);
}

/// Fresh probe of get_credential_status, then every open panel slot for
/// this provider is repainted in place ("?" status line, gear chips, the
/// saved-credential list) — a re-render would also pick the cache up.
function refreshCredStatus(id: string): void {
  void invoke<CredStatus>("get_credential_status", { provider: id })
    .then((status) => {
      credStatusCache.set(id, status);
      paintCredStatus(id);
    })
    .catch(() => {
      credStatusCache.set(id, { storedKey: false, envKey: false, localCli: null, oauth: null });
      paintCredStatus(id);
    });
}

function paintCredStatus(id: string): void {
  const sel = (attr: string) =>
    document.querySelector<HTMLElement>(`#drawer-body [${attr}="${CSS.escape(id)}"]`);
  const line = sel("data-cred-status");
  if (line) line.innerHTML = credStatusLine(id);
  const chips = sel("data-cred-chips");
  if (chips) chips.innerHTML = credChipsHtml(id);
  const accounts = sel("data-cred-accounts");
  if (accounts) accounts.innerHTML = credAccountsHtml(id);
  paintOAuth(id);
}

// Providers whose credential is a plain API key saved through set_api_key.
// The rest sign in through their own CLI or desktop login instead.
const KEY_PROVIDERS = new Set([
  "openrouter",
  "zai",
  "minimax",
  "deepseek",
  "moonshot",
  "elevenlabs",
  "codebuff",
  "kilo",
  "aihubmix",
  "qwen",
  "kimi",
  "opencode",
  "stepfun",
  "siliconflow",
  "novita",
  "relaybalance",
]);

/// Credential facts for the Customize "?" panel — how each provider gets
/// its quota read and which sign-in methods exist. Every entry was checked
/// against the matching src-tauri/src/providers/*.rs source (docstring +
/// key lookup); `auto` is an i18n key because the facts render in the UI
/// language. Methods: paste = an API key field works, oauth = the vendor's
/// OAuth flow, local = its own CLI/desktop sign-in.
type CredMethod = "paste" | "oauth" | "local";
const PROVIDER_CRED_INFO: Record<string, { auto: string; methods: CredMethod[] }> = {
  claude: { auto: "customize.cred.claude", methods: ["local"] },
  codex: { auto: "customize.cred.codex", methods: ["local", "oauth"] },
  cursor: { auto: "customize.cred.cursor", methods: ["local"] },
  opencode: { auto: "customize.cred.opencode", methods: ["paste", "local"] },
  copilot: { auto: "customize.cred.copilot", methods: ["local"] },
  grok: { auto: "customize.cred.grok", methods: ["local", "oauth"] },
  devin: { auto: "customize.cred.devin", methods: ["local"] },
  minimax: { auto: "customize.cred.minimax", methods: ["paste", "local"] },
  openrouter: { auto: "customize.cred.openrouter", methods: ["paste", "local"] },
  zai: { auto: "customize.cred.zai", methods: ["paste", "local"] },
  antigravity: { auto: "customize.cred.antigravity", methods: ["local"] },
  deepseek: { auto: "customize.cred.deepseek", methods: ["paste"] },
  moonshot: { auto: "customize.cred.moonshot", methods: ["paste"] },
  elevenlabs: { auto: "customize.cred.elevenlabs", methods: ["paste"] },
  ollama: { auto: "customize.cred.ollama", methods: [] },
  codebuff: { auto: "customize.cred.codebuff", methods: ["paste", "local"] },
  kilo: { auto: "customize.cred.kilo", methods: ["paste", "local"] },
  aihubmix: { auto: "customize.cred.aihubmix", methods: ["paste", "local"] },
  qwen: { auto: "customize.cred.qwen", methods: ["paste"] },
  hermes: { auto: "customize.cred.hermes", methods: [] },
  kimi: { auto: "customize.cred.kimi", methods: ["oauth", "paste"] },
  stepfun: { auto: "customize.cred.stepfun", methods: ["paste"] },
  siliconflow: { auto: "customize.cred.siliconflow", methods: ["paste"] },
  novita: { auto: "customize.cred.novita", methods: ["paste"] },
  relaybalance: { auto: "customize.cred.relaybalance", methods: ["paste"] },
};

/// The "?" panel's read-only fact sheet: what gets read automatically,
/// which sign-in methods exist, and the live stored/env key status.
function renderCustInfo(id: string): string {
  const info = PROVIDER_CRED_INFO[providerFamily(id)];
  const auto = info ? escapeHtml(t(info.auto)) : "";
  const methods = info
    ? info.methods.map((m) => escapeHtml(t(`customize.credMethod.${m}`))).join(" · ")
    : "";
  const methodLine =
    info && info.methods.length
      ? methods
      : `<span class="dim">${escapeHtml(t("customize.credMethodNone"))}</span>`;
  return `<div class="cust-config cust-info">
      <p><span class="cust-info-label">${escapeHtml(t("customize.credAutoLabel"))}</span>${auto}</p>
      <p><span class="cust-info-label">${escapeHtml(t("customize.credMethodsLabel"))}</span>${methodLine}</p>
      <p><span class="cust-info-label">${escapeHtml(t("customize.credStatusLabel"))}</span><span class="dim" data-cred-status="${escapeHtml(id)}">${escapeHtml(t("customize.credStatusLoading"))}</span></p>
    </div>`;
}

/// One status line ("?" panel): saved key, env var, local sign-in, in
/// that order.
function credStatusLine(id: string): string {
  const status = credStatusCache.get(id);
  if (!status) return escapeHtml(t("customize.credStatusLoading"));
  const parts: string[] = [];
  parts.push(
    status.storedKey
      ? escapeHtml(t("customize.credStored"))
      : escapeHtml(t("customize.credNotStored")),
  );
  if (status.envKey) parts.push(escapeHtml(t("customize.credEnv")));
  if (status.localCli) parts.push(escapeHtml(status.localCli));
  return parts.join(" · ");
}

/// The gear panel's live status chips: one green chip per credential
/// source found on this machine, or a single grey "not configured" chip.
function credChipsHtml(id: string): string {
  const status = credStatusCache.get(id);
  if (!status) return `<span class="dim">${escapeHtml(t("customize.credStatusLoading"))}</span>`;
  const chips: string[] = [];
  if (status.storedKey)
    chips.push(`<span class="cred-chip ok">${escapeHtml(t("customize.chipStoredKey"))}</span>`);
  if (status.envKey)
    chips.push(`<span class="cred-chip ok">${escapeHtml(t("customize.chipEnvKey"))}</span>`);
  if (status.localCli)
    chips.push(
      `<span class="cred-chip ok">${escapeHtml(t("customize.chipLocal", { x: status.localCli }))}</span>`,
    );
  // Pane's own browser sign-in (codex/grok) — the family row carries it;
  // extra CLI account cards don't own the OAuth credential.
  if (status.oauth && providerFamily(id) === id)
    chips.push(
      `<span class="cred-chip ok">${escapeHtml(t("customize.chipOAuth", { x: status.oauth }))}</span>`,
    );
  if (!chips.length)
    chips.push(`<span class="cred-chip none">${escapeHtml(t("customize.chipNone"))}</span>`);
  return chips.join("");
}

/// Phase 2.3 — the credentials Pane itself has saved for this provider,
/// label + source, display only (deletion arrives with Phase 3's
/// multi-account work). Storage holds a single key per provider today, so
/// this is at most one row.
function credAccountsHtml(id: string): string {
  const status = credStatusCache.get(id);
  if (!status?.storedKey) return "";
  return `<li><span class="cust-label">API key</span><span class="dim">${escapeHtml(t("customize.credSourcePane"))}</span></li>`;
}

// ---------------------------------------------------------------------------
// Pane's own OAuth (device code) login — codex/grok, one account each.
// ---------------------------------------------------------------------------

// Providers with a browser sign-in owned by Pane itself (Phase 3.1). The
// backend stores tokens under %APPDATA%\Pane\oauth\<provider>.json.
const OAUTH_PROVIDERS = new Set(["codex", "grok"]);

// ---------------------------------------------------------------------------
// Extra API-key accounts (Phase 3.2) — deepseek/stepfun/siliconflow/novita/
// relaybalance. The gear panel's single-key field stays the family's main
// card; each entry below adds a <provider>@<n> card on the dashboard.
// ---------------------------------------------------------------------------

const MULTI_ACCOUNT_PROVIDERS = new Set([
  "deepseek",
  "stepfun",
  "siliconflow",
  "novita",
  "relaybalance",
]);

/// One account_list row: the masked key ("sk-…abcd") is all that comes
/// back — the full key never leaves the backend.
interface AccountEntry {
  label: string;
  hasKey: boolean;
  maskedKey: string;
  baseUrl?: string | null;
}

// Cached account lists so a drawer re-render doesn't blank the panels,
// same trade-off as credStatusCache.
const accountsCache = new Map<string, AccountEntry[]>();

// Whether the "Add account" mini-form is unfolded on the open panel.
let acctFormOpen = false;

function fetchAccounts(family: string): void {
  if (accountsCache.has(family)) return;
  refreshAccounts(family);
}

function refreshAccounts(family: string): void {
  void invoke<AccountEntry[]>("account_list", { provider: family })
    .then((list) => {
      accountsCache.set(family, list);
      paintAccounts(family);
    })
    .catch(() => {
      accountsCache.set(family, []);
      paintAccounts(family);
    });
}

function paintAccounts(family: string): void {
  const el = document.querySelector<HTMLElement>(
    `#drawer-body [data-accounts-block="${CSS.escape(family)}"]`,
  );
  if (el) el.innerHTML = accountsBlockInner(family);
}

function accountLabelAt(list: AccountEntry[], index: number): string {
  const entry = list[index];
  return entry?.label || t("customize.acctDefaultName", { n: index + 1 });
}

/// The gear panel's account section (multi-account key providers, family
/// row only): saved accounts with per-item delete, plus the collapsible
/// "Add account" form whose Test reuses the test_api_key flow.
function accountsBlockInner(family: string): string {
  const list = accountsCache.get(family);
  let rows: string;
  if (!list) {
    rows = `<p class="dim">${escapeHtml(t("customize.credStatusLoading"))}</p>`;
  } else if (!list.length) {
    rows = `<p class="dim">${escapeHtml(t("customize.acctNone"))}</p>`;
  } else {
    rows = `<ul class="acct-list">${list
      .map(
        (a, i) => `
        <li class="acct-item">
          <span class="acct-label">${escapeHtml(accountLabelAt(list, i))}</span>
          <span class="dim">${escapeHtml(a.maskedKey)}</span>
          <button class="mini-btn acct-del" data-acct-del="${family}|${i}" title="${escapeHtml(t("customize.acctDelete"))}">✕</button>
        </li>`,
      )
      .join("")}</ul>`;
  }
  const baseUrlInput =
    family === "relaybalance"
      ? `<input type="text" data-acct-baseurl="${family}" placeholder="https://api.example.com" spellcheck="false" />`
      : "";
  const form = acctFormOpen
    ? `<div class="acct-form">
        <input type="text" data-acct-label="${family}" placeholder="${escapeHtml(t("customize.acctLabelPh"))}" spellcheck="false" />
        ${baseUrlInput}
        <input type="password" data-acct-key="${family}" placeholder="${escapeHtml(t("settings.keyPlaceholder"))}" />
        <button class="mini-btn" data-acct-test="${family}">${escapeHtml(t("customize.test"))}</button>
        <button class="mini-btn" data-acct-add="${family}" disabled title="${escapeHtml(t("customize.saveAfterTest"))}">${escapeHtml(t("customize.acctAddBtn"))}</button>
        <span class="cust-test-result" data-acct-result="${family}"></span>
      </div>`
    : "";
  return `${rows}
    <button class="mini-btn" data-acct-toggle="${family}">${escapeHtml(acctFormOpen ? t("customize.acctClose") : t("customize.acctAdd"))}</button>
    ${form}`;
}

/// The section only renders on the family's own row — a deepseek@1 card
/// has no accounts of its own.
function renderAccountsBlock(id: string): string {
  if (!MULTI_ACCOUNT_PROVIDERS.has(id) || providerFamily(id) !== id) return "";
  return `<div class="cust-accounts-mgmt" data-accounts-block="${escapeHtml(id)}">${accountsBlockInner(id)}</div>`;
}

/// "Test" inside the Add-account form: the same probe the main key field
/// uses, but it unlocks the form's Add button instead of Save.
async function runAccountKeyTest(family: string): Promise<void> {
  const block = document.querySelector<HTMLElement>(
    `#drawer-body [data-accounts-block="${CSS.escape(family)}"]`,
  );
  const keyInp = block?.querySelector<HTMLInputElement>("[data-acct-key]");
  const result = block?.querySelector<HTMLElement>("[data-acct-result]");
  const addBtn = block?.querySelector<HTMLButtonElement>("[data-acct-add]");
  if (!keyInp || !result) return;
  const show = (text: string, ok: boolean | null) => {
    result.textContent = text;
    result.classList.toggle("ok", ok === true);
    result.classList.toggle("err", ok === false);
  };
  const key = keyInp.value.trim();
  if (!key) {
    show(t("customize.testEmpty"), false);
    if (addBtn) addBtn.disabled = true; // an account without a key is meaningless
    return;
  }
  show(t("customize.testing"), null);
  try {
    const baseUrl =
      block?.querySelector<HTMLInputElement>("[data-acct-baseurl]")?.value.trim() ?? "";
    const r = await invoke<{ ok: boolean; metrics: number; message: string }>("test_api_key", {
      provider: family,
      key,
      baseUrl: baseUrl || null,
    });
    if (r.ok) {
      show(t("customize.testOk", { n: r.metrics }), true);
      if (addBtn) addBtn.disabled = false;
    } else {
      show(`${t("customize.testFailed")}: ${r.message}`, false);
    }
  } catch (err) {
    show(`${t("customize.testFailed")}: ${String(err)}`, false);
  }
}

/// Appends the tested account; the new <provider>@<n> card appears on the
/// follow-up refresh, like a saved main key does.
async function doAccountAdd(family: string): Promise<void> {
  const block = document.querySelector<HTMLElement>(
    `#drawer-body [data-accounts-block="${CSS.escape(family)}"]`,
  );
  const keyInp = block?.querySelector<HTMLInputElement>("[data-acct-key]");
  if (!keyInp?.value.trim()) return;
  const status = document.querySelector("#status")!;
  try {
    await invoke("account_add", {
      provider: family,
      label: block?.querySelector<HTMLInputElement>("[data-acct-label]")?.value.trim() ?? "",
      apiKey: keyInp.value.trim(),
      baseUrl:
        block?.querySelector<HTMLInputElement>("[data-acct-baseurl]")?.value.trim() || null,
    });
    acctFormOpen = false;
    refreshAccounts(family);
    status.textContent = t("customize.acctAdded", { name: providerDisplayName(family) });
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    status.textContent = t("customize.acctAddFailed", { err: String(err) });
  }
}

/// Deletes an account after a confirm; its card vanishes on the refresh
/// this triggers (fetch_usage simply stops spawning it).
async function doAccountRemove(family: string, index: number): Promise<void> {
  const label = accountLabelAt(accountsCache.get(family) ?? [], index);
  const ok = await appConfirm({
    title: t("customize.acctDelTitle"),
    message: t("customize.acctDelBody", { label }),
    confirmLabel: t("customize.acctDelConfirm"),
    danger: true,
  });
  if (!ok) return;
  const status = document.querySelector("#status")!;
  try {
    await invoke("account_remove", { provider: family, index });
    refreshAccounts(family);
    status.textContent = t("customize.acctRemoved");
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    status.textContent = t("customize.acctRemoveFailed", { err: String(err) });
  }
}

/// A login flow between "Sign in with browser" and completion/cancel.
/// Lives outside the DOM so a drawer re-render doesn't lose the code.
interface OAuthFlowState {
  deviceAuthId: string;
  userCode: string;
  error: string | null;
  timer?: number;
}
const oauthFlow = new Map<string, OAuthFlowState>();

function stopOauthFlow(family: string): void {
  const flow = oauthFlow.get(family);
  if (flow?.timer !== undefined) window.clearInterval(flow.timer);
}

function paintOAuth(id: string): void {
  const el = document.querySelector<HTMLElement>(
    `#drawer-body [data-oauth-block="${CSS.escape(id)}"]`,
  );
  if (el) el.innerHTML = oauthBlockInner(id);
}

function oauthBlockInner(id: string): string {
  const flow = oauthFlow.get(id);
  const status = credStatusCache.get(id);
  if (flow?.error) {
    return `<p class="cust-test-result err">${escapeHtml(t("customize.oauth.failed", { err: flow.error }))}</p>
      <div class="cust-actions">
        <button class="mini-btn" data-oauth-login="${id}">${escapeHtml(t("customize.oauth.login"))}</button>
      </div>`;
  }
  if (flow) {
    return `<p class="dim">${escapeHtml(t("customize.oauth.code"))}</p>
      <p class="oauth-code">${escapeHtml(flow.userCode)}</p>
      <p class="dim">${escapeHtml(t("customize.oauth.waiting"))}</p>
      <div class="cust-actions">
        <button class="mini-btn" data-oauth-cancel="${id}">${escapeHtml(t("customize.oauth.cancel"))}</button>
      </div>`;
  }
  // With a CLI sign-in present, the button offers the alternative.
  const loginLabel = status?.localCli
    ? t("customize.oauth.loginAlt")
    : t("customize.oauth.login");
  const logoutBtn = status?.oauth
    ? `<button class="mini-btn" data-oauth-logout="${id}">${escapeHtml(t("customize.oauth.logout"))}</button>`
    : "";
  return `<div class="cust-actions">
      <button class="mini-btn" data-oauth-login="${id}">${escapeHtml(loginLabel)}</button>
      ${logoutBtn}
    </div>`;
}

/// The gear panel's OAuth section (codex/grok only, and only on the
/// family row — extra CLI account cards don't own Pane's OAuth login).
function renderOAuthBlock(id: string): string {
  if (!OAUTH_PROVIDERS.has(id) || providerFamily(id) !== id) return "";
  return `<div class="cust-oauth" data-oauth-block="${escapeHtml(id)}">${oauthBlockInner(id)}</div>`;
}

/// "Sign in with browser": start the device-code flow, open the
/// verification page (through the same open_link gate as quick links),
/// show the user code for verification, and poll until done.
async function startOauthLogin(family: string): Promise<void> {
  stopOauthFlow(family);
  let started: { device_auth_id: string; user_code: string; verify_url: string };
  try {
    started = await invoke("oauth_start", { provider: family });
  } catch (err) {
    oauthFlow.set(family, { deviceAuthId: "", userCode: "", error: String(err) });
    paintOAuth(family);
    return;
  }
  void invoke("open_link", { url: started.verify_url }).catch((err) => {
    const status = document.querySelector("#status");
    if (status) status.textContent = t("footer.openLinkFailed", { err: String(err) });
  });
  const flow: OAuthFlowState = {
    deviceAuthId: started.device_auth_id,
    userCode: started.user_code,
    error: null,
  };
  oauthFlow.set(family, flow);
  paintOAuth(family);
  flow.timer = window.setInterval(() => void pollOauth(family), 3000);
  void pollOauth(family);
}

/// One poll tick. The backend paces itself against the server-asked
/// interval, so a fixed 3s timer here is safe.
async function pollOauth(family: string): Promise<void> {
  const flow = oauthFlow.get(family);
  if (!flow || !flow.deviceAuthId) return;
  let r: { done: boolean; label: string | null; error: string | null };
  try {
    r = await invoke("oauth_poll", { provider: family, deviceAuthId: flow.deviceAuthId });
  } catch (err) {
    flow.error = String(err);
    flow.deviceAuthId = "";
    stopOauthFlow(family);
    paintOAuth(family);
    return;
  }
  if (!r.done && !r.error) return; // still waiting for the user
  stopOauthFlow(family);
  if (r.error) {
    flow.error = r.error;
    flow.deviceAuthId = "";
    paintOAuth(family);
    return;
  }
  oauthFlow.delete(family);
  credStatusCache.delete(family);
  refreshCredStatus(family); // chips pick up the OAuth account label
  paintOAuth(family);
  void forceUsageRefreshAttempt(false).then(requestTraySync);
}

async function doOauthLogout(family: string): Promise<void> {
  try {
    await invoke("oauth_logout", { provider: family });
  } catch (err) {
    const status = document.querySelector("#status");
    if (status) status.textContent = t("customize.oauth.failed", { err: String(err) });
  }
  credStatusCache.delete(family);
  refreshCredStatus(family);
  paintOAuth(family);
  void forceUsageRefreshAttempt(false).then(requestTraySync);
}

/// The gear panel's status section: the one-line fact of what this
/// provider reads on its own, the live detection chips, and the
/// credentials saved in Pane.
function renderCustStatus(id: string): string {
  const info = PROVIDER_CRED_INFO[providerFamily(id)];
  const auto = info ? escapeHtml(t(info.auto)) : "";
  return `<div class="cust-status">
      <p class="cust-status-fact"><span class="cust-info-label">${escapeHtml(t("customize.credAutoLabel"))}</span>${auto}</p>
      <p class="cust-chips" data-cred-chips="${escapeHtml(id)}">${credChipsHtml(id)}</p>
      <ul class="cust-accounts" data-cred-accounts="${escapeHtml(id)}">${credAccountsHtml(id)}</ul>
    </div>`;
}

/// Inline config panel behind a provider's ⚙ button: a status section
/// (what the provider reads on its own + live credential chips + the
/// credentials Pane has saved) above an action section. Key-based
/// providers get an API-key field (Custom Balance also its base URL), a
/// "Test" button that validates the pasted key without saving it, and
/// Save — disabled until a test passes (an empty field stays savable:
/// that path clears the stored key). The rest sign in through their own
/// CLI or desktop login, so their action section is that provider's
/// login hint. The "?" button on the row stays: it shows the static
/// facts, this panel the live detection.
function renderCustConfig(id: string): string {
  const status = renderCustStatus(id);
  if (!KEY_PROVIDERS.has(id)) {
    // An account card (deepseek@1) manages nothing itself — its account
    // lives on the family row's account section.
    const fam = providerFamily(id);
    if (id !== fam && MULTI_ACCOUNT_PROVIDERS.has(fam)) {
      const famName = ALL_PROVIDERS.find(([pid]) => pid === fam)?.[1] ?? fam;
      return `<div class="cust-config"><p class="settings-note">${escapeHtml(t("customize.acctCardHint", { name: famName }))}</p></div>`;
    }
    const hintKey = `customize.loginHint.${providerFamily(id)}`;
    const hint = t(hintKey) !== hintKey ? t(hintKey) : t("customize.cliLoginHint");
    return `<div class="cust-config">${status}${renderOAuthBlock(id)}<p class="settings-note">${escapeHtml(hint)}</p></div>`;
  }
  // Reuse the Settings key placeholders (settings.keyPhXxx) where defined.
  const phKey = `settings.keyPh${id[0].toUpperCase()}${id.slice(1)}`;
  const ph = t(phKey) !== phKey ? t(phKey) : t("settings.keyPlaceholder");
  const baseUrl =
    id === "relaybalance"
      ? `<input type="text" data-cust-baseurl="${id}" placeholder="https://api.example.com" spellcheck="false" />`
      : "";
  return `<div class="cust-config">
      ${status}
      <div class="cust-actions">
        ${baseUrl}
        <input type="password" data-cust-key="${id}" placeholder="${escapeHtml(ph)}" />
        <button class="mini-btn" data-cust-test="${id}">${escapeHtml(t("customize.test"))}</button>
        <button class="mini-btn" data-cust-save="${id}" title="${escapeHtml(t("customize.saveAfterTest"))}">${escapeHtml(t("settings.save"))}</button>
        <span class="cust-test-result" data-cust-result="${id}"></span>
      </div>
      ${renderAccountsBlock(id)}
    </div>`;
}

function renderCustomize(): string {
  // A-Z by English display name, locale-independent. Card order is owned by
  // dragging cards on the main view; the drawer is for enabling,
  // configuring and per-row management, so a stable sorted list reads best.
  const nameOf = (id: string): string =>
    ALL_PROVIDERS.find(([pid]) => pid === id)?.[1] ??
    lastSnapshots.find((s) => s.id === id)?.name ??
    id;
  const query = custSearchQuery.trim().toLowerCase();
  const ids = [...(config.layout?.providerOrder ?? ALL_PROVIDERS.map(([id]) => id))]
    .filter((id) => {
      const snapshot = lastSnapshots.find((s) => s.id === id);
      // A retired account card (its login left this machine) keeps its
      // layout for reattachment but must not haunt Customize as a bare
      // "claude@ab12cd34" block with nothing under it. A card the USER
      // disabled also has no snapshot (disabled providers are never
      // fetched) — that one must keep rendering, or its re-enable toggle
      // vanishes with it and the account is stuck off forever.
      // One/New API keys with the family off have no snapshot either;
      // configured ones still render (name from sites) so per-key toggles
      // survive. Deleted keys with no snapshot and not in disabled skip.
      if (id.includes("@") && !snapshot && !config.disabled.includes(id) && !onaFindConfiguredKey(id)) {
        return false;
      }
      // Deleted One/New API keys must not linger as `onenewapi@…` ghosts,
      // even when they are still in `disabled` (Claude-style re-enable
      // does not apply — the site is gone).
      if (onaSitesLoaded && isOnaKeyCardId(id) && !onaFindConfiguredKey(id)) {
        return false;
      }
      // Family master lives in Settings once any key exists. Keep the
      // empty family row only so Customize can still discover the family
      // before the first key.
      if (id === ONA_FAMILY && onaTotalKeys() > 0) {
        return false;
      }
      return !(id.includes("@") && !snapshot && !config.disabled.includes(id));
    })
    .sort((a, b) => nameOf(a).localeCompare(nameOf(b), "en"));
  // A-Z index strip: only the letters that actually have a provider.
  const letters = [
    ...new Set(
      ids.map((id) => {
        const n = nameOf(id);
        return /^[a-z]/i.test(n) ? n[0].toUpperCase() : "#";
      }),
    ),
  ];
  const blocks = ids
    .map((id) => {
      const snapshot = lastSnapshots.find((s) => s.id === id);
      // The leftover Moonshot *card* folds into Kimi Code on the dashboard.
      // This toggle (labeled "Kimi API") still owns the wallet: off means
      // no Moonshot HTTP and no API bar on the Kimi card. Hide it and
      // there's no way to stop those calls.
      // Dynamic account cards carry their name in the snapshot
      // ("Claude — Org"); static providers come from the fixed list.
      const name =
        ALL_PROVIDERS.find(([pid]) => pid === id)?.[1] ?? snapshot?.name ?? onaCardName(id) ?? id;
      const L = providerLayout(id);
      // Per-row checkbox is exact-id only: family `onenewapi` stays its
      // own toggle, and key cards keep independent enable state while
      // the family is off.
      const enabled = !config.disabled.includes(id);

      const row = (key: string) => {
        const starrable = isStarrable(snapshot, key);
        const starred = L.starred.includes(key);
        const visible = !L.hidden.includes(key);
        return `
          <div class="cust-row" draggable="true" data-cust-row="${id}|${escapeHtml(key)}">
            <span class="grip" title="${escapeHtml(t("customize.dragRows"))}">⠿</span>
            <label class="toggle mini"><input type="checkbox" data-visible="${id}|${escapeHtml(key)}"${visible ? " checked" : ""} /></label>
            <span class="cust-label">${escapeHtml(displayMetricLabel(key))}</span>
            ${starrable ? `<button class="star${starred ? " on" : ""}" data-star="${id}|${escapeHtml(key)}" title="${escapeHtml(t("customize.star"))}">★</button>` : ""}
          </div>`;
      };

      const always = L.metricOrder.filter((k) => !L.onDemand.includes(k));
      const onDemand = L.metricOrder.filter((k) => L.onDemand.includes(k));
      const rows = L.metricOrder.length
        ? `${always.map(row).join("")}
           <div class="cust-divider" data-divider="${id}">${escapeHtml(t("customize.onDemand"))}</div>
           ${onDemand.map(row).join("")}`
        : `<p class="placeholder">${escapeHtml(t("customize.noData"))}</p>`;

      const open = custExpanded.has(id);
      const letter = /^[a-z]/i.test(name) ? name[0].toUpperCase() : "#";
      const filteredOut = query !== "" && !name.toLowerCase().includes(query);
      return `
        <article class="provider customize-block${enabled ? "" : " muted"}${open ? " open" : ""}" data-cust-provider="${id}" data-letter="${letter}" data-name="${escapeHtml(name.toLowerCase())}"${filteredOut ? " hidden" : ""}>
          <div class="provider-head">
            <button class="cust-expand" data-cust-expand="${id}" title="${open ? t("customize.collapse") : t("customize.expand")}">
              <span class="provider-name">${escapeHtml(name)}</span>
              <span class="chev">⌄</span>
            </button>
            <span class="spacer"></span>
            <button class="mini-btn cust-info-btn${custInfoOpen === id ? " on" : ""}" data-info="${id}" title="${escapeHtml(t("customize.credInfo"))}">?</button>
            <button class="mini-btn cust-config-btn${custConfigOpen === id ? " on" : ""}" data-config="${id}" title="${escapeHtml(t("customize.configure"))}">⚙</button>
            <button class="mini-btn" data-reset="${id}" title="${escapeHtml(t("customize.resetLayoutTip"))}">${escapeHtml(t("customize.resetLayout"))}</button>
            <label class="toggle mini" title="${escapeHtml(t("customize.enable"))}"><input type="checkbox" data-enable="${id}"${enabled ? " checked" : ""} /></label>
          </div>
          ${custConfigOpen === id ? renderCustConfig(id) : ""}
          ${custInfoOpen === id ? renderCustInfo(id) : ""}
          <div class="acc-body"><div class="acc-inner cust-rows">${rows}</div></div>
        </article>`;
    })
    .join("");

  const starCount = Object.values(config.layout?.providers ?? {}).reduce((n, l) => n + l.starred.length, 0);
  return `
    <div class="customize-bar glass-bar">
      <button class="dock-btn" data-customize-close>${escapeHtml(t("customize.done"))}</button>
      <span class="detail">${escapeHtml(t("customize.starred", { n: starCount }))}</span>
      <button class="dock-btn danger" data-reset-all title="${escapeHtml(t("customize.resetAllTip"))}">${escapeHtml(t("customize.resetAll"))}</button>
    </div>
    <div class="cust-search-wrap">
      <input id="cust-search" type="text" value="${escapeHtml(custSearchQuery)}" placeholder="${escapeHtml(t("customize.search"))}" spellcheck="false" />
    </div>
    <nav class="cust-az${query ? " hidden" : ""}">${letters
      .map((l) => `<button data-az="${l}">${l}</button>`)
      .join("")}</nav>
    ${blocks}`;
}

// ---------------------------------------------------------------------------
// Render root
// ---------------------------------------------------------------------------

function renderWelcome(): string {
  if (config.welcomeDismissed || !lastSnapshots.length) return "";
  return `
    <article class="provider welcome-card">
      <div class="provider-head">
        <span class="provider-name">${escapeHtml(t("welcome.title"))}</span>
        <span class="spacer"></span>
        <button class="share-btn welcome-close" data-welcome-close title="${escapeHtml(t("welcome.dismiss"))}">✕</button>
      </div>
      <p class="placeholder" style="margin:2px 0 8px">
        ${escapeHtml(t("welcome.body"))}
      </p>
      <button class="mini-btn" data-welcome-customize>${escapeHtml(t("welcome.open"))}</button>
    </article>`;
}

function renderAll(): void {
  const el = document.querySelector("#providers")!;
  el.innerHTML =
    renderWelcome() + renderTotalSpend() + orderedSnapshots().map(renderCard).join("");
  if (customizeOpen) renderDrawerBody();
  rebuildTrail();
}

function renderDrawerBody(): void {
  const body = document.querySelector<HTMLElement>("#drawer-body");
  if (!body) return;
  // A background refresh re-renders the drawer even mid-typing; hand the
  // search box its focus and caret back afterwards.
  const search = document.activeElement as HTMLInputElement | null;
  const caret = search?.id === "cust-search" ? search.selectionStart : null;
  body.innerHTML = renderCustomize();
  if (caret !== null) {
    const input = body.querySelector<HTMLInputElement>("#cust-search");
    input?.focus();
    input?.setSelectionRange(caret, caret);
  }
}

/// Live search filter: hides non-matching provider blocks in place (no
/// re-render, so the box keeps focus) and parks the A-Z strip meanwhile.
function applyCustomizeFilter(query: string): void {
  custSearchQuery = query;
  const q = query.trim().toLowerCase();
  document.querySelectorAll<HTMLElement>("#drawer-body .customize-block").forEach((el) => {
    el.hidden = q !== "" && !(el.dataset.name ?? "").includes(q);
  });
  document.querySelector<HTMLElement>("#drawer-body .cust-az")?.classList.toggle("hidden", q !== "");
}

/// Customize lives in a drawer that slides in from the left edge.
function setDrawer(open: boolean): void {
  customizeOpen = open;
  if (open) {
    renderDrawerBody();
    // Local JSON list — cheap, and required if Customize opens before Settings.
    void loadOneNewApiSites();
  }
  document.body.classList.toggle("drawer-open", open);
  document.querySelector("#customize-btn")?.classList.toggle("active", open);
}

// ---------------------------------------------------------------------------
// Navigation trail: a slim rail of ticks — one per card — that shows where
// you are in the scroll and jumps to a card on click.
// ---------------------------------------------------------------------------

function trailCards(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>("#providers > article"));
}

function rebuildTrail(): void {
  const trail = document.querySelector<HTMLElement>("#trail")!;
  const cards = trailCards();
  if (cards.length < 2) {
    trail.innerHTML = "";
    trail.hidden = true;
    return;
  }
  trail.hidden = false;
  trail.innerHTML = cards
    .map((card, i) => {
      const name = card.querySelector(".provider-name")?.textContent ?? `Card ${i + 1}`;
      const family = card.dataset.provider ? providerFamily(card.dataset.provider) : "";
      const icon = PROVIDER_ICONS[family];
      if (icon) {
        const extra = [
          TRAIL_RECOLOR_ICONS.has(family) ? " trail-recolor" : "",
          TRAIL_INVERT_DARK_ICONS.has(family) ? " trail-invert-dark" : "",
        ].join("");
        return `<button class="trail-tick trail-icon${extra}" data-trail="${i}" title="${escapeHtml(name)}">${icon}</button>`;
      }
      return `<button class="trail-tick" data-trail="${i}" title="${escapeHtml(name)}"></button>`;
    })
    .join("");
  // Minimap feel: tick width follows the card's height, like Codex's rail.
  // Icon ticks keep a fixed square box instead — the mark itself is the
  // height signal.
  const ticks = trail.querySelectorAll<HTMLElement>(".trail-tick");
  ticks.forEach((tick, i) => {
    if (tick.classList.contains("trail-icon")) return;
    const h = cards[i]?.offsetHeight ?? 80;
    tick.style.width = `${Math.max(7, Math.min(16, Math.round(5 + h / 45)))}px`;
  });
  updateTrailActive();
}

/// Codex-style magnetic rail: ticks near the cursor stretch and brighten
/// with a smooth falloff; everything settles back when the mouse leaves.
/// Icon ticks scale uniformly (the mark grows) instead of stretching and
/// skip the background wash, which would paint over the artwork.
function setupTrailFisheye(): void {
  const sidebar = document.querySelector<HTMLElement>(".sidebar")!;
  let raf = 0;

  const reset = () => {
    cancelAnimationFrame(raf);
    document.querySelectorAll<HTMLElement>("#trail .trail-tick").forEach((t) => {
      t.style.transform = "";
      t.style.background = "";
    });
  };

  sidebar.addEventListener("mousemove", (e) => {
    const y = e.clientY;
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(() => {
      document.querySelectorAll<HTMLElement>("#trail .trail-tick").forEach((tick) => {
        const r = tick.getBoundingClientRect();
        const d = Math.abs(y - (r.top + r.height / 2));
        const g = Math.exp(-(d * d) / (2 * 26 * 26)); // gaussian falloff, σ≈26px
        const active = tick.classList.contains("active");
        if (tick.classList.contains("trail-icon")) {
          tick.style.transform = `scale(${(1 + 0.5 * g).toFixed(3)})`;
          return;
        }
        tick.style.transform = `scaleX(${(1 + 0.9 * g).toFixed(3)})`;
        const mix = Math.round(Math.max(g * 85, active ? 100 : 12));
        tick.style.background = `color-mix(in srgb, var(--foreground) ${mix}%, var(--border))`;
      });
    });
  });
  sidebar.addEventListener("mouseleave", reset);
}

function updateTrailActive(): void {
  const providersEl = document.querySelector<HTMLElement>("#providers")!;
  const cards = trailCards();
  if (!cards.length) return;
  const anchor = providersEl.scrollTop + 70;
  let active = 0;
  for (let i = 0; i < cards.length; i++) {
    if (cards[i].offsetTop <= anchor) active = i;
  }
  // Bottom of the list: light up the last tick even if a tall card above
  // still owns the anchor line.
  if (providersEl.scrollTop + providersEl.clientHeight >= providersEl.scrollHeight - 4) {
    active = cards.length - 1;
  }
  document.querySelectorAll<HTMLElement>("#trail .trail-tick").forEach((tick, i) => {
    tick.classList.toggle("active", i === active);
  });
}

// ---------------------------------------------------------------------------
// Spend row model tooltip
// ---------------------------------------------------------------------------

/// Tooltip for one Usage Trend bar: date, tokens used, share of 30 days.
function showTrendTip(el: HTMLElement): void {
  const tip = document.querySelector<HTMLElement>("#model-tip")!;
  const [id, idxStr] = (el.dataset.trend ?? "").split("|");
  const spend = lastSpend.find((s) => s.id === id);
  const i = Number(idxStr);
  if (!spend || Number.isNaN(i)) return;

  const tokens = spend.trend[i] ?? 0;
  const total = spend.trend.reduce((a, b) => a + b, 0);
  const share = total > 0 ? (tokens / total) * 100 : 0;
  const date = new Date(Date.now() - (29 - i) * 86_400_000).toLocaleDateString(localeTag(), {
    weekday: "short",
    month: "short",
    day: "numeric",
  });
  tip.innerHTML = `
    <div class="tip-line"><span class="tip-name">${escapeHtml(date)}</span><span>${
      tokens > 0 ? escapeHtml(t("card.tokens", { n: fmtTokens(tokens) })) : escapeHtml(t("spend.noUsage"))
    }</span></div>
    ${tokens > 0 ? `<div class="tip-line detail"><span>${escapeHtml(t("spend.of30", { n: share < 1 ? "<1" : share.toFixed(0) }))}</span></div>` : ""}`;

  const rect = el.getBoundingClientRect();
  tip.hidden = false;
  const top = Math.min(rect.bottom + 6, window.innerHeight - tip.offsetHeight - 8);
  tip.style.top = `${Math.max(4, top)}px`;
  tip.style.left = `${Math.max(8, Math.min(rect.left - 50, window.innerWidth - tip.offsetWidth - 8))}px`;
}

function showModelTip(row: HTMLElement): void {
  const tip = document.querySelector<HTMLElement>("#model-tip")!;
  const [id, key] = (row.dataset.spend ?? "").split("|");
  const spend = lastSpend.find((s) => s.id === id);
  const w = spend?.[key as SpendTab];
  if (!w) return;

  if (!w.models.length) {
    tip.innerHTML = `<p class="placeholder">${escapeHtml(t("spend.noModelData"))}</p>`;
  } else {
    tip.innerHTML = w.models
      .map((m) => {
        const share = w.cost > 0 ? (m.cost / w.cost) * 100 : 0;
        return `
          <div class="tip-model">
            <div class="tip-line"><span class="tip-name">${escapeHtml(m.model)}</span><span>${fmtMoney(m.cost)}</span></div>
            <div class="tip-line detail"><span>${share.toFixed(0)}%</span><span>${escapeHtml(t("card.tokens", { n: fmtTokens(m.tokens) }))}</span></div>
            <div class="tip-bar"><div style="width:${Math.max(2, share)}%"></div></div>
          </div>`;
      })
      .join("");
  }

  const rect = row.getBoundingClientRect();
  tip.hidden = false;
  const top = Math.min(rect.bottom + 4, window.innerHeight - tip.offsetHeight - 8);
  tip.style.top = `${Math.max(4, top)}px`;
  tip.style.left = `${Math.max(8, Math.min(rect.left + 20, window.innerWidth - tip.offsetWidth - 8))}px`;
}

// ---------------------------------------------------------------------------
// Refresh + tray strip
// ---------------------------------------------------------------------------

/// Background refreshes must not pay DOM costs nobody can see: while the
/// popover is hidden (99% of the time), rendering is deferred to the next
/// open instead of rebuilding a filter-heavy DOM every refresh interval.
let pendingRender = false;

function renderIfVisible(): void {
  if (document.hidden) {
    pendingRender = true;
    return;
  }
  pendingRender = false;
  renderAll();
  populatePinnedOptions();
}

function hideFoldedMoonshot(snapshots: Snapshot[]): Snapshot[] {
  const kimi = snapshots.find((s) => s.id === "kimi" && s.status === "ok");
  if (!kimi) return snapshots;
  const wallet = (s: Snapshot) =>
    s.metrics.some((m) => ["API", "Credits used", "Balance", "Vouchers", "Cash"].includes(m.label));
  const moon = snapshots.find((s) => s.id === "moonshot");
  if (wallet(kimi) || !moon || moon.metrics.length === 0) {
    return snapshots.filter((s) => s.id !== "moonshot");
  }
  return snapshots;
}

/// First paint from the previous run's snapshots (disk cache): numbers on
/// screen in milliseconds instead of a blank "Refreshing…" while the
/// slowest provider answers — at boot that wait ran 30-40 seconds. Cards
/// arrive marked stale ("Outdated") and the live fetch replaces them.
async function paintCachedSnapshots(): Promise<void> {
  // Only when a saved layout exists: on a true first run there is no cache
  // anyway, and refresh()'s first-launch detection must see the live list.
  if (config.layout === null) return;
  try {
    const cached = hideFoldedMoonshot(await invoke<Snapshot[]>("cached_usage"));
    // The live fetch may have already landed — never paint over it.
    if (!cached.length || lastSnapshots.length) return;
    lastSnapshots = cached;
    ensureLayout();
    renderIfVisible();
    requestTraySync();
  } catch {
    // No cache readable — the live fetch paints, as before.
  }
}

function setRefreshLock(on: boolean): void {
  refreshing = on;
  document.body.classList.toggle("refreshing", on);
  document.querySelector("#refresh")?.setAttribute("aria-busy", on ? "true" : "false");
}

function completeRefreshAttempt(generation: number): void {
  completedRefreshGeneration = Math.max(completedRefreshGeneration, generation);
  for (let index = refreshAttemptWaiters.length - 1; index >= 0; index -= 1) {
    if (refreshAttemptWaiters[index].generation <= completedRefreshGeneration) {
      refreshAttemptWaiters.splice(index, 1)[0].resolve();
    }
  }
}

async function forceUsageRefreshAttempt(usageOnly = true): Promise<void> {
  const generation = refreshGeneration + 1;
  const completed = new Promise<void>((resolve) => {
    refreshAttemptWaiters.push({ generation, resolve });
  });
  void refresh(true, usageOnly);
  await completed;
}

async function refresh(force = false, usageOnly = false): Promise<void> {
  if (refreshing) {
    // Remember a forced request instead of dropping it: the in-flight
    // fetch may have started before whatever prompted this one (a saved
    // key, a toggle), so one more pass runs when it finishes.
    if (force) {
      refreshQueuedUsageOnly = refreshQueued ? refreshQueuedUsageOnly && usageOnly : usageOnly;
      refreshQueued = true;
      // The save message (or a stale "Updated") would otherwise sit on
      // the footer until the in-flight fetch ends, and Refresh looks dead.
      document.querySelector("#status")!.textContent = t("footer.refreshing");
    }
    return;
  }
  if (!force && Date.now() - lastFetch < STALE_MS) return;
  setRefreshLock(true);
  const myGen = ++refreshGeneration;
  const status = document.querySelector("#status")!;
  status.textContent = t("footer.refreshing");
  // The spend scan re-reads every session log on a cold start and can take
  // tens of seconds — it must never hold up the usage cards' first paint,
  // or the Refresh button (the lock used to stay on until spend finished).
  const spendPromise = usageOnly
    ? Promise.resolve<ProviderSpend[] | null>(null)
    : invoke<ProviderSpend[]>("fetch_spend").catch(() => null);
  try {
    await unparkRecentlyKeyed();
    let snapshots = await invoke<Snapshot[]>("fetch_usage", { disabled: [...config.disabled] });
    // First launch ever (no layout yet): start with only the providers that
    // actually have credentials on this PC, like the Mac app's first-run
    // detection. The rest stay available in Customize.
    if (config.layout === null && snapshots.length > 0) {
      // Claude and Codex always start enabled — their "connect me" cards are
      // the new-user onboarding. Everything else without credentials waits
      // in Customize (a fresh PC with zero AI tools sees just those two).
      const starters = new Set(["claude", "codex"]);
      const noCreds = snapshots
        .filter(
          (s) =>
            s.status === "no_credentials" &&
            !starters.has(s.id) &&
            !recentlyKeyed.has(s.id)
        )
        .map((s) => s.id);
      if (noCreds.length) {
        snapshots = snapshots.filter((s) => !noCreds.includes(s.id));
        await patchConfig({ disabled: noCreds }).catch(() => {});
        await unparkRecentlyKeyed();
      }
    } else if (config.layout) {
      // App updates ship new providers; ones this PC has no credentials for
      // start disabled instead of piling up dead cards. Seen once (a layout
      // entry marks that), so enabling one in Customize sticks.
      const known = config.layout.providers;
      const fresh = snapshots
        .filter(
          (s) =>
            s.status === "no_credentials" &&
            !(s.id in known) &&
            !config.disabled.includes(s.id) &&
            !recentlyKeyed.has(s.id)
        )
        .map((s) => s.id);
      if (fresh.length) {
        for (const id of fresh) known[id] = providerLayout(id);
        await patchConfig({
          disabled: [...config.disabled, ...fresh],
          layout: config.layout,
        }).catch(() => {});
        await unparkRecentlyKeyed();
      }

      // Updates also RETIRE providers; saved layouts keep referencing their
      // ids, which rendered ghost rows in Customize. Prune anything the app
      // no longer knows.
      const valid = new Set(ALL_PROVIDERS.map(([id]) => id));
      // Account-scoped ids (claude@<hash>) are valid whenever their family
      // is — pruning them here would wipe a multi-account user's layout and
      // disabled choices on every launch.
      const isValid = (id: string) => valid.has(id) || valid.has(providerFamily(id));
      const prunedOrder = config.layout.providerOrder.filter(isValid);
      const staleLayout = Object.keys(config.layout.providers).filter((id) => !isValid(id));
      const prunedDisabled = config.disabled.filter(isValid);
      if (
        prunedOrder.length !== config.layout.providerOrder.length ||
        staleLayout.length ||
        prunedDisabled.length !== config.disabled.length
      ) {
        config.layout.providerOrder = prunedOrder;
        for (const id of staleLayout) delete config.layout.providers[id];
        await patchConfig({ layout: config.layout, disabled: prunedDisabled }).catch(() => {});
      }
    }
    snapshots = hideFoldedMoonshot(snapshots);
    for (const s of snapshots) {
      if (s.status !== "no_credentials") recentlyKeyed.delete(s.id);
    }
    // Drop exemptions from saves that happened before this refresh started
    // (a failed save is removed in the catch; a cleared key is removed on
    // empty paste). One follow-up fetch is enough to pick the new key up.
    for (const [id, gen] of [...recentlyKeyed]) {
      if (gen < myGen) recentlyKeyed.delete(id);
    }
    const firstData = lastSnapshots.length === 0;
    lastFetch = Date.now();
    lastSnapshots = snapshots;
    ensureLayout();
    if (!lastLayoutSnapshot && config.layout) {
      lastLayoutSnapshot = JSON.stringify(config.layout);
    }
    renderIfVisible();
    if (firstData && !customizeOpen && !document.hidden) playReveal();
    requestTraySync();
    const time = new Date().toLocaleTimeString(localeTag(), { hour: "2-digit", minute: "2-digit" });
    status.textContent = configSaveError
      ? t("footer.configSaveFailed", { err: configSaveError })
      : t("footer.updated", { time });
  } catch (err) {
    status.textContent = configSaveError
      ? t("footer.configSaveFailed", { err: configSaveError })
      : t("footer.refreshFailed", { err: String(err) });
  } finally {
    setRefreshLock(false);
    completeRefreshAttempt(myGen);
    if (refreshQueued) {
      refreshQueued = false;
      const queuedUsageOnly = refreshQueuedUsageOnly;
      refreshQueuedUsageOnly = true;
      void refresh(true, queuedUsageOnly);
    }
  }
  const spend = await spendPromise;
  if (usageOnly) return;
  spendLoaded = true;
  // Overlapping scans are allowed now that Refresh unlocks before spend
  // finishes. Keep the newest successful result — a later failed scan
  // (null) must not discard dollars an older pass already computed.
  if (spend && myGen >= lastAppliedSpendGen) {
    lastSpend = spend;
    lastAppliedSpendGen = myGen;
  }
  if (lastSnapshots.length) ensureLayout();
  if (!customizeOpen && lastSnapshots.length) renderIfVisible();
}

function scheduleAutoRefresh(): void {
  if (refreshTimer !== undefined) window.clearInterval(refreshTimer);
  const minutes = Math.max(1, config.refreshMinutes || 5);
  refreshTimer = window.setInterval(() => void refresh(), minutes * 60 * 1000);
}

const logoPixels = new Map<string, number[]>();

async function rasterizeLogo(id: string): Promise<number[] | null> {
  const cached = logoPixels.get(id);
  if (cached) return cached;
  const svg = PROVIDER_ICONS[id];
  if (!svg) return null;

  const white = svg
    .replace(/fill="(?!none)[^"]*"/g, 'fill="#ffffff"')
    .replace(/stroke="(?!none)[^"]*"/g, 'stroke="#ffffff"');
  const url = URL.createObjectURL(new Blob([white], { type: "image/svg+xml" }));
  try {
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error("svg load failed"));
      img.src = url;
    });
    const canvas = document.createElement("canvas");
    canvas.width = 32;
    canvas.height = 32;
    const ctx = canvas.getContext("2d")!;
    const scale = 28 / Math.max(img.width || 28, img.height || 28);
    const w = (img.width || 28) * scale;
    const h = (img.height || 28) * scale;
    ctx.drawImage(img, (32 - w) / 2, (32 - h) / 2, w, h);
    const pixels = Array.from(ctx.getImageData(0, 0, 32, 32).data);
    logoPixels.set(id, pixels);
    return pixels;
  } catch {
    return null;
  } finally {
    URL.revokeObjectURL(url);
  }
}

interface TraySyncState {
  snapshots: Snapshot[];
  projection: TrayProjectionConfig;
}

let pendingTraySync: TraySyncState | null = null;
let traySyncRunning = false;
let traySyncFailureShown = false;
let traySyncFailureText = "";

function captureTraySyncState(): TraySyncState {
  const providerOrder = [...(config.layout?.providerOrder ?? [])];
  const providers: Record<string, TrayProjectionProvider> = {};
  for (const id of providerOrder) {
    const layout = providerLayout(id);
    providers[id] = {
      metricOrder: [...layout.metricOrder],
      hidden: [...layout.hidden],
      starred: [...layout.starred],
    };
  }
  return {
    snapshots: lastSnapshots
      .filter((snapshot) => !isCardDisabled(snapshot.id))
      .map((snapshot) => ({
        ...snapshot,
        metrics: snapshot.metrics.map((metric) => ({ ...metric })),
      })),
    projection: {
      disabled: [...new Set([...config.disabled, ...pendingProviderEnables.keys()])],
      providerOrder,
      providers,
      pinned: config.pinned ? { ...config.pinned } : null,
      locale: resolveLocale(config.locale),
    },
  };
}

async function buildTrayStripEntries(state: TraySyncState): Promise<TrayStripEntry[]> {
  const entries: TrayStripEntry[] = [];
  for (const id of state.projection.providerOrder) {
    if (entries.length >= 4) break;
    if (isCardDisabled(id, state.projection.disabled)) continue;
    const layout = state.projection.providers[id];
    if (!layout?.starred.length) continue;
    const snapshot = state.snapshots.find((candidate) => candidate.id === id && candidate.status === "ok");
    if (!snapshot) continue;
    const starredMetrics = layout.starred
      .filter((label) => !layout.hidden.includes(label))
      .map((label) =>
        snapshot.metrics.find((metric) => metric.label === label && metric.kind === "progress"),
      )
      .filter((metric): metric is Metric => Boolean(metric))
      .slice(0, 2);
    if (!starredMetrics.length) continue;
    const logo = await rasterizeLogo(providerFamily(id));
    if (!logo) continue;
    const values = starredMetrics.map(remainingPercent);
    const name = snapshot.stale ? `⚠ ${snapshot.name}` : snapshot.name;
    const tooltip = `${name}\n${starredMetrics
      .map((metric) =>
        t("tray.left", {
          label: displayMetricLabel(metric.label),
          n: remainingPercent(metric),
        }),
      )
      .join("\n")}`;
    entries.push({ id, logo, values, tooltip });
  }
  return entries;
}

function requestTraySync(): void {
  pendingTraySync = captureTraySyncState();
  if (!traySyncRunning) void drainTraySyncQueue();
}

async function drainTraySyncQueue(): Promise<void> {
  if (traySyncRunning) return;
  traySyncRunning = true;
  try {
    while (pendingTraySync) {
      const state = pendingTraySync;
      pendingTraySync = null;
      const entries = await buildTrayStripEntries(state);
      // Rasterizing a logo may yield while the user changes configuration.
      // Skip this stale generation before it reaches either native surface.
      if (pendingTraySync) continue;
      try {
        await invoke("sync_tray_surfaces", {
          snapshots: state.snapshots,
          projection: state.projection,
          entries,
        });
        if (traySyncFailureShown) {
          const status = document.querySelector("#status");
          if (status && status.textContent === traySyncFailureText) {
            status.textContent = configSaveError
              ? t("footer.configSaveFailed", { err: configSaveError })
              : "";
          }
        }
        traySyncFailureShown = false;
        traySyncFailureText = "";
      } catch (err) {
        if (!traySyncFailureShown) {
          const status = document.querySelector("#status");
          const message = t("footer.traySyncFailed", { err: String(err) });
          if (status) status.textContent = message;
          traySyncFailureShown = true;
          traySyncFailureText = message;
        }
      }
    }
  } finally {
    traySyncRunning = false;
    if (pendingTraySync) void drainTraySyncQueue();
  }
}

// ---------------------------------------------------------------------------
// Customize interactions
// ---------------------------------------------------------------------------

// Only metric rows drag inside the drawer now — provider order belongs to
// the card drag on the main view.
interface DragPayload {
  id: string;
  key: string;
}

let dragPayload: DragPayload | null = null;

/// Rebuilds order + On-Demand membership after a row drop. The sequence is
/// [always..., DIVIDER, onDemand...]; where the row lands relative to the
/// divider decides which side it lives on.
function moveRow(L: ProviderLayout, key: string, target: string): void {
  const always = L.metricOrder.filter((k) => !L.onDemand.includes(k));
  const onDemand = L.metricOrder.filter((k) => L.onDemand.includes(k));
  const seq = [...always, DIVIDER, ...onDemand].filter((k) => k !== key);
  const at = target === DIVIDER ? seq.indexOf(DIVIDER) + 1 : seq.indexOf(target);
  if (at < 0) return;
  seq.splice(at, 0, key);
  const dividerIdx = seq.indexOf(DIVIDER);
  L.metricOrder = seq.filter((k) => k !== DIVIDER);
  L.onDemand = seq.slice(dividerIdx + 1).filter((k) => k !== DIVIDER);
}

function handleCustomizeClick(target: HTMLElement): boolean {
  const expand = target.closest<HTMLElement>("[data-cust-expand]");
  if (expand) {
    const id = expand.dataset.custExpand!;
    if (custExpanded.has(id)) {
      custExpanded.delete(id);
    } else {
      custExpanded.add(id);
    }
    // Toggle in place so the accordion animates instead of re-rendering.
    expand.closest(".customize-block")?.classList.toggle("open", custExpanded.has(id));
    return true;
  }
  const cfgBtn = target.closest<HTMLElement>("[data-config]");
  if (cfgBtn) {
    const id = cfgBtn.dataset.config!;
    custConfigOpen = custConfigOpen === id ? null : id;
    custInfoOpen = null; // one panel at a time per row
    acctFormOpen = false; // a fresh panel starts with the form folded
    renderDrawerBody();
    if (custConfigOpen) {
      fetchCredStatus(id); // the status section's live chips
      fetchAccounts(id); // the account section's saved list
      const block = document.querySelector<HTMLElement>(
        `#drawer-body [data-cust-provider="${CSS.escape(custConfigOpen)}"]`,
      );
      block?.querySelector<HTMLInputElement>("[data-cust-key]")?.focus();
      // Custom Balance: pre-fill the relay base URL saved with its key.
      const baseInp = block?.querySelector<HTMLInputElement>("[data-cust-baseurl]");
      if (baseInp) {
        void invoke<string | null>("get_base_url", { provider: custConfigOpen })
          .then((v) => {
            baseInp.value = v ?? "";
          })
          .catch(() => {});
      }
    }
    return true;
  }
  const infoBtn = target.closest<HTMLElement>("[data-info]");
  if (infoBtn) {
    const id = infoBtn.dataset.info!;
    custInfoOpen = custInfoOpen === id ? null : id;
    custConfigOpen = null; // one panel at a time per row
    renderDrawerBody();
    if (custInfoOpen) fetchCredStatus(id);
    return true;
  }
  const custTest = target.closest<HTMLElement>("[data-cust-test]");
  if (custTest) {
    void runCustKeyTest(custTest.dataset.custTest!);
    return true;
  }
  const oauthLogin = target.closest<HTMLElement>("[data-oauth-login]");
  if (oauthLogin) {
    void startOauthLogin(oauthLogin.dataset.oauthLogin!);
    return true;
  }
  const oauthLogout = target.closest<HTMLElement>("[data-oauth-logout]");
  if (oauthLogout) {
    void doOauthLogout(oauthLogout.dataset.oauthLogout!);
    return true;
  }
  const oauthCancel = target.closest<HTMLElement>("[data-oauth-cancel]");
  if (oauthCancel) {
    const family = oauthCancel.dataset.oauthCancel!;
    stopOauthFlow(family);
    oauthFlow.delete(family);
    paintOAuth(family);
    return true;
  }
  const acctToggle = target.closest<HTMLElement>("[data-acct-toggle]");
  if (acctToggle) {
    const family = acctToggle.dataset.acctToggle!;
    acctFormOpen = !acctFormOpen;
    paintAccounts(family); // repaint the section, not the whole drawer
    if (acctFormOpen) {
      document
        .querySelector<HTMLElement>(
          `#drawer-body [data-accounts-block="${CSS.escape(family)}"] [data-acct-label]`,
        )
        ?.focus();
    }
    return true;
  }
  const acctTest = target.closest<HTMLElement>("[data-acct-test]");
  if (acctTest) {
    void runAccountKeyTest(acctTest.dataset.acctTest!);
    return true;
  }
  const acctAdd = target.closest<HTMLElement>("[data-acct-add]");
  if (acctAdd) {
    void doAccountAdd(acctAdd.dataset.acctAdd!);
    return true;
  }
  const acctDel = target.closest<HTMLElement>("[data-acct-del]");
  if (acctDel) {
    const [family, idxStr] = acctDel.dataset.acctDel!.split("|");
    const index = Number(idxStr);
    if (family && Number.isInteger(index)) void doAccountRemove(family, index);
    return true;
  }
  const custSave = target.closest<HTMLElement>("[data-cust-save]");
  if (custSave) {
    const id = custSave.dataset.custSave!;
    const panel = custSave.closest(".cust-config");
    const keyInp = panel?.querySelector<HTMLInputElement>("[data-cust-key]");
    if (keyInp) {
      void saveApiKey(id, {
        key: keyInp,
        baseUrl: panel?.querySelector<HTMLInputElement>("[data-cust-baseurl]") ?? null,
      });
    }
    return true;
  }
  const az = target.closest<HTMLElement>("[data-az]");
  if (az) {
    document
      .querySelector<HTMLElement>(`#drawer-body [data-letter="${az.dataset.az}"]:not([hidden])`)
      ?.scrollIntoView({ behavior: "smooth", block: "start" });
    return true;
  }
  const closeBtn = target.closest("[data-customize-close]");
  if (closeBtn) {
    setDrawer(false);
    return true;
  }
  const resetAll = target.closest("[data-reset-all]");
  if (resetAll) {
    void appConfirm({
      title: t("customize.resetTitle"),
      message: t("customize.resetBody"),
      confirmLabel: t("customize.resetConfirm"),
      danger: true,
    }).then((ok) => {
      if (!ok) return;
      // Clearing layout + disabled re-arms the first-launch detection path:
      // the next refresh probes every provider and re-disables only the
      // ones with no credentials on this PC.
      config.layout = null;
      config.disabled = [];
      void patchConfig({ layout: null, disabled: [] }).catch(() => {}).then(() => {
        setDrawer(false);
        void forceUsageRefreshAttempt(false).then(requestTraySync);
      });
    });
    return true;
  }
  const reset = target.closest<HTMLElement>("[data-reset]");
  if (reset && config.layout) {
    const id = reset.dataset.reset!;
    const snapshot = lastSnapshots.find((s) => s.id === id);
    const spend = lastSpend.find((sp) => sp.id === id);
    config.layout.providers[id] = defaultProviderLayout(snapshot, spend, false);
    saveLayout();
    renderAll();
    return true;
  }
  const star = target.closest<HTMLElement>("[data-star]");
  if (star) {
    const [id, key] = star.dataset.star!.split("|");
    const L = providerLayout(id);
    if (L.starred.includes(key)) {
      L.starred = L.starred.filter((k) => k !== key);
    } else if (L.starred.length >= 2) {
      document.querySelector("#status")!.textContent = t("footer.twoStars");
      return true;
    } else {
      L.starred.push(key);
    }
    saveLayout();
    renderAll();
    return true;
  }
  return false;
}

/// "Test connection" behind the ⚙ panel: validates the pasted key through
/// test_api_key (a live probe that never writes anything) and only a
/// passing test enables Save. The result line shows the metric count on
/// success or the backend's error verbatim on failure.
async function runCustKeyTest(id: string): Promise<void> {
  const panel = document.querySelector<HTMLElement>(
    `#drawer-body [data-cust-provider="${CSS.escape(id)}"] .cust-config`,
  );
  const keyInp = panel?.querySelector<HTMLInputElement>("[data-cust-key]");
  const result = panel?.querySelector<HTMLElement>("[data-cust-result]");
  const saveBtn = panel?.querySelector<HTMLButtonElement>("[data-cust-save]");
  if (!keyInp || !result) return;
  const show = (text: string, ok: boolean | null) => {
    result.textContent = text;
    result.classList.toggle("ok", ok === true);
    result.classList.toggle("err", ok === false);
  };
  const key = keyInp.value.trim();
  if (!key) {
    show(t("customize.testEmpty"), false);
    if (saveBtn) saveBtn.disabled = false; // empty = clear the stored key
    return;
  }
  show(t("customize.testing"), null);
  try {
    const baseUrl =
      panel?.querySelector<HTMLInputElement>("[data-cust-baseurl]")?.value.trim() ?? "";
    const r = await invoke<{ ok: boolean; metrics: number; message: string }>("test_api_key", {
      provider: id,
      key,
      baseUrl: baseUrl || null,
    });
    if (r.ok) {
      show(t("customize.testOk", { n: r.metrics }), true);
      if (saveBtn) saveBtn.disabled = false;
    } else {
      show(`${t("customize.testFailed")}: ${r.message}`, false);
    }
  } catch (err) {
    show(`${t("customize.testFailed")}: ${String(err)}`, false);
  }
}

/// Any edit to the key/base-URL inputs invalidates the previous test:
/// Save re-locks (an empty field stays unlocked — it clears the key).
function resetCustTestState(panel: HTMLElement | null): void {
  if (!panel) return;
  const keyInp = panel.querySelector<HTMLInputElement>("[data-cust-key]");
  const saveBtn = panel.querySelector<HTMLButtonElement>("[data-cust-save]");
  const result = panel.querySelector<HTMLElement>("[data-cust-result]");
  if (saveBtn) saveBtn.disabled = keyInp?.value.trim() ? true : false;
  if (result) {
    result.textContent = "";
    result.classList.remove("ok", "err");
  }
}

/// Any edit to the account form's inputs invalidates its passing test:
/// Add re-locks until the new values are tested again.
function resetAcctTestState(form: HTMLElement | null): void {
  if (!form) return;
  const addBtn = form.querySelector<HTMLButtonElement>("[data-acct-add]");
  const result = form.querySelector<HTMLElement>("[data-acct-result]");
  if (addBtn) addBtn.disabled = true;
  if (result) {
    result.textContent = "";
    result.classList.remove("ok", "err");
  }
}

// Rapid toggles used to race: each one snapshotted config.disabled before
// the previous save landed, so only the last toggle survived. Toggles are
// kept as a ledger of pending deltas merged onto whatever config.disabled
// currently is — so changes made by refresh() in the meantime (auto-disable
// of new providers, pruning) survive instead of being overwritten.
let disabledSaveQueue: Promise<unknown> = Promise.resolve();
const pendingToggles: Array<{ id: string; enable: boolean }> = [];

function withPendingToggles(base: string[]): string[] {
  const s = new Set(base);
  for (const t of pendingToggles) {
    if (t.enable) s.delete(t.id);
    else s.add(t.id);
  }
  // A just-saved key wins over a Customize disable still in the queue —
  // the save is "show me this provider".
  for (const id of recentlyKeyed.keys()) s.delete(id);
  return [...s];
}

function handleCustomizeChange(target: HTMLInputElement): void {
  if (target.dataset.enable !== undefined) {
    const id = target.dataset.enable;
    const enable = target.checked;
    const enableGeneration = enable ? markProviderEnablePending(id) : null;
    if (!enable) pendingProviderEnables.delete(id);
    pendingToggles.push({ id, enable });
    config.disabled = withPendingToggles(config.disabled); // optimistic
    renderAll(); // disabled cards vanish from the dashboard immediately
    if (id === ONA_FAMILY) syncOneNewApiFamilyToggle();
    if (!enable) requestTraySync();
    disabledSaveQueue = disabledSaveQueue.then(async () => {
      // Fresh base at save time: includes server truth plus anything
      // refresh() changed while earlier saves were in flight.
      const want = withPendingToggles(config.disabled);
      try {
        await patchConfig({ disabled: want });
      } catch {
        // keep going — the delta stays applied locally
      }
      pendingToggles.shift(); // this task's toggle is now persisted
      // Merge any newer still-pending toggles back on top of the saved state.
      config.disabled = withPendingToggles(config.disabled);
      if (id === ONA_FAMILY) syncOneNewApiFamilyToggle();
      // Only an unmatched enable generation still needs a usage attempt.
      if (
        enableGeneration !== null &&
        pendingProviderEnables.get(id) === enableGeneration
      ) {
        await forceUsageRefreshAttempt();
        finishProviderEnable(id, enableGeneration);
      }
    });
    return;
  }
  if (target.dataset.visible !== undefined) {
    const [id, key] = target.dataset.visible.split("|");
    const L = providerLayout(id);
    if (target.checked) L.hidden = L.hidden.filter((k) => k !== key);
    else if (!L.hidden.includes(key)) L.hidden.push(key);
    saveLayout();
  }
}

// Chromium's default drag snapshot on backdrop-filtered elements captures the
// glass layers behind the card too — a smeared ghost of the whole list. Hand
// it a small opaque pill instead and dim the real card while it's in flight.
let dragGhost: HTMLElement | null = null;

function setDragGhost(e: DragEvent, src: HTMLElement): void {
  const rect = src.getBoundingClientRect();
  const g = src.cloneNode(true) as HTMLElement;
  g.classList.add("drag-ghost");
  g.classList.remove("open"); // ghost of a provider card shows just its header bar
  g.style.width = `${rect.width}px`;
  document.body.appendChild(g);
  e.dataTransfer?.setDragImage(g, e.clientX - rect.left, e.clientY - rect.top);
  dragGhost = g;
  requestAnimationFrame(() => src.classList.add("drag-src"));
}

function setupCustomizeDnD(providersEl: HTMLElement): void {
  providersEl.addEventListener("dragstart", (e) => {
    const row = (e.target as HTMLElement).closest<HTMLElement>("[data-cust-row]");
    if (row) {
      const [id, key] = row.dataset.custRow!.split("|");
      dragPayload = { id, key };
      setDragGhost(e as DragEvent, row);
      e.stopPropagation();
    }
  });

  providersEl.addEventListener("dragend", () => {
    dragGhost?.remove();
    dragGhost = null;
    providersEl.querySelectorAll(".drag-src").forEach((el) => el.classList.remove("drag-src"));
  });

  providersEl.addEventListener("dragover", (e) => {
    if (dragPayload) e.preventDefault();
  });

  providersEl.addEventListener("drop", (e) => {
    if (!dragPayload) return;
    e.preventDefault();
    const target = e.target as HTMLElement;

    const L = providerLayout(dragPayload.id);
    const divider = target.closest<HTMLElement>("[data-divider]");
    const row = target.closest<HTMLElement>("[data-cust-row]");
    if (divider && divider.dataset.divider === dragPayload.id) {
      moveRow(L, dragPayload.key, DIVIDER);
    } else if (row) {
      const [tid, tkey] = row.dataset.custRow!.split("|");
      if (tid === dragPayload.id && tkey !== dragPayload.key) moveRow(L, dragPayload.key, tkey);
    }
    saveLayout();
    renderAll();
    dragPayload = null;
    // renderAll() replaces the dragged node, so dragend may never bubble
    // back up — clean the ghost here too.
    dragGhost?.remove();
    dragGhost = null;
  });
}

// ---------------------------------------------------------------------------
// Settings pane
// ---------------------------------------------------------------------------

interface OneNewApiKeyDto {
  id: string;
  label: string;
  has_api_key: boolean;
}

interface OneNewApiSiteDto {
  id: string;
  name: string;
  base_url: string;
  keys: OneNewApiKeyDto[];
}

interface OneNewApiCreatedKeyDto {
  site: OneNewApiSiteDto;
  key_id: string;
  first_key: boolean;
}

type OneNewApiCreateSiteResult =
  | { status: "created"; site: OneNewApiSiteDto }
  | { status: "duplicate"; site_id: string };

const ONA_FAMILY = "onenewapi";

let onaSites: OneNewApiSiteDto[] = [];
let onaSitesLoaded = false;
const onaExpanded = new Set<string>();
let onaEditingId: string | null = null;
let onaEditingKeyId: string | null = null;
let onaBusy = false;

function isOnaKeyCardId(id: string): boolean {
  return providerFamily(id) === ONA_FAMILY && id !== ONA_FAMILY;
}

function onaSnapshotId(keyId: string): string {
  return `${ONA_FAMILY}@${keyId}`;
}

function onaFindConfiguredKey(
  snapshotId: string,
): { site: OneNewApiSiteDto; key: OneNewApiKeyDto } | undefined {
  if (providerFamily(snapshotId) !== ONA_FAMILY) return undefined;
  const keyId = snapshotId.slice(ONA_FAMILY.length + 1);
  if (!keyId) return undefined;
  for (const site of onaSites) {
    const key = site.keys.find((k) => k.id === keyId);
    if (key) return { site, key };
  }
  return undefined;
}

function onaCardName(snapshotId: string): string | undefined {
  const found = onaFindConfiguredKey(snapshotId);
  return found ? `${found.site.name} · ${found.key.label}` : undefined;
}

function syncOneNewApiFamilyToggle(): void {
  const el = document.querySelector<HTMLInputElement>("#ona-family-enabled");
  if (el) el.checked = !config.disabled.includes(ONA_FAMILY);
}

function foldOnaKeysIntoLayout(layout: Layout | null = config.layout): boolean {
  if (!layout) return false;
  let changed = false;
  for (const site of onaSites) {
    for (const key of site.keys) {
      const id = onaSnapshotId(key.id);
      if (!layout.providerOrder.includes(id)) {
        layout.providerOrder.push(id);
        changed = true;
      }
    }
  }
  return changed;
}

function configuredOnaSnapshotIds(): Set<string> {
  const keep = new Set<string>();
  for (const site of onaSites) {
    for (const key of site.keys) keep.add(onaSnapshotId(key.id));
  }
  return keep;
}

/// Drop layout/disabled/pin/cache for keys that are no longer in onaSites.
/// Only call after a successful list — an empty failed load would wipe live cards.
function pruneGoneOnaKeys(): boolean {
  const keep = configuredOnaSnapshotIds();
  let changed = false;
  const beforeSnaps = lastSnapshots.length;
  lastSnapshots = lastSnapshots.filter((s) => !isOnaKeyCardId(s.id) || keep.has(s.id));
  if (lastSnapshots.length !== beforeSnaps) changed = true;

  const layout = config.layout;
  if (layout) {
    const nextOrder = layout.providerOrder.filter((id) => !isOnaKeyCardId(id) || keep.has(id));
    if (nextOrder.length !== layout.providerOrder.length) {
      layout.providerOrder = nextOrder;
      changed = true;
    }
    for (const id of Object.keys(layout.providers)) {
      if (isOnaKeyCardId(id) && !keep.has(id)) {
        delete layout.providers[id];
        changed = true;
      }
    }
  }

  const nextDisabled = config.disabled.filter((id) => !isOnaKeyCardId(id) || keep.has(id));
  if (nextDisabled.length !== config.disabled.length) {
    config.disabled = nextDisabled;
    changed = true;
  }

  if (config.pinned && isOnaKeyCardId(config.pinned.provider) && !keep.has(config.pinned.provider)) {
    config.pinned = null;
    changed = true;
  }
  return changed;
}

function onaTotalKeys(sites: OneNewApiSiteDto[] = onaSites): number {
  return sites.reduce((n, site) => n + site.keys.length, 0);
}

function applyOneNewApiSite(site: OneNewApiSiteDto): void {
  const i = onaSites.findIndex((s) => s.id === site.id);
  if (i >= 0) onaSites[i] = site;
  else onaSites.push(site);
}

function paintOneNewApiCardNames(site: OneNewApiSiteDto): void {
  for (const key of site.keys) {
    const id = onaSnapshotId(key.id);
    const name = `${site.name} · ${key.label}`;
    const snap = lastSnapshots.find((s) => s.id === id);
    if (snap) snap.name = name;
  }
  renderIfVisible();
  requestTraySync();
}

/// Match Pane's origin canonicalization enough to skip a fake migrate
/// confirm when the user only added `/` or `/v1`.
function oneNewApiOriginKey(raw: string): string | null {
  try {
    const url = new URL(raw.trim());
    if (
      !["http:", "https:"].includes(url.protocol) ||
      url.username ||
      url.password ||
      url.search ||
      url.hash ||
      !["/", "/v1", "/v1/"].includes(url.pathname)
    ) {
      return null;
    }
    return url.origin.toLowerCase();
  } catch {
    return null;
  }
}

function setOneNewApiStatus(key: string, vars?: Record<string, string | number>): void {
  const el = document.querySelector("#status");
  if (el) el.textContent = t(key, vars);
}

function isOnaFingerprintMismatch(err: unknown): boolean {
  const raw = String(err);
  return raw.includes("status fingerprint mismatch") || /status endpoint:\s*HTTP 404\b/i.test(raw);
}

function setOneNewApiCaughtError(err: unknown, probe = false): void {
  if (isOnaFingerprintMismatch(err)) {
    setOneNewApiStatus("footer.onenewapiNotCompatible");
    return;
  }
  setOneNewApiStatus(probe ? "footer.onenewapiProbeFailed" : "footer.onenewapiFailed", {
    err: String(err),
  });
}

function renderOneNewApiKey(site: OneNewApiSiteDto, key: OneNewApiKeyDto): string {
  if (onaEditingKeyId === key.id) {
    return `<li class="ona-key">
      <form class="ona-key-edit" data-ona-edit-key-form="${escapeHtml(site.id)}" data-ona-key="${escapeHtml(key.id)}" autocomplete="off">
        <input type="text" spellcheck="false" data-ona-key-label value="${escapeHtml(key.label)}" placeholder="${escapeHtml(t("settings.onenewapiKeyLabelPh"))}" />
        <input type="password" data-ona-key-secret value="" placeholder="${escapeHtml(t("settings.onenewapiKeySecretPh"))}" autocomplete="new-password" />
        <p class="settings-note ona-key-hint">${escapeHtml(t("settings.onenewapiKeyKeepHint"))}</p>
        <div class="ona-edit-actions">
          <button type="submit">${escapeHtml(t("settings.onenewapiSaveKey"))}</button>
          <button type="button" data-ona-cancel-key>${escapeHtml(t("dialog.cancel"))}</button>
        </div>
      </form>
    </li>`;
  }
  const present = key.has_api_key
    ? `<span class="ona-key-has" title="${escapeHtml(t("footer.onenewapiKeySaved"))}">✓</span>`
    : "";
  return `<li class="ona-key">
    <div class="ona-key-row">
      <span class="ona-key-label">${escapeHtml(key.label)}</span>
      ${present}
      <button type="button" class="mini-btn" data-ona-edit-key="${escapeHtml(key.id)}">${escapeHtml(t("settings.onenewapiEdit"))}</button>
      <button type="button" class="mini-btn danger" data-ona-delete-key="${escapeHtml(key.id)}">${escapeHtml(t("settings.onenewapiDeleteKey"))}</button>
    </div>
  </li>`;
}

function renderOneNewApiSite(site: OneNewApiSiteDto): string {
  const open = onaExpanded.has(site.id) || site.keys.length === 0;
  const editing = onaEditingId === site.id;
  const keysHtml = site.keys.length
    ? `<ul class="ona-keys">${site.keys.map((k) => renderOneNewApiKey(site, k)).join("")}</ul>`
    : `<p class="ona-keys-empty">${escapeHtml(t("settings.onenewapiNoKeys"))}</p>`;
  const head = editing
    ? `<form class="ona-edit" data-ona-edit-form="${escapeHtml(site.id)}">
        <input type="text" spellcheck="false" data-ona-edit-name value="${escapeHtml(site.name)}" placeholder="${escapeHtml(t("settings.onenewapiNamePh"))}" />
        <input type="text" spellcheck="false" data-ona-edit-url value="${escapeHtml(site.base_url)}" placeholder="${escapeHtml(t("settings.onenewapiUrlPh"))}" />
        <div class="ona-edit-actions">
          <button type="submit">${escapeHtml(t("settings.save"))}</button>
          <button type="button" data-ona-cancel>${escapeHtml(t("dialog.cancel"))}</button>
        </div>
      </form>`
    : `<p class="ona-site-url">${escapeHtml(site.base_url)}</p>`;
  const addKey = `<form class="ona-key-add" data-ona-add-key="${escapeHtml(site.id)}" autocomplete="off">
        <input type="text" spellcheck="false" data-ona-add-label placeholder="${escapeHtml(t("settings.onenewapiKeyLabelPh"))}" />
        <div class="ona-add-row">
          <input type="password" data-ona-add-secret placeholder="${escapeHtml(t("settings.onenewapiKeySecretPh"))}" autocomplete="new-password" />
          <button type="submit">${escapeHtml(t("settings.onenewapiAddKey"))}</button>
        </div>
      </form>`;
  return `
    <article class="ona-site${open ? " open" : ""}" data-ona-site="${escapeHtml(site.id)}">
      <div class="ona-site-head">
        <button type="button" class="ona-site-toggle" data-ona-toggle="${escapeHtml(site.id)}">
          <span class="ona-site-name">${escapeHtml(site.name)}</span>
          <span class="chev">⌄</span>
        </button>
        <button type="button" class="mini-btn" data-ona-edit="${escapeHtml(site.id)}">${escapeHtml(t("settings.onenewapiEdit"))}</button>
        <button type="button" class="mini-btn danger" data-ona-delete="${escapeHtml(site.id)}">${escapeHtml(t("settings.onenewapiDelete"))}</button>
      </div>
      <div class="acc-body"><div class="acc-inner">${head}${keysHtml}${addKey}</div></div>
    </article>`;
}

function renderOneNewApiSettings(): void {
  const host = document.querySelector("#onenewapi-sites");
  if (!host) return;
  host.innerHTML = onaSites.map(renderOneNewApiSite).join("");
  syncOneNewApiFamilyToggle();
}

function focusOneNewApiSite(id: string): void {
  onaExpanded.add(id);
  document.querySelector("#onenewapi-sites")?.closest(".acc-group")?.classList.add("open");
  renderOneNewApiSettings();
  requestAnimationFrame(() => {
    document.querySelector(`[data-ona-site="${CSS.escape(id)}"]`)?.scrollIntoView({
      block: "nearest",
      behavior: "smooth",
    });
  });
}

async function loadOneNewApiSites(opts?: { focusId?: string }): Promise<void> {
  try {
    onaSites = await invoke<OneNewApiSiteDto[]>("onenewapi_list_sites");
    onaSitesLoaded = true;
  } catch (err) {
    onaSites = [];
    setOneNewApiStatus("footer.onenewapiFailed", { err: String(err) });
    if (opts?.focusId) {
      focusOneNewApiSite(opts.focusId);
    } else {
      renderOneNewApiSettings();
    }
    if (customizeOpen) renderDrawerBody();
    return;
  }
  const folded = foldOnaKeysIntoLayout();
  const pruned = pruneGoneOnaKeys();
  if ((folded || pruned) && config.layout) {
    void patchConfig({
      layout: config.layout,
      disabled: config.disabled,
      pinned: config.pinned,
    }).catch(() => {});
  }
  for (const site of onaSites) {
    if (site.keys.length === 0) onaExpanded.add(site.id);
  }
  if (opts?.focusId) {
    focusOneNewApiSite(opts.focusId);
  } else {
    renderOneNewApiSettings();
  }
  if (customizeOpen) renderDrawerBody();
}

async function createOneNewApiSite(): Promise<void> {
  if (onaBusy) return;
  const nameInput = document.querySelector<HTMLInputElement>("#ona-add-name");
  const urlInput = document.querySelector<HTMLInputElement>("#ona-add-url");
  const secretInput = document.querySelector<HTMLInputElement>("#ona-add-secret");
  const name = nameInput?.value.trim() ?? "";
  const baseUrl = urlInput?.value.trim() ?? "";
  const apiKey = secretInput?.value ?? "";
  if (!baseUrl) {
    setOneNewApiStatus("settings.onenewapiUrlRequired");
    urlInput?.focus();
    return;
  }
  onaBusy = true;
  try {
    const result = await invoke<OneNewApiCreateSiteResult>("onenewapi_create_site", {
      name,
      baseUrl,
    });
    const siteId = result.status === "duplicate" ? result.site_id : result.site.id;
    if (nameInput) nameInput.value = "";
    if (urlInput) urlInput.value = "";
    if (result.status === "duplicate" && !apiKey.trim()) {
      if (secretInput) secretInput.value = "";
      setOneNewApiStatus("footer.onenewapiDuplicate");
      await loadOneNewApiSites(siteId ? { focusId: siteId } : undefined);
      return;
    }
    if (apiKey.trim() && siteId) {
      const wasZeroKeys = onaTotalKeys() === 0;
      const keyResult = await invoke<OneNewApiCreatedKeyDto>("onenewapi_create_key", {
        siteId,
        label: "",
        apiKey,
      });
      if (secretInput) secretInput.value = "";
      applyOneNewApiSite(keyResult.site);
      setOneNewApiStatus("footer.onenewapiKeySaved");
      await enableNewOneNewApiKey(keyResult.key_id, wasZeroKeys);
      await loadOneNewApiSites(siteId ? { focusId: siteId } : undefined);
      return;
    }
    if (secretInput) secretInput.value = "";
    setOneNewApiStatus("footer.onenewapiSaved");
    await loadOneNewApiSites(siteId ? { focusId: siteId } : undefined);
  } catch (err) {
    setOneNewApiCaughtError(err);
  } finally {
    onaBusy = false;
  }
}

async function saveOneNewApiSite(id: string): Promise<void> {
  if (onaBusy) return;
  const block = document.querySelector(`[data-ona-site="${CSS.escape(id)}"]`);
  const name = block?.querySelector<HTMLInputElement>("[data-ona-edit-name]")?.value.trim() ?? "";
  const baseUrl = block?.querySelector<HTMLInputElement>("[data-ona-edit-url]")?.value.trim() ?? "";
  const current = onaSites.find((s) => s.id === id);
  if (!current) return;
  if (!baseUrl) {
    setOneNewApiStatus("settings.onenewapiUrlRequired");
    return;
  }
  const candidateOrigin = oneNewApiOriginKey(baseUrl);
  if (!candidateOrigin) {
    setOneNewApiStatus("footer.onenewapiProbeFailed", { err: "invalid URL" });
    return;
  }
  const urlChanged = candidateOrigin !== oneNewApiOriginKey(current.base_url);
  onaBusy = true;
  try {
    if (urlChanged) {
      try {
        await invoke("onenewapi_probe_site", { baseUrl });
      } catch (err) {
        setOneNewApiCaughtError(err, true);
        return;
      }
      const ok = await appConfirm({
        title: t("settings.onenewapiMigrateTitle"),
        message: t("settings.onenewapiMigrateBody", { n: current.keys.length }),
        confirmLabel: t("settings.onenewapiMigrateConfirm"),
        danger: true,
      });
      if (!ok) return;
    }
    const patch = { id, name, baseUrl };
    await invoke("onenewapi_update_site", patch);
    onaEditingId = null;
    onaExpanded.add(id);
    setOneNewApiStatus("footer.onenewapiSaved");
    await loadOneNewApiSites();
    if (urlChanged) await forceUsageRefreshAttempt();
    else {
      const site = onaSites.find((s) => s.id === id);
      if (site) paintOneNewApiCardNames(site);
    }
  } catch (err) {
    setOneNewApiCaughtError(err);
  } finally {
    onaBusy = false;
  }
}

async function deleteOneNewApiSite(id: string): Promise<void> {
  if (onaBusy) return;
  const site = onaSites.find((s) => s.id === id);
  if (!site) return;
  const ok = await appConfirm({
    title: t("settings.onenewapiDeleteTitle"),
    message: t("settings.onenewapiDeleteBody", { n: site.keys.length }),
    confirmLabel: t("settings.onenewapiDeleteConfirm"),
    danger: true,
  });
  if (!ok) return;
  onaBusy = true;
  try {
    await invoke("onenewapi_delete_site", { id });
    onaExpanded.delete(id);
    if (onaEditingId === id) onaEditingId = null;
    if (site.keys.some((k) => k.id === onaEditingKeyId)) onaEditingKeyId = null;
    setOneNewApiStatus("footer.onenewapiDeleted");
    await loadOneNewApiSites();
    await forceUsageRefreshAttempt();
    requestTraySync();
  } catch (err) {
    setOneNewApiStatus("footer.onenewapiFailed", { err: String(err) });
  } finally {
    onaBusy = false;
  }
}

async function enableNewOneNewApiKey(keyId: string, wasZeroKeys: boolean): Promise<void> {
  const snapshotId = keyId ? onaSnapshotId(keyId) : "";
  // Mark before patch/refresh so first-run auto-disable cannot park them.
  if (snapshotId) recentlyKeyed.set(snapshotId, refreshGeneration);
  if (wasZeroKeys) recentlyKeyed.set(ONA_FAMILY, refreshGeneration);

  const pending: Array<{ id: string; gen: number }> = [];
  if (wasZeroKeys) {
    if (snapshotId) pending.push({ id: snapshotId, gen: markProviderEnablePending(snapshotId) });
    pending.push({ id: ONA_FAMILY, gen: markProviderEnablePending(ONA_FAMILY) });
  } else if (snapshotId && config.disabled.includes(snapshotId)) {
    pending.push({ id: snapshotId, gen: markProviderEnablePending(snapshotId) });
  }

  const remove = new Set<string>();
  if (snapshotId) remove.add(snapshotId);
  if (wasZeroKeys) remove.add(ONA_FAMILY);
  if (remove.size && config.disabled.some((id) => remove.has(id))) {
    await patchConfig({
      disabled: config.disabled.filter((id) => !remove.has(id)),
    }).catch(() => {});
  }
  await forceUsageRefreshAttempt();
  if (pending.length) {
    for (const p of pending) finishProviderEnable(p.id, p.gen);
  } else {
    requestTraySync();
  }
}

async function createOneNewApiKey(siteId: string): Promise<void> {
  if (onaBusy) return;
  const block = document.querySelector(`[data-ona-site="${CSS.escape(siteId)}"]`);
  const labelInput = block?.querySelector<HTMLInputElement>("[data-ona-add-label]");
  const secretInput = block?.querySelector<HTMLInputElement>("[data-ona-add-secret]");
  const label = labelInput?.value.trim() ?? "";
  const apiKey = secretInput?.value ?? "";
  if (!apiKey.trim()) {
    secretInput?.focus();
    return;
  }
  const wasZeroKeys = onaTotalKeys() === 0;
  onaBusy = true;
  try {
    const created = await invoke<OneNewApiCreatedKeyDto>("onenewapi_create_key", {
      siteId,
      label,
      apiKey,
    });
    if (secretInput) secretInput.value = "";
    if (labelInput) labelInput.value = "";
    applyOneNewApiSite(created.site);
    onaEditingKeyId = null;
    onaExpanded.add(siteId);
    setOneNewApiStatus("footer.onenewapiKeySaved");
    await enableNewOneNewApiKey(created.key_id, wasZeroKeys);
    await loadOneNewApiSites();
  } catch (err) {
    setOneNewApiStatus("footer.onenewapiFailed", { err: String(err) });
  } finally {
    onaBusy = false;
  }
}

async function saveOneNewApiKey(siteId: string, keyId: string): Promise<void> {
  if (onaBusy) return;
  const form = document.querySelector(
    `[data-ona-edit-key-form="${CSS.escape(siteId)}"][data-ona-key="${CSS.escape(keyId)}"]`,
  );
  const label = form?.querySelector<HTMLInputElement>("[data-ona-key-label]")?.value.trim() ?? "";
  const apiKey = form?.querySelector<HTMLInputElement>("[data-ona-key-secret]")?.value ?? "";
  onaBusy = true;
  try {
    const patch: { siteId: string; keyId: string; label: string; apiKey?: string } = {
      siteId,
      keyId,
      label,
    };
    if (apiKey.trim()) patch.apiKey = apiKey;
    const site = await invoke<OneNewApiSiteDto>("onenewapi_update_key", patch);
    const secret = form?.querySelector<HTMLInputElement>("[data-ona-key-secret]");
    if (secret) secret.value = "";
    applyOneNewApiSite(site);
    onaEditingKeyId = null;
    onaExpanded.add(siteId);
    setOneNewApiStatus("footer.onenewapiKeySaved");
    await loadOneNewApiSites();
    await forceUsageRefreshAttempt();
    requestTraySync();
  } catch (err) {
    setOneNewApiStatus("footer.onenewapiFailed", { err: String(err) });
  } finally {
    onaBusy = false;
  }
}

async function deleteOneNewApiKey(siteId: string, keyId: string): Promise<void> {
  if (onaBusy) return;
  onaBusy = true;
  try {
    const site = await invoke<OneNewApiSiteDto>("onenewapi_delete_key", { siteId, keyId });
    applyOneNewApiSite(site);
    if (onaEditingKeyId === keyId) onaEditingKeyId = null;
    onaExpanded.add(siteId);
    await loadOneNewApiSites();
    await forceUsageRefreshAttempt();
    requestTraySync();
  } catch (err) {
    setOneNewApiStatus("footer.onenewapiFailed", { err: String(err) });
  } finally {
    onaBusy = false;
  }
}

function handleOneNewApiClick(target: HTMLElement): void {
  const toggle = target.closest<HTMLElement>("[data-ona-toggle]");
  if (toggle) {
    const id = toggle.dataset.onaToggle!;
    if (onaExpanded.has(id)) {
      onaExpanded.delete(id);
      if (onaEditingId === id) onaEditingId = null;
      const site = onaSites.find((s) => s.id === id);
      if (site?.keys.some((k) => k.id === onaEditingKeyId)) onaEditingKeyId = null;
    } else {
      onaExpanded.add(id);
    }
    renderOneNewApiSettings();
    return;
  }
  const editKey = target.closest<HTMLElement>("[data-ona-edit-key]");
  if (editKey) {
    const keyId = editKey.dataset.onaEditKey!;
    if (onaEditingKeyId === keyId) return;
    onaEditingKeyId = keyId;
    const siteId = editKey.closest<HTMLElement>("[data-ona-site]")?.dataset.onaSite;
    if (siteId) onaExpanded.add(siteId);
    renderOneNewApiSettings();
    requestAnimationFrame(() => {
      document
        .querySelector<HTMLInputElement>(`[data-ona-key="${CSS.escape(keyId)}"] [data-ona-key-label]`)
        ?.focus();
    });
    return;
  }
  const edit = target.closest<HTMLElement>("[data-ona-edit]");
  if (edit) {
    const id = edit.dataset.onaEdit!;
    if (onaEditingId === id) return;
    onaEditingId = id;
    onaExpanded.add(id);
    renderOneNewApiSettings();
    requestAnimationFrame(() => {
      document
        .querySelector<HTMLInputElement>(`[data-ona-site="${CSS.escape(id)}"] [data-ona-edit-name]`)
        ?.focus();
    });
    return;
  }
  const cancelKey = target.closest<HTMLElement>("[data-ona-cancel-key]");
  if (cancelKey) {
    onaEditingKeyId = null;
    renderOneNewApiSettings();
    return;
  }
  const cancel = target.closest<HTMLElement>("[data-ona-cancel]");
  if (cancel) {
    onaEditingId = null;
    renderOneNewApiSettings();
  }
}

function initOneNewApiSettings(): void {
  const addForm = document.querySelector<HTMLFormElement>("#onenewapi-add");
  addForm?.addEventListener("submit", (e) => {
    e.preventDefault();
    void createOneNewApiSite();
  });
  document.querySelector<HTMLInputElement>("#ona-family-enabled")?.addEventListener("change", (e) => {
    const input = e.currentTarget as HTMLInputElement;
    input.dataset.enable = ONA_FAMILY;
    handleCustomizeChange(input);
  });
  const host = document.querySelector("#onenewapi-sites");
  host?.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;
    const delKey = target.closest<HTMLElement>("[data-ona-delete-key]");
    if (delKey) {
      e.preventDefault();
      const siteId = delKey.closest<HTMLElement>("[data-ona-site]")?.dataset.onaSite;
      if (siteId) void deleteOneNewApiKey(siteId, delKey.dataset.onaDeleteKey!);
      return;
    }
    const del = target.closest<HTMLElement>("[data-ona-delete]");
    if (del) {
      e.preventDefault();
      void deleteOneNewApiSite(del.dataset.onaDelete!);
      return;
    }
    handleOneNewApiClick(target);
  });
  host?.addEventListener("submit", (e) => {
    const target = e.target as HTMLElement;
    const addKey = target.closest<HTMLElement>("[data-ona-add-key]");
    if (addKey) {
      e.preventDefault();
      void createOneNewApiKey(addKey.dataset.onaAddKey!);
      return;
    }
    const editKey = target.closest<HTMLElement>("[data-ona-edit-key-form]");
    if (editKey) {
      e.preventDefault();
      void saveOneNewApiKey(editKey.dataset.onaEditKeyForm!, editKey.dataset.onaKey!);
      return;
    }
    const form = target.closest<HTMLElement>("[data-ona-edit-form]");
    if (!form) return;
    e.preventDefault();
    void saveOneNewApiSite(form.dataset.onaEditForm!);
  });
}

async function unparkRecentlyKeyed(): Promise<void> {
  if (!config.disabled.some((id) => recentlyKeyed.has(id))) return;
  await patchConfig({
    disabled: config.disabled.filter((id) => !recentlyKeyed.has(id)),
  }).catch(() => {});
}

// Settings rows pass nothing and are found by their #key-/#baseurl- ids;
// the Customize gear panel passes its own inputs explicitly.
async function saveApiKey(
  provider: string,
  fields?: { key: HTMLInputElement; baseUrl?: HTMLInputElement | null },
): Promise<void> {
  const input = fields?.key ?? document.querySelector<HTMLInputElement>(`#key-${provider}`)!;
  const status = document.querySelector("#status")!;
  let enableGeneration: number | undefined;
  try {
    const key = input.value;
    if (!key.trim()) {
      recentlyKeyed.delete(provider);
    } else {
      // Mark before any await so an in-flight first-run pass cannot park
      // this provider after our save returns.
      recentlyKeyed.set(provider, refreshGeneration);
      if (config.disabled.includes(provider)) {
        enableGeneration = markProviderEnablePending(provider);
      }
    }
    // Providers with a user-chosen endpoint (relaybalance) carry a base
    // URL input next to the key field; Save persists both together.
    const baseUrlInput = fields
      ? (fields.baseUrl ?? null)
      : document.querySelector<HTMLInputElement>(`#baseurl-${provider}`);
    const baseUrl = baseUrlInput?.value.trim() || null;
    await invoke("set_api_key", { provider, key, baseUrl });
    input.value = "";
    // The gear panel's chips + saved-credential list reflect the new key.
    credStatusCache.delete(provider);
    refreshCredStatus(provider);
    // Pasting a key says "show me this provider" — pull it out of Disabled.
    // First-run auto-disable parks keyless providers there, and a key saved
    // against a still-disabled toggle would otherwise never produce a bar
    // no matter how often Refresh is clicked.
    if (key.trim() && config.disabled.includes(provider)) {
      await patchConfig({
        disabled: config.disabled.filter((id) => id !== provider),
      }).catch(() => {});
    }
    const name = providerDisplayName(provider);
    status.textContent = t("footer.keySaved", { name });
    await forceUsageRefreshAttempt();
    if (enableGeneration !== undefined) finishProviderEnable(provider, enableGeneration);
    else requestTraySync();
  } catch (err) {
    recentlyKeyed.delete(provider);
    if (enableGeneration !== undefined) finishProviderEnable(provider, enableGeneration);
    else requestTraySync();
    status.textContent = t("footer.keySaveFailed", { err: String(err) });
  }
}

function populatePinnedOptions(): void {
  const select = document.querySelector<HTMLSelectElement>("#pinned")!;
  const current = config.pinned ? `${config.pinned.provider}::${config.pinned.label}` : "";
  select.replaceChildren(new Option(t("settings.pinAuto"), ""));
  for (const s of lastSnapshots) {
    if (isCardDisabled(s.id) || s.status !== "ok") continue;
    for (const m of s.metrics) {
      if (m.kind !== "progress") continue;
      const value = `${s.id}::${m.label}`;
      select.add(
        new Option(
          t("settings.pinOption", { name: s.name, label: displayMetricLabel(m.label) }),
          value,
          false,
          value === current,
        ),
      );
    }
  }
}

function applyLocale(): void {
  config.locale = normalizeLocalePref(config.locale);
  setActiveLocale(resolveLocale(config.locale));
  applyStaticI18n();
  renderOneNewApiSettings();
  applyAppearance();
  const status = document.querySelector("#status");
  if (status) {
    if (lastSnapshots.length) {
      const time = new Date().toLocaleTimeString(localeTag(), {
        hour: "2-digit",
        minute: "2-digit",
      });
      status.textContent = t("footer.updated", { time });
    } else {
      status.textContent = t("footer.starting");
    }
  }
  if (lastSnapshots.length) renderIfVisible();
  populatePinnedOptions();
  renderBuildInfo();
}

async function initSettings(): Promise<void> {
  config = await invoke<Config>("get_config");
  config.locale = normalizeLocalePref(config.locale);
  try {
    const sys = await invoke<string>("system_ui_locale");
    setSystemLocale(sys === "zh" || sys === "ru" ? sys : "en");
  } catch {
    // Dev / missing command — fall back to navigator.language.
  }
  applyLocale();
  if (["today", "yesterday", "last30"].includes(config.spendTab)) {
    spendTab = config.spendTab;
  }

  const interval = document.querySelector<HTMLInputElement>("#interval")!;
  interval.value = String(config.refreshMinutes);
  interval.addEventListener("change", () => {
    const minutes = Math.max(1, Math.min(120, Number(interval.value) || 5));
    interval.value = String(minutes);
    void patchConfig({ refreshMinutes: minutes }).then(scheduleAutoRefresh);
  });

  const autostart = document.querySelector<HTMLInputElement>("#autostart")!;
  autostart.checked = await invoke<boolean>("get_autostart");
  autostart.addEventListener("change", () => {
    void invoke("set_autostart", { enabled: autostart.checked }).catch((err) => {
      document.querySelector("#status")!.textContent = t("footer.autostartFailed", { err: String(err) });
      autostart.checked = !autostart.checked;
    });
  });

  const pacing = document.querySelector<HTMLInputElement>("#pacing")!;
  pacing.checked = config.pacingAlways;
  pacing.addEventListener("change", () => {
    void patchConfig({ pacingAlways: pacing.checked }).then(renderAll);
  });

  const timeFormat = document.querySelector<HTMLSelectElement>("#timeformat")!;
  timeFormat.value = config.timeFormat;
  timeFormat.addEventListener("change", () => {
    void patchConfig({ timeFormat: timeFormat.value as Config["timeFormat"] }).then(renderAll);
  });

  const localeSel = document.querySelector<HTMLSelectElement>("#locale")!;
  localeSel.value = config.locale;
  localeSel.addEventListener("change", () => {
    const next = normalizeLocalePref(localeSel.value);
    void patchConfig({ locale: next }).catch(() => {});
    applyLocale();
    requestTraySync();
  });

  const notifyToggles: [string, keyof Config][] = [
    ["#notify-almost", "notifyAlmostOut"],
    ["#notify-close", "notifyCuttingClose"],
    ["#notify-runout", "notifyWillRunOut"],
    ["#telemetry", "telemetry"],
  ];
  for (const [selector, key] of notifyToggles) {
    const box = document.querySelector<HTMLInputElement>(selector)!;
    box.checked = Boolean(config[key]);
    box.addEventListener("change", () => {
      void patchConfig({ [key]: box.checked } as Partial<Config>);
    });
  }

  const pinned = document.querySelector<HTMLSelectElement>("#pinned")!;
  pinned.addEventListener("change", () => {
    const [provider, label] = pinned.value.split("::");
    const value = provider && label ? { provider, label } : null;
    void patchConfig({ pinned: value }).catch(() => {});
    requestTraySync();
  });

  const showSpend = document.querySelector<HTMLInputElement>("#show-total-spend")!;
  showSpend.checked = config.showTotalSpend;
  showSpend.addEventListener("change", () => {
    void patchConfig({ showTotalSpend: showSpend.checked }).then(renderAll);
  });

  applyAppearance();
  const appearance = document.querySelector<HTMLSelectElement>("#appearance")!;
  appearance.value = config.appearance;
  appearance.addEventListener("change", () => {
    void patchConfig({ appearance: appearance.value as Config["appearance"] }).then(applyAppearance);
  });

  const density = document.querySelector<HTMLInputElement>("#density")!;
  density.checked = config.density === "compact";
  density.addEventListener("change", () => {
    void patchConfig({ density: density.checked ? "compact" : "regular" }).then(applyAppearance);
  });

  const glass = document.querySelector<HTMLInputElement>("#glass")!;
  glass.checked = config.glassEffects !== false;
  glass.addEventListener("change", () => {
    void patchConfig({ glassEffects: glass.checked }).then(applyGlass);
  });
  applyGlass();

  const reduceAnim = document.querySelector<HTMLInputElement>("#reduce-anim")!;
  reduceAnim.checked = config.reduceAnimations === true;
  reduceAnim.addEventListener("change", () => {
    void patchConfig({ reduceAnimations: reduceAnim.checked }).then(applyReduceMotion);
  });
  applyReduceMotion();

  const hideShare = document.querySelector<HTMLInputElement>("#hide-while-sharing")!;
  hideShare.checked = config.hideUsageWhileSharing === true;
  hideShare.addEventListener("change", () => {
    void patchConfig({ hideUsageWhileSharing: hideShare.checked }).catch(() => {});
    requestTraySync();
  });

  const shortcut = document.querySelector<HTMLInputElement>("#shortcut")!;
  shortcut.value = config.shortcut;
  shortcut.addEventListener("change", async () => {
    const status = document.querySelector("#status")!;
    try {
      await invoke("set_shortcut", { shortcut: shortcut.value });
      await patchConfig({ shortcut: shortcut.value });
      status.textContent = shortcut.value.trim() ? t("footer.shortcutSaved") : t("footer.shortcutCleared");
    } catch (err) {
      status.textContent = `${err}`;
    }
  });

  const proxyEnabled = document.querySelector<HTMLInputElement>("#proxy-enabled")!;
  const proxyUrl = document.querySelector<HTMLInputElement>("#proxy-url")!;
  proxyEnabled.checked = config.proxy?.enabled ?? false;
  proxyUrl.value = config.proxy?.url ?? "";
  const saveProxy = () => {
    void patchConfig({ proxy: { enabled: proxyEnabled.checked, url: proxyUrl.value.trim() } }).then(
      () => {
        document.querySelector("#status")!.textContent = t("footer.proxySaved");
      },
    );
  };
  proxyEnabled.addEventListener("change", saveProxy);
  proxyUrl.addEventListener("change", saveProxy);

  // Custom Balance: pre-fill the relay base URL saved with its key.
  const relayBase = document.querySelector<HTMLInputElement>("#baseurl-relaybalance");
  if (relayBase) {
    relayBase.value =
      (await invoke<string | null>("get_base_url", { provider: "relaybalance" }).catch(
        () => null,
      )) ?? "";
  }

  populatePinnedOptions();

  document.querySelector("#reset-all-settings")!.addEventListener("click", () => {
    void resetAllSettings();
  });

  initOneNewApiSettings();
}

/// Restore every preference to the same defaults a fresh install gets.
/// API keys, lastSeenVersion, and welcomeDismissed stay (keys are not
/// "settings"; What's-new shouldn't pop again).
async function resetAllSettings(): Promise<void> {
  const ok = await appConfirm({
    title: t("settings.resetTitle"),
    message: t("settings.resetBody"),
    confirmLabel: t("settings.resetConfirm"),
    danger: true,
  });
  if (!ok) return;
  try {
    await invoke("set_autostart", { enabled: true });
  } catch {
    // Dev builds skip autostart; the preference is still saved below.
  }
  try {
    await invoke("set_shortcut", { shortcut: "" });
  } catch {
    // Invalid leftover shortcut shouldn't block the rest of the reset.
  }
  await patchConfig({
    refreshMinutes: 1,
    disabled: [],
    pinned: null,
    trayProviders: [],
    pacingAlways: true,
    telemetry: true,
    notifyAlmostOut: true,
    notifyCuttingClose: true,
    notifyWillRunOut: true,
    spendTab: "today",
    spendMetric: "cost",
    showUsed: false,
    resetExact: false,
    timeFormat: "auto",
    layout: null,
    appearance: "dark",
    density: "compact",
    glassEffects: true,
    shortcut: "",
    proxy: { enabled: false, url: "" },
    showTotalSpend: true,
    reduceAnimations: false,
    hideUsageWhileSharing: false,
    locale: "auto",
  }).catch(() => {});
  spendTab = "today";
  applyLocale();
  syncSettingsControls();
  scheduleAutoRefresh();
  applyAppearance();
  applyGlass();
  applyReduceMotion();
  document.body.classList.remove("settings-open");
  document.querySelector("#settings-btn")?.classList.remove("active");
  void forceUsageRefreshAttempt(false).then(requestTraySync);
}

function syncSettingsControls(): void {
  const setNum = (sel: string, v: string) => {
    const el = document.querySelector<HTMLInputElement>(sel);
    if (el) el.value = v;
  };
  const setCheck = (sel: string, v: boolean) => {
    const el = document.querySelector<HTMLInputElement>(sel);
    if (el) el.checked = v;
  };
  const setSelect = (sel: string, v: string) => {
    const el = document.querySelector<HTMLSelectElement>(sel);
    if (el) el.value = v;
  };
  setNum("#interval", String(config.refreshMinutes));
  setCheck("#pacing", config.pacingAlways);
  setSelect("#timeformat", config.timeFormat);
  setSelect("#locale", config.locale);
  setCheck("#notify-almost", config.notifyAlmostOut);
  setCheck("#notify-close", config.notifyCuttingClose);
  setCheck("#notify-runout", config.notifyWillRunOut);
  setCheck("#telemetry", config.telemetry);
  setCheck("#hide-while-sharing", config.hideUsageWhileSharing === true);
  setCheck("#show-total-spend", config.showTotalSpend);
  setSelect("#appearance", config.appearance);
  setCheck("#density", config.density === "compact");
  setCheck("#glass", config.glassEffects !== false);
  setCheck("#reduce-anim", config.reduceAnimations === true);
  setNum("#shortcut", config.shortcut);
  setCheck("#proxy-enabled", config.proxy?.enabled ?? false);
  setNum("#proxy-url", config.proxy?.url ?? "");
  const autostart = document.querySelector<HTMLInputElement>("#autostart");
  if (autostart) autostart.checked = true;
  populatePinnedOptions();
}

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

window.addEventListener("DOMContentLoaded", () => {
  const appLogo = document.querySelector<HTMLElement>("#app-logo")!;
  appLogo.innerHTML = `<img src="${paneLogo}" alt="Pane" />`;
  // Party mode, the easy way: triple-click the logo. (The Konami code
  // still works, for the culture.)
  let logoClicks = 0;
  let logoClickReset: number | undefined;
  appLogo.addEventListener("click", () => {
    logoClicks += 1;
    window.clearTimeout(logoClickReset);
    logoClickReset = window.setTimeout(() => (logoClicks = 0), 1200);
    if (logoClicks >= 3) {
      logoClicks = 0;
      toggleParty();
    }
  });
  document.querySelector("#theme-btn")!.addEventListener("click", toggleTheme);
  setupTrailFisheye();
  setupTooltips();
  // No lens init here: applyGlass() (via initSettings, after the saved
  // config arrives) owns it — a fixed timer raced the config load and
  // built the maps even for users who turned glass off.
  window.addEventListener("keydown", (e) => {
    konamiListen(e);
    if (e.ctrlKey && e.key.toLowerCase() === "z" && customizeOpen) {
      e.preventDefault();
      undoLayout();
    }
    // Esc backs out of Customize/Settings (Mac parity).
    if (e.key === "Escape") {
      setDrawer(false);
      setSettings(false);
    }
    // Ctrl+R refreshes data — and must NOT reload the webview.
    if (e.ctrlKey && e.key.toLowerCase() === "r") {
      e.preventDefault();
      void refresh(true);
    }
  });
  void getVersion().then((v) => {
    appVersion = v;
    buildText = `v${v} · build ${__BUILD_STAMP__}`;
    renderBuildInfo();
    void checkForUpdate();
  });
  document.querySelector("#refresh")!.addEventListener("click", () => void refresh(true));

  const setSettings = (open: boolean) => {
    document.body.classList.toggle("settings-open", open);
    document.querySelector("#settings-btn")?.classList.toggle("active", open);
    if (open) void loadOneNewApiSites();
  };
  document.querySelector("#settings-btn")!.addEventListener("click", () => {
    setDrawer(false);
    setSettings(!document.body.classList.contains("settings-open"));
  });
  document.querySelector("#settings-close")!.addEventListener("click", () => setSettings(false));
  document.querySelector("#changelog-btn")!.addEventListener("click", () => {
    setSettings(false);
    showChangelogDialog(t("dialog.changelog"), parseChangelog());
  });
  document.querySelectorAll<HTMLElement>(".acc-head").forEach((head) => {
    head.addEventListener("click", () => head.parentElement!.classList.toggle("open"));
  });
  document.querySelector("#customize-btn")!.addEventListener("click", () => {
    setSettings(false);
    setDrawer(!customizeOpen);
  });
  const drawerBody = document.querySelector<HTMLElement>("#drawer-body")!;
  drawerBody.addEventListener("click", (e) => {
    handleCustomizeClick(e.target as HTMLElement);
  });
  drawerBody.addEventListener("change", (e) => {
    handleCustomizeChange(e.target as HTMLInputElement);
  });
  drawerBody.addEventListener("input", (e) => {
    const el = e.target as HTMLInputElement;
    if (el.id === "cust-search") applyCustomizeFilter(el.value);
    else if (el.matches("[data-cust-key], [data-cust-baseurl]")) {
      resetCustTestState(el.closest(".cust-config"));
    } else if (el.matches("[data-acct-key], [data-acct-baseurl], [data-acct-label]")) {
      resetAcctTestState(el.closest(".acct-form"));
    }
  });
  setupCustomizeDnD(drawerBody);
  document.querySelectorAll<HTMLButtonElement>("[data-save]").forEach((btn) => {
    btn.addEventListener("click", () => void saveApiKey(btn.dataset.save!));
  });

  const providersEl = document.querySelector<HTMLElement>("#providers")!;
  // The donut center toggles what the card meters: dollars ⇄ raw tokens.
  // Left or right click both work; the choice persists.
  const toggleSpendMetric = (back = false) => {
    config.spendMetric = nextSpendMetric(back);
    void patchConfig({ spendMetric: config.spendMetric });
    renderAll();
  };
  providersEl.addEventListener("contextmenu", (e) => {
    if ((e.target as Element).closest?.(".donut-wrap")) {
      e.preventDefault();
      toggleSpendMetric(true); // right-click cycles backward
    }
  });

  // Donut hover: pointing at a segment or its legend row swells the arc
  // and dims the others, Mac-style. [data-pid] links the two.
  const setDonutHot = (id: string | null) => {
    document.querySelectorAll<HTMLElement>(".total-spend [data-pid]").forEach((el) => {
      el.classList.toggle("hot", id !== null && el.dataset.pid === id);
    });
  };
  providersEl.addEventListener("mouseover", (e) => {
    const t = (e.target as Element).closest?.<HTMLElement>(".total-spend [data-pid]");
    if (t) setDonutHot(t.dataset.pid ?? null);
  });
  providersEl.addEventListener("mouseout", (e) => {
    if ((e.target as Element).closest?.(".total-spend [data-pid]")) setDonutHot(null);
  });

  // In-popover reordering: drag a card by the grip in its header. The new
  // order saves to the same layout Customize edits, so both stay in sync.
  let dragCard: HTMLElement | null = null;
  let armedCard: HTMLElement | null = null;
  providersEl.addEventListener("mousedown", (e) => {
    const grip = (e.target as HTMLElement).closest(".drag-grip");
    const card = grip?.closest<HTMLElement>("article[data-provider]");
    if (card) {
      card.draggable = true;
      armedCard = card;
    }
  });
  // A grip press that never turns into a drag would otherwise leave the
  // card grab-anywhere; disarm on release when no drag started.
  document.addEventListener("mouseup", () => {
    if (armedCard && !dragCard) armedCard.draggable = false;
    armedCard = null;
  });
  providersEl.addEventListener("dragstart", (e) => {
    dragCard = (e.target as HTMLElement).closest?.("article[data-provider]") ?? null;
    dragCard?.classList.add("dragging");
  });
  providersEl.addEventListener("dragover", (e) => {
    if (!dragCard) return;
    e.preventDefault();
    const over = (e.target as HTMLElement).closest?.<HTMLElement>("article[data-provider]");
    if (!over || over === dragCard) return;
    const r = over.getBoundingClientRect();
    const before = e.clientY < r.top + r.height / 2;
    over.parentElement!.insertBefore(dragCard, before ? over : over.nextElementSibling);
  });
  const endCardDrag = () => {
    if (!dragCard) return;
    dragCard.classList.remove("dragging");
    dragCard.draggable = false;
    dragCard = null;
    ensureLayout();
    const domIds = Array.from(
      providersEl.querySelectorAll<HTMLElement>("article[data-provider]")
    ).map((a) => a.dataset.provider!);
    const L = config.layout!;
    L.providerOrder = [...domIds, ...L.providerOrder.filter((id) => !domIds.includes(id))];
    void patchConfig({ layout: L });
    requestTraySync();
    updateTrailActive();
  };
  providersEl.addEventListener("drop", (e) => {
    e.preventDefault();
    endCardDrag();
  });
  providersEl.addEventListener("dragend", endCardDrag);

  providersEl.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;

    const link = target.closest<HTMLElement>("[data-link]");
    if (link) {
      void invoke("open_link", { url: link.dataset.link }).catch((err) => {
        document.querySelector("#status")!.textContent = t("footer.openLinkFailed", { err: String(err) });
      });
      return;
    }
    const shareBtn = target.closest<HTMLElement>("[data-share]");
    if (shareBtn) {
      void shareCard(shareBtn.dataset.share!);
      return;
    }
    if (target.closest(".donut-wrap")) {
      toggleSpendMetric();
      return;
    }
    if (target.closest("[data-welcome-close]")) {
      config.welcomeDismissed = true;
      void patchConfig({ welcomeDismissed: true });
      renderAll();
      return;
    }
    if (target.closest("[data-welcome-customize]")) {
      config.welcomeDismissed = true;
      void patchConfig({ welcomeDismissed: true });
      renderAll();
      setDrawer(true);
      return;
    }
    const redeem = target.closest<HTMLElement>("[data-redeem]");
    if (redeem) {
      const creditId = redeem.dataset.redeem!;
      // Multi-account: the redeem must ride the account whose card offered
      // the credit, not the default login's token.
      const providerId =
        redeem.closest<HTMLElement>("article.provider")?.dataset.provider ?? "codex";
      void appConfirm({
        title: t("redeem.title"),
        message: t("redeem.body"),
        confirmLabel: t("redeem.confirm"),
      }).then((ok) => {
        if (!ok) return;
        const status = document.querySelector("#status")!;
        status.textContent = t("footer.redeeming");
        void invoke<string>("codex_redeem_credit", { creditId, providerId })
          .then((msg) => {
            status.textContent = msg;
            void refresh(true);
          })
          .catch((err) => {
            status.textContent = t("footer.redeemFailed", { err: String(err) });
          });
      });
      return;
    }
    const tab = target.closest("[data-tab]");
    if (tab) {
      switchSpendTab(tab.getAttribute("data-tab") as SpendTab);
      return;
    }
    const caret = target.closest<HTMLElement>("[data-caret]");
    if (caret) {
      const id = caret.dataset.caret!;
      const L = providerLayout(id);
      L.expanded = !L.expanded;
      saveLayout(false);
      animateExpandId = L.expanded ? id : null;
      renderAll();
      animateExpandId = null;
      return;
    }
    const flip = target.closest<HTMLElement>("[data-flip]");
    if (flip) {
      if (flip.dataset.flip === "usage") {
        config.showUsed = !config.showUsed;
        void patchConfig({ showUsed: config.showUsed });
      } else {
        config.resetExact = !config.resetExact;
        void patchConfig({ resetExact: config.resetExact });
      }
      renderAll();
    }
  });


  const tip = document.querySelector<HTMLElement>("#model-tip")!;
  providersEl.addEventListener("mouseover", (e) => {
    if (customizeOpen) return;
    const target = e.target as HTMLElement;
    const bar = target.closest<HTMLElement>("[data-trend]");
    if (bar) {
      showTrendTip(bar);
      return;
    }
    const row = target.closest<HTMLElement>("[data-spend]");
    if (row) showModelTip(row);
  });
  providersEl.addEventListener("mouseout", (e) => {
    const target = e.target as HTMLElement;
    const hovered = target.closest<HTMLElement>("[data-spend], [data-trend]");
    const to = e.relatedTarget as HTMLElement | null;
    if (hovered && (!to || !hovered.contains(to))) tip.hidden = true;
  });
  let scrollRaf = 0;
  providersEl.addEventListener("scroll", () => {
    tip.hidden = true;
    cancelAnimationFrame(scrollRaf);
    scrollRaf = requestAnimationFrame(updateTrailActive);
  });

  document.querySelector("#trail")!.addEventListener("click", (e) => {
    const tick = (e.target as HTMLElement).closest<HTMLElement>("[data-trail]");
    if (!tick) return;
    const card = trailCards()[Number(tick.dataset.trail)];
    card?.scrollIntoView({ behavior: reduceMotion() ? "auto" : "smooth", block: "start" });
  });

  // The 4-hourly background checker feeds the same footer button.
  void listen<string>("update-available", (e) => {
    updateVersion = e.payload;
    renderBuildInfo();
  });

  void listen("tray-strip-restore", () => {
    requestTraySync();
  });

  void listen("popover-shown", () => {
    void checkForUpdate();
    // Always reopen on the main page, at the top — leftover Customize/
    // Settings panels, a stale confirm dialog, or a stale scroll position
    // from the previous visit feel like the app is stuck mid-page.
    setDrawer(false);
    setSettings(false);
    dismissConfirm?.();
    dismissWhatsNew?.();
    // A fresh update's notes present on the first open after launch.
    if (pendingWhatsNew) {
      showChangelogDialog(t("dialog.whatsNew", { version: appVersion }), pendingWhatsNew);
      pendingWhatsNew = null;
    }
    // Replay any renders skipped while hidden, before the reveal plays.
    if (pendingRender) {
      pendingRender = false;
      renderAll();
      populatePinnedOptions();
    }
    providersEl.scrollTop = 0;
    updateTrailActive();
    if (lastSnapshots.length && !customizeOpen) playReveal();
    requestTraySync();
    void refresh();
  });
  void initSettings().then(() => {
    scheduleAutoRefresh();
    void paintCachedSnapshots();
    void refresh(true);
    // Queued, not shown: the window is usually still hidden in the tray at
    // startup — the first popover-shown presents it. Runs after the config
    // load so lastSeenVersion is the real stored value, not the default.
    void getVersion().then((v) => {
      pendingWhatsNew = computeWhatsNew(v);
    });
  });

  // Countdown texts ("Resets in 3h 41m") tick every 30 s — but only for
  // eyes that can see them; hidden ticks fold into the deferred render.
  setInterval(() => {
    if (lastSnapshots.length && !customizeOpen) renderIfVisible();
  }, 30_000);
});
