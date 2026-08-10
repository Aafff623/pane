import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";

// Injected by vite.config.ts at build time, e.g. "0707.1432".
declare const __BUILD_STAMP__: string;

// Official provider marks from the MIT-licensed macOS OpenUsage, rendered
// inline so CSS can recolor them like template icons.
import antigravityIcon from "./assets/providers/antigravity.svg?raw";
import claudeIcon from "./assets/providers/claude.svg?raw";
import codexIcon from "./assets/providers/codex.svg?raw";
import copilotIcon from "./assets/providers/copilot.svg?raw";
import cursorIcon from "./assets/providers/cursor.svg?raw";
import devinIcon from "./assets/providers/devin.svg?raw";
import grokIcon from "./assets/providers/grok.svg?raw";
import minimaxIcon from "./assets/providers/minimax.svg?raw";
import opencodeIcon from "./assets/providers/opencode.svg?raw";
import openrouterIcon from "./assets/providers/openrouter.svg?raw";
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
  claude: claudeIcon,
  codex: codexIcon,
  copilot: copilotIcon,
  cursor: cursorIcon,
  devin: devinIcon,
  grok: grokIcon,
  minimax: minimaxIcon,
  opencode: opencodeIcon,
  openrouter: openrouterIcon,
  zai: zaiIcon,
};

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
const RELOGIN: Record<string, string> = {
  claude: "run `claude` in a terminal and sign in",
  codex: "run `codex login` in a terminal",
  grok: "run `grok` in a terminal and sign in",
  copilot: "run `gh auth login` in a terminal",
  cursor: "open Cursor and sign in again",
  devin: "run `devin` in a terminal and sign in",
  opencode: "run `opencode auth login` in a terminal",
  antigravity: "open Antigravity and sign in again",
  ollama: "make sure Ollama is running",
};

/// The ⚠ Outdated tooltip: what went wrong, what fixes it, and the
/// reassurance that the visible numbers are the last good ones. Errors are
/// classified into sign-in / rate-limit / vendor-outage / connection
/// buckets so the fix is concrete instead of a bare HTTP code.
function staleHelp(s: Snapshot): string {
  const w = (s.warning ?? "The last refresh failed").replace(/[.\s]+$/, "");
  const lw = w.toLowerCase();
  const relogin =
    RELOGIN[s.id] ?? "add the API key again in Settings (or sign in with the tool once)";
  let fix = "Pane keeps retrying automatically — nothing to do unless this persists.";
  if (/run `|open the/.test(lw)) {
    // The provider's own message already says what to do.
    fix = "Pane recovers automatically once that's done.";
  } else if (/http 40[13]|invalid_grant|expired|no refresh token|sign[- ]?in|log ?in|credentials/.test(lw)) {
    fix = `Fix: ${relogin} — Pane picks it up on the next refresh.`;
  } else if (/http 429|rate limit/.test(lw)) {
    fix = "The vendor is rate-limiting; Pane waits exactly as long as it asked, then retries by itself.";
  } else if (/http 5\d\d/.test(lw)) {
    fix = "The vendor's API is having trouble; Pane retries automatically until it recovers.";
  } else if (/error sending request|timed? ?out|connect|network|dns|proxy/.test(lw)) {
    fix = "Pane couldn't reach the vendor — check your internet connection (or the proxy in Settings).";
  }
  return `${w}.\n${fix}\nShowing the last good data meanwhile.`;
}

/// ⚠ shown when some events have no known model price — their tokens are
/// counted, but no dollars are guessed, so dollar totals under-report.
function unpricedWarn(sp: ProviderSpend | undefined): string {
  if (!sp || sp.unpriced <= 0) return "";
  const models = sp.unpriced_models.join(", ") || "unknown models";
  return `<span class="stale" title="${escapeHtml(
    `${sp.unpriced} requests ran on models with no public pricing (${models}). ` +
      `Their tokens are included, but they can't be turned into dollars — ` +
      `so the real cost is a little higher than shown.`,
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
  ["moonshot", "Moonshot"],
  ["elevenlabs", "ElevenLabs"],
  ["ollama", "Ollama"],
  ["codebuff", "Codebuff"],
  ["kilo", "Kilo"],
  ["aihubmix", "AihubMix"],
  ["qwen", "Qwen Code"],
];

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
  grok: [{ label: "Usage", url: "https://grok.com/?_s=usage" }],
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
};
let lastFetch = 0;
let refreshing = false;
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
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${String(rem).padStart(2, "0")}m`;
  return `${rem}m`;
}

// "today at 6:38 PM" / "tomorrow at 18:38" / "Sat, Jul 11 at 9:00 AM",
// honoring the Time Format setting.
function fmtExact(ts: number): string {
  const d = new Date(ts);
  const now = new Date();
  const hour12 =
    config.timeFormat === "12" ? true : config.timeFormat === "24" ? false : undefined;
  const time = d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit", hour12 });
  const dayStart = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diffDays = Math.round((dayStart(d) - dayStart(now)) / 86400000);
  if (diffDays === 0) return `today at ${time}`;
  if (diffDays === 1) return `tomorrow at ${time}`;
  return `${d.toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" })} at ${time}`;
}

async function patchConfig(patch: Partial<Config>): Promise<void> {
  config = await invoke<Config>("set_config", { patch });
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
    if (m.kind !== "progress") onDemand.push(m.label); // balances etc. tuck away
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
        if (m.kind !== "progress") L.onDemand.push(m.label);
        changed = true;
      }
      // A metric that upgraded from text to a progress bar (Extra
      // credits/balance when funded) must surface like one: bars never
      // hide behind Show more, and they sit above the Usage Trend.
      if (m.kind === "progress") {
        const tucked = L.onDemand.indexOf(m.label);
        if (tucked >= 0) {
          L.onDemand.splice(tucked, 1);
          changed = true;
        }
        const at = L.metricOrder.indexOf(m.label);
        const trendAt = L.metricOrder.indexOf(TREND_KEY);
        if (at >= 0 && trendAt >= 0 && at > trendAt) {
          L.metricOrder.splice(at, 1);
          L.metricOrder.splice(L.metricOrder.indexOf(TREND_KEY), 0, m.label);
          changed = true;
        }
      }
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

function saveLayout(): void {
  if (!config.layout) return;
  // Undo history: remember the state we're moving away from.
  const next = JSON.stringify(config.layout);
  if (lastLayoutSnapshot && lastLayoutSnapshot !== next) {
    undoStack.push(lastLayoutSnapshot);
    if (undoStack.length > 50) undoStack.shift();
  }
  lastLayoutSnapshot = next;
  void patchConfig({ layout: config.layout });
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
    return { cls: "low", note: "🔥 Limit reached", noteClass: "danger", title: "Limit reached", tick: null };
  }

  const byLevel = (): Pace => {
    if (left <= 10) return { ...none, cls: "low", title: `${Math.round(left)}% left` };
    if (used >= 80) return { ...none, cls: "warn", title: `${Math.round(used)}% used` };
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
        ? `Limit ${fmtExact(runOutAt)}`
        : `Limit in ${fmtDuration(runOutAt - now)}`;
      return { cls: "low", note: `🔥 ${when}`, noteClass: "danger", title: `~${over}% over limit at reset`, tick };
    }
    return { cls: "low", note: "🔥", noteClass: "danger", title: "~100% used at reset", tick };
  }

  const spare = Math.max(1, Math.round(100 - projected));
  if (projected >= 90) {
    return {
      cls: "warn",
      note: `~${spare}% spare`,
      noteClass: "warn",
      title: `~${Math.round(projected)}% used at reset`,
      tick,
    };
  }
  return {
    cls: "",
    note: config.pacingAlways ? `~${spare}% left at reset` : "",
    noteClass: "",
    title: `~${spare}% left at reset`,
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
    const headline = config.showUsed ? `${Math.round(used)}% used` : `${left}% left`;
    const headlineAlt = config.showUsed ? `${left}% left` : `${Math.round(used)}% used`;

    let resetHtml = "";
    if (m.resets_at !== null && m.resets_at > Date.now()) {
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
        resetHtml = `<span title="Sessions start after you send your first message.">Not started</span>`;
      } else {
        const remain = m.resets_at - Date.now();
        const countdown = remain < 60_000 ? "Resets soon" : `Resets in ${fmtDuration(remain)}`;
        const exact = `Resets ${fmtExact(m.resets_at)}`;
        const [text, alt] = config.resetExact ? [exact, countdown] : [countdown, exact];
        resetHtml = `<span class="clickable" data-flip="reset" title="${escapeHtml(alt)}">${escapeHtml(text)}</span>`;
      }
    }
    const detailHtml = [m.detail ? escapeHtml(m.detail) : "", resetHtml].filter(Boolean).join(" · ");
    return `
      <div class="metric">
        <div class="metric-head">
          <span class="metric-label">${escapeHtml(m.label)}</span>
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
  // Actionable row (Codex reset credits): exact expiry + a Use button that
  // spends the credit after a confirm. A credit dying within 24h gets an
  // amber dot so it isn't wasted.
  if (m.kind === "action" && m.detail) {
    const expiry =
      m.resets_at !== null
        ? `Expires ${fmtExact(m.resets_at)}`
        : (m.value ?? "Available");
    const soon =
      m.resets_at !== null && m.resets_at - Date.now() < 86_400_000
        ? `<span class="warn-dot" title="${escapeHtml(`This credit expires in ${fmtDuration(Math.max(0, m.resets_at - Date.now()))} — use it or lose it.`)}">●</span> `
        : "";
    return `
      <div class="metric-text action-row">
        <span>${soon}${escapeHtml(m.label)}</span>
        <span class="action-right">
          <span class="detail">${escapeHtml(expiry)}</span>
          <button class="redeem-btn" data-redeem="${escapeHtml(m.detail)}" title="Spend this credit to reset your Codex rate limits now">Use</button>
        </span>
      </div>`;
  }
  return `
    <div class="metric-text">
      <span>${escapeHtml(m.label)}</span>
      <span class="detail">${escapeHtml(m.value ?? "")}</span>
    </div>`;
}

function renderTrend(spend: ProviderSpend): string {
  if (!spend.trend.some((v) => v > 0)) return "";
  const max = Math.max(...spend.trend);
  const peakIdx = spend.trend.indexOf(max);
  const dayMs = 86_400_000;
  const dateOf = (i: number) =>
    new Date(Date.now() - (29 - i) * dayMs).toLocaleDateString([], { month: "short", day: "numeric" });
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
  const title = `Last 30 days (${dateOf(0)} – ${dateOf(29)}) · peak ${fmtTokens(max)} tokens on ${dateOf(peakIdx)} · from local logs`;
  return `
    <div class="metric trend">
      <span class="metric-label" title="${escapeHtml(title)}">Usage Trend</span>
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
  const est = providerId === "cursor" ? " · estimated" : "";
  const text =
    w.tokens > 0 || w.cost > 0.005
      ? `${fmtMoney(w.cost)} · ${fmtTokens(w.tokens)} tokens${est}`
      : "No data";
  const warn = key === "last30" ? unpricedWarn(sp) : "";
  return `
    <div class="metric-text spend-row" data-spend="${providerId}|${key}">
      <span>${label} ${warn}</span>
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
        <button class="card-caret" data-caret="${s.id}" title="${L.expanded ? "Show less" : "Show more"}">${L.expanded ? "⌃" : "⌄"}</button>
        ${L.expanded ? `<div class="on-demand${anim}">${onDemandHtml}</div>` : ""}`;
    }
  } else {
    body = `<p class="placeholder">${escapeHtml(s.error ?? "Not connected")}</p>`;
  }

  const stale = s.stale
    ? `<span class="stale" title="${escapeHtml(staleHelp(s))}">⚠ Outdated</span>`
    : "";
  const links = (PROVIDER_LINKS[s.id] ?? PROVIDER_LINKS[providerFamily(s.id)] ?? [])
    .map((l) => `<button class="quick-link" data-link="${escapeHtml(l.url)}">${escapeHtml(l.label)}</button>`)
    .join("<span class='quick-sep'>·</span>");
  const linksRow = links ? `<div class="quick-links">${links}</div>` : "";
  const share =
    s.status === "ok"
      ? `<button class="share-btn" data-share="${s.id}" title="Copy card as image">⧉</button>`
      : "";
  return `
    <article class="provider${muted}" data-provider="${s.id}">
      <div class="provider-head">
        <span class="drag-grip" title="Drag to reorder">⠿</span>
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
  return lastSnapshots.filter((s) => !config.disabled.includes(s.id)).sort((a, b) => {
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
    .filter((s) => !config.disabled.includes(s.id)) // disabled = gone everywhere
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
      name: "Others",
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
    const t = entries.reduce((s, e) => s + e.w.tokens, 0);
    return { primary: fmtTokens(t), sub: "tokens", exact: `${fmtTokens(t)} tokens` };
  }
  const c = entries.reduce((s, e) => s + e.w.cost, 0);
  return { primary: fmtMoney(c), sub: "dollars", exact: `$${c.toFixed(2)}` };
}

/// The metric a click (or right-click, reversed) moves to next — the Mac
/// menu's order: Cost, Cost/MTok, Tokens.
function nextSpendMetric(back: boolean): "cost" | "tokens" | "mtok" {
  const order: ("cost" | "tokens" | "mtok")[] = ["cost", "mtok", "tokens"];
  const i = order.indexOf(config.spendMetric);
  return order[(i + (back ? order.length - 1 : 1)) % order.length];
}

const METRIC_NAMES = { cost: "dollars", mtok: "cost per MTok", tokens: "tokens" } as const;

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
    `Under $${e.foldLimit ?? 1} each:\n` +
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
    wrap.title = `${center.exact} — computed locally from session logs. Click to show ${METRIC_NAMES[nextSpendMetric(false)]}.`;
  }
}

function renderTotalSpend(): string {
  if (!config.showTotalSpend) return "";
  const entries = donutEntries(spendTab);
  if (lastSpend.length === 0) {
    // Quiet state instead of a missing card — on a fresh PC the donut only
    // appears after a CLI (Claude Code, Codex, Grok…) has logged some usage.
    const note = spendLoaded
      ? "No spend data yet — appears once Claude Code, Codex, or another CLI logs some usage on this PC."
      : "Scanning session logs…";
    return `
      <article class="provider total-spend">
        <div class="provider-head">
          <span class="provider-name">Total Spend</span>
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
  const exact = `${center.exact} — computed locally from session logs. Click to show ${METRIC_NAMES[nextSpendMetric(false)]}.`;
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
      <div class="donut-wrap donut-empty" title="No spend recorded in this period.">
        <svg width="96" height="96" viewBox="0 0 96 96">
          <path class="seg donut-zero" data-full="1" fill-rule="evenodd" d="${sectorPath(0, TAU)}"/>
          <text class="donut-total" x="48" y="50" text-anchor="middle" font-size="14" font-weight="600">${center.primary}</text>
          <text class="donut-sub" x="48" y="62" text-anchor="middle" font-size="8">${center.sub}</text>
        </svg>
        <div class="legend"><p class="placeholder" style="margin:0">No spend in this period.</p></div>
      </div>`;

  const contributors = lastSpend.map((s) => s.name).join(", ");
  return `
    <article class="provider total-spend">
      <div class="provider-head">
        <span class="provider-name">Total Spend</span>
        <span class="info" title="Fed by: ${escapeHtml(contributors)}. All figures are local estimates from each tool's own logs.">&#9432;</span>
        <span class="spacer"></span>
        <button class="share-btn" data-share="__total__" title="Copy card as image">⧉</button>
      </div>
      <div class="card-panel">
        <div class="tabs">
          ${tab("today", "Today")}${tab("yesterday", "Yesterday")}${tab("last30", "30 Days")}
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
    const btn = document.createElement("button");
    btn.id = "update-btn";
    btn.textContent = `⬆ Update to v${updateVersion}`;
    btn.addEventListener("click", () => {
      btn.textContent = "Installing…";
      btn.disabled = true;
      // On success the app restarts, so only the failure path matters:
      // re-enable the button and surface the reason.
      invoke("install_update").catch((err) => {
        btn.textContent = `⬆ Update to v${updateVersion} — retry`;
        btn.disabled = false;
        const status = document.querySelector("#status");
        if (status) status.textContent = `Update failed: ${err}`;
      });
    });
    el.replaceChildren(btn);
  } else {
    el.textContent = checkingUpdate ? "Checking for updates…" : buildText;
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
          <button id="confirm-cancel" type="button">Cancel</button>
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
        <button id="whatsnew-ok" type="button">Got it</button>
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
      `<div id="snap-foot"><img src="${paneIcon}" alt="" /><span>Monitor Your AI Subscriptions with Pane</span></div>` +
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
    status.textContent = "Copied to clipboard";
  } catch (err) {
    status.textContent = `Share failed: ${err}`;
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
  void updateTrayStrip();
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
    btn.title = mode === "light" ? "Switch to dark mode" : "Switch to light mode";
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
  if (doc.startViewTransition) {
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
  // Account-scoped cards (claude@<hash>) can't reach the tray strip — the
  // Rust side validates ids against a fixed allowlist — so offering the ★
  // there would be a silent no-op. Default family cards only.
  if (s && s.id.includes("@")) return false;
  return s?.metrics.some((m) => m.label === key && m.kind === "progress") ?? false;
}

// Providers start collapsed in Customize; only what you're editing unfolds.
// Session-only — collapsing again on reopen keeps the list scannable.
const custExpanded = new Set<string>();

function renderCustomize(): string {
  const order = config.layout?.providerOrder ?? ALL_PROVIDERS.map(([id]) => id);
  const blocks = order
    .map((id) => {
      const snapshot = lastSnapshots.find((s) => s.id === id);
      // A retired account card (its login left this machine) keeps its
      // layout for reattachment but must not haunt Customize as a bare
      // "claude@ab12cd34" block with nothing under it. A card the USER
      // disabled also has no snapshot (disabled providers are never
      // fetched) — that one must keep rendering, or its re-enable toggle
      // vanishes with it and the account is stuck off forever.
      if (id.includes("@") && !snapshot && !config.disabled.includes(id)) return "";
      // Dynamic account cards carry their name in the snapshot
      // ("Claude — Org"); static providers come from the fixed list.
      const name = ALL_PROVIDERS.find(([pid]) => pid === id)?.[1] ?? snapshot?.name ?? id;
      const L = providerLayout(id);
      const enabled = !config.disabled.includes(id);

      const row = (key: string) => {
        const starrable = isStarrable(snapshot, key);
        const starred = L.starred.includes(key);
        const visible = !L.hidden.includes(key);
        return `
          <div class="cust-row" draggable="true" data-cust-row="${id}|${escapeHtml(key)}">
            <span class="grip" title="Drag to reorder">⠿</span>
            <label class="toggle mini"><input type="checkbox" data-visible="${id}|${escapeHtml(key)}"${visible ? " checked" : ""} /></label>
            <span class="cust-label">${escapeHtml(key)}</span>
            ${starrable ? `<button class="star${starred ? " on" : ""}" data-star="${id}|${escapeHtml(key)}" title="Star for tray strip (max 2)">★</button>` : ""}
          </div>`;
      };

      const always = L.metricOrder.filter((k) => !L.onDemand.includes(k));
      const onDemand = L.metricOrder.filter((k) => L.onDemand.includes(k));
      const rows = L.metricOrder.length
        ? `${always.map(row).join("")}
           <div class="cust-divider" data-divider="${id}">On Demand — behind the card's caret</div>
           ${onDemand.map(row).join("")}`
        : `<p class="placeholder">No data yet — refresh with this provider enabled first.</p>`;

      const open = custExpanded.has(id);
      return `
        <article class="provider customize-block${enabled ? "" : " muted"}${open ? " open" : ""}" data-cust-provider="${id}" draggable="true">
          <div class="provider-head">
            <span class="grip" title="Drag to reorder providers">⠿</span>
            <button class="cust-expand" data-cust-expand="${id}" title="${open ? "Collapse" : "Expand"}">
              <span class="provider-name">${escapeHtml(name)}</span>
              <span class="chev">⌄</span>
            </button>
            <span class="spacer"></span>
            <button class="mini-btn" data-reset="${id}" title="Restore this card's default layout — does not touch your usage limits">Reset layout</button>
            <label class="toggle mini" title="Enable provider"><input type="checkbox" data-enable="${id}"${enabled ? " checked" : ""} /></label>
          </div>
          <div class="acc-body"><div class="acc-inner cust-rows">${rows}</div></div>
        </article>`;
    })
    .join("");

  const starCount = Object.values(config.layout?.providers ?? {}).reduce((n, l) => n + l.starred.length, 0);
  return `
    <div class="customize-bar glass-bar">
      <button class="dock-btn" data-customize-close>← Done</button>
      <span class="detail">${starCount} starred · drag ⠿ to reorder</span>
      <button class="dock-btn danger" data-reset-all title="Restore all cards' default layouts — does not touch your usage limits">↺ Reset all</button>
    </div>
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
        <span class="provider-name">Welcome 👋</span>
        <span class="spacer"></span>
        <button class="share-btn welcome-close" data-welcome-close title="Dismiss">✕</button>
      </div>
      <p class="placeholder" style="margin:2px 0 8px">
        You're set up with the AI tools found on this PC. Arrange cards, star
        tray metrics, and hide rows in Customize.
      </p>
      <button class="mini-btn" data-welcome-customize>Open Customize</button>
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
  if (body) body.innerHTML = renderCustomize();
}

/// Customize lives in a drawer that slides in from the left edge.
function setDrawer(open: boolean): void {
  customizeOpen = open;
  if (open) renderDrawerBody();
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
      return `<button class="trail-tick" data-trail="${i}" title="${escapeHtml(name)}"></button>`;
    })
    .join("");
  // Minimap feel: tick width follows the card's height, like Codex's rail.
  const ticks = trail.querySelectorAll<HTMLElement>(".trail-tick");
  ticks.forEach((tick, i) => {
    const h = cards[i]?.offsetHeight ?? 80;
    tick.style.width = `${Math.max(7, Math.min(16, Math.round(5 + h / 45)))}px`;
  });
  updateTrailActive();
}

/// Codex-style magnetic rail: ticks near the cursor stretch and brighten
/// with a smooth falloff; everything settles back when the mouse leaves.
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
  const date = new Date(Date.now() - (29 - i) * 86_400_000).toLocaleDateString([], {
    weekday: "short",
    month: "short",
    day: "numeric",
  });
  tip.innerHTML = `
    <div class="tip-line"><span class="tip-name">${escapeHtml(date)}</span><span>${
      tokens > 0 ? `${fmtTokens(tokens)} tokens` : "No usage"
    }</span></div>
    ${tokens > 0 ? `<div class="tip-line detail"><span>${share < 1 ? "<1" : share.toFixed(0)}% of the last 30 days</span></div>` : ""}`;

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
    tip.innerHTML = `<p class="placeholder">No model data for this period.</p>`;
  } else {
    tip.innerHTML = w.models
      .map((m) => {
        const share = w.cost > 0 ? (m.cost / w.cost) * 100 : 0;
        return `
          <div class="tip-model">
            <div class="tip-line"><span class="tip-name">${escapeHtml(m.model)}</span><span>${fmtMoney(m.cost)}</span></div>
            <div class="tip-line detail"><span>${share.toFixed(0)}%</span><span>${fmtTokens(m.tokens)} tokens</span></div>
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

async function refresh(force = false): Promise<void> {
  if (refreshing) return;
  if (!force && Date.now() - lastFetch < STALE_MS) return;
  refreshing = true;

  const status = document.querySelector("#status")!;
  status.textContent = "Refreshing…";
  // The spend scan re-reads every session log on a cold start and can take
  // tens of seconds — it must never hold up the usage cards' first paint.
  const spendPromise = invoke<ProviderSpend[]>("fetch_spend").catch(() => null);
  try {
    let snapshots = await invoke<Snapshot[]>("fetch_usage");
    // First launch ever (no layout yet): start with only the providers that
    // actually have credentials on this PC, like the Mac app's first-run
    // detection. The rest stay available in Customize.
    if (config.layout === null && snapshots.length > 0) {
      // Claude and Codex always start enabled — their "connect me" cards are
      // the new-user onboarding. Everything else without credentials waits
      // in Customize (a fresh PC with zero AI tools sees just those two).
      const starters = new Set(["claude", "codex"]);
      const noCreds = snapshots
        .filter((s) => s.status === "no_credentials" && !starters.has(s.id))
        .map((s) => s.id);
      if (noCreds.length) {
        snapshots = snapshots.filter((s) => !noCreds.includes(s.id));
        await patchConfig({ disabled: noCreds }).catch(() => {});
      }
    } else if (config.layout) {
      // App updates ship new providers; ones this PC has no credentials for
      // start disabled instead of piling up dead cards. Seen once (a layout
      // entry marks that), so enabling one in Customize sticks.
      const known = config.layout.providers;
      const fresh = snapshots
        .filter(
          (s) => s.status === "no_credentials" && !(s.id in known) && !config.disabled.includes(s.id)
        )
        .map((s) => s.id);
      if (fresh.length) {
        for (const id of fresh) known[id] = providerLayout(id);
        await patchConfig({
          disabled: [...config.disabled, ...fresh],
          layout: config.layout,
        }).catch(() => {});
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
    const firstData = lastSnapshots.length === 0;
    lastFetch = Date.now();
    lastSnapshots = snapshots;
    ensureLayout();
    if (!lastLayoutSnapshot && config.layout) {
      lastLayoutSnapshot = JSON.stringify(config.layout);
    }
    renderIfVisible();
    if (firstData && !customizeOpen && !document.hidden) playReveal();
    void updateTrayStrip();
    const time = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    status.textContent = `Updated ${time}`;
  } catch (err) {
    status.textContent = `Refresh failed: ${err}`;
  }
  const spend = await spendPromise;
  spendLoaded = true;
  if (spend) lastSpend = spend;
  if (!customizeOpen && lastSnapshots.length) renderIfVisible();
  refreshing = false;
}

function scheduleAutoRefresh(): void {
  if (refreshTimer !== undefined) clearInterval(refreshTimer);
  const minutes = Math.max(1, config.refreshMinutes || 5);
  refreshTimer = setInterval(() => void refresh(), minutes * 60 * 1000);
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

/// The tray strip now follows the stars: any provider with starred metrics
/// gets a [logo][numbers] pair, in customize order (max 4 providers).
async function updateTrayStrip(): Promise<void> {
  const entries = [];
  const order = config.layout?.providerOrder ?? [];
  for (const id of order) {
    if (entries.length >= 4) break;
    if (config.disabled.includes(id)) continue; // no tray icons for disabled providers
    const L = providerLayout(id);
    if (!L.starred.length) continue;
    const snap = lastSnapshots.find((s) => s.id === id && s.status === "ok");
    if (!snap) continue;
    const starredMetrics = L.starred
      .map((label) => snap.metrics.find((m) => m.label === label && m.kind === "progress"))
      .filter((m): m is Metric => Boolean(m))
      .slice(0, 2);
    if (!starredMetrics.length) continue;
    const logo = await rasterizeLogo(id);
    if (!logo) continue;
    const values = starredMetrics.map((m) => Math.round(100 - clampPercent(m.used_percent ?? 0)));
    const tooltip = `${snap.name}\n${starredMetrics
      .map((m) => `${m.label}: ${Math.round(100 - clampPercent(m.used_percent ?? 0))}% left`)
      .join("\n")}`;
    entries.push({ id, logo, values, tooltip });
  }
  try {
    await invoke("update_tray_strip", { entries });
  } catch {
    // Tray strip is cosmetic — never let it break a refresh.
  }
}

// ---------------------------------------------------------------------------
// Customize interactions
// ---------------------------------------------------------------------------

interface DragPayload {
  t: "row" | "provider";
  id: string;
  key?: string;
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
  const closeBtn = target.closest("[data-customize-close]");
  if (closeBtn) {
    setDrawer(false);
    return true;
  }
  const resetAll = target.closest("[data-reset-all]");
  if (resetAll) {
    void appConfirm({
      title: "Reset all layouts?",
      message:
        "Order, stars, and hidden rows go back to defaults, and installed AI tools are re-detected. Your usage limits are not affected.",
      confirmLabel: "Reset all",
      danger: true,
    }).then((ok) => {
      if (!ok) return;
      // Clearing layout + disabled re-arms the first-launch detection path:
      // the next refresh probes every provider and re-disables only the
      // ones with no credentials on this PC.
      config.layout = null;
      config.disabled = [];
      void patchConfig({ layout: null, disabled: [] }).then(() => {
        setDrawer(false);
        void refresh(true).then(() => void updateTrayStrip());
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
    void updateTrayStrip();
    return true;
  }
  const star = target.closest<HTMLElement>("[data-star]");
  if (star) {
    const [id, key] = star.dataset.star!.split("|");
    const L = providerLayout(id);
    if (L.starred.includes(key)) {
      L.starred = L.starred.filter((k) => k !== key);
    } else if (L.starred.length >= 2) {
      document.querySelector("#status")!.textContent = "Up to 2 stars per provider";
      return true;
    } else {
      L.starred.push(key);
    }
    saveLayout();
    renderAll();
    void updateTrayStrip();
    return true;
  }
  return false;
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
  return [...s];
}

function handleCustomizeChange(target: HTMLInputElement): void {
  if (target.dataset.enable !== undefined) {
    const id = target.dataset.enable;
    const enable = target.checked;
    pendingToggles.push({ id, enable });
    config.disabled = withPendingToggles(config.disabled); // optimistic
    renderAll(); // disabled cards vanish from the dashboard immediately
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
      // patchConfig replaced config with the server echo; merge any newer
      // still-pending toggles back on top of it.
      config.disabled = withPendingToggles(config.disabled);
      // Only an enable needs fresh data; a disable is purely local.
      if (enable) await refresh(true).catch(() => {});
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
      dragPayload = { t: "row", id, key };
      setDragGhost(e as DragEvent, row);
      e.stopPropagation();
      return;
    }
    const block = (e.target as HTMLElement).closest<HTMLElement>("[data-cust-provider]");
    if (block) {
      dragPayload = { t: "provider", id: block.dataset.custProvider! };
      setDragGhost(e as DragEvent, block);
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

    if (dragPayload.t === "row") {
      const L = providerLayout(dragPayload.id);
      const divider = target.closest<HTMLElement>("[data-divider]");
      const row = target.closest<HTMLElement>("[data-cust-row]");
      if (divider && divider.dataset.divider === dragPayload.id) {
        moveRow(L, dragPayload.key!, DIVIDER);
      } else if (row) {
        const [tid, tkey] = row.dataset.custRow!.split("|");
        if (tid === dragPayload.id && tkey !== dragPayload.key) moveRow(L, dragPayload.key!, tkey);
      }
      saveLayout();
      renderAll();
    } else if (config.layout) {
      const block = target.closest<HTMLElement>("[data-cust-provider]");
      if (block && block.dataset.custProvider !== dragPayload.id) {
        const order = config.layout.providerOrder.filter((p) => p !== dragPayload!.id);
        const at = order.indexOf(block.dataset.custProvider!);
        order.splice(at < 0 ? order.length : at, 0, dragPayload.id);
        config.layout.providerOrder = order;
        saveLayout();
        renderAll();
        void updateTrayStrip();
      }
    }
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

async function saveApiKey(provider: string): Promise<void> {
  const input = document.querySelector<HTMLInputElement>(`#key-${provider}`)!;
  const status = document.querySelector("#status")!;
  try {
    await invoke("set_api_key", { provider, key: input.value });
    input.value = "";
    status.textContent = `${provider} key saved`;
    await refresh(true);
  } catch (err) {
    status.textContent = `Could not save key: ${err}`;
  }
}

function populatePinnedOptions(): void {
  const select = document.querySelector<HTMLSelectElement>("#pinned")!;
  const current = config.pinned ? `${config.pinned.provider}::${config.pinned.label}` : "";
  select.replaceChildren(new Option("Auto (first live metric)", ""));
  for (const s of lastSnapshots) {
    if (s.status !== "ok") continue;
    for (const m of s.metrics) {
      if (m.kind !== "progress") continue;
      const value = `${s.id}::${m.label}`;
      select.add(new Option(`${s.name} — ${m.label}`, value, false, value === current));
    }
  }
}

async function initSettings(): Promise<void> {
  config = await invoke<Config>("get_config");
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
      document.querySelector("#status")!.textContent = `Autostart failed: ${err}`;
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
    void patchConfig({ pinned: value }).then(() => refresh(true));
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

  const shortcut = document.querySelector<HTMLInputElement>("#shortcut")!;
  shortcut.value = config.shortcut;
  shortcut.addEventListener("change", async () => {
    const status = document.querySelector("#status")!;
    try {
      await invoke("set_shortcut", { shortcut: shortcut.value });
      await patchConfig({ shortcut: shortcut.value });
      status.textContent = shortcut.value.trim() ? "Shortcut saved" : "Shortcut cleared";
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
        document.querySelector("#status")!.textContent = "Proxy saved — takes effect after restart";
      },
    );
  };
  proxyEnabled.addEventListener("change", saveProxy);
  proxyUrl.addEventListener("change", saveProxy);

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
  };
  document.querySelector("#settings-btn")!.addEventListener("click", () => {
    setDrawer(false);
    setSettings(!document.body.classList.contains("settings-open"));
  });
  document.querySelector("#settings-close")!.addEventListener("click", () => setSettings(false));
  document.querySelector("#changelog-btn")!.addEventListener("click", () => {
    setSettings(false);
    showChangelogDialog("Changelog", parseChangelog());
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
        document.querySelector("#status")!.textContent = `Could not open link: ${err}`;
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
        title: "Use a reset credit?",
        message:
          "This resets your Codex rate-limit windows immediately and cannot be undone. The refreshed windows can take a couple of minutes to appear.",
        confirmLabel: "Use credit",
      }).then((ok) => {
        if (!ok) return;
        const status = document.querySelector("#status")!;
        status.textContent = "Redeeming reset credit…";
        void invoke<string>("codex_redeem_credit", { creditId, providerId })
          .then((msg) => {
            status.textContent = msg;
            void refresh(true);
          })
          .catch((err) => {
            status.textContent = `Redeem failed: ${err}`;
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
      saveLayout();
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
    card?.scrollIntoView({ behavior: "smooth", block: "start" });
  });

  // The 4-hourly background checker feeds the same footer button.
  void listen<string>("update-available", (e) => {
    updateVersion = e.payload;
    renderBuildInfo();
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
      showChangelogDialog(`What's new in v${appVersion}`, pendingWhatsNew);
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
    void refresh();
  });
  void initSettings().then(() => {
    scheduleAutoRefresh();
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
