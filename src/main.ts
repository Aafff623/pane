import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  providerCatalog,
  providerCategory,
  providerDefinition,
  providerFamily,
  supportsApiKey,
  supportsExtraAccounts,
} from "./providerCatalog";
import { PEAK_RULES, isProviderInPeak, type PeakRule } from "./peakHours";
import { providerVisual } from "./providerVisuals";
import { MECHANISMS } from "./providerMechanisms";
import { uiIcon, type UiIconName } from "./uiIcons";
import {
  applyStaticI18n,
  displayLinkLabel,
  displayMetricDetail,
  displayMetricLabel,
  detectSystemLocale,
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

// Dual-form: the "panel" window loads this same bundle but presents the
// settings large panel; the tray popover ("main") keeps the floating
// dashboard. The label comes from local window metadata — no IPC, so no
// capability permission is needed. Outside Tauri (plain vite in a browser)
// the metadata is absent: fall back to "main".
const WINDOW_LABEL = (() => {
  try {
    return getCurrentWebviewWindow().label;
  } catch {
    return "main";
  }
})();
const IS_PANEL_FORM = WINDOW_LABEL === "panel";
if (IS_PANEL_FORM) document.body.classList.add("panel-form");

// Inlined as data URIs (not URLs) so the share-card SVG snapshot can
// embed them — rasterized SVG images can't load external resources.
// The bare ring suits the sidebar; the footer uses the full rounded
// app icon, which stays legible at tiny sizes.
import { gsap } from "gsap";
import paneLogo from "./assets/pane-logo.png?inline";
import paneIcon from "./assets/pane-icon.png?inline";
import auroraWallpaper from "./assets/skins/aurora.webp";
// The repo's changelog ships inside the bundle, so the "What's new" dialog
// and the Settings changelog viewer read the exact file releases maintain.
import changelogRaw from "../CHANGELOG.md?raw";

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

interface MetricTokenUsage {
  tokens: number;
  source: "provider" | "ledger";
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
  trend_cost: number[];
  unpriced: number;
  unpriced_models: string[];
  estimated?: boolean;
}

interface HistorySpend {
  id: string;
  cost: number;
  tokens: number;
  active_days: number;
  models: ModelSpend[];
}

interface DailyModelSpend {
  model: string;
  cost: number;
  tokens: number;
}

interface DailySpendRow {
  day: string;
  id: string;
  cost: number;
  tokens: number;
  models: DailyModelSpend[];
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
  qodercn: "stale.relogin.qodercn",
  traecn: "stale.relogin.traecn",
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

/// Small tag for rows whose tokens are a credits-derived blend (Qoder CN),
/// not measured facts.
function estBadgeFor(id: string): string {
  const sp = lastSpend.find((s) => s.id === id);
  if (!sp?.estimated) return "";
  return `<span class="est-badge" title="${escapeHtml(t("spend.estimateTip"))}">${escapeHtml(t("spend.estimate"))}</span>`;
}
type RangeTab = "d7" | "d30" | "all";
const OVERVIEW_TABS = ["5h", "week", "month"] as const;
type OverviewTab = (typeof OVERVIEW_TABS)[number];
const OVERVIEW_CATEGORIES = ["coding", "productivity", "mcp"] as const;
type OverviewCategory = (typeof OVERVIEW_CATEGORIES)[number];

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
  // Card fold: the family owns the decision (every account maxed → auto),
  // and the user can override the family default. undefined = follow the
  // family auto state. Stored at the FAMILY id so all sibling cards in a
  // multi-account family share one fold state.
  collapsed?: boolean;
  // Which card group this card belongs to (CardGroup.id). Empty/missing =
  // ungrouped. Account cards follow their family (the group lives on the
  // family's layout entry only).
  group?: string;
  // User note replacing the card's display name everywhere it renders
  // (card head, overview tiles, trail hover). Empty/missing = original
  // catalog name. Stored per CARD id, so parallel-account cards each keep
  // their own note.
  note?: string;
  // Account selected as the provider's explicit pinned seat.
  pinnedAccount?: string;
}

interface Layout {
  providerOrder: string[];
  providers: Record<string, ProviderLayout>;
  overviewCollapsed?: boolean;
  /// Total-spend card folded: mirrors overviewCollapsed, remembered so a
  /// restart reopens the card exactly where the user left it.
  spendCollapsed?: boolean;
  // Card groups: user-named buckets a card can be tagged with ("常用",
  // "不常用", …). Cards without a tag render ungrouped (always visible);
  // groups with zero cards vanish from the UI until reused. Stored here
  // so groups ride the same save/restore/migration path as card order.
  groups?: CardGroup[];
}

interface CardGroup {
  id: string;
  name: string;
  collapsed?: boolean;
}

interface Config {
  refreshMinutes: number;
  disabled: string[];
  pinned: { provider: string; label: string } | null;
  trayProviders: string[];
  telemetry: boolean;
  notifyAlmostOut: boolean;
  notifyCuttingClose: boolean;
  notifyWillRunOut: boolean;
  notifyResetSoon: boolean;
  spendTab: SpendTab;
  overviewTab: OverviewTab;
  overviewCategory: OverviewCategory;
  overviewStyle: "rings" | "bars";
  categoryOverrides: Record<string, OverviewCategory>;
  spendMetric: "cost" | "tokens" | "mtok";
  spendGrouping: "tool" | "model";
  showUsed: boolean;
  showTrend: boolean;
  resetExact: boolean;
  timeFormat: "auto" | "12" | "24";
  layout: Layout | null;
  appearance: "system" | "light" | "dark";
  density: "regular" | "compact";
  uiFont: string;
  glassEffects: boolean;
  shortcut: string;
  categoryShortcut: string;
  localShortcuts: Record<string, string>;
  proxy: { enabled: boolean; url: string };
  showTotalSpend: boolean;
  welcomeDismissed: boolean;
  lastSeenVersion: string;
  reduceAnimations: boolean;
  jumpAnimation: "smooth" | "instant";
  hideUsageWhileSharing: boolean;
  locale: LocalePref;
  windowForm: "floating" | "panel";
  silentStart: boolean;
  startupAnimation: boolean;
  lastStartupBootId: number | null;
  overviewExpanded: string[];
  experimentalFeatures: boolean;
  spendIconTiers: { medium: number; high: number; max: number } | null;
  overviewCatFull: boolean;
  mainScrollTop: number;
  qoderCheckin: boolean;
  removedProviders: string[];
}

const FRONTEND_CONFIG_KEYS = [
  "refreshMinutes",
  "disabled",
  "pinned",
  "trayProviders",
  "telemetry",
  "notifyAlmostOut",
  "notifyCuttingClose",
  "notifyWillRunOut",
  "notifyResetSoon",
  "spendTab",
  "overviewTab",
  "overviewCategory",
  "overviewStyle",
  "categoryOverrides",
  "spendMetric",
  "spendGrouping",
  "showUsed",
  "showTrend",
  "resetExact",
  "timeFormat",
  "layout",
  "appearance",
  "density",
  "uiFont",
  "glassEffects",
  "shortcut",
  "categoryShortcut",
  "localShortcuts",
  "proxy",
  "showTotalSpend",
  "welcomeDismissed",
  "lastSeenVersion",
  "reduceAnimations",
  "jumpAnimation",
  "hideUsageWhileSharing",
  "locale",
  "windowForm",
  "silentStart",
  "startupAnimation",
  "lastStartupBootId",
  "overviewExpanded",
  "experimentalFeatures",
  "spendIconTiers",
  "overviewCatFull",
  "mainScrollTop",
  "qoderCheckin",
  "removedProviders",
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

const ALL_PROVIDERS: [string, string][] = providerCatalog.map(
  ({ familyId, displayName }) => [familyId, displayName],
);

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
  stepfun: [{ label: "Platform", url: "https://platform.stepfun.com/" }],
  siliconflow: [{ label: "Dashboard", url: "https://cloud.siliconflow.cn/" }],
  novita: [{ label: "Dashboard", url: "https://novita.ai/" }],
  relaybalance: [],
  kimi: [
    { label: "Console", url: "https://www.kimi.com/code/console" },
    { label: "Quota", url: "https://www.kimi.com/membership/subscription?tab=quota" },
    { label: "API", url: "https://platform.moonshot.ai/console" },
  ],
};

/// The "Get API key" page for each key provider, for the gear panel and
/// account dialog. CC-Switch's `apiKeyUrl` per preset — the vendor's own
/// key-management page, never a proxy or a mirror.
const API_KEY_URLS: Record<string, string> = {
  deepseek: "https://platform.deepseek.com/api_keys",
  stepfun: "https://platform.stepfun.com/api-keys",
  siliconflow: "https://cloud.siliconflow.cn/account/ak",
  novita: "https://novita.ai/settings/account#api-key",
  zai: "https://z.ai/manage-apikey/apikey-list",
  minimax: "https://platform.minimax.io/user-center/basic-information/interface-key",
  openrouter: "https://openrouter.ai/settings/keys",
  moonshot: "https://platform.moonshot.ai/console/api-keys",
  aihubmix: "https://console.aihubmix.com/settings",
  qwen: "https://modelstudio.console.alibabacloud.com/ap-southeast-1/?tab=globalset#/efm/coding_plan_apikey",
  elevenlabs: "https://elevenlabs.io/app/settings/keys",
  codebuff: "https://www.codebuff.com/profile",
  kilo: "https://app.kilo.ai/settings/keys",
  opencode: "https://opencode.ai/console/keys",
};

function getApiKeyLink(family: string): string | undefined {
  return API_KEY_URLS[providerFamily(family)];
}

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
  qodercn: "#7c5cfc", // Qoder brand violet
  traecn: "#4d6bfe", // Trae brand blue
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
  telemetry: true,
  notifyAlmostOut: false,
  notifyCuttingClose: false,
  notifyWillRunOut: false,
  notifyResetSoon: false,
  spendTab: "today",
  overviewTab: "5h",
  overviewCategory: "coding",
  overviewStyle: "rings",
  categoryOverrides: {},
  spendMetric: "cost",
  spendGrouping: "tool",
  showUsed: false,
  showTrend: true,
  resetExact: false,
  timeFormat: "auto",
  layout: null,
  appearance: "system",
  density: "regular",
  uiFont: "",
  glassEffects: true,
  shortcut: "",
  categoryShortcut: "Shift+1",
  localShortcuts: {},
  proxy: { enabled: false, url: "" },
  showTotalSpend: true,
  welcomeDismissed: false,
  lastSeenVersion: "",
  reduceAnimations: false,
  jumpAnimation: "smooth",
  hideUsageWhileSharing: false,
  locale: "auto",
  windowForm: "floating",
  silentStart: false,
  startupAnimation: true,
  lastStartupBootId: null,
  overviewExpanded: [],
  experimentalFeatures: false,
  spendIconTiers: null,
  overviewCatFull: false,
  mainScrollTop: 0,
  qoderCheckin: false,
  removedProviders: [],
};
let pendingJumpAnimation: "smooth" | "instant" | null = null;
let settingsDirty = false;

function setSettingsDirty(dirty: boolean): void {
  settingsDirty = dirty;
  document.querySelectorAll<HTMLButtonElement>(".settings-save-all").forEach((button) => {
    button.disabled = !dirty;
  });
}

async function applySettingsAndReload(): Promise<void> {
  if (!settingsDirty) return;
  if (pendingJumpAnimation && pendingJumpAnimation !== config.jumpAnimation) {
    await patchConfig({ jumpAnimation: pendingJumpAnimation });
    pendingJumpAnimation = null;
  }
  setSettingsDirty(false);
  window.location.reload();
}
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
let lastSpendHistory: { d7: HistorySpend[]; all: HistorySpend[] } = { d7: [], all: [] };
let lastSpendDaily: DailySpendRow[] = [];
let spendLoaded = false;
let spendDetailOpen = false;
let spendDetailDay = "";
/// Sampled quota history per card id (backend usage_history.json). Cards
/// with local CLI logs trend from spend; every other card falls back to
/// these daily "worst used percent" samples.
let lastQuotaTrend: Record<string, (number | null)[]> = {};
let lastCreditTrend: Record<string, (number | null)[]> = {};

/// Tracks manual account-tab selections made by the user in the current view.
/// Cleared on popover reopening or account mutations.
const userSelectedAccountFor = new Map<string, string>();

/// Determines which account snapshot to display on the provider's home card.
/// When a multi-account provider's default account is exhausted / maxed out (red dot),
/// the card automatically prioritizes displaying an account with available quota (green dot).
/// Failed, exhausted and stale accounts yield to a healthy sibling. This routes
/// dashboard display without reordering saved credentials or changing other tools.
function resolveDisplayedAccount(family: string, defaultId: string, accountIds: string[]): string {
  // Automatic quota preference applies only until the user chooses a tab.
  // Exhausted or failed accounts must remain inspectable.
  const manual = userSelectedAccountFor.get(family) ?? providerLayout(family).pinnedAccount;
  if (manual && accountIds.includes(manual)) return manual;
  const available = accountIds.filter((id) => {
    const snap = lastSnapshots.find((s) => s.id === id);
    return snap?.status === "ok" && !isSnapshotMaxed(snap) && accountHealthDot(id) === "green";
  });
  // Fresh successful queries win over cached readings whose latest query failed.
  const fresh = available.filter((id) => !lastSnapshots.find((s) => s.id === id)?.stale);
  const candidates = fresh.length ? fresh : available;
  if (candidates.includes(defaultId)) return defaultId;
  if (candidates.length) return candidates[0];
  // All accounts blocked, but a sibling that is ok-and-maxed with a known
  // upcoming reset means capacity is coming back — show the soonest one's
  // countdown ("awaiting refresh") instead of a dead default's re-login
  // error. Per-account resets only: @-cards scan their own metrics here,
  // and the status gate keeps an errored bare card out of the set.
  const recovering = accountIds
    .filter((id) => {
      const snap = lastSnapshots.find((s) => s.id === id);
      return snap?.status === "ok" && isSnapshotMaxed(snap) && nearestResetSeconds(id) > 0;
    })
    .sort((a, b) => nearestResetSeconds(a) - nearestResetSeconds(b));
  if (recovering.length) {
    return recovering[0];
  }
  return accountIds.includes(defaultId) ? defaultId : accountIds[0] ?? defaultId;
}

type TrendSource = { id: string; trend: (number | null)[]; quota: boolean; credits?: boolean; fmt: (v: number) => string };

/// Credit counts are small whole numbers (38 of 2000), so no k/M suffix.
const fmtCredits = (v: number): string => Math.round(v).toLocaleString(localeTag());

/// The trend data for one card: local-log spend when the id has one, else
/// the credits ledger for credits-billed cards (Qoder CN / Trae CN), else
/// the backend's sampled quota history (API-key accounts, relay keys).
function trendSourceFor(id: string): TrendSource | undefined {
  const local = lastSpend.find((sp) => sp.id === id);
  const metricCost = config.spendMetric === "cost";
  if (local) return { id, trend: metricCost ? local.trend_cost : local.trend, quota: false, fmt: metricCost ? fmtMoney : fmtTokens };
  const credits = lastCreditTrend[id];
  if (credits?.some((v) => v != null && v > 0)) return { id, trend: credits, quota: false, credits: true, fmt: fmtCredits };
  const sampled = lastQuotaTrend[id];
  if (sampled?.some((v) => v != null && v > 0)) return { id, trend: sampled, quota: true, fmt: fmtTokens };
  return undefined;
}
let spendTab: SpendTab = "today";
let rangeTab: RangeTab = "d30";
let rangeSelected = false;
let overviewTab: OverviewTab = "5h";
let overviewCategory: OverviewCategory = "coding";
/// Header toggle (right side, beside ⟳): when on, the overview body is a
/// single reset-time-sorted list instead of the 可用/不可用 sections —
/// the "act soon" view. Follows the 5h/week tab like the sections do.
let overviewExpiringOpen = false;
let customizeOpen = false;
let skinMarketOpen = false;
let skinPreviewId: string | null = null;

type SkinId = "aurora" | "paper" | "arcade";
interface SkinDefinition {
  id: SkinId;
  name: string;
  tagline: string;
  colors: [string, string, string];
  wallpaper: string;
  mascot: string;
}

const SKINS: SkinDefinition[] = [
  {
    id: "aurora",
    name: "Aurora Desk",
    tagline: "极光玻璃 · 夜间专注",
    colors: ["#07111f", "#203d59", "#9af2d1"],
    wallpaper: `url(${auroraWallpaper}), radial-gradient(circle at 18% 22%, rgba(126,249,210,.25), transparent 28%), radial-gradient(circle at 82% 8%, rgba(113,150,255,.3), transparent 34%), linear-gradient(145deg,#07111f 0%,#132943 48%,#1e3f4c 100%)`,
    mascot: `<svg viewBox="0 0 120 120"><path d="M22 77c0-31 17-51 39-51s37 20 37 51c0 10-8 18-18 18H40c-10 0-18-8-18-18Z" fill="#b9ffe5"/><path d="M34 55c7-18 17-27 28-27 14 0 25 10 31 29-15-7-37-8-59-2Z" fill="#70d7c0"/><circle cx="47" cy="64" r="5" fill="#102333"/><circle cx="73" cy="64" r="5" fill="#102333"/><path d="M49 78c7 6 15 6 22 0" fill="none" stroke="#102333" stroke-width="4" stroke-linecap="round"/><path d="M57 25l3-13m8 14 8-10" stroke="#b9ffe5" stroke-width="4" stroke-linecap="round"/></svg>`,
  },
  {
    id: "paper",
    name: "Paper Orbit",
    tagline: "纸张轨道 · 清爽工作台",
    colors: ["#f2eadb", "#d6b98a", "#4f6c55"],
    wallpaper: "radial-gradient(circle at 12% 78%, rgba(255,255,255,.65), transparent 24%), repeating-linear-gradient(115deg, rgba(92,72,45,.08) 0 1px, transparent 1px 18px), linear-gradient(135deg,#f2eadb,#ded0b7 54%,#b8c3a8)",
    mascot: `<svg viewBox="0 0 120 120"><path d="m25 81 19-52 43 16-19 52Z" fill="#fff8e9" stroke="#765b3f" stroke-width="3"/><path d="m43 29 10-10 43 16-9 10Z" fill="#d7b37b" stroke="#765b3f" stroke-width="3"/><circle cx="55" cy="55" r="5" fill="#765b3f"/><circle cx="76" cy="63" r="5" fill="#765b3f"/><path d="M54 70c7 5 14 6 21 1" fill="none" stroke="#765b3f" stroke-width="3" stroke-linecap="round"/></svg>`,
  },
  {
    id: "arcade",
    name: "Neon Arcade",
    tagline: "霓虹像素 · 快速唤醒",
    colors: ["#110d22", "#36205f", "#ff7ad9"],
    wallpaper: "linear-gradient(120deg, rgba(255,71,190,.22) 0 2px, transparent 2px 44px), linear-gradient(35deg, rgba(90,220,255,.18) 0 2px, transparent 2px 52px), radial-gradient(circle at 50% 36%, rgba(255,84,199,.24), transparent 34%), #110d22",
    mascot: `<svg viewBox="0 0 120 120"><rect x="22" y="31" width="76" height="62" rx="20" fill="#ff7ad9" stroke="#72e9ff" stroke-width="4"/><circle cx="46" cy="61" r="8" fill="#17102e"/><circle cx="74" cy="61" r="8" fill="#17102e"/><path d="M45 78h30" stroke="#17102e" stroke-width="5" stroke-linecap="round"/><path d="M30 23v-8m60 8v-8" stroke="#72e9ff" stroke-width="4" stroke-linecap="round"/></svg>`,
  },
];

const SKIN_STORAGE_KEY = "pane.skin.id";
function activeSkin(): SkinDefinition | undefined {
  const id = localStorage.getItem(SKIN_STORAGE_KEY) as SkinId | null;
  return SKINS.find((skin) => skin.id === id);
}

function applySkin(skin = activeSkin()): void {
  const root = document.documentElement;
  const mascot = document.querySelector<HTMLElement>("#skin-mascot");
  if (!skin) {
    delete root.dataset.skin;
    root.style.removeProperty("--skin-wallpaper");
    root.style.removeProperty("--skin-accent");
    if (mascot) mascot.innerHTML = "";
    return;
  }
  root.dataset.skin = skin.id;
  root.style.setProperty("--skin-wallpaper", skin.wallpaper);
  root.style.setProperty("--skin-accent", skin.colors[2]);
  if (mascot) mascot.innerHTML = skin.mascot;
}

function selectSkin(id: SkinId | null): void {
  localStorage.setItem(SKIN_STORAGE_KEY, id ?? "");
  applySkin(id ? SKINS.find((skin) => skin.id === id) : undefined);
  document.querySelector("#status")!.textContent = id
    ? `${SKINS.find((skin) => skin.id === id)?.name ?? "Skin"} applied`
    : "Default skin applied";
  renderDrawerBody();
}

/// Experimental-features gate: off = the skin market's entries don't even
/// render. Hot-unplug — turning it off closes the market and drops back to
/// the native skin in one step.
function applyExperimental(): void {
  const on = config.experimentalFeatures === true;
  const skinBtn = document.querySelector<HTMLElement>("#skin-btn");
  if (skinBtn) skinBtn.hidden = !on;
  if (on) return;
  if (skinMarketOpen || skinPreviewId) {
    skinMarketOpen = false;
    skinPreviewId = null;
    if (customizeOpen) renderDrawerBody();
  }
  if (activeSkin()) selectSkin(null);
}
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

// Token counts must come from an actual provider report or the local spend
// ledger. A quota percentage is deliberately never converted into tokens.
function parseTokenAmount(text: string | null | undefined): number | null {
  if (!text || /\b(?:credit|credits|积分)\b/i.test(text)) return null;
  const match = text.match(/([\d,.]+)\s*([kmbt])?\s*tokens?\b/i);
  if (!match) return null;
  const amount = Number(match[1].replace(/,/g, ""));
  if (!Number.isFinite(amount) || amount < 0) return null;
  const multiplier = ({ k: 1e3, m: 1e6, b: 1e9, t: 1e12 } as Record<string, number>)[(match[2] ?? "").toLowerCase()] ?? 1;
  return amount * multiplier;
}

function metricTokenUsage(providerId: string, metric: Metric): MetricTokenUsage | null {
  const raw = [metric.detail, metric.value, metric.label].filter(Boolean).join(" · ");
  const reported = parseTokenAmount(raw);
  if (reported !== null) return { tokens: reported, source: "provider" };

  // Local CLI/SQLite ledgers are real token counts, but their window is 30
  // days rather than the provider quota window. The UI labels that source.
  const ledger = lastSpend.find((sp) => sp.id === providerId);
  if (ledger && ledger.last30.tokens > 0) return { tokens: ledger.last30.tokens, source: "ledger" };
  return null;
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

// Cross-window sync (dual-form M3): set_config broadcasts "config-updated"
// after every persisted write. A window must reload on OTHER windows'
// writes but not re-load its own echo — and its queued writes must win over
// a reload landing mid-flight. Track in-flight writes + the last send time;
// the listener skips events that are plausibly our own.
let configWritesInFlight = 0;
let lastLocalConfigWriteAt = 0;

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
  if (document.body.classList.contains("settings-open") || IS_PANEL_FORM) setSettingsDirty(true);
  // Send a full current snapshot. If an earlier serialized write failed,
  // the next save retries that still-live in-memory state as well.
  const payload = snapshotConfig();
  const save = configSaveQueue.then(async () => {
    configWritesInFlight += 1;
    lastLocalConfigWriteAt = Date.now();
    try {
      const echoed = await invoke<Config>("set_config", { patch: payload });
      applyConfigEcho(payload, echoed);
      configSaveError = null;
    } finally {
      configWritesInFlight -= 1;
      lastLocalConfigWriteAt = Date.now();
    }
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

/// Another window persisted a config write (the backend broadcasts
/// "config-updated" after every successful set_config): pull the canonical
/// file and re-apply everything the boot/reset paths apply. Without this
/// the other window keeps its stale in-memory config and its next
/// patchConfig would write the old values back over this write.
async function reloadConfigFromBackend(): Promise<void> {
  // Our own write's echo, or a write still queued/in flight here: the local
  // snapshot is newer (or about to be), so the event is not news.
  if (configWritesInFlight > 0 || Date.now() - lastLocalConfigWriteAt < 800) return;
  let fresh: Config;
  try {
    fresh = await invoke<Config>("get_config");
  } catch {
    return;
  }
  // A local write that started while get_config was in flight wins.
  if (configWritesInFlight > 0) return;
  fresh.localShortcuts = fresh.localShortcuts ?? {};
  config = fresh;
  pruneEmptyCardGroups();
  applyLocale();
  syncSettingsControls();
  if (!IS_PANEL_FORM) scheduleAutoRefresh();
  applyAppearance();
  applyGlass();
  applyReduceMotion();
  applyExperimental();
}

// ---------------------------------------------------------------------------
// Layout: defaults, repair, persistence
// ---------------------------------------------------------------------------

function defaultProviderLayout(
  s: Snapshot | undefined,
  spend: ProviderSpend | undefined,
  hasTrend: boolean,
  migrateStar: boolean,
): ProviderLayout {
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
  if (spend && config.showTrend) {
    order.push(TREND_KEY); // trend stays always-visible when opted in, like Mac
    for (const [label] of SPEND_KEYS) {
      order.push(label);
      onDemand.push(label);
    }
  } else if (spend) {
    // Spend source present but trend opt-in is off: surface the spend
    // breakdown without the bar.
    for (const [label] of SPEND_KEYS) {
      order.push(label);
      onDemand.push(label);
    }
  } else if (hasTrend && config.showTrend) {
    // Quota-history trend (no local logs): the bars only, no spend rows.
    order.push(TREND_KEY);
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

  // Positional API-key account ids are not recoverable identities. Remove
  // their old projections before stable fingerprint ids are appended below.
  const withoutLegacyAccounts = layout.providerOrder.filter(
    (id) => !isLegacyExtraAccountId(id),
  );
  if (withoutLegacyAccounts.length !== layout.providerOrder.length) {
    layout.providerOrder = withoutLegacyAccounts;
    changed = true;
  }
  for (const id of Object.keys(layout.providers)) {
    if (isLegacyExtraAccountId(id)) {
      delete layout.providers[id];
      changed = true;
    }
  }
  const disabledWithoutLegacyAccounts = config.disabled.filter(
    (id) => !isLegacyExtraAccountId(id),
  );
  if (disabledWithoutLegacyAccounts.length !== config.disabled.length) {
    config.disabled = disabledWithoutLegacyAccounts;
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
          Boolean(trendSourceFor("kimi")),
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

  // The Work-scope Trae loyalty row first shipped as "Loyalty credits
  // (Work)", which wraps the text-row label column. Rename the saved key
  // in place and slot it beside its general sibling.
  const traecnL = layout.providers.traecn;
  if (traecnL?.metricOrder.includes("Loyalty credits (Work)")) {
    const rename = (list: string[], from: string, to: string) => {
      const at = list.indexOf(from);
      if (at >= 0) list[at] = to;
    };
    for (const list of [traecnL.metricOrder, traecnL.onDemand, traecnL.hidden, traecnL.starred]) {
      rename(list, "Loyalty credits (Work)", "Loyalty (Work)");
    }
    const generalAt = traecnL.metricOrder.indexOf("Loyalty credits");
    const workAt = traecnL.metricOrder.indexOf("Loyalty (Work)");
    if (generalAt >= 0 && workAt >= 0) {
      traecnL.metricOrder.splice(workAt, 1);
      traecnL.metricOrder.splice(traecnL.metricOrder.indexOf("Loyalty credits") + 1, 0, "Loyalty (Work)");
    }
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
      L = defaultProviderLayout(s, spend, Boolean(trendSourceFor(s.id)), config.trayProviders.includes(s.id));
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
        // Trae CN's pack rows are the card's point — the split the user
        // wants to see — so its text rows ship always-visible instead of
        // behind the Show-more caret (progress bars never tuck anyway).
        if (m.kind !== "progress" && m.label !== "Used" && providerFamily(s.id) !== "traecn") {
          L.onDemand.push(m.label);
        }
        changed = true;
      }
      // Do not yank an existing progress row out of Show more or shuffle
      // it above Usage Trend on later refreshes. Extra credits flips
      // text↔progress with balance; a Customize drag would otherwise
      // bounce back on the next snapshot (issue #166). New rows still
      // land always-visible above the trend via the first-seen branch.
    }
    // Same exemption for traecn layouts saved before the rule existed —
    // the first-seen tuck had already buried the pack rows. Runs every
    // refresh, so a pack row dragged back into on-demand pops out again;
    // revisit if tucking Trae packs ever needs to stick.
    if (providerFamily(s.id) === "traecn") {
      const packs = new Set(s.metrics.filter((m) => m.kind !== "progress").map((m) => m.label));
      if (L.onDemand.some((k) => packs.has(k))) {
        L.onDemand = L.onDemand.filter((k) => !packs.has(k));
        changed = true;
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
    } else if (trendSourceFor(s.id)) {
      // Quota-history trend (API-key accounts): bars only, no spend rows.
      if (!L.metricOrder.includes(TREND_KEY)) {
        L.metricOrder.push(TREND_KEY);
        changed = true;
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

  // One/New API merged card: the family layout now owns the card's rows,
  // but the metric config historically lived on each per-key entry. Seed
  // the family entry from the first configured key's layout — once; after
  // that the user customizes the merged card directly.
  const onaFamily = layout.providers[ONA_FAMILY];
  if (!onaFamily || onaFamily.metricOrder.length === 0) {
    const seedId = Object.keys(layout.providers).find(
      (id) => isOnaKeyCardId(id) && layout.providers[id].metricOrder.length > 0,
    );
    if (seedId) {
      layout.providers[ONA_FAMILY] = { ...layout.providers[seedId] };
      changed = true;
    }
  }

  // A merged card owns one layout, even when its bare account is offline
  // and has no metrics. Collect rows from every sibling without resetting
  // the user's hidden/on-demand choices.
  for (const s of lastSnapshots) {
    const family = providerFamily(s.id);
    if (s.id !== family || !supportsExtraAccounts(family) || isParallelAccountFamily(family)) continue;
    const L = layout.providers[family];
    if (!L) continue;
    const siblings = lastSnapshots.filter((snap) => providerFamily(snap.id) === family && !isCardDisabled(snap.id));
    for (const snap of siblings) {
      const defaults = defaultProviderLayout(snap, lastSpend.find((sp) => sp.id === snap.id), Boolean(trendSourceFor(snap.id)), false);
      for (const key of defaults.metricOrder) {
        if (L.metricOrder.includes(key)) continue;
        L.metricOrder.push(key);
        if (defaults.onDemand.includes(key)) L.onDemand.push(key);
        changed = true;
      }
    }
  }

  // Remove the obsolete autoCollapsed marker once. Explicit collapsed
  // choices otherwise survive refreshes and changes in account health.
  for (const L of Object.values(layout.providers)) {
    if ("autoCollapsed" in L) {
      delete (L as any).autoCollapsed;
      if (L.collapsed === false) {
        delete L.collapsed;
      }
      changed = true;
    }
  }

  config.layout = layout;
  if (changed) void patchConfig({ layout, disabled: config.disabled });
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

// ── Card groups ─────────────────────────────────────────────────────────────
// User-named buckets ("常用", "不常用", …) a card is tagged with via its ⚙
// panel. The dashboard renders one section per group; the Quota Overview
// nests them as sub-headers inside the 可用/不可用 status sections.

function cardGroups(): CardGroup[] {
  return config.layout?.groups ?? [];
}

function cardGroup(id: string): CardGroup | undefined {
  return cardGroups().find((g) => g.id === id);
}

/// A card's group id: its own layout entry, falling back to the family's
/// (account cards share the family's tag). Only meaningful for families
/// that show their own card — One/New API synthesizes one family card, so
/// tagging any site tags the family entry.
function cardGroupId(cardId: string): string {
  const fam = providerFamily(cardId);
  return providerLayout(fam).group ?? "";
}

function setCardGroup(cardId: string, groupId: string): void {
  const fam = providerFamily(cardId);
  const layout = config.layout ?? { providerOrder: [], providers: {} };
  if (!layout.providers[fam]) {
    layout.providers[fam] = {
      metricOrder: [],
      onDemand: [],
      hidden: [],
      starred: [],
      expanded: false,
    };
  }
  if (groupId) layout.providers[fam].group = groupId;
  else delete layout.providers[fam].group;
  void patchConfig({ layout });
  renderAll();
}

/// Category a family renders under: the user's per-family override wins
/// over the catalog default. Right-click menu → 分类 moves a family
/// between Coding Agent / productivity without touching the catalog.
function effectiveCategory(family: string): OverviewCategory {
  return config.categoryOverrides?.[family] ?? providerCategory(family);
}

/// Clicking the family's effective category clears the override (back to
/// the catalog default); clicking the other one sets it.
function setFamilyCategory(family: string, category: OverviewCategory): void {
  if (effectiveCategory(family) === category) return;
  const next = { ...(config.categoryOverrides ?? {}) };
  if (providerCategory(family) === category) delete next[family];
  else next[family] = category;
  void patchConfig({ categoryOverrides: next });
  renderAll();
}

/// The card's user note (custom display name). Unlike the group tag, notes
/// live on the exact card id — parallel-account siblings stay independent.
function cardNote(cardId: string): string {
  return (config.layout?.providers[cardId]?.note ?? "").trim();
}

/// Display name for a card: its note when set, otherwise the fallback.
function notedName(cardId: string, fallback: string): string {
  return cardNote(cardId) || fallback;
}

/// Storage key for an account-level note. The default (bare) account shares
/// the family card's id, so its note parks under a synthetic sibling key —
/// otherwise renaming "account 1" would silently retitle the provider.
/// The @-shape keeps the key valid through layout pruning.
function accountNoteKey(accountId: string): string {
  return accountId.includes("@") ? accountId : `${accountId}@__default__`;
}

/// An account's OWN note (never the provider's). Empty when unset.
function accountNote(accountId: string): string {
  return cardNote(accountNoteKey(accountId));
}

// Keep compact labels readable without letting a long custom note stretch a
// card.  The inline value is numeric-only and the CSS still enforces the
// minimum size plus ellipsis when the available width is genuinely too small.
function compactLabelStyle(text: string, maxPx: number, minPx: number): string {
  const length = [...text].length;
  const size = Math.max(minPx, Math.min(maxPx, maxPx - Math.max(0, length - 8) * 0.45));
  return `style="--compact-label-size:${size.toFixed(1)}px"`;
}

/// Measured label fitting: start at the max size and step down only while the
/// text actually overflows the space the flex layout gave the element, so
/// short names stay large and ellipsis is a last resort (min size exceeded).
/// Character-count guesses (compactLabelStyle) got this wrong in both
/// directions — tiny fonts with room to spare, and ellipsis while space sat
/// unused next to badges.
function fitProviderNames(): void {
  const dense = document.documentElement.dataset.density === "compact";
  for (const el of document.querySelectorAll<HTMLElement>(".provider-name[data-fit-max]")) {
    const max = Number(el.dataset.fitMax ?? 16) - (dense ? 1 : 0);
    const min = Number(el.dataset.fitMin ?? 10);
    let size = max;
    el.style.fontSize = `${size}px`;
    while (size > min && el.scrollWidth > el.clientWidth) {
      size -= 0.5;
      el.style.fontSize = `${size}px`;
    }
  }
}

function setCardNote(cardId: string, note: string): void {
  config.layout ??= { providerOrder: [], providers: {} };
  if (!config.layout.providers[cardId]) {
    config.layout.providers[cardId] = {
      metricOrder: [],
      onDemand: [],
      hidden: [],
      starred: [],
      expanded: false,
    };
  }
  if (note) config.layout.providers[cardId].note = note;
  else delete config.layout.providers[cardId].note;
  void patchConfig({ layout: config.layout });
  renderAll();
}

/// Hides a provider card from the dashboard while keeping its credentials and
/// layout available in Customize for a later re-enable.
function removeProviderCard(cardId: string): void {
  const family = providerFamily(cardId);
  // Account cards (kimi@<fp>, antigravity@<fp>, an One/New API key, …)
  // keep the old hide-this-card semantics — the family row stays put.
  if (cardId.includes("@")) {
    if (!config.disabled.includes(cardId)) config.disabled = [...config.disabled, cardId];
    void patchConfig({ disabled: config.disabled });
    renderAll();
    return;
  }
  // Provider-level delete: gone from the Customize list AND every
  // dashboard surface (disabled stops the fetch), credentials kept on
  // disk — the drawer footer's removed list is the way back.
  config.removedProviders = [...new Set([...config.removedProviders, family])];
  if (!config.disabled.includes(family)) config.disabled = [...config.disabled, family];
  void patchConfig({ removedProviders: config.removedProviders, disabled: config.disabled });
  custConfigOpen = null;
  renderAll();
  renderDrawerBody();
}

/// Undo a provider delete: back into the drawer and dashboards, enabled.
function restoreProviderFamily(family: string): void {
  config.removedProviders = config.removedProviders.filter((id) => id !== family);
  config.disabled = config.disabled.filter((id) => id !== family);
  void patchConfig({ removedProviders: config.removedProviders, disabled: config.disabled });
  renderAll();
  renderDrawerBody();
}

/// Groups that actually have at least one card — empty groups don't render.
function usedCardGroups(): CardGroup[] {
  const used = new Set(
    orderedSnapshots().map((s) => cardGroupId(s.id)).filter(Boolean),
  );
  return cardGroups().filter((g) => used.has(g.id));
}

/// Generate a fresh group id ("g1", "g2", …) that isn't taken yet.
function newGroupId(): string {
  let n = cardGroups().length + 1;
  while (cardGroups().some((g) => g.id === `g${n}`)) n += 1;
  return `g${n}`;
}

function upsertCardGroup(id: string, name: string): void {
  // First group ever (layout null, or groups never written): the ?? fallback
  // is a throwaway array — create the real lists before pushing, or the
  // group definition is silently lost while card tags still persist.
  config.layout ??= { providerOrder: [], providers: {} };
  config.layout.groups ??= [];
  const groups = config.layout.groups;
  const existing = groups.find((g) => g.id === id);
  if (existing) existing.name = name;
  else groups.push({ id, name });
  void patchConfig({ layout: config.layout });
}

function cardGroupMemberCount(id: string): number {
  const members = new Set<string>();
  for (const snap of orderedSnapshots()) {
    if (cardGroupId(snap.id) === id) members.add(snap.id);
  }
  for (const [providerId, entry] of Object.entries(config.layout?.providers ?? {})) {
    if (entry.group === id) members.add(providerId);
  }
  return members.size;
}

function pruneEmptyCardGroups(): boolean {
  const groups = config.layout?.groups;
  if (!groups?.length) return false;
  const kept = groups.filter((g) => cardGroupMemberCount(g.id) > 0);
  if (kept.length === groups.length) return false;
  config.layout!.groups = kept;
  void patchConfig({ layout: config.layout });
  return true;
}

function deleteCardGroup(id: string): boolean {
  const layout = config.layout;
  if (!layout || cardGroupMemberCount(id) > 0) return false;
  layout.groups = (layout.groups ?? []).filter((g) => g.id !== id);
  void patchConfig({ layout });
  renderAll();
  return true;
}

// Before stable account ids, API-key families used positional ids
// such as `deepseek@1`. They cannot be safely mapped back after an account
// was deleted or reordered, so discard them instead of attaching old layout
// or disabled state to a different key.
function isLegacyExtraAccountId(id: string): boolean {
  const at = id.indexOf("@");
  return (
    at > 0 &&
    supportsExtraAccounts(providerFamily(id)) &&
    /^\d+$/.test(id.slice(at + 1))
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
// Dashboard rendering
// ---------------------------------------------------------------------------

function renderMetric(m: Metric, providerId?: string): string {
  if (m.kind === "progress" && m.used_percent !== null) {
    const used = clampPercent(m.used_percent);
    const left = Math.round(100 - used);
    // Usage-tier coloring (user spec, uniform for every provider):
    // 0-60% used → blue, 60-75% → amber, 75-100% → red. The bar's width
    // already IS the used percent, so the thresholds compare `used`.
    const level = used >= 75 ? "low" : used >= 60 ? "warn" : "";
    const tokenUsage = providerId ? metricTokenUsage(providerId, m) : null;
    const tokenHeadline = tokenUsage ? t("card.tokens", { n: fmtTokens(tokenUsage.tokens) }) : null;
    const headline = tokenHeadline ?? (config.showUsed ? t("card.pctUsed", { n: Math.round(used) }) : t("card.pctLeft", { n: left }));
    const headlineAlt = tokenHeadline
      ? tokenUsage?.source === "ledger"
        ? t("card.tokensWindow", { n: fmtTokens(tokenUsage.tokens) })
        : t("card.tokensSourceProvider")
      : config.showUsed
        ? t("card.pctLeft", { n: left })
        : t("card.pctUsed", { n: Math.round(used) });

    let resetHtml = "";
    let resetPlain = "";
    if (m.resets_at === null && m.period_ms !== null && m.period_ms <= 6 * 3_600_000 && used <= 1) {
      // GLM-style rolling session windows expose NO reset timestamp while
      // idle — the clock only starts on the first request after the last
      // window closed. An untouched ≤6h window with nothing to count down
      // to is "not started", not "missing data".
      resetPlain = t("card.notStarted");
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
        resetPlain = t("card.notStarted");
        resetHtml = `<span title="${escapeHtml(t("card.notStartedTip"))}">${escapeHtml(t("card.notStarted"))}</span>`;
      } else {
        const remain = m.resets_at - Date.now();
        const countdown = remain < 60_000 ? t("card.resetsSoon") : t("card.resetsIn", { time: fmtDuration(remain) });
        const exact = t("card.resetsAt", { when: fmtExact(m.resets_at) });
        const [text, alt] = config.resetExact ? [exact, countdown] : [countdown, exact];
        resetPlain = text;
        resetHtml = `<span class="clickable" data-flip="reset" data-reset-at="${m.resets_at}" title="${escapeHtml(alt)}">${escapeHtml(text)}</span>`;
      }
    }
    // Countdown leads the foot: on a narrow card the detail side shrinks
    // with an ellipsis from the end, so the time-sensitive part must come
    // first — the used figure is already backed by the percentage on the
    // left. The full foot text rides on the title for hover.
    const detailText = m.detail ? displayMetricDetail(m.detail) : "";
    const detailHtml = [resetHtml, detailText ? escapeHtml(detailText) : ""].filter(Boolean).join(" · ");
    const footTitle = [resetPlain, detailText].filter(Boolean).join(" · ");
    const metricLabel = displayMetricLabel(m.label);
    return `
      <div class="metric">
        <div class="metric-head">
          <span class="metric-label" title="${escapeHtml(metricLabel)}">${escapeHtml(metricLabel)}</span>
        </div>
        <div class="bar">
          <div class="fill ${level}" style="width:${used}%"></div>
        </div>
        <div class="metric-foot">
          <span class="left-val clickable" data-flip="usage" title="${escapeHtml(headlineAlt)}">${headline}</span>
          <span class="detail" title="${escapeHtml(footTitle)}">${detailHtml}</span>
        </div>
      </div>`;
  }
  // Action row (reset credits): exact expiry, plus a Use button only when
  // the metric carries redeem detail. A credit dying within 24h gets an
  // amber dot so it isn't wasted.
  if (m.kind === "action") {
    const metricLabel = displayMetricLabel(m.label);
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
        <span title="${escapeHtml(metricLabel)}">${soon}${escapeHtml(metricLabel)}</span>
        <span class="action-right">
          <span class="detail" title="${escapeHtml(expiry)}">${escapeHtml(expiry)}</span>
          ${useBtn}
        </span>
      </div>`;
  }
  const textValue = displayMetricDetail(m.value ?? "");
  const metricLabel = displayMetricLabel(m.label);
  return `
    <div class="metric-text">
      <span title="${escapeHtml(metricLabel)}">${escapeHtml(metricLabel)}</span>
      <span class="detail" title="${escapeHtml(textValue)}">${escapeHtml(textValue)}</span>
    </div>`;
}

function renderTrend(source: TrendSource): string {
  if (!source.trend.some((v) => v != null && v > 0)) return "";
  const max = Math.max(...source.trend.map((v) => v ?? 0));
  const peakIdx = source.trend.indexOf(max);
  const dayMs = 86_400_000;
  const dateOf = (i: number, weekday = false) =>
    new Date(Date.now() - (29 - i) * dayMs).toLocaleDateString(localeTag(), {
      weekday: weekday ? "short" : undefined,
      month: "short",
      day: "numeric",
    });
  // Each day is a group: the visible bar plus a full-height invisible hit
  // area so thin bars are easy to hover; [data-trend] drives the tooltip.
  const bars = source.trend
    .map((v, i) => {
      const h = v != null && v > 0 ? Math.max(2, (v / max) * 30) : 1;
      return `<g class="trend-day">
        <title>${escapeHtml(dateOf(i, true))}</title>
        <rect class="${v == null ? "trend-nodata" : v > 0 ? "trend-bar" : "trend-zero"}" x="${i * 10}" y="${32 - h}" width="7" height="${h}" rx="1.5"/>
        <rect class="trend-hit" data-trend="${source.id}|${i}" x="${i * 10 - 1.5}" y="0" width="10" height="32" fill="transparent"/>
      </g>`;
    })
    .join("");
  const title = source.credits
    ? t("spend.creditTrendTip", { from: dateOf(0), to: dateOf(29) })
    : source.quota
    ? t("spend.quotaTrendTip", { from: dateOf(0), to: dateOf(29) })
    : t("spend.trendTip", {
        from: dateOf(0),
        to: dateOf(29),
        value: source.fmt(max),
        peak: dateOf(peakIdx),
      });
  const trendLabel = source.credits
    ? t("spend.creditTrend")
    : source.quota
      ? t("spend.quotaTrend")
      : config.spendMetric === "cost"
        ? t("spend.costTrend")
        : t("spend.tokenTrend");
  return `
    <div class="metric trend">
      <span class="metric-label" title="${escapeHtml(title)}">${escapeHtml(trendLabel)}</span>
      <div class="trend-plot">
        <svg class="trend-chart" viewBox="0 0 297 32" preserveAspectRatio="none">${bars}</svg>
        <div class="trend-dates" aria-hidden="true">
          <span>${escapeHtml(dateOf(0, true))}</span>
          <span>${escapeHtml(dateOf(14, true))}</span>
          <span>${escapeHtml(dateOf(29, true))}</span>
        </div>
      </div>
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
  if (key === TREND_KEY) {
    const trend = trendSourceFor(s.id);
    return trend ? renderTrend(trend) : "";
  }
  const spendKey = SPEND_KEYS.find(([label]) => label === key);
  if (spendKey)
    return spend ? renderSpendRow(s.id, spendKey[0], spendKey[1], spend[spendKey[1]], spend) : "";
  const metric = s.metrics.find((m) => m.label === key);
  return metric ? renderMetric(metric, s.id) : "";
}

/// One/New API is two-level: family id `onenewapi` hides every key card.
/// Claude/Codex extra accounts stay independent of the bare family id.
function isCardDisabled(id: string, disabled: string[] = config.disabled): boolean {
  if (disabled.includes(id)) return true;
  const fam = providerFamily(id);
  return fam === "onenewapi" && disabled.includes("onenewapi");
}

/// Families whose extra accounts are PARALLEL cards (Antigravity captured
/// slots, Cursor imported logins, Codex Pane sign-ins) — the bare family
/// card stays the local login and never merges into tabs. Every other
/// multi-account family renders ONE merged card with account tabs.
function isParallelAccountFamily(family: string): boolean {
  return family === "antigravity" || family === "cursor" || family === "codex";
}

/// The "maxed out" threshold for the account-tab health dot.
const MAXED_PCT = 99.5;

/// Determines whether a progress metric represents a core AI coding / model usage quota
/// (Session, Weekly, Monthly, Credits, Usage) rather than an auxiliary tool quota (such
/// as Web Searches). Auxiliary limits must not trigger full provider card exhaustion.
function isCoreQuotaMetric(m: Metric): boolean {
  if (m.kind !== "progress") return false;
  const label = m.label.toLowerCase();
  if (label.includes("search")) return false;
  // On-demand is a paid overflow pool, not the plan quota. A spent
  // overflow must not paint the whole card (or its overview ring) red.
  if (label.includes("on-demand")) return false;
  return true;
}

function maxProgressUsed(s: Snapshot): number {
  return peakCoreProgress(s) ?? 0;
}

/// Peak percentage over the core quota windows, or null when the snapshot has
/// no percentage metric at all (text-only cards). 0 is a real reading here, so
/// callers that print a percentage must not confuse it with "nothing to show".
function peakCoreProgress(s: Snapshot): number | null {
  let best: number | null = null;
  for (const m of s.metrics) {
    if (!isCoreQuotaMetric(m) || m.used_percent === null) continue;
    if (best === null || m.used_percent > best) best = m.used_percent;
  }
  return best;
}

/// Health dot for an account tab: red = some window (session or weekly)
/// is maxed out — the account is waiting for a reset; green = room left
/// everywhere; gray = no successful fetch yet.
function accountHealthDot(id: string): "red" | "green" | "gray" | "error" {
  const snap = lastSnapshots.find((s) => s.id === id);
  if (!snap) return "gray";
  // A rejected key is actionable, not an account with no data. Keep the
  // gray dot for accounts that have not produced a snapshot yet.
  if (snap.status === "error") return "error";
  if (snap.status !== "ok") return "gray";
  if (!snap.metrics.length) return "gray";
  return isSnapshotMaxed(snap) ? "red" : "green";
}

// ── Card fold (grouped by provider family or per-card) ─────────────────────────

/// Returns true when a snapshot has reached its quota limit.
///
/// Independent pools (Cursor Auto vs API, Antigravity Gemini vs Claude)
/// are maxed only when every *present* pool is exhausted — one full bar
/// must not paint the whole card red while another pool still has room.
/// Nested windows on a single pool (Session + Weekly) still max the card
/// when any of them hits the ceiling: that account is waiting on a reset.
function isSnapshotMaxed(s: Snapshot): boolean {
  if (s.status !== "ok") return false;
  if (s.plan && /out of credit/i.test(s.plan)) return true;
  const family = providerFamily(s.id);
  const pools = METRIC_POOLS[family];
  if (pools) {
    const present = new Set<string>();
    const maxed = new Set<string>();
    for (const m of s.metrics) {
      if (!isCoreQuotaMetric(m) || m.used_percent === null) continue;
      const pool = pools[m.label];
      if (!pool) continue;
      present.add(pool);
      if (m.used_percent >= MAXED_PCT) maxed.add(pool);
    }
    if (present.size > 0) return [...present].every((p) => maxed.has(p));
  }
  return maxProgressUsed(s) >= MAXED_PCT;
}

/// Returns true when this card should be automatically collapsed:
///
///   - For merged multi-account families (kimi, onenewapi), all valid active
///     accounts must be maxed out (all quota exhausted).
///
///   - For standalone providers (zai, claude, codex, copilot, grok, devin, etc.)
///     and parallel-account cards (antigravity slots, cursor imported logins),
///     the card collapses when its own quota is exhausted.
function isCardFoldCandidate(cardId: string): boolean {
  const family = providerFamily(cardId);
  const snap = lastSnapshots.find((s) => s.id === cardId);
  if (!snap || snap.status !== "ok" || isCardDisabled(cardId)) return false;

  // Merged multi-account family card (kimi, onenewapi)
  if (cardId === family && supportsExtraAccounts(family) && !isParallelAccountFamily(family)) {
    const activeAccountIds = lastSnapshots
      .filter((s) => {
        if (providerFamily(s.id) !== family || isCardDisabled(s.id)) return false;
        // Exclude deleted accounts if accountsCache is populated for this family
        if (s.id.includes("@") && accountsCache.has(family)) {
          return accountsCache.get(family)?.some((entry) => entry.id === s.id);
        }
        return true;
      })
      .map((s) => s.id);

    if (activeAccountIds.length === 0) return isSnapshotMaxed(snap);
    return activeAccountIds.every((id) => {
      const s = lastSnapshots.find((x) => x.id === id);
      return s ? isSnapshotMaxed(s) : false;
    });
  }

  // Standalone provider card or parallel account card
  return isSnapshotMaxed(snap);
}

/// True when the card should render in the collapsed single-line state.
/// Respects the user's manual override (layout.providers[cardId].collapsed):
///   - undefined  → follow auto-detection (isCardFoldCandidate)
///   - true       → always collapsed
///   - false      → always expanded
function isCardCollapsed(cardId: string): boolean {
  const layout = providerLayout(cardId);
  // Detail cards are EXPANDED by default; folding is a deliberate user
  // choice made through the card's fold control and remembered in layout.
  return layout.collapsed === true;
}

/// When collapsed, returns the nearest reset across relevant snapshots for
/// this card. Prioritizes the reset countdown of metrics that are actually
/// maxed out (so a 0% session window does not hide a weekly exhaustion).
function nearestResetSeconds(cardId: string): number {
  const family = providerFamily(cardId);
  const isMerged =
    cardId === family && supportsExtraAccounts(family) && !isParallelAccountFamily(family);
  const cards = isMerged
    ? lastSnapshots.filter((s) => {
        if (providerFamily(s.id) !== family || isCardDisabled(s.id)) return false;
        if (s.id.includes("@") && accountsCache.has(family)) {
          return accountsCache.get(family)?.some((entry) => entry.id === s.id);
        }
        return true;
      })
    : lastSnapshots.filter((s) => s.id === cardId);

  let nearest = Infinity;
  for (const s of cards) {
    // Any maxed row counts as maxed even without a reset instant — a row
    // that is maxed but carries no reset (e.g. a monthly cap whose error
    // names no date) must NOT fall back to a healthy row's countdown,
    // which read as "the maxed window resets in 4h" when it doesn't.
    const maxed = s.metrics.filter(
      (m) => isCoreQuotaMetric(m) && (m.used_percent ?? 0) >= MAXED_PCT,
    );
    const pool =
      maxed.length > 0
        ? maxed
        : s.metrics.filter((m) => isCoreQuotaMetric(m) && m.resets_at !== null);
    for (const m of pool) {
      if (m.resets_at === null) continue;
      const secs = Math.max(0, m.resets_at - Date.now()) / 1000;
      if (secs < nearest) nearest = secs;
    }
  }
  return nearest === Infinity ? 0 : nearest;
}

/// When the card is maxed, name the maxed row (e.g. "Monthly") if only
/// some rows are maxed; returns null when every core row is maxed (the
/// family-wide "all maxed" copy fits) or when the maxing is account-level
/// (red dot from a sibling account). Mirrors nearestResetSeconds' card set
/// so merged families see every account's rows.
function maxedRowLabel(cardId: string): string | null {
  const family = providerFamily(cardId);
  const isMerged =
    cardId === family && supportsExtraAccounts(family) && !isParallelAccountFamily(family);
  const cards = isMerged
    ? lastSnapshots.filter((s) => {
        if (providerFamily(s.id) !== family || isCardDisabled(s.id)) return false;
        if (s.id.includes("@") && accountsCache.has(family)) {
          return accountsCache.get(family)?.some((entry) => entry.id === s.id);
        }
        return true;
      })
    : lastSnapshots.filter((s) => s.id === cardId);

  const rows = cards.flatMap((s) =>
    s.metrics.filter((m) => isCoreQuotaMetric(m) && m.used_percent !== null),
  );
  if (rows.length === 0) return null;
  const maxed = rows.filter((m) => (m.used_percent ?? 0) >= MAXED_PCT);
  if (maxed.length === 0 || maxed.length === rows.length) return null;
  return displayMetricLabel(maxed[0].label);
}

/// Combines the overall family health dot: green if any account is green (quota available),
/// red if all are red, gray otherwise.
function familyHealthDot(family: string): "red" | "green" | "gray" | "error" {
  const cards = lastSnapshots.filter((s) => {
    if (providerFamily(s.id) !== family || isCardDisabled(s.id)) return false;
    if (s.id.includes("@") && accountsCache.has(family)) {
      return accountsCache.get(family)?.some((entry) => entry.id === s.id);
    }
    return true;
  });
  if (cards.length === 0) return "gray";
  const dots = cards.map((s) => accountHealthDot(s.id));
  if (dots.some((d) => d === "green")) return "green";
  // A sibling with a red, resettable quota is still a known exhausted
  // account. Gray is reserved for a pool with no numeric quota at all.
  if (dots.some((d) => d === "red")) return "red";
  if (dots.some((d) => d === "error")) return "error";
  return "gray";
}

function cardHealthDot(cardId: string): "red" | "green" | "gray" | "error" {
  const family = providerFamily(cardId);
  if (cardId === family && supportsExtraAccounts(family) && !isParallelAccountFamily(family)) {
    return familyHealthDot(family);
  }
  return accountHealthDot(cardId);
}

/// Peak-hours tint: a provider currently inside its peak window shows a
/// yellow dot instead of green — but only while it is actually available
/// (green). Red (maxed / error) and gray keep their own meaning.
function peakTintedDot(
  family: string,
  dot: "red" | "green" | "gray" | "error",
): "red" | "green" | "gray" | "yellow" | "error" {
  return dot === "green" && isProviderInPeak(family) ? "yellow" : dot;
}

function healthDotTitle(dot: "red" | "green" | "gray" | "yellow" | "error"): string {
  if (dot === "error") return t("customize.acctDotError");
  return dot === "red"
    ? t("customize.acctDotRed")
    : dot === "yellow"
      ? t("customize.acctDotYellow")
      : dot === "green"
        ? t("customize.acctDotGreen")
        : t("customize.acctDotGray");
}

// Quota pools: which independent meter group a metric label belongs to.
// Antigravity meters Gemini and Claude separately; Cursor separates its
// Auto bucket from the API bucket. Single-pool providers return one pool
// and the card renders no group headers.
const METRIC_POOLS: Record<string, Record<string, string>> = {
  antigravity: {
    Session: "Gemini",
    Weekly: "Gemini",
    Claude: "Claude",
    "Claude Weekly": "Claude",
  },
  cursor: {
    "Cursor Models": "Auto",
    "Other Models": "API",
  },
  // Qoder CN's dedicated model packages (Qwen-only credits, …) are side
  // pools: only the main "Credits" pool decides whether the seat is done.
  // Labels outside this map are ignored by the maxed check, so a spent
  // package can't fold the card while the main pool still has room.
  qodercn: {
    Credits: "credits",
  },
  // Trae CN's credit packs (loyalty/monthly/check-in) are side pools for
  // the same reason: the merged "Credits" row is the seat's real budget,
  // and a drained bonus pack must not fold the card while loyalty
  // credits remain.
  traecn: {
    Credits: "credits",
  },
};

/// When a family has independent pools, the overview ring follows this
/// label instead of the most-used sibling. Cursor's Auto bucket is the
/// plan people actually work in; the API bucket can sit at 100% without
/// meaning the seat is done.
const OVERVIEW_PRIMARY_LABEL: Record<string, string> = {
  cursor: "Cursor Models",
  qodercn: "Credits",
  traecn: "Credits",
};

/// Hover on a 5h ring should not repeat that same window. These families
/// have no useful weekly sibling (Copilot is monthly; Z.ai's 5h *is* the
/// story the user wants in the tip), so the tip stays on the ring window.
const OVERVIEW_HOVER_KEEP_RING = new Set(["copilot", "zai"]);

/// Week capsule leaves these families on the default binding (5h first,
/// otherwise shortest period). Z.ai and One/New API keep their own
/// windows; Copilot has no weekly meter so the week tab would go empty.
const OVERVIEW_KEEP_BINDING = new Set(["copilot", "zai", "onenewapi"]);

const is5hPeriod = (p: number | null) =>
  p !== null && p >= 14_400_000 && p <= 21_600_000;

const is5hLabel = (label: string) =>
  /session|5-?hour|5h/i.test(label) && !/week|month|day|year/i.test(label);

function metricWindow(m: Metric): QuotaWindow {
  if (is5hPeriod(m.period_ms) || is5hLabel(m.label)) return "5h";
  const label = m.label.toLowerCase();
  if ((m.period_ms !== null && m.period_ms <= 36 * 3_600_000) || /day|daily|today/.test(label)) {
    return "day";
  }
  if ((m.period_ms !== null && m.period_ms <= 8 * 24 * 3_600_000) || /week/.test(label)) {
    return "week";
  }
  if (m.period_ms !== null || /month|cycle/.test(label)) return "month";
  return "generic";
}

function formatHoverReset(m: Metric): string {
  if (m.resets_at !== null && m.resets_at > Date.now()) {
    return t("overview.resetsIn", { time: fmtDuration(m.resets_at - Date.now()) });
  }
  if (metricWindow(m) === "5h") return t("card.notStarted");
  return "";
}

function formatHoverMetric(m: Metric, label = displayMetricLabel(m.label)): string {
  const pct = Math.round(m.used_percent ?? 0);
  const reset = formatHoverReset(m);
  return reset ? `${label} ${pct}% · ${reset}` : `${label} ${pct}%`;
}

/// Tip next to the ring: weekly for ordinary 5h cards, the ring window
/// for Copilot/Z.ai, every independent pool for Cursor/Antigravity.
function overviewHoverTip(s: Snapshot, quota: OverviewQuota, displayName: string): string {
  if (quota.status === "error") return `${displayName}: ${t("overview.offline")}`;
  if (quota.status === "no_data") return `${displayName}: ${t("overview.noData")}`;

  const family = providerFamily(s.id);
  const core = (s.metrics || []).filter((m) => isCoreQuotaMetric(m) && m.used_percent !== null);
  const pools = METRIC_POOLS[family];

  if (pools) {
    const names = [...new Set(Object.values(pools))];
    const parts: string[] = [];
    for (const pool of names) {
      const ms = core.filter((m) => pools[m.label] === pool);
      if (!ms.length) continue;
      const primaryLabel = Object.keys(pools).find((k) => pools[k] === pool) ?? pool;
      const session = ms.find((m) => metricWindow(m) === "5h") ?? ms[0];
      const weekly = ms.find((m) => metricWindow(m) === "week" && m !== session);
      const head = formatHoverMetric(session, displayMetricLabel(primaryLabel));
      parts.push(weekly ? `${head} · ${formatHoverMetric(weekly, t("overview.winWeek"))}` : head);
    }
    if (parts.length) return `${displayName}: ${parts.join(" · ")}`;
  }

  if (!OVERVIEW_HOVER_KEEP_RING.has(family) && quota.window === "5h") {
    const weeklies = core.filter((m) => metricWindow(m) === "week");
    if (weeklies.length === 1) {
      return `${displayName}: ${formatHoverMetric(weeklies[0], t("overview.winWeek"))}`;
    }
    if (weeklies.length > 1) {
      return `${displayName}: ${weeklies.map((m) => formatHoverMetric(m)).join(" · ")}`;
    }
  }

  // Week-tab rings already show the weekly percent — tip the 5h sibling
  // so the hover still surfaces the other window.
  if (!OVERVIEW_HOVER_KEEP_RING.has(family) && quota.window === "week") {
    const sessions = core.filter((m) => metricWindow(m) === "5h");
    if (sessions.length === 1) {
      return `${displayName}: ${formatHoverMetric(sessions[0], t("overview.win5h"))}`;
    }
    if (sessions.length > 1) {
      return `${displayName}: ${sessions.map((m) => formatHoverMetric(m)).join(" · ")}`;
    }
  }

  const win = overviewWindowLabel(quota);
  const pct = Math.round(quota.usedPercent);
  if (quota.resetsAt && quota.resetsAt > Date.now()) {
    return `${displayName}: ${win} ${pct}% · ${t("overview.resetsIn", { time: fmtDuration(quota.resetsAt - Date.now()) })}`;
  }
  if (quota.window === "5h") {
    return `${displayName}: ${win} ${pct}% · ${t("card.notStarted")}`;
  }
  return `${displayName}: ${win} ${pct}%`;
}

function metricPool(family: string, label: string): string | undefined {
  return METRIC_POOLS[family]?.[label];
}

/// Number of live account cards represented by an overview family tile.
/// Keep this separate from the health dot: an account count is informational,
/// so a red/orange provider state must never change its visual meaning.
function overviewAccountCount(family: string): number {
  const snapshotCount = lastSnapshots.filter(
    (snapshot) => providerFamily(snapshot.id) === family && !isCardDisabled(snapshot.id),
  ).length;
  const configuredCount = accountsCache.get(family)?.length ?? 0;
  return Math.max(snapshotCount, configuredCount);
}

function overviewAccountSnapshots(family: string): Snapshot[] {
  return lastSnapshots.filter((snapshot) =>
    providerFamily(snapshot.id) === family &&
    !isCardDisabled(snapshot.id) &&
    (!snapshot.id.includes("@") || !accountsCache.has(family) || accountsCache.get(family)?.some((entry) => entry.id === snapshot.id)),
  );
}

function accountAvailabilitySummary(family: string): { total: number; available: number; unavailable: number } {
  const accounts = overviewAccountSnapshots(family);
  const available = accounts.filter((snapshot) => accountHealthDot(snapshot.id) === "green").length;
  const configured = accountsCache.get(family)?.length ?? 0;
  const total = Math.max(accounts.length, configured);
  return { total, available, unavailable: Math.max(0, total - available) };
}

function setPinnedAccount(family: string, accountId: string): void {
  const layout = config.layout ?? { providerOrder: [], providers: {} };
  const current = layout.providers[family] ?? {
    metricOrder: [], onDemand: [], hidden: [], starred: [], expanded: false,
  };
  current.pinnedAccount = current.pinnedAccount === accountId ? undefined : accountId;
  layout.providers[family] = current;
  config.layout = layout;
  saveLayout(false);
  renderAll();
}

function renderCard(s: Snapshot): string {
  const family = providerFamily(s.id);
  // Multi-account families render ONE dashboard card per family (the bare
  // family id), with the account tabs under the head. The card body shows
  // the selected account's snapshot; s (the family card) is the anchor.
  let shown = s;
  let accountCount = "";
  let accountTabs = "";
  let accountIds: string[] = [];
  if (s.id === family && supportsExtraAccounts(family) && !isParallelAccountFamily(family)) {
    accountIds = lastSnapshots
      .filter((snap) => providerFamily(snap.id) === family && !isCardDisabled(snap.id))
      .map((snap) => snap.id);
    if (accountIds.length > 1) {
      // Families whose bare card IS an account (kimi) default to it; One/New
      // API has no bare snapshot at all, so the default tab is the first
      // healthy account — otherwise the computed tabs would never light up.
      const defaultId = lastSnapshots.some((snap) => snap.id === s.id)
        ? s.id
        : (accountIds.find((a) => accountHealthDot(a) === "green") ?? accountIds[0]);
      const active = resolveDisplayedAccount(family, defaultId, accountIds);
      const activeSnap = lastSnapshots.find(
        (snap) => snap.id === active && !isCardDisabled(snap.id),
      );
      if (activeSnap) shown = activeSnap;
      const availableAccounts = accountIds.filter((id) => accountHealthDot(id) === "green").length;
      const accountState = availableAccounts === 0 ? "is-empty" : availableAccounts === accountIds.length ? "is-ready" : "is-partial";
      accountCount = `<span class="provider-account-badge ${accountState}" title="${escapeHtml(`${accountIds.length} accounts · ${availableAccounts} available`)}"><span class="provider-account-glyph" aria-hidden="true">●</span><span>${accountIds.length}/${availableAccounts}</span></span>`;
      const compactTabs = accountIds.length >= 4;
      accountTabs = `<div class="card-account-tabs${compactTabs ? " compact" : ""}">${accountIds
        .map((id, index) => {
          const label = id === s.id
            ? (accountNote(id) || accountsCache.get(family)?.[0]?.label || t("customize.acctDefaultShort"))
            : labelForAccount(id, accountsCache.get(family) ?? []);
          const on = id === shown.id;
          const dot = peakTintedDot(family, accountHealthDot(id));
          const dotTitle = healthDotTitle(dot);
          // Compact number capsules exist to save space — but a named or
          // noted account shows its label/note ("njf3", "TTA"); only unnamed
          // ones keep the number. Named capsules get text width + auto-shrink
          // instead of the fixed number cell.
          const list = accountsCache.get(family) ?? [];
          const entry = id === s.id ? list[0] : list.find((a) => a.id === id);
          const note = accountNote(id);
          const hasCustomName = Boolean(note || entry?.label?.trim() || entry?.email?.trim());
          const tabLabel = compactTabs && !hasCustomName ? String(index + 1) : label;
          const tabTitle = `${label} · ${dotTitle}`;
          return `<button type="button" class="card-account-tab${on ? " on" : ""}${compactTabs && !hasCustomName ? " compact" : ""}${compactTabs && hasCustomName ? " compact-noted" : ""}" data-card-account="${family}|${escapeHtml(id)}" title="${escapeHtml(tabTitle)}" aria-label="${escapeHtml(label)}"><span class="acct-dot ${dot}"></span><span class="card-account-tab-label" ${compactLabelStyle(tabLabel, 10.5, 8)}>${escapeHtml(tabLabel)}</span></button>`;
        })
        .join("")}</div>`;
    }
    // The synthesized One/New API family card carries the family id but no
    // real snapshot backs it — display the first healthy account directly
    // (same rule the anchor used) so tabs, trends and refresh all hit the
    // real account snapshot. Families with a real bare card are untouched.
    if (
      shown.id === family &&
      !lastSnapshots.some((snap) => snap.id === family)
    ) {
      const accountIds = lastSnapshots
        .filter((snap) => providerFamily(snap.id) === family && !isCardDisabled(snap.id))
        .map((snap) => snap.id);
      const target =
        accountIds.find((a) => accountHealthDot(a) === "green") ?? accountIds[0];
      const borrowed =
        target !== undefined
          ? lastSnapshots.find((snap) => snap.id === target && !isCardDisabled(snap.id))
          : undefined;
      if (borrowed) shown = borrowed;
    }
  }
  const plan = shown.plan ? `<span class="plan" title="${escapeHtml(shown.plan)}">${escapeHtml(shown.plan)}</span>` : "";
  // Peak-hours marker on the head: expanded standalone cards carry no
  // health dot, so the yellow peak state needs its own spot (next to the
  // plan badge). Same predicate as the dot tint: available AND in peak.
  // Folded cards already tint their fold-row dot yellow — no double mark.
  const peakBadge =
    !isCardCollapsed(s.id) && cardHealthDot(s.id) === "green" && isProviderInPeak(family)
      ? `<span class="peak-dot" title="${escapeHtml(`${t("peak.now")} ${t(PEAK_RULES[family].tipKey)}`)}"></span>`
      : "";
  const pinnedId = providerLayout(family).pinnedAccount;
  const pinnedSnap = pinnedId ? lastSnapshots.find((snap) => snap.id === pinnedId) : undefined;
  const pinnedBadge = pinnedSnap && isSnapshotMaxed(pinnedSnap) && familyHealthDot(family) === "green"
    ? `<span class="pinned-status-warn" title="${escapeHtml(t("customize.acctPinnedExhaustedHint"))}">★ ${escapeHtml(t("customize.acctPinnedExhausted"))}</span>`
    : "";
  const icon = providerVisual(shown.id, shown.dashboard_url ?? undefined)?.iconSvg ?? "";
  const muted = shown.status === "ok" ? "" : " muted";

  let body: string;
  let caret = "";
  if (shown.status === "ok") {
    const L = providerLayout(s.id);
    const spend = lastSpend.find((sp) => sp.id === shown.id);
    const visible = L.metricOrder.filter((k) => !L.hidden.includes(k));
    const always = visible.filter((k) => !L.onDemand.includes(k));
    const onDemand = visible.filter((k) => L.onDemand.includes(k));
    // Pool-grouped metric rows. A pool header is inserted when the card's
    // metrics span 2+ distinct pools (Antigravity Gemini vs Claude, Cursor
    // Auto vs API).
    const withPools = always.map((k) => ({
      key: k,
      html: renderItem(shown, spend, k),
      pool: metricPool(family, k),
    }));
    let lastPool: string | undefined;
    body = withPools
      .map((row) => {
        const header =
          row.pool && row.pool !== lastPool && withPools.some((r) => r.pool && r.pool !== row.pool)
            ? `<div class="pool-head">${escapeHtml(row.pool)}</div>`
            : "";
        lastPool = row.pool;
        return header + row.html;
      })
      .join("");
    const onDemandHtml = onDemand.map((k) => renderItem(shown, spend, k)).join("");
    if (onDemandHtml.trim()) {
      const anim = L.expanded && animateExpandId === s.id ? " anim" : "";
      caret = `
        <button class="card-caret" data-caret="${s.id}" title="${L.expanded ? t("card.showLess") : t("card.showMore")}">${uiIcon(L.expanded ? "caretUp" : "caretDown")}</button>
        ${L.expanded ? `<div class="on-demand${anim}">${onDemandHtml}</div>` : ""}`;
    }
  } else {
    body = `<p class="placeholder">${escapeHtml(shown.error ?? t("card.notConnected"))}</p>`;
  }

  const stale = shown.stale
    ? `<span class="stale" title="${escapeHtml(staleHelp(shown))}">${escapeHtml(t("card.outdated"))}</span>`
    : "";
  const dashUrl = (shown.dashboard_url ?? "").trim();
  const dashOk = /^https?:\/\//i.test(dashUrl);
  const staticLinks = PROVIDER_LINKS[shown.id] ?? PROVIDER_LINKS[family] ?? [];
  const linkItems = dashOk
    ? [{ label: "Dashboard", url: dashUrl }, ...staticLinks.filter((l) => l.label !== "Dashboard")]
    : staticLinks;
  const links = linkItems
    .filter((l) => l.label !== "API" || shown.metrics.some((m) => m.label === "API"))
    .map((l) => `<button class="quick-link" data-link="${escapeHtml(l.url)}">${escapeHtml(displayLinkLabel(l.label))}</button>`)
    .join("<span class='quick-sep'>·</span>");
  const linksRow = links ? `<div class="quick-links">${links}</div>` : "";
  // Folded state: replace the card body with a single-line summary (one
  // row per quota window showing TIME-elapsed + the nearest reset countdown).
  // The card head stays so the user can still read the provider name, plan,
  // and family state. The chevron toggles between collapsed/expanded and
  // remembers the choice per family.
  const cardCollapsed = isCardCollapsed(s.id);
  const foldChevron = cardCollapsed
    ? `<button class="card-fold-toggle" data-card-fold="${escapeHtml(s.id)}" title="${escapeHtml(t("card.expand"))}">${uiIcon("caretDown")}</button>`
    : `<button class="card-fold-toggle" data-card-fold="${escapeHtml(s.id)}" title="${escapeHtml(t("card.collapse"))}">${uiIcon("caretUp")}</button>`;
  const finalBody = cardCollapsed ? "" : body;
  // Hide per-account tabs and the ×N badge when folded — the family health
  // dot and reset countdown already summarise the whole family.
  const finalAccountTabs = cardCollapsed ? "" : accountTabs;
  const finalAccountCount = accountCount;
  const refreshBtn =
    shown.status === "ok" || shown.status === "error"
      ? `<button class="card-refresh" data-card-refresh="${shown.id}" title="${escapeHtml(t("card.refresh"))}">${uiIcon("arrowsClockwise")}</button>`
      : "";
  // One permanent pin entry in the head, acting on whichever account is
  // currently shown — the per-tab stars were pure noise on 6-account cards.
  const headPin =
    accountIds.length > 1
      ? (() => {
          const pinnedNow = providerLayout(family).pinnedAccount === shown.id;
          return `<button type="button" class="mini-btn card-account-pin-head${pinnedNow ? " on" : ""}" data-card-pin="${family}|${escapeHtml(shown.id)}" title="${escapeHtml(pinnedNow ? t("customize.acctUnpin") : t("customize.acctPin"))}">${pinnedNow ? "★" : "☆"}</button>`;
        })()
      : "";
  const share =
    shown.status === "ok"
      ? `<button class="share-btn" data-share="${shown.id}" title="${escapeHtml(t("card.share"))}">${uiIcon("shareNetwork")}</button>`
      : "";
  // Head status lives in the right zone (before the action cluster) so the
  // provider name keeps the whole left side and can render larger. Text-only
  // snapshots have no percentage to show — render nothing rather than "0%".
  const headPct = shown.status === "ok" ? peakCoreProgress(shown) : null;
  const headStatus =
    headPct === null
      ? ""
      : `<span class="head-status" title="${escapeHtml(`${Math.round(headPct)}% · ${healthDotTitle(peakTintedDot(family, cardHealthDot(s.id)))}`)}"><span class="acct-dot ${peakTintedDot(family, cardHealthDot(s.id))}"></span><span class="head-pct">${Math.round(headPct)}%</span></span>`;
  // Folded state: visually prominent reset countdown badge with health status
  // and generous breathing room instead of a cramped raw text sliver.
  let foldLine = "";
  if (cardCollapsed) {
    const dot = peakTintedDot(family, cardHealthDot(s.id));
    const dotTitle = healthDotTitle(dot);
    const resetSecs = nearestResetSeconds(s.id);
    const isMaxed = isCardFoldCandidate(s.id) || dot === "red";
    const maxedLabel = isMaxed ? maxedRowLabel(s.id) : null;
    if (resetSecs > 0) {
      const label = isMaxed
        ? maxedLabel
          ? t("card.rowMaxedResetsIn", { label: maxedLabel })
          : t("card.familyAllMaxed")
        : t("card.foldedResetsIn");
      const badgeTone = isMaxed ? "warn" : "normal";
      foldLine = `
        <div class="fold-row">
          <div class="fold-badge ${badgeTone}">
            <span class="acct-dot ${dot}" title="${escapeHtml(dotTitle)}"></span>
            <span class="fold-main-quota">${headPct === null ? "—" : `${Math.round(headPct)}%`}</span>
            <span class="fold-label">${escapeHtml(label)}</span>
            <span class="fold-timer" data-reset-at="${Date.now() + resetSecs * 1000}">${escapeHtml(fmtDuration(resetSecs * 1000))}</span>
          </div>
        </div>`;
    } else {
      const label = isMaxed
        ? maxedLabel
          ? t("card.rowMaxedPending", { label: maxedLabel })
          : t("card.familyAllMaxedPending")
        : t("card.familyReady");
      foldLine = `
        <div class="fold-row">
          <div class="fold-badge normal">
            <span class="acct-dot ${dot}" title="${escapeHtml(dotTitle)}"></span>
            <span class="fold-main-quota">${headPct === null ? "—" : `${Math.round(headPct)}%`}</span>
            <span class="fold-label">${escapeHtml(label)}</span>
          </div>
        </div>`;
    }
  }
  return `
    <article class="provider${muted} ${cardCollapsed ? "is-folded" : ""}" data-provider="${s.id}" data-shown-account="${escapeHtml(shown.id)}" data-origin="${escapeHtml(shown.dashboard_url ?? "")}">
      <div class="provider-head">
        <span class="drag-grip" title="${escapeHtml(t("card.drag"))}">⠿</span>
        <span class="provider-name" data-fit-max="17" data-fit-min="11" title="${escapeHtml(notedName(s.id, s.name))}">${escapeHtml(notedName(s.id, s.name))}</span>
        ${finalAccountCount}
        ${plan}
        ${pinnedBadge}
        ${peakBadge}
        ${stale}
        <span class="spacer"></span>
        <span class="head-right">
          ${headStatus}
          <span class="head-actions">
            <button class="mini-btn card-group-btn" data-card-group-menu="${escapeHtml(s.id)}" title="${escapeHtml(t("customize.cardSettings"))}">${uiIcon("gear")}</button>
            ${foldChevron}
            ${headPin}
            ${refreshBtn}
            ${share}
          </span>
        </span>
        <span class="provider-icon">${icon}</span>
      </div>
      ${cardCollapsed ? foldLine : `<div class="card-panel">
        ${finalAccountTabs}
        ${finalBody}
        ${linksRow}
        ${caret}
      </div>`}
    </article>`;
}

function orderedSnapshots(): Snapshot[] {
  const order = config.layout?.providerOrder ?? [];
  // Multi-account families render ONE card on the dashboard (the family id);
  // the per-account cards (kimi@<fp>) surface as account tabs inside that
  // card. Antigravity is the exception: its bare card is the logged-in
  // account and the slots are independent captured accounts — they stay
  // as separate cards (multi-account parallel monitoring, not a switcher).
  // One/New API has no bare snapshot of its own (every card is a site
  // account), so the family card is synthesized: it borrows the first
  // healthy account's data for the default display, tabs switch for real.
  const snaps = withOnaFamilyAnchor(lastSnapshots);
  return snaps
    .filter((s) => {
      const fam = providerFamily(s.id);
      if (s.id !== fam && supportsExtraAccounts(fam) && !isParallelAccountFamily(fam)) return false;
      return !isCardDisabled(s.id);
    })
    .sort((a, b) => {
      const ia = order.indexOf(a.id);
      const ib = order.indexOf(b.id);
      if (ia !== -1 && ib !== -1) return ia - ib;
      return rankSnapshot(a) - rankSnapshot(b);
    });
}

/// Overview is one ring per family. Parallel extra cards stay on the
/// dashboard, but repeating the same truncated name in the grid looks
/// like a duplicate (two "Antig..." tiles).
function overviewSnapshots(): Snapshot[] {
  const seen = new Set<string>();
  const out: Snapshot[] = [];
  for (const s of orderedSnapshots()) {
    const fam = providerFamily(s.id);
    if (seen.has(fam)) continue;
    seen.add(fam);
    out.push(s);
  }
  return out;
}

/// One overview tile per family follows an available account, even when the
/// stored default failed or a parallel sibling has exhausted its quota.
function pickOverviewShown(card: Snapshot): Snapshot {
  const family = providerFamily(card.id);
  const siblings = lastSnapshots.filter(
    (snap) => providerFamily(snap.id) === family && !isCardDisabled(snap.id) &&
      (!snap.id.includes("@") || !accountsCache.has(family) ||
        accountsCache.get(family)?.some((account) => account.id === snap.id)),
  );
  if (siblings.length <= 1) return card;
  const selected = resolveDisplayedAccount(family, card.id, siblings.map((s) => s.id));
  return siblings.find((s) => s.id === selected) ?? card;
}

/// Views `snapshots` as if a bare `onenewapi` family card existed: when any
/// site-account card does, clone the first healthy one (else the first) under
/// the family id. The clone is render-only — it never enters lastSnapshots,
/// so the site cards keep their real ids for tabs, trends, and refresh.
function withOnaFamilyAnchor(snapshots: Snapshot[]): Snapshot[] {
  if (!snapshots.some((s) => isOnaKeyCardId(s.id))) return snapshots;
  if (snapshots.some((s) => s.id === ONA_FAMILY)) return snapshots;
  const accounts = snapshots.filter((s) => isOnaKeyCardId(s.id));
  const healthy = accounts.find((s) => s.status === "ok" && accountHealthDot(s.id) === "green");
  const anchor = { ...(healthy ?? accounts[0]) };
  anchor.id = ONA_FAMILY;
  anchor.name = providerDisplayName(ONA_FAMILY);
  return [...snapshots, anchor];
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
  activeDays?: number;
  /// Present on the synthetic "Others" entry: the folded-in providers,
  /// largest first, for the hover breakdown.
  parts?: { id: string; name: string; w: SpendWindow }[];
};

const OTHERS_ID = "__others__";
/// Providers under this many dollars (in the visible window) fold into
/// one "Others" wedge; hovering it lists who spent what. The bar scales
/// with the period — a day's ring earns a slice at $5, a month's at $10.
function othersFoldUsd(tab: SpendTab): number {
  return tab === "last30" ? 10 : 5;
}

function providerNameForSpend(id: string): string {
  return lastSpend.find((s) => s.id === id)?.name || providerDisplayName(providerFamily(id)) || id;
}

function historyEntries(range: Exclude<RangeTab, "d30">): DonutEntry[] {
  const rows = range === "d7" ? lastSpendHistory.d7 : lastSpendHistory.all;
  const base = rows
    .filter((row) => !isCardDisabled(row.id))
    .map((row) => ({
      s: {
        id: row.id,
        name: providerNameForSpend(row.id),
        today: emptyWindow(),
        yesterday: emptyWindow(),
        last30: { cost: row.cost, tokens: row.tokens, models: row.models },
        trend: [],
        trend_cost: [],
        unpriced: 0,
        unpriced_models: [],
      } as ProviderSpend,
      w: { cost: row.cost, tokens: row.tokens, models: row.models },
      activeDays: range === "all" ? row.active_days : undefined,
    }));
  return foldDonutEntries(base, "last30");
}

function foldDonutEntries(all: DonutEntry[], tab: SpendTab): DonutEntry[] {
  const visible = all
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
  // all-Others ring says nothing. The dollar threshold only applies to
  // the dollar view: in the token views every provider with tokens earns
  // its own row (a cheap model can move millions of tokens under $10,
  // and hiding it would defeat the whole point of the view).
  if (config.spendMetric !== "cost") return visible;
  const limit = othersFoldUsd(tab);
  const small = visible.filter((e) => e.w.cost < limit);
  if (small.length === 0 || small.length === visible.length) return visible;

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
    parts: small.map((e) => ({ id: e.s.id, name: e.s.name, w: e.w })),
  };
  return [...visible.filter((e) => e.w.cost >= limit), others].sort(
    (a, b) => spendVal(b.w) - spendVal(a.w),
  );
}

function donutEntries(tab: SpendTab): DonutEntry[] {
  const all: DonutEntry[] = lastSpend
    .filter((s) => !isCardDisabled(s.id)) // disabled = gone everywhere
    .map((s) => ({ s, w: s[tab] }));
  return foldDonutEntries(all, tab);
}

function donutEntriesForRange(range: RangeTab): DonutEntry[] {
  return range === "d30" ? donutEntries("last30") : historyEntries(range);
}

/// Model-name normalization for the by-model spend view: strips relay
/// route prefixes ("api2/glm-5.3", "Command-Code-Goat/deepseek/…" → the
/// last path segment) and case-folds, so one model reached through
/// different tools/relays merges into a single slice.
function normalizeModelKey(model: string): string {
  let m = model.trim();
  const slash = m.lastIndexOf("/");
  if (slash !== -1) m = m.slice(slash + 1);
  return m.toLowerCase();
}

type ModelAgg = {
  name: string;
  w: SpendWindow;
  parts: { id: string; name: string; w: SpendWindow }[];
};

/// Merge per-tool model rows into per-model entries. `parts` carries the
/// per-tool breakdown so the hover card can show who burned what.
function aggregateModels(rows: { id: string; name: string; models: ModelSpend[] }[]): Map<string, ModelAgg> {
  const acc = new Map<string, ModelAgg>();
  for (const row of rows) {
    for (const m of row.models) {
      if (m.cost <= 0 && m.tokens <= 0) continue;
      const key = normalizeModelKey(m.model);
      let agg = acc.get(key);
      if (!agg) {
        agg = { name: m.model, w: { cost: 0, tokens: 0, models: [] }, parts: [] };
        acc.set(key, agg);
      }
      agg.w.cost += m.cost;
      agg.w.tokens += m.tokens;
      let part = agg.parts.find((p) => p.id === row.id);
      if (!part) {
        part = { id: row.id, name: row.name, w: { cost: 0, tokens: 0, models: [] } };
        agg.parts.push(part);
      }
      part.w.cost += m.cost;
      part.w.tokens += m.tokens;
    }
  }
  return acc;
}

function entriesFromModelAgg(acc: Map<string, ModelAgg>, tab: SpendTab): DonutEntry[] {
  const base: DonutEntry[] = [...acc.entries()].map(([key, v]) => ({
    s: { id: key, name: v.name } as ProviderSpend,
    w: v.w,
    parts: v.parts,
  }));
  return foldDonutEntries(base, tab);
}

/// By-model slices for the live windows (today/yesterday/last30).
function modelDonutEntries(tab: SpendTab): DonutEntry[] {
  const rows = lastSpend
    .filter((s) => !isCardDisabled(s.id))
    .map((s) => ({ id: s.id, name: s.name, models: s[tab].models }));
  return entriesFromModelAgg(aggregateModels(rows), tab);
}

function modelHistoryEntries(range: Exclude<RangeTab, "d30">): DonutEntry[] {
  const rows = (range === "d7" ? lastSpendHistory.d7 : lastSpendHistory.all)
    .filter((row) => !isCardDisabled(row.id))
    .map((row) => ({ id: row.id, name: providerNameForSpend(row.id), models: row.models }));
  return entriesFromModelAgg(aggregateModels(rows), "last30");
}

function modelEntriesForRange(range: RangeTab): DonutEntry[] {
  return range === "d30" ? modelDonutEntries("last30") : modelHistoryEntries(range);
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

/// Shrink the ring's center number when it outgrows the hole: base size
/// fits ≤maxChars, longer strings scale down linearly (floor 8).
function fitFontSize(text: string, base: number, maxChars: number): number {
  return text.length <= maxChars ? base : Math.max(8, Math.floor((base * maxChars) / text.length));
}

/// The metric a click (or right-click, reversed) moves to next — the Mac
/// menu's order: Cost, Cost/MTok, Tokens.
function nextSpendMetric(back: boolean): "cost" | "tokens" {
  const order: ("cost" | "tokens")[] = ["cost", "tokens"];
  const i = order.indexOf(config.spendMetric as "cost" | "tokens");
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

function legendRowHtml(e: DonutEntry): string {
  const detail = e.activeDays === undefined ? "" : ` <span class="legend-detail">${escapeHtml(t("spend.activeDays", { n: e.activeDays }))}</span>`;
  const icon = providerVisual(e.s.id)?.iconSvg;
  const lead = icon
    ? `<span class="legend-ico">${icon}</span>`
    : `<span class="dot" style="background:${spendColor(e.s.id)}"></span>`;
  return `
        <div class="legend-row" data-pid="${e.s.id}">
          ${lead}
          <span class="legend-name">${escapeHtml(e.s.name)}${estBadgeFor(e.s.id)}${detail}</span>
          <span class="legend-val">${fmtSpendVal(e.w)}</span>
        </div>`;
}

/// Hover detail for donut wedges and legend rows: brand icon + full name +
/// both exact metrics. Native <title> can't render icons, so the popup is
/// drawn; "Others" additionally lists its folded-in providers with icons.
let spendPopEl: HTMLDivElement | null = null;

function hideSpendPop(): void {
  spendPopEl?.remove();
  spendPopEl = null;
}

function showSpendPop(anchor: HTMLElement): void {
  const pid = anchor.dataset.pid ?? "";
  const byModel = config.spendGrouping === "model";
  const live = byModel ? modelDonutEntries(spendTab) : donutEntries(spendTab);
  const ranged = byModel ? modelEntriesForRange(rangeTab) : donutEntriesForRange(rangeTab);
  const entry = (rangeSelected ? ranged : live).find((e) => e.s.id === pid);
  hideSpendPop();
  if (!entry) return;
  const lead = (id: string, colorId = id) => {
    const icon = providerVisual(id)?.iconSvg;
    return icon
      ? `<span class="legend-ico">${icon}</span>`
      : `<span class="dot" style="background:${spendColor(colorId)}"></span>`;
  };
  const parts = (entry.parts ?? [])
    .map(
      (p) =>
        `<div class="spend-pop-row">${lead(p.id)}<span class="spend-pop-name">${escapeHtml(p.name)}</span><span class="spend-pop-val">${escapeHtml(fmtSpendVal(p.w))}</span></div>`,
    )
    .join("");
  const el = document.createElement("div");
  el.className = "spend-pop";
  el.innerHTML = `
    <div class="spend-pop-head">${lead(entry.s.id)}<span class="spend-pop-name">${escapeHtml(entry.s.name)}${estBadgeFor(entry.s.id)}</span></div>
    <div class="spend-pop-val">${escapeHtml(fmtMoney(entry.w.cost))} · ${escapeHtml(fmtTokens(entry.w.tokens))}</div>
    ${parts ? `<div class="spend-pop-sep"></div>${parts}` : ""}`;
  document.body.appendChild(el);
  spendPopEl = el;
  const r = anchor.getBoundingClientRect();
  let x = r.left + r.width / 2 - el.offsetWidth / 2;
  x = Math.max(6, Math.min(x, window.innerWidth - el.offsetWidth - 6));
  let y = r.top - el.offsetHeight - 6;
  if (y < 6) y = r.bottom + 6;
  el.style.left = `${x}px`;
  el.style.top = `${y}px`;
}

/// Zeroed window so a column's summed totals can ride fmtSpendVal.
function emptyWindow(): SpendWindow {
  return { cost: 0, tokens: 0, models: [] };
}

/// True while focus sits in something the user types into (form field,
/// inline editor) — bare-Shift board switching must not hijack their
/// capitals or the IME's own Shift handling.
function isTypingTarget(el: Element | null): boolean {
  return (
    el instanceof HTMLElement &&
    (el.tagName === "INPUT" ||
      el.tagName === "TEXTAREA" ||
      el.tagName === "SELECT" ||
      el.isContentEditable)
  );
}

function switchOverviewTab(tab: OverviewTab): void {
  if (overviewTab === tab) return;
  overviewTab = tab;
  void patchConfig({ overviewTab: tab });
  renderIfVisible();
}

function switchOverviewCategory(category: OverviewCategory): void {
  if (overviewCategory === category) return;
  overviewCategory = category;
  void patchConfig({ overviewCategory: category });
  renderIfVisible();
}

/// Tab switch morphs the existing arcs in place (identity-keyed per
/// provider, CSS-transitioned) instead of rebuilding the card.
function switchSpendTab(tab: SpendTab): void {
  spendTab = tab;
  rangeSelected = tab === "last30";
  void patchConfig({ spendTab });
  renderAll();
}

function localDayKey(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function spendDetailValue(row: { cost: number; tokens: number }): number {
  return config.spendMetric === "tokens" ? row.tokens : row.cost;
}

function spendDetailDayRows(day: string): DailySpendRow[] {
  return lastSpendDaily
    .filter((row) => row.day === day && !isCardDisabled(row.id))
    .sort((a, b) => spendDetailValue(b) - spendDetailValue(a));
}

function renderSpendDetailOverlay(): string {
  const today = new Date();
  const end = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  const start = new Date(end);
  // 26 Sunday-aligned weeks (~half a year): square cells shrink to the panel
  // width (ZCode's usage board layout) instead of forcing a horizontal scroll.
  const heatWeeks = 26;
  start.setDate(start.getDate() - (heatWeeks * 7 - 1) - start.getDay());
  const daily = new Map<string, { cost: number; tokens: number; providers: number }>();
  for (const row of lastSpendDaily) {
    if (isCardDisabled(row.id)) continue;
    const cell = daily.get(row.day) ?? { cost: 0, tokens: 0, providers: 0 };
    cell.cost += row.cost;
    cell.tokens += row.tokens;
    cell.providers += 1;
    daily.set(row.day, cell);
  }
  const values = [...daily.values()].map(spendDetailValue).filter((value) => value > 0);
  const maxValue = Math.max(...values, 1);
  const cells: string[] = [];
  for (let index = 0; index < heatWeeks * 7; index += 1) {
    const date = new Date(start);
    date.setDate(start.getDate() + index);
    const day = localDayKey(date);
    const value = daily.get(day);
    const amount = value ? spendDetailValue(value) : 0;
    const level = amount <= 0 ? 0 : Math.min(4, Math.max(1, Math.ceil((amount / maxValue) * 4)));
    const outside = date > end ? " outside" : "";
    const selected = day === spendDetailDay ? " selected" : "";
    const tip = value
      ? `${day} · ${fmtMoney(value.cost)} · ${fmtTokens(value.tokens)} · ${value.providers} tools`
      : `${day} · ${t("spendDetail.noUsage")}`;
    cells.push(`<button class="spend-heat-cell level-${level}${outside}${selected}" data-spend-detail-day="${day}" title="${escapeHtml(tip)}" aria-label="${escapeHtml(tip)}"></button>`);
  }
  // Month labels under the grid: one span per run of same-month columns, and
  // a column containing the 1st belongs to the new month (ZCode's rule).
  const monthSpans: string[] = [];
  let monthLabel = "";
  let monthSpan = 0;
  const flushMonth = () => {
    if (monthSpan > 0) {
      monthSpans.push(`<span style="grid-column: span ${monthSpan}">${escapeHtml(monthLabel)}</span>`);
    }
  };
  for (let week = 0; week < heatWeeks; week += 1) {
    let label = "";
    for (let d = 0; d < 7; d += 1) {
      const date = new Date(start);
      date.setDate(start.getDate() + week * 7 + d);
      if (date.getDate() === 1) {
        label = date.toLocaleDateString(localeTag(), { month: "short" });
        break;
      }
    }
    if (label && label !== monthLabel) {
      flushMonth();
      monthLabel = label;
      monthSpan = 1;
    } else {
      monthSpan += 1;
    }
  }
  flushMonth();
  const selectedDay = spendDetailDay || localDayKey(end);
  const rows = spendDetailDayRows(selectedDay);
  const selectedTotals = rows.reduce((sum, row) => ({ cost: sum.cost + row.cost, tokens: sum.tokens + row.tokens }), { cost: 0, tokens: 0 });
  const detailRows = rows.length
    ? rows.map((row) => {
        const models = row.models.map((model) => `<div class="spend-detail-model"><span>${escapeHtml(model.model)}</span><span>${escapeHtml(fmtSpendVal({ cost: model.cost, tokens: model.tokens, models: [] }))}</span></div>`).join("");
        return `<article class="spend-detail-tool"><div class="spend-detail-tool-head"><strong>${escapeHtml(providerNameForSpend(row.id))}</strong><span>${escapeHtml(fmtSpendVal({ cost: row.cost, tokens: row.tokens, models: [] }))}</span></div>${models || `<div class="spend-detail-model muted">${escapeHtml(t("spendDetail.noModelBreakdown"))}</div>`}</article>`;
      }).join("")
    : `<div class="spend-detail-empty">${escapeHtml(t("spendDetail.noUsage"))}</div>`;
  return `<div class="spend-detail-overlay" role="dialog" aria-modal="true" aria-labelledby="spend-detail-title">
    <section class="spend-detail-panel">
      <header class="spend-detail-header"><div><p class="spend-detail-kicker">${escapeHtml(t("spendDetail.kicker"))}</p><h2 id="spend-detail-title">${escapeHtml(t("spendDetail.title"))}</h2><p>${escapeHtml(t("spendDetail.subtitle"))}</p></div><button class="mini-btn" data-spend-detail-close>${uiIcon("x")}</button></header>
      <div class="spend-detail-summary"><div><span>${escapeHtml(selectedDay)}</span><strong>${escapeHtml(fmtMoney(selectedTotals.cost))}</strong></div><div><span>${escapeHtml(t("spendDetail.tokens"))}</span><strong>${escapeHtml(fmtTokens(selectedTotals.tokens))}</strong></div><div><span>${escapeHtml(t("spendDetail.tools"))}</span><strong>${rows.length}</strong></div></div>
      <div class="spend-heat-wrap"><div class="spend-heat-top"><div class="spend-heat-legend"><span>${escapeHtml(t("spendDetail.less"))}</span><i class="spend-heat-cell spend-heat-swatch"></i><i class="spend-heat-cell spend-heat-swatch level-1"></i><i class="spend-heat-cell spend-heat-swatch level-2"></i><i class="spend-heat-cell spend-heat-swatch level-3"></i><i class="spend-heat-cell spend-heat-swatch level-4"></i><span>${escapeHtml(t("spendDetail.more"))}</span></div></div><div class="spend-heat-grid">${cells.join("")}</div><div class="spend-heat-months">${monthSpans.join("")}</div></div>
      <div class="spend-detail-day-head"><h3>${escapeHtml(selectedDay)}</h3><span>${escapeHtml(t("spendDetail.dayHint"))}</span></div>
      <div class="spend-detail-tools">${detailRows}</div>
    </section>
  </div>`;
}

function renderTotalSpend(): string {
  if (!config.showTotalSpend) return "";
  const isFolded = isSpendFolded();
  /// Grouping-aware entry sources: by tool (default) or by model, where
  /// the same model through different tools merges into one slice.
  const groupedEntries = (tab: SpendTab): DonutEntry[] =>
    config.spendGrouping === "model" ? modelDonutEntries(tab) : donutEntries(tab);
  const groupedEntriesForRange = (range: RangeTab): DonutEntry[] =>
    config.spendGrouping === "model" ? modelEntriesForRange(range) : donutEntriesForRange(range);
  const foldChevron = isFolded
    ? `<button class="card-fold-toggle" data-spend-fold title="${escapeHtml(t("card.expand"))}">${uiIcon("caretDown")}</button>`
    : `<button class="card-fold-toggle" data-spend-fold title="${escapeHtml(t("card.collapse"))}">${uiIcon("caretUp")}</button>`;
  const entries = rangeSelected ? groupedEntriesForRange(rangeTab) : groupedEntries(spendTab);
  if (lastSpend.length === 0 && lastSpendHistory.d7.length === 0 && lastSpendHistory.all.length === 0) {
    // Quiet state instead of a missing card — on a fresh PC the donut only
    // appears after a CLI (Claude Code, Codex, Grok…) has logged some usage.
    const note = spendLoaded ? t("spend.emptyFirst") : t("spend.scanning");
    return `
      <article class="provider total-spend${isFolded ? " is-folded" : ""}">
        <div class="provider-head">
          <span class="provider-name">${escapeHtml(t("spend.title"))}</span>
          <span class="spacer"></span>
          <button class="mini-btn spend-detail-btn" data-spend-details title="${escapeHtml(t("spendDetail.open"))}" aria-label="${escapeHtml(t("spendDetail.open"))}">${uiIcon("rows")}</button>
          ${foldChevron}
        </div>
        ${isFolded ? "" : `<div class="card-panel"><p class="placeholder" style="margin:4px 0">${note}</p></div>`}
      </article>`;
  }

  const { geo } = donutGeometry(entries);
  const segments = entries
    .filter((e) => geo.has(e.s.id))
    .map((e) => {
      const g = geo.get(e.s.id)!;
      const pop = donutPop(g);
      const full = g.a1 - g.a0 >= TAU - 0.0001 ? ` data-full="1"` : "";
      return `<path class="seg" data-pid="${e.s.id}"${full} fill-rule="evenodd"
        d="${sectorPath(g.a0, g.a1)}" style="fill:${spendColor(e.s.id)};--tx:${pop.tx};--ty:${pop.ty}"></path>`;
    })
    .join("");

  /// One period column: header (switches the donut), the window total in
  /// the active metric, and every provider that spent in it. Columns
  /// reuse the legend-row styles so wedge hover keeps lighting them up.
  /// Long totals shrink one step so wide numbers (e.g. "$1,234.56 · 12.3M")
  /// don't blow out the fixed-width column.
  const colTotalHtml = (w: SpendWindow) => {
    const s = fmtSpendVal(w);
    return `<div class="col-total${s.length > 10 ? " long" : ""}" title="${escapeHtml(s)}">${escapeHtml(s)}</div>`;
  };

  const periodControls = `
    <button class="tab col-head${!rangeSelected && spendTab === "today" ? " active" : ""}" data-tab="today">${t("spend.today")}</button>
    <button class="tab col-head${!rangeSelected && spendTab === "yesterday" ? " active" : ""}" data-tab="yesterday">${t("spend.yesterday")}</button>
    ${(["d7", "d30", "all"] as const).map((id) => `<button class="tab col-head${rangeSelected && rangeTab === id ? " active" : ""}" data-range-tab="${id}">${t(id === "d7" ? "spend.days7" : id === "d30" ? "spend.days30" : "spend.rangeAll")}</button>`).join("")}`;

  const rangeCol = () => {
    const colEntries = groupedEntriesForRange(rangeTab);
    const totals = colEntries.reduce(
      (acc, e) => ({ cost: acc.cost + e.w.cost, tokens: acc.tokens + e.w.tokens }),
      { cost: 0, tokens: 0 },
    );
    const shown = colEntries.slice(0, 9);
    const overflow = colEntries.length - shown.length;
    return `
      <div class="spend-col spend-range-col">
        <div class="col-head-row">
          ${periodControls}
        </div>
        ${colTotalHtml({ ...emptyWindow(), ...totals, models: [] })}
        <div class="legend">
          ${shown.map((e) => legendRowHtml(e)).join("")}
          ${overflow > 0 ? `<div class="legend-row more">+${overflow}</div>` : ""}
        </div>
      </div>`;
  };

  /// The left column shows today or yesterday — whichever is selected
  /// (last30 keeps the ring on the right column while the left falls back
  /// to today).
  const leftTab: SpendTab = spendTab === "yesterday" ? "yesterday" : "today";
  const leftEntries = groupedEntries(leftTab);
  const leftTotals = leftEntries.reduce(
    (acc, e) => ({ cost: acc.cost + e.w.cost, tokens: acc.tokens + e.w.tokens }),
    { cost: 0, tokens: 0 },
  );
  const leftShown = leftEntries.slice(0, 10);
  const leftOverflow = leftEntries.length - leftShown.length;
  const leftColHtml = `
      <div class="spend-col">
        <div class="col-head-row">
          ${periodControls}
        </div>
        ${colTotalHtml({ ...emptyWindow(), ...leftTotals, models: [] })}
        <div class="legend">
          ${leftShown.map((e) => legendRowHtml(e)).join("")}
          ${leftOverflow > 0 ? `<div class="legend-row more">+${leftOverflow}</div>` : ""}
        </div>
      </div>`;

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
          <text class="donut-total" x="48" y="50" text-anchor="middle" font-size="${fitFontSize(center.primary, 14, 7)}" font-weight="600">${center.primary}</text>
          <text class="donut-sub" x="48" y="62" text-anchor="middle" font-size="${fitFontSize(center.sub, 8, 12)}">${center.sub}</text>
        </svg>
        <div class="spend-cols">
          ${rangeSelected ? rangeCol() : leftColHtml}
        </div>
      </div>`
    : `
      <div class="donut-wrap donut-empty" title="${escapeHtml(t("spend.emptyPeriodTip"))}">
        <svg width="96" height="96" viewBox="0 0 96 96">
          <path class="seg donut-zero" data-full="1" fill-rule="evenodd" d="${sectorPath(0, TAU)}"/>
          <text class="donut-total" x="48" y="50" text-anchor="middle" font-size="${fitFontSize(center.primary, 14, 7)}" font-weight="600">${center.primary}</text>
          <text class="donut-sub" x="48" y="62" text-anchor="middle" font-size="${fitFontSize(center.sub, 8, 12)}">${center.sub}</text>
        </svg>
        <div class="spend-cols">
          ${rangeSelected ? rangeCol() : leftColHtml}
        </div>
      </div>`;

  const contributors = lastSpend.map((s) => s.name).join(", ");
  // The ⚡ stamp mirrors the selected window below (today/yesterday/7d/30d/all)
  // — the old standalone head-range tabs duplicated that choice.
  const headRangeKey = rangeSelected
    ? rangeTab === "d7" ? "spend.days7" : rangeTab === "d30" ? "spend.days30" : "spend.rangeAll"
    : spendTab === "yesterday" ? "spend.yesterday" : "spend.today";
  const headTokens = entries.reduce((sum, e) => sum + e.w.tokens, 0);
  // ⚡ tier by daily-equivalent usage: the built-in ladder is 100M / 250M /
  // 500M tokens per day; the 7d/30d windows rescale by their day count
  // (700M/week reads "medium"). config.spendIconTiers overrides the ladder.
  const windowDays = rangeSelected ? (rangeTab === "d7" ? 7 : 30) : 1;
  const tiers = config.spendIconTiers ?? { medium: 100_000_000, high: 250_000_000, max: 500_000_000 };
  const boltTier =
    headTokens >= tiers.max * windowDays ? 3
    : headTokens >= tiers.high * windowDays ? 2
    : headTokens >= tiers.medium * windowDays ? 1
    : 0;
  // Gradient-filled bolt per tier (emerald→cyan / amber→orange / violet→
  // fuchsia→cyan) so the stamp reads at a glance in both light and dark.
  const boltSvg = (tier: number): string => {
    const svg = uiIcon("lightning");
    if (tier === 0) return svg;
    const stops = tier === 1
      ? ["#34d399", "#22d3ee"]
      : tier === 2
        ? ["#fbbf24", "#f97316"]
        : ["#a78bfa", "#e879f9", "#22d3ee"];
    const stopsHtml = stops
      .map((c, i) => `<stop offset="${Math.round((i / (stops.length - 1)) * 100)}%" stop-color="${c}"/>`)
      .join("");
    return svg
      .replace('fill="currentColor"', `fill="url(#bolt-grad-${tier})"`)
      .replace(/(<svg[^>]*>)/, `$1<defs><linearGradient id="bolt-grad-${tier}" x1="0" y1="0" x2="1" y2="1">${stopsHtml}</linearGradient></defs>`);
  };
  const boltTip = boltTier > 0
    ? t(`spend.boltTier${boltTier}` as "spend.boltTier1", { n: fmtTokens((boltTier === 1 ? tiers.medium : boltTier === 2 ? tiers.high : tiers.max) * windowDays) })
    : "";
  const boltHtml = `<span class="spend-bolt tier-${boltTier}"${boltTip ? ` title="${escapeHtml(boltTip)}"` : ""}>${boltSvg(boltTier)}</span>`;
  return `
    <article class="provider total-spend${isFolded ? " is-folded" : ""}">
      <div class="provider-head">
        <span class="provider-name">${escapeHtml(t("spend.title"))}</span>
        <span class="spend-head-value" title="${escapeHtml(t("spend.headTokens", { range: t(headRangeKey) }))}">${boltHtml}${escapeHtml(fmtTokens(headTokens))}</span>
        ${isFolded ? "" : `<span class="info" title="${escapeHtml(t("spend.info", { names: contributors }))}">${uiIcon("info")}</span>`}
        <span class="spacer"></span>
        ${isFolded ? "" : `
        <div class="tabs spend-group-tabs" role="group" aria-label="${escapeHtml(t("spend.groupLabel"))}">
          <button class="tab spend-group-tab${config.spendGrouping === "tool" ? " active" : ""}" data-spend-group="tool">${escapeHtml(t("spend.byTool"))}</button>
          <button class="tab spend-group-tab${config.spendGrouping === "model" ? " active" : ""}" data-spend-group="model">${escapeHtml(t("spend.byModel"))}</button>
        </div>
        <div class="spend-metric-tabs" role="group" aria-label="${escapeHtml(t("spend.metricLabel"))}">
          ${(["cost", "tokens"] as const).map((metric) => `<button class="tab spend-metric-tab${config.spendMetric === metric ? " active" : ""}" data-spend-metric="${metric}">${escapeHtml(t(METRIC_NAMES[metric]))}</button>`).join("")}
        </div>`}
        ${updateVersion ? `<button class="spend-update-btn${updatePushing ? " busy" : ""}" data-update-push${updatePushing && updatePct != null ? ` style="--pct:${updatePct}"` : ""} title="${escapeHtml(t("update.pushTip", { version: updateVersion }))}" aria-label="${escapeHtml(t("update.pushTip", { version: updateVersion }))}">${uiIcon("arrowUp")}</button>` : ""}
        <button class="mini-btn spend-detail-btn" data-spend-details title="${escapeHtml(t("spendDetail.open"))}" aria-label="${escapeHtml(t("spendDetail.open"))}">${uiIcon("rows")}</button>
        <button class="share-btn" data-share="__total__" title="${escapeHtml(t("card.share"))}">${uiIcon("shareNetwork")}</button>
        ${foldChevron}
      </div>
      ${isFolded ? "" : `<div class="card-panel">
        ${body}
      </div>`}
    </article>`;
}

// ---------------------------------------------------------------------------
// Quota & Status Overview module
// Displays availability status and each provider's most binding quota
// (circular SVG progress). Default / "5h" tab: session window when one
// is reported, otherwise the shortest-period usage percent. "Week" tab:
// weekly meters for ordinary families; Z.ai / One/New API / Copilot keep
// the default binding. "Month" selects a reported monthly meter, falling
// back to the default binding when absent. Maxed (100%) renders in red.
// ---------------------------------------------------------------------------

type QuotaWindow = "5h" | "day" | "week" | "month" | "generic";

const windowLabelKey: Record<QuotaWindow, string> = {
  "5h": "overview.win5h",
  day: "overview.winDay",
  week: "overview.winWeek",
  month: "overview.winMonth",
  generic: "overview.winGeneric",
};

interface OverviewQuota {
  // Period the shown percent belongs to; null = the snapshot carries no
  // usable percentage at all (ring renders as "—").
  window: QuotaWindow | null;
  usedPercent: number;
  resetsAt: number | null;
  metricLabel: string;
  // The shown metric's own "x / y" line, surfaced on overview tiles so a
  // bare 0% still reads with its total (MCP/search cards).
  metricDetail: string | null;
  // Text-metric value ("¥0.00 …" balance rows): tiles render this instead
  // of "no usage data" when the snapshot has no percent metric.
  valueText: string | null;
  periodFallback: boolean;
  isMaxed: boolean;
  status: "ok" | "maxed" | "error" | "no_data" | "text";
}

function extractOverviewQuota(s: Snapshot, cardIsMaxed = false, cardId = ""): OverviewQuota {
  if (s.status !== "ok") {
    return {
      window: null,
      usedPercent: 0,
      resetsAt: null,
      metricLabel: "",
      metricDetail: null,
      valueText: null,
      periodFallback: false,
      isMaxed: false,
      status: "error",
    };
  }

  const percents = (s.metrics || []).filter(
    (m) => isCoreQuotaMetric(m) && m.used_percent !== null,
  );
  // Session-window metrics win: they are the most binding quota a provider
  // reports. Week/month labels stay excluded even when a metric happens to
  // carry a short period.
  const sessionPercents = percents.filter(
    (m) =>
      (is5hPeriod(m.period_ms) || is5hLabel(m.label)) && !/week|month|year/i.test(m.label),
  );

  const pickBest = (list: Metric[]): Metric =>
    list.reduce((prev, curr) =>
      (curr.used_percent ?? 0) > (prev.used_percent ?? 0) ? curr : prev,
    );

  const pickOverview = (list: Metric[]): Metric => {
    const primary = OVERVIEW_PRIMARY_LABEL[providerFamily(s.id)];
    if (primary) {
      const preferred = list.find((m) => m.label === primary);
      if (preferred) return preferred;
    }
    return pickBest(list);
  };

  const family = providerFamily(s.id);
  const preferWeek = overviewTab === "week" && !OVERVIEW_KEEP_BINDING.has(family);
  const weekPercents = percents.filter((m) => metricWindow(m) === "week");
  const monthPercents = percents.filter((m) => metricWindow(m) === "month");

  let best: Metric | null = null;
  let periodFallback = false;
  if (overviewTab === "month" && monthPercents.length > 0) {
    best = pickOverview(monthPercents);
  } else if (overviewTab === "month" && weekPercents.length > 0) {
    // Some providers expose only a rolling session and weekly window. Keep
    // the ring useful and make the fallback explicit in the tile metadata.
    best = pickOverview(weekPercents);
    periodFallback = true;
  } else if (preferWeek && weekPercents.length > 0) {
    best = pickOverview(weekPercents);
  } else if (sessionPercents.length > 0) {
    best = pickOverview(sessionPercents);
  } else if (percents.length > 0) {
    // Shortest period first (untimed metrics last). Independent sibling
    // bars (Cursor Auto vs API) do not collapse to the fuller one — the
    // family's primary pool wins when it is in the shortlist.
    const shortest = percents.reduce((prev, curr) =>
      (curr.period_ms ?? Infinity) < (prev.period_ms ?? Infinity) ? curr : prev,
    );
    best = pickOverview(
      percents.filter((m) => (m.period_ms ?? Infinity) === (shortest.period_ms ?? Infinity)),
    );
  }

  if (best) {
    const usedPercent = Math.min(100, Math.max(0, best.used_percent ?? 0));
    // The ring follows the metric it displays — a sibling pool at 100%
    // must not force this one to a red 100. Fold state (`cardIsMaxed`)
    // only fills in a reset time when the shown bar itself is spent.
    const isMaxed = usedPercent >= MAXED_PCT || (!!s.plan && /out of credit/i.test(s.plan));
    const resetSecs = cardIsMaxed && cardId ? nearestResetSeconds(cardId) : 0;
    const resetsAt =
      best.resets_at ?? (resetSecs > 0 ? Date.now() + resetSecs * 1000 : null);
    return {
      window: metricWindow(best),
      usedPercent,
      resetsAt,
      metricLabel: best.label,
      metricDetail: best.detail ?? null,
      valueText: null,
      periodFallback,
      isMaxed,
      status: isMaxed ? "maxed" : "ok",
    };
  }

  // No percent metric, but a text row ("¥12.34" balances, MCP notes)?
  // Surface its value so tiles read real content instead of 无用量数据.
  const textMetric = (s.metrics || []).find(
    (m) => m.kind === "text" && (m.value ?? m.detail),
  );
  if (textMetric) {
    return {
      window: null,
      usedPercent: 0,
      resetsAt: null,
      metricLabel: textMetric.label,
      metricDetail: textMetric.detail ?? null,
      valueText: textMetric.value ?? textMetric.detail ?? null,
      periodFallback: false,
      isMaxed: false,
      status: "text",
    };
  }

  return {
    window: null,
    usedPercent: 0,
    resetsAt: null,
    metricLabel: "",
    metricDetail: null,
    valueText: null,
    periodFallback: false,
    isMaxed: false,
    status: "no_data",
  };
}

function overviewFailureLabel(s: Snapshot): string {
  if (s.status === "no_credentials") return t("overview.needsCredentials");
  if (/expired|cookie|sign in|log in|unauthori[sz]ed|401|403/i.test(s.error ?? "")) return t("overview.needsLogin");
  return t("overview.queryFailed");
}

function overviewWindowLabel(quota: OverviewQuota): string {
  if (quota.periodFallback) return t("overview.monthFallback");
  return quota.window ? t(windowLabelKey[quota.window]) : t("overview.winGeneric");
}

function isOverviewCollapsed(): boolean {
  return config.layout?.overviewCollapsed ?? false;
}

function isSpendFolded(): boolean {
  return config.layout?.spendCollapsed ?? false;
}

interface OverviewItem {
  cardSnap: Snapshot;
  shownSnap: Snapshot;
  quota: OverviewQuota;
  accountSummary: { total: number; available: number; unavailable: number };
}

/// A grouped overview panel: untagged cards first, then one sub-header per
/// card group. Status stays on each card, so a group does not split when a
/// provider reaches its limit and later recovers.
function overviewSectionHtml(
  title: string,
  tone: "ok" | "down",
  items: OverviewItem[],
  render: (it: OverviewItem) => string,
): string {
  const groups = usedCardGroups();
  const usedIds = new Set(groups.map((g) => g.id));
  const byGroup = new Map<string, OverviewItem[]>();
  const flat: OverviewItem[] = [];
  for (const it of items) {
    // Stale tags (group deleted, card still points at it) render flat.
    const gid = cardGroupId(it.cardSnap.id);
    if (!gid || !usedIds.has(gid)) flat.push(it);
    else byGroup.set(gid, [...(byGroup.get(gid) ?? []), it]);
  }
  const grid = (list: OverviewItem[]) =>
    `<div class="overview-grid${config.overviewStyle === "bars" ? " ovbars" : ""}">${list.map(render).join("")}</div>`;
  let html = title
    ? `<div class="overview-section-head tone-${tone}">
        <span class="overview-section-title">${escapeHtml(title)}</span>
        <span class="overview-section-count">${items.length}</span>
      </div>`
    : "";
  if (flat.length > 0) html += grid(flat);
  for (const g of groups) {
    const members = byGroup.get(g.id) ?? [];
    if (members.length === 0) continue;
      html += `<div class="overview-subgroup-head" data-overview-drop-group="${escapeHtml(g.id)}">
        <span class="overview-subgroup-name">${escapeHtml(g.name)}</span>
        <span class="overview-subgroup-count">${members.length}</span>
      </div>${grid(members)}`;
  }
  return html;
}

function renderQuotaOverview(): string {
  const visibleSnaps = overviewSnapshots();
  if (visibleSnaps.length === 0) return "";
  // Category scope: rings, sections, badges and the expiring list all
  // follow the active top-level category. The article still renders when
  // the active category has no cards, so the switch stays reachable.
  const scopedSnaps = visibleSnaps.filter(
    (s) => effectiveCategory(providerFamily(s.id)) === overviewCategory,
  );

  const isFolded = isOverviewCollapsed();

  const items: OverviewItem[] = scopedSnaps.map((s) => {
    const shown = pickOverviewShown(s);
    const cardIsMaxed = isCardFoldCandidate(s.id);
    const quota = extractOverviewQuota(shown, cardIsMaxed, s.id);
    // A card walled by a row the ring isn't showing (Kimi monthly-capped
    // while its 5h window is fresh) must not advertise the healthy
    // window's reset in the soonest-reset list — the reset that matters
    // is the wall's. No wall reset parseable → no known reset at all.
    if (isSnapshotMaxed(shown)) {
      const wallSecs = nearestResetSeconds(s.id);
      quota.resetsAt = wallSecs > 0 ? Date.now() + wallSecs * 1000 : null;
    }
    return { cardSnap: s, shownSnap: shown, quota, accountSummary: accountAvailabilitySummary(providerFamily(s.id)) };
  });

  const totalCount = items.length;
  const errorCount = items.filter((it) => it.quota.status === "error").length;

  // Section split mirrors the red dot exactly: a card is 不可用 when ANY
  // core quota row is exhausted (isSnapshotMaxed — the ring's own window
  // may still read a healthy 0%, e.g. Kimi monthly-capped while its 5h
  // window is fresh) or the snapshot errored. 可用 = everything else.
  // Badges count the same predicates, so chips, dots and sections can
  // never disagree. An empty section renders nothing.
  const isDown = (it: OverviewItem) =>
    it.quota.status === "error" || isSnapshotMaxed(it.shownSnap);
  const upItems = items.filter((it) => !isDown(it));
  const maxedCount = totalCount - errorCount - upItems.length;

  // Peak tint lives INSIDE the available set — same predicate that
  // yellows the tile dot, so the chip, the dots and the section never
  // disagree. 可用 counts every up tile (in-peak included); the yellow
  // 高峰 chip counts the in-peak subset. Both always render (a 0 tells
  // the user the dimension exists even off-peak).
  const inPeak = (it: OverviewItem) => isProviderInPeak(providerFamily(it.cardSnap.id));
  const peakItems = upItems.filter(inPeak);
  const availableCount = upItems.length;

  // "Soonest reset" view: EVERY non-error provider gets a row. Timed ones
  // lead, soonest at the top; providers without a reset instant (not
  // started, generic meters, walls carrying no reset date) follow as a
  // muted tail — walled cards first there, since they are blocked right
  // now and the row says so instead of inventing a countdown.
  const timedItems = items
    .filter((it) => it.quota.status !== "error" && it.quota.resetsAt !== null)
    .sort((a, b) => (a.quota.resetsAt ?? 0) - (b.quota.resetsAt ?? 0));
  const pendingItems = items
    .filter((it) => it.quota.status !== "error" && it.quota.resetsAt === null)
    .sort(
      (a, b) =>
        Number(isSnapshotMaxed(b.shownSnap)) - Number(isSnapshotMaxed(a.shownSnap)),
    );

  const accountMetersHtml = (family: string): string => {
    const accounts = overviewAccountSnapshots(family);
    if (accounts.length <= 1) return "";
    return `<div class="overview-account-meters">${accounts.map((account) => {
      const q = extractOverviewQuota(account, isSnapshotMaxed(account), account.id);
      const label = labelForAccount(account.id, accountsCache.get(family) ?? []);
      const tone = accountHealthDot(account.id);
      const pct = q.status === "maxed" ? 100 : q.window !== null ? Math.round(q.usedPercent) : 0;
      return `<div class="overview-account-meter" title="${escapeHtml(label)} · ${pct}%"><span class="acct-dot ${tone}"></span><span class="overview-account-meter-label">${escapeHtml(label)}</span><span class="overview-account-meter-track"><i class="${tone}" style="width:${pct}%"></i></span><b>${pct}%</b></div>`;
    }).join("")}</div>`;
  };

  const accountRingsHtml = (family: string): string => {
    const accounts = overviewAccountSnapshots(family);
    if (accounts.length <= 1) return "";
    const circumference = 2 * Math.PI * 10;
    return `<div class="overview-account-rings">${accounts.map((account) => {
      const q = extractOverviewQuota(account, isSnapshotMaxed(account), account.id);
      const tone = accountHealthDot(account.id);
      const pct = q.status === "maxed" ? 100 : q.window !== null ? Math.round(q.usedPercent) : 0;
      const label = labelForAccount(account.id, accountsCache.get(family) ?? []);
      const stroke = tone === "red" ? "#ef4444" : tone === "error" ? "#f97316" : tone === "gray" ? "var(--muted-foreground)" : "#10b981";
      const offset = circumference * (1 - pct / 100);
      return `<div class="overview-account-ring" title="${escapeHtml(label)} · ${pct}%"><svg viewBox="0 0 28 28" aria-hidden="true"><circle cx="14" cy="14" r="10" class="ring-track" stroke-width="2.4" fill="none"/><circle cx="14" cy="14" r="10" stroke="${stroke}" stroke-width="2.4" fill="none" stroke-linecap="round" stroke-dasharray="${circumference.toFixed(2)}" stroke-dashoffset="${offset.toFixed(2)}" transform="rotate(-90 14 14)"/></svg><span>${escapeHtml(label)}</span></div>`;
    }).join("")}</div>`;
  };

  /// Multi-account overview tiles fold by default: the main account's
  /// ring/bar plus the N/available badge stay, per-account rings/meters hide
  /// until the user expands the family (remembered in config).
  const overviewAcctFolded = (family: string, accountCount: number): boolean =>
    accountCount > 1 && !(config.overviewExpanded ?? []).includes(family);

  const overviewAcctFoldBtn = (family: string, accountCount: number): string => {
    if (accountCount <= 1) return "";
    const folded = overviewAcctFolded(family, accountCount);
    return `<button type="button" class="overview-acct-fold" data-overview-acct-fold="${escapeHtml(family)}" title="${escapeHtml(t(folded ? "card.expand" : "card.collapse"))}" aria-label="${escapeHtml(t(folded ? "card.expand" : "card.collapse"))}">${uiIcon(folded ? "caretDown" : "caretUp")}</button>`;
  };

  const itemHtml = ({ cardSnap, shownSnap, quota, accountSummary }: OverviewItem): string => {
      const family = providerFamily(cardSnap.id);
      const jumpId = isParallelAccountFamily(family) ? shownSnap.id : cardSnap.id;
      const origin = shownSnap.dashboard_url ?? undefined;
      const visual = providerVisual(jumpId || family, origin);
      const icon = visual?.iconSvg ?? `<span class="icon-fallback">${escapeHtml(cardSnap.name.slice(0, 2))}</span>`;
      const displayName = notedName(jumpId, providerDisplayName(family) || cardSnap.name);
      const accountCount = overviewAccountCount(family);
      const accountBadge = accountCount > 1
        ? `<span class="overview-account-count ${accountSummary.available === 0 ? "is-empty" : ""}" title="${escapeHtml(t("overview.accountSummary", { available: accountSummary.available, unavailable: accountSummary.unavailable, total: accountSummary.total }))}"><b>${accountCount}/${accountSummary.available}</b></span>`
        : "";

      const r = 16;
      const cx = 22;
      const cy = 22;
      const circumference = 100.53; // 2 * Math.PI * 16

      let strokeColor = "var(--border)";
      let progressCircle = "";
      let ringLabel = "—";
      let textClass = "";
      let itemTone = "normal";
      let statusDot = "green";
      if (quota.status === "error") {
        itemTone = "error";
        statusDot = "error";
        ringLabel = "!";
        textClass = "is-error";
        progressCircle = `<circle class="ring-progress is-error" cx="${cx}" cy="${cy}" r="${r}" stroke="#f97316" stroke-width="3.2" fill="none" />`;
      } else if (quota.isMaxed) {
        itemTone = "maxed";
        statusDot = "red";
        strokeColor = "#ef4444";
        ringLabel = "100%";
        textClass = "is-maxed";
        progressCircle = `<circle class="ring-progress is-maxed" cx="${cx}" cy="${cy}" r="${r}"
          stroke="${strokeColor}" stroke-width="3.2" fill="none"
          stroke-linecap="round"
          stroke-dasharray="${circumference.toFixed(2)}"
          stroke-dashoffset="0"
          transform="rotate(-90 ${cx} ${cy})" />`;
      } else if (quota.window !== null) {
        const pct = Math.round(quota.usedPercent);
        const dashoffset = circumference * (1 - pct / 100);

        if (pct >= 80) {
          itemTone = "warn";
          statusDot = "green";
          strokeColor = "#f59e0b";
          ringLabel = `${pct}%`;
        } else {
          itemTone = "normal";
          statusDot = "green";
          strokeColor = "#10b981";
          ringLabel = `${pct}%`;
        }

        progressCircle = `<circle class="ring-progress ${itemTone}" cx="${cx}" cy="${cy}" r="${r}"
          stroke="${strokeColor}" stroke-width="3.2" fill="none"
          stroke-linecap="round"
          stroke-dasharray="${circumference.toFixed(2)}"
          stroke-dashoffset="${dashoffset.toFixed(2)}"
          transform="rotate(-90 ${cx} ${cy})" />`;
      } else {
        statusDot = "green";
        ringLabel = "—";
        textClass = "is-nodata";
      }

      // Any maxed core row (e.g. a maxed monthly cap) means the provider
      // is walled off even when the ring's own window looks healthy —
      // flag the dot so the tile doesn't read as available.
      if (statusDot !== "error" && isSnapshotMaxed(shownSnap)) statusDot = "red";

      // Peak-hours tint: available (green) tiles inside the provider's
      // peak window read yellow; red (maxed / error) stays red.
      if (statusDot === "green" && isProviderInPeak(family)) statusDot = "yellow";

      // The user asked for the multiplier rule on hover: the peak window,
      // what it costs now, and how much cheaper the off-peak hours are.
      const peakRule = PEAK_RULES[family];
      let fullTooltip = overviewHoverTip(shownSnap, quota, displayName);
      if (peakRule) {
        const peakPrefix = isProviderInPeak(family) ? `${t("peak.now")} ` : "";
        fullTooltip += ` · ${peakPrefix}${t(peakRule.tipKey)}`;
      }
      return `
        <div class="overview-item tone-${itemTone}" data-jump-provider="${escapeHtml(jumpId)}" title="${escapeHtml(fullTooltip)} · ${escapeHtml(t("overview.groupHint"))}">
          <div class="overview-item-head">
            <span class="overview-item-icon">${icon}</span>
            <span class="overview-item-name" ${compactLabelStyle(displayName, 10.5, 8.5)} title="${escapeHtml(displayName)}">${escapeHtml(displayName)}</span>
            ${accountBadge}
            ${overviewAcctFoldBtn(family, accountCount)}
            <span class="overview-dot ${statusDot}"></span>
          </div>
          <div class="overview-ring-wrap">
            <svg width="44" height="44" viewBox="0 0 44 44" class="overview-ring">
              <circle class="ring-track" cx="${cx}" cy="${cy}" r="${r}" stroke="var(--border)" stroke-width="3.2" fill="none" opacity="0.4" />
              ${progressCircle}
              <text class="ring-text ${textClass}" x="${cx}" y="${cy + 4}" text-anchor="middle">${ringLabel}</text>
            </svg>
          </div>
          ${overviewAcctFolded(family, accountCount) ? "" : accountRingsHtml(family)}
          ${overviewAcctFolded(family, accountCount) ? "" : accountMetersHtml(family)}
          <div class="overview-item-foot">
            <span class="overview-item-meta ${quota.isMaxed ? 'meta-maxed' : ''}"${quota.resetsAt ? ` data-reset-at="${quota.resetsAt}"` : ""}>
              ${quota.status === "error" ? escapeHtml(overviewFailureLabel(shownSnap)) : quota.isMaxed
                ? (quota.resetsAt
                    ? escapeHtml(fmtDuration(Math.max(0, quota.resetsAt - Date.now())))
                    : escapeHtml(t("overview.maxedBadge", { n: "" }).trim()))
                : quota.window !== null
                  ? (quota.resetsAt
                      ? escapeHtml(fmtDuration(Math.max(0, quota.resetsAt - Date.now())))
                      : quota.window === "5h"
                        ? escapeHtml(t("card.notStarted"))
                        : escapeHtml(overviewWindowLabel(quota)))
                  : escapeHtml(t("overview.noData"))}
            </span>
          </div>
        </div>`;
  };

  /// Compact two-line bar alternative to the ring tiles: slimmer rows in a
  /// two-column grid — line 1 = icon/name/percent, line 2 = thin bar plus
  /// the reset countdown or the relevant window state.
  const barItemHtml = ({ cardSnap, shownSnap, quota, accountSummary }: OverviewItem): string => {
    const family = providerFamily(cardSnap.id);
    const jumpId = isParallelAccountFamily(family) ? shownSnap.id : cardSnap.id;
    const origin = shownSnap.dashboard_url ?? undefined;
    const visual = providerVisual(jumpId || family, origin);
    const icon = visual?.iconSvg ?? `<span class="icon-fallback">${escapeHtml(cardSnap.name.slice(0, 2))}</span>`;
    const displayName = notedName(jumpId, providerDisplayName(family) || cardSnap.name);
    const nameClass = cardNote(jumpId) ? " is-note" : "";
    const accountCount = overviewAccountCount(family);
    const accountBadge = accountCount > 1
      ? `<span class="overview-account-count ${accountSummary.available === 0 ? "is-empty" : ""}" title="${escapeHtml(t("overview.accountSummary", { available: accountSummary.available, unavailable: accountSummary.unavailable, total: accountSummary.total }))}"><b>${accountCount}/${accountSummary.available}</b></span>`
      : "";

    let itemTone = "normal";
    let pct: number | null = null;
    let pctClass = "";
    if (quota.status === "error") {
      itemTone = "error";
      pctClass = "is-error";
    } else if (quota.isMaxed) {
      itemTone = "maxed";
      pct = 100;
      pctClass = "is-maxed";
    } else if (quota.window !== null) {
      pct = Math.round(quota.usedPercent);
      if (pct >= 80) itemTone = "warn";
    }

    const meta = quota.status === "error"
      ? overviewFailureLabel(shownSnap)
      : quota.resetsAt
        ? fmtDuration(Math.max(0, quota.resetsAt - Date.now()))
        : quota.window === "5h"
          ? t("card.notStarted")
          : quota.window !== null
            ? overviewWindowLabel(quota)
            : t("overview.noData");

    let statusDot = "green";
    if (quota.status === "error") statusDot = "error";
    else if (quota.isMaxed) statusDot = "red";
    if (statusDot === "green" && isSnapshotMaxed(shownSnap)) statusDot = "red";
    if (statusDot === "green" && isProviderInPeak(family)) statusDot = "yellow";

    let fullTooltip = overviewHoverTip(shownSnap, quota, displayName);
    const peakRule = PEAK_RULES[family];
    if (peakRule) {
      const peakPrefix = isProviderInPeak(family) ? `${t("peak.now")} ` : "";
      fullTooltip += ` · ${peakPrefix}${t(peakRule.tipKey)}`;
    }

    return `
      <div class="overview-bar-item tone-${itemTone}" data-jump-provider="${escapeHtml(jumpId)}" title="${escapeHtml(fullTooltip)} · ${escapeHtml(t("overview.groupHint"))}">
        <div class="ovbar-line1">
          <span class="overview-item-icon">${icon}</span>
          <span class="overview-item-name${nameClass}" ${compactLabelStyle(displayName, nameClass ? 9 : 10.5, 8.5)} title="${escapeHtml(displayName)}">${escapeHtml(displayName)}</span>
          ${accountBadge}
          ${overviewAcctFoldBtn(family, accountCount)}
          <span class="overview-dot ${statusDot}"></span>
          <span class="ovbar-pct ${pctClass}">${pct === null ? "—" : `${pct}%`}</span>
        </div>
        <div class="ovbar-line2">
          <span class="ovbar"><span class="ovbar-fill tone-${itemTone}" style="width:${pct ?? 0}%"></span></span>
          <span class="ovbar-meta">${escapeHtml(meta || t("overview.noData"))}</span>
        </div>
        ${overviewAcctFolded(family, accountCount) ? "" : accountMetersHtml(family)}
      </div>`;
  };

  const barsView = config.overviewStyle === "bars";
  const overviewRender = barsView ? barItemHtml : itemHtml;

  const foldChevron = isFolded
    ? `<button class="card-fold-toggle" data-overview-fold title="${escapeHtml(t("card.expand"))}">${uiIcon("caretDown")}</button>`
    : `<button class="card-fold-toggle" data-overview-fold title="${escapeHtml(t("card.collapse"))}">${uiIcon("caretUp")}</button>`;

  const styleTabs = `
        <div class="tabs overview-style-tabs" role="group" aria-label="${escapeHtml(t("overview.styleLabel"))}">
          <button type="button" class="tab${config.overviewStyle === "rings" ? " active" : ""}" data-overview-style="rings" title="${escapeHtml(t("overview.styleRings"))}">${uiIcon("circleNotch")}</button>
          <button type="button" class="tab${config.overviewStyle === "bars" ? " active" : ""}" data-overview-style="bars" title="${escapeHtml(t("overview.styleBars"))}">${uiIcon("rows")}</button>
        </div>`;

  // Availability badges: three disjoint symbols — green shows the
  // off-peak available count, red the maxed count, yellow the in-peak
  // available count (always rendered, 0 included).
  const badgeHtml = `
        <span class="overview-chip is-ok" title="${escapeHtml(t("overview.badge", { avail: availableCount, total: totalCount }))}">
          <span class="overview-chip-dot green"></span>
          <span class="overview-chip-text">${availableCount} ${escapeHtml(t("overview.availShort"))}</span>
        </span>${maxedCount > 0 ? `
        <span class="overview-chip is-warn" title="${escapeHtml(t("overview.maxedBadge", { n: maxedCount }))}">
          <span class="overview-chip-dot red"></span>
          <span class="overview-chip-text">${maxedCount} ${escapeHtml(t("overview.maxedShort"))}</span>
        </span>` : ""}
        <span class="overview-peak-row">
          <span class="overview-chip is-peak" title="${escapeHtml(t("overview.peakBadge", { n: peakItems.length }))}">
            <span class="overview-chip-dot yellow"></span>
            <span class="overview-chip-text">${peakItems.length} ${escapeHtml(t("overview.peakShort"))}</span>
          </span>
          <button type="button" class="overview-peak-help" data-overview-peak-help title="${escapeHtml(t("peak.helpTitle"))}">${uiIcon("question")}</button>
        </span>`;

  // Reset-sorted reminder rows. Window label rides each quota; remaining
  // under an hour reads red so the top of the list is the "act now" part.
  const expiringRow = (
    { cardSnap, shownSnap, quota }: OverviewItem,
    remainText: string,
    rowClass: string,
  ): string => {
    const family = providerFamily(cardSnap.id);
    const jumpId = isParallelAccountFamily(family) ? shownSnap.id : cardSnap.id;
    const origin = shownSnap.dashboard_url ?? undefined;
    const visual = providerVisual(jumpId || family, origin);
    const icon = visual?.iconSvg ?? `<span class="icon-fallback">${escapeHtml(cardSnap.name.slice(0, 2))}</span>`;
    const displayName = notedName(jumpId, providerDisplayName(family) || cardSnap.name);
    const win = quota.window === null ? "" : escapeHtml(t(windowLabelKey[quota.window]));
    return `
        <div class="expiring-row${rowClass}" data-jump-provider="${escapeHtml(jumpId)}">
          <span class="expiring-icon">${icon}</span>
          <span class="expiring-name">${escapeHtml(displayName)}</span>
          ${win ? `<span class="expiring-win">${win}</span>` : ""}
          <span class="expiring-remain">${remainText}</span>
        </div>`;
  };
  const expiringRows =
    timedItems
      .map((it) => {
        const remainMs = Math.max(0, (it.quota.resetsAt ?? 0) - Date.now());
        return expiringRow(
          it,
          escapeHtml(fmtDuration(remainMs)),
          remainMs < 60 * 60_000 ? " urgent" : "",
        );
      })
      .join("") +
    pendingItems
      .map((it) => {
        if (isSnapshotMaxed(it.shownSnap)) {
          return expiringRow(it, escapeHtml(t("overview.pendingReset")), " urgent");
        }
        if (it.quota.window === "5h") {
          return expiringRow(it, escapeHtml(t("card.notStarted")), " muted");
        }
        return expiringRow(it, escapeHtml(t("overview.noReset")), " muted");
      })
      .join("");
  const expiringView = !isFolded && overviewExpiringOpen
    ? `<div class="card-panel overview-expiring-panel">
        ${expiringRows || `<div class="expiring-empty">${escapeHtml(t("overview.expiringEmpty"))}</div>`}
      </div>`
    : "";
  const sectionsView = !isFolded && !overviewExpiringOpen
    ? `<div class="card-panel overview-panel">${overviewSectionHtml("", "ok", items, overviewRender)}</div>`
    : "";

  return `
    <article class="provider quota-overview ${isFolded ? "is-folded" : ""}" data-provider="__overview__">
      <div class="provider-head">
        <span class="overview-title-icon" title="${escapeHtml(t("overview.title"))}">
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m12 14 4-4"></path>
            <path d="M3.34 19a10 10 0 1 1 17.32 0"></path>
          </svg>
        </span>
        <span class="provider-name">${escapeHtml(t("overview.title"))}</span>
        <span class="overview-badges-tools">
          <span class="overview-badges">${badgeHtml}</span>
          <button type="button" class="card-refresh overview-group-manage" data-overview-group-manage title="${escapeHtml(t("overview.groupManage"))}">${uiIcon("gear")}</button>
        </span>
        <span class="spacer"></span>
        ${styleTabs}
        <button type="button" class="card-refresh overview-expiring${overviewExpiringOpen ? " on" : ""}" data-overview-expiring title="${escapeHtml(t("overview.expiring"))}">⏱</button>
        <button type="button" class="card-refresh overview-refresh" data-overview-refresh title="${escapeHtml(t("overview.refresh"))}">${uiIcon("arrowsClockwise")}</button>
        ${foldChevron}
      </div>
      ${isFolded ? "" : `<div class="overview-switch-row">
        <div class="tabs overview-cat-tabs${config.overviewCatFull === true ? " full" : ""}">
          ${(() => {
            // Full mode: a two-row grid, 3 chips per row, 6 max — more
            // categories collapse into a … chip that opens the board.
            const chips =
              config.overviewCatFull === true && OVERVIEW_CATEGORIES.length > 6
                ? OVERVIEW_CATEGORIES.slice(0, 5)
                : OVERVIEW_CATEGORIES;
            const html = chips
              .map(
                (c) =>
                  `<button type="button" class="tab${overviewCategory === c ? " active" : ""}" data-overview-cat="${c}" title="${escapeHtml(t(`category.${c}`))}">${escapeHtml(t(`category.${c}`))}</button>`,
              )
              .join("");
            const more =
              config.overviewCatFull === true && OVERVIEW_CATEGORIES.length > 6
                ? `<button type="button" class="tab overview-cat-more" data-overview-cat-more title="${escapeHtml(t("overview.groupManage"))}">…</button>`
                : "";
            return html + more;
          })()}
        </div>
        ${overviewCategory === "coding" ? `
        <div class="tabs overview-tabs">
          <button type="button" class="tab${overviewTab === "5h" ? " active" : ""}" data-overview-tab="5h">${escapeHtml(t("overview.tab5h"))}</button>
          <button type="button" class="tab${overviewTab === "week" ? " active" : ""}" data-overview-tab="week">${escapeHtml(t("overview.tabWeek"))}</button>
          <button type="button" class="tab${overviewTab === "month" ? " active" : ""}" data-overview-tab="month">${escapeHtml(t("overview.tabMonth"))}</button>
        </div>` : ""}
      </div>`}
      ${expiringView}
      ${sectionsView}
    </article>`;
}

// ---------------------------------------------------------------------------
// Footer update flow — every popover open re-checks; the version stamp
// becomes "Checking for updates…" and then an Update button on a hit.
// ---------------------------------------------------------------------------

let buildText = "";
let updateVersion: string | null = null;
/// In-flight push state for the spend-header indicator. Module-level so a
/// dashboard re-render mid-download keeps the ring (the button element is
/// replaced on every refresh cycle).
let updatePushing = false;
let updatePct: number | null = null;
let updateSeen = 0;

/// Repaint the push indicator from module state — the button element can be
/// replaced by a card re-render while a download runs.
function syncPushProgress(): void {
  const btn = document.querySelector<HTMLElement>("[data-update-push]");
  if (!btn) return;
  btn.classList.toggle("busy", updatePushing);
  if (updatePushing && updatePct != null) btn.style.setProperty("--pct", String(updatePct));
  else btn.style.removeProperty("--pct");
}
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
    if (v) {
      updateVersion = v;
      // The Total Spend header carries the push indicator — repaint so the
      // green arrow appears the moment a version is announced.
      renderIfVisible();
      maybePromptUpdate(v);
    }
  } catch {
    // Offline or GitHub unreachable — the stamp just returns; the
    // 4-hourly background checker will try again anyway.
  }
  checkingUpdate = false;
  renderBuildInfo();
}

// Update popup — a newly discovered version (popover-open check or the
// 4-hourly background checker) prompts once per version per run; the footer
// button stays as the always-visible entry point. Mirrors appConfirm's
// overlay, with "later" dismissing via Esc/backdrop like a confirm cancel.
let updatePromptedFor: string | null = null;

function maybePromptUpdate(version: string): void {
  if (updatePromptedFor === version || document.querySelector("#update-overlay")) return;
  updatePromptedFor = version;
  const overlay = document.createElement("div");
  overlay.id = "update-overlay";
  overlay.innerHTML = `
    <div id="update-box" role="dialog" aria-modal="true">
      <h3>${escapeHtml(t("update.availableTitle", { version }))}</h3>
      <p>${escapeHtml(t("update.availableBody", { version }))}</p>
      <div id="update-actions">
        <button id="update-later" type="button">${escapeHtml(t("update.later"))}</button>
        <button id="update-now" type="button">${escapeHtml(t("update.now"))}</button>
      </div>
    </div>`;
  const close = () => {
    document.removeEventListener("keydown", onKey, true);
    overlay.remove();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      close();
    }
  };
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) close();
  });
  overlay.querySelector("#update-later")!.addEventListener("click", close);
  const nowBtn = overlay.querySelector<HTMLButtonElement>("#update-now")!;
  nowBtn.addEventListener("click", () => {
    // On success the app restarts, so only the failure path matters here.
    nowBtn.textContent = t("update.installing");
    nowBtn.disabled = true;
    invoke("install_update").catch((err) => {
      nowBtn.textContent = t("update.now");
      nowBtn.disabled = false;
      const body = overlay.querySelector("#update-box p");
      if (body) body.textContent = t("footer.updateFailed", { err: String(err) });
    });
  });
  document.addEventListener("keydown", onKey, true);
  document.body.appendChild(overlay);
  nowBtn.focus();
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

/// In-app replacement for window.prompt (unavailable in Tauri's WebView):
/// same overlay as appConfirm plus one text input. Resolves the trimmed
/// name, or null on cancel/Esc/backdrop.
function appPrompt(opts: {
  title: string;
  placeholder?: string;
  initial?: string;
  confirmLabel: string;
  /// Confirm with an empty field resolves "" instead of null — needed to
  /// clear a value (e.g. a note falling back to the original name).
  allowEmpty?: boolean;
  /// Password-style input (master-password prompts).
  secret?: boolean;
}): Promise<string | null> {
  return new Promise((resolve) => {
    const overlay = document.createElement("div");
    overlay.id = "confirm-overlay";
    overlay.innerHTML = `
      <div id="confirm-box" role="dialog" aria-modal="true">
        <h3>${escapeHtml(opts.title)}</h3>
        <input id="prompt-input" class="form-input" type="${opts.secret ? "password" : "text"}" spellcheck="false" autocomplete="off" />
        <div id="confirm-actions">
          <button id="confirm-cancel" type="button">${escapeHtml(t("dialog.cancel"))}</button>
          <button id="confirm-ok" type="button">${escapeHtml(opts.confirmLabel)}</button>
        </div>
      </div>`;
    const input = overlay.querySelector<HTMLInputElement>("#prompt-input")!;
    input.value = opts.initial ?? "";
    input.placeholder = opts.placeholder ?? "";
    const done = (ok: boolean) => {
      dismissConfirm = null;
      document.removeEventListener("keydown", onKey, true);
      overlay.remove();
      resolve(ok ? (input.value.trim() || (opts.allowEmpty ? "" : null)) : null);
    };
    dismissConfirm = () => done(false);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        done(false);
      } else if (e.key === "Enter") {
        e.preventDefault();
        done(true);
      }
    };
    overlay.addEventListener("click", (e) => {
      if (e.target === overlay) done(false);
    });
    overlay.querySelector("#confirm-cancel")!.addEventListener("click", () => done(false));
    overlay.querySelector("#confirm-ok")!.addEventListener("click", () => done(true));
    input.addEventListener("keydown", onKey);
    document.addEventListener("keydown", onKey, true);
    document.body.appendChild(overlay);
    input.focus();
    input.select();
  });
}

/// The "?" help for one provider: how its quota is queried, in plain words.
/// The kind is derived from the catalog — API-key providers get their
/// console link and paste steps, browser-sign-in providers point at the
/// gear-menu sign-in, everything else is a local-credential family that
/// Pane discovers from the official client. Never mentions endpoints or
/// reverse-engineering — user-facing wording only.
function openProviderHelp(fam: string) {
  const def = providerDefinition(fam);
  const keyUrl = getApiKeyLink(fam);
  // What this provider actually accepts, from the curated table — a family
  // that only reads a local sign-in must never be told to paste a key.
  const methods = PROVIDER_CRED_INFO[fam]?.methods ?? (def?.supportsApiKey ? ["paste"] : ["local"]);
  const badgeLabels: Record<CredMethod, string> = {
    paste: t("customize.helpBadgeKey"),
    oauth: t("customize.helpBadgeOauth"),
    local: t("customize.helpBadgeLocal"),
  };
  const badge = methods.length ? methods.map((m) => badgeLabels[m]).join(" + ") : t("customize.helpBadgeNone");
  const steps: string[] = [];
  if (methods.includes("paste")) {
    steps.push(t("customize.helpStepKey1"));
    steps.push(t("customize.helpStepKey2"));
  }
  if (methods.includes("oauth")) steps.push(t("customize.helpStepOauth1"));
  if (methods.includes("local")) {
    steps.push(t("customize.helpStepLocal1"));
    steps.push(t("customize.helpStepLocal2"));
  }
  if (methods.length === 0 && !(PROVIDER_LINKS[fam]?.length)) {
    steps.push(t("customize.helpUnknown"));
  }
  steps.push(t("customize.helpStepTest"));
  const links = [...(PROVIDER_LINKS[fam] ?? [])];
  if (keyUrl && !links.some((l) => l.url === keyUrl)) {
    links.unshift({ label: t("customize.helpOpenConsole"), url: keyUrl });
  }

  // The mechanism table answers what the generic steps cannot: which local
  // files this machine is read from, which env vars count, and where the
  // numbers come from. Values stay technical (paths, env names, hosts) so a
  // path can never drift into a translated string.
  const mech = MECHANISMS[fam];
  const live = lastSnapshots.find((s) => s.id === fam);
  const provideText = methods.length
    ? methods.map((m) => t(`customize.helpProvide_${m}`)).join("  +  ")
    : t("customize.helpProvide_none");
  const mechRows: [string, string][] = mech
    ? [
        [
          t("customize.helpMechReads"),
          mech.reads?.length ? mech.reads.join("  ·  ") : t("customize.helpMechReadsNone"),
        ],
        ...((mech.env?.length
          ? [[t("customize.helpMechEnv"), mech.env.join("  ·  ")]]
          : []) as [string, string][]),
        [t("customize.helpMechProvide"), provideText],
        [
          t("customize.helpMechSource"),
          mech.hosts?.length ? mech.hosts.join("  ·  ") : t("customize.helpMechSourceLocal"),
        ],
        ...((live?.metrics?.length
          ? [
              [
                t("customize.helpMechLive"),
                live.metrics
                  .slice(0, 3)
                  .map((m) => `${m.label}${m.used_percent != null ? ` ${Math.round(m.used_percent)}%` : ""}`)
                  .join("  ·  "),
              ],
            ]
          : []) as [string, string][]),
      ]
    : [];

  const overlay = document.createElement("div");
  overlay.id = "confirm-overlay";
  overlay.innerHTML = `
    <div id="confirm-box" role="dialog" aria-modal="true">
      <h3>${escapeHtml(providerDisplayName(fam))} · ${escapeHtml(t("customize.helpMenu"))}</h3>
      <div class="help-kind">${escapeHtml(badge)}</div>
      ${
        mechRows.length
          ? `<dl class="help-mech">${mechRows
              .map(([k, v]) => `<div><dt>${escapeHtml(k)}</dt><dd>${escapeHtml(v)}</dd></div>`)
              .join("")}</dl>`
          : ""
      }
      <ol class="help-steps">${steps.map((s) => `<li>${escapeHtml(s)}</li>`).join("")}</ol>
      ${links.length ? `<div class="help-links">${links.map((l) => `<a href="${escapeHtml(l.url)}" target="_blank" rel="noreferrer">${escapeHtml(l.label)} ↗</a>`).join("")}</div>` : ""}
      <div id="confirm-actions">
        <button id="help-test" type="button">⚡ ${escapeHtml(t("customize.helpTest"))}</button>
        <button id="confirm-ok" type="button">${escapeHtml(t("dialog.ok"))}</button>
      </div>
    </div>`;
  const close = () => {
    overlay.remove();
    document.removeEventListener("keydown", onKey, true);
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  };
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) close();
  });
  overlay.querySelector("#confirm-ok")!.addEventListener("click", close);
  const testBtn = overlay.querySelector<HTMLButtonElement>("#help-test")!;
  testBtn.addEventListener("click", () => {
    const cardId = fam; // test_provider resolves the family's active card
    testBtn.disabled = true;
    testBtn.textContent = t("customize.helpTesting");
    invoke<string>("test_provider", { providerId: cardId })
      .then((res) => {
        const ms = res.split(":").pop() ?? "";
        testBtn.textContent = res.startsWith("ok:")
          ? t("card.testOk", { ms })
          : t("card.testFail", { err: res.slice(0, res.lastIndexOf(":")) });
      })
      .catch((err: unknown) => {
        testBtn.textContent = t("card.testFail", { err: String(err) });
      })
      .finally(() => {
        testBtn.disabled = false;
      });
  });
  document.addEventListener("keydown", onKey, true);
  document.body.appendChild(overlay);
}

/// The dashboard card's group menu — the main-page tagging entry. Lists
/// every group (✓ on the card's current one), "Ungrouped", and
/// "New group…". Tagging lives on the family, so a parallel-account card
/// moves its whole family. Opens from the card-head ⚙ and right-click.
/// The note (custom display name) rides along: the note itself is per card.
function openGroupManagementPanel(): void {
  document.querySelector<HTMLElement>(".group-menu-overlay")?.remove();
  const builtins = [
    ...OVERVIEW_CATEGORIES.map((id) => ({ id, name: t(`category.${id}`), kind: t("overview.groupBuiltin") })),
  ];
  const overlay = document.createElement("div");
  overlay.className = "group-menu-overlay";
  overlay.innerHTML = `<div class="group-menu group-manage-panel" role="dialog" aria-label="${escapeHtml(t("overview.groupManageTitle"))}">
    <div class="group-menu-title">${escapeHtml(t("overview.groupManageTitle"))}</div>
    <div class="group-manage-hint">${escapeHtml(t("overview.groupManageHint"))}</div>
    <div class="group-manage-list">
      ${builtins.map((g) => `<div class="group-manage-row is-builtin"><span class="group-manage-name">${escapeHtml(g.name)}</span><span class="group-manage-kind">${escapeHtml(g.kind)}</span></div>`).join("")}
      ${cardGroups().map((g) => `<div class="group-manage-row"><span class="group-manage-name">${escapeHtml(g.name)}</span><span class="group-manage-count">${cardGroupMemberCount(g.id)}</span><span class="group-manage-actions"><button type="button" class="group-menu-item" data-group-manage-rename="${escapeHtml(g.id)}">${escapeHtml(t("overview.groupRename"))}</button><button type="button" class="group-menu-item danger" data-group-manage-delete="${escapeHtml(g.id)}">${escapeHtml(t("overview.groupDelete"))}</button></span></div>`).join("")}
      ${cardGroups().length === 0 ? `<div class="group-manage-empty">${escapeHtml(t("overview.groupEmpty"))}</div>` : ""}
    </div>
    <div class="group-menu-sep"></div>
    <button type="button" class="group-menu-item group-manage-add" data-group-manage-add>＋ ${escapeHtml(t("overview.groupNew"))}</button>
  </div>`;
  document.body.appendChild(overlay);
  const close = () => {
    overlay.remove();
    document.removeEventListener("keydown", onKey);
  };
  const onKey = (event: KeyboardEvent) => { if (event.key === "Escape") close(); };
  document.addEventListener("keydown", onKey);
  overlay.addEventListener("click", async (event) => {
    if (event.target === overlay) { close(); return; }
    const target = event.target as HTMLElement;
    if (target.closest("[data-group-manage-add]")) {
      const name = await appPrompt({ title: t("overview.groupCreatePrompt"), confirmLabel: t("dialog.ok") });
      if (name?.trim()) { upsertCardGroup(newGroupId(), name.trim()); renderAll(); openGroupManagementPanel(); }
      return;
    }
    const rename = target.closest<HTMLElement>("[data-group-manage-rename]");
    if (rename) {
      const g = cardGroup(rename.dataset.groupManageRename!);
      if (!g) return;
      const name = await appPrompt({ title: t("customize.groupRenamePrompt"), initial: g.name, confirmLabel: t("dialog.ok") });
      if (name?.trim() && name.trim() !== g.name) { upsertCardGroup(g.id, name.trim()); renderAll(); openGroupManagementPanel(); }
      return;
    }
    const del = target.closest<HTMLElement>("[data-group-manage-delete]");
    if (del) {
      const g = cardGroup(del.dataset.groupManageDelete!);
      if (!g) return;
      const count = cardGroupMemberCount(g.id);
      if (count > 0) {
        await appConfirm({ title: t("overview.groupDeleteBlockedTitle"), message: t("overview.groupDeleteBlockedBody", { name: g.name, n: count }), confirmLabel: t("dialog.ok") });
        return;
      }
      if (await appConfirm({ title: t("customize.groupDelete"), message: t("customize.groupDeleteConfirm", { name: g.name }), confirmLabel: t("customize.groupDelete"), danger: true })) {
        deleteCardGroup(g.id); renderAll(); openGroupManagementPanel();
      }
    }
  });
}

function shortcutKeyToken(event: KeyboardEvent): string {
  if (/^Digit[0-9]$/.test(event.code)) return event.code.slice(5);
  if (/^Numpad[0-9]$/.test(event.code)) return event.code.slice(6);
  if (/^Key[A-Z]$/.test(event.code)) return event.code.slice(3);
  if (/^F[0-9]{1,2}$/.test(event.code)) return event.code;
  return event.key.length === 1 ? event.key.toUpperCase() : event.key;
}

function shortcutMatches(event: KeyboardEvent, shortcut: string): boolean {
  const parts = shortcut.split("+").map((part) => part.trim()).filter(Boolean);
  if (!parts.length) return false;
  const key = parts[parts.length - 1].toUpperCase();
  const modifiers = new Set(parts.slice(0, -1).map((part) => part.toLowerCase()));
  const expected = {
    ctrl: modifiers.has("ctrl"),
    alt: modifiers.has("alt"),
    shift: modifiers.has("shift"),
    meta: modifiers.has("meta") || modifiers.has("cmd"),
  };
  return expected.ctrl === event.ctrlKey && expected.alt === event.altKey &&
    expected.shift === event.shiftKey && expected.meta === event.metaKey &&
    shortcutKeyToken(event).toUpperCase() === key;
}

const LOCAL_SHORTCUTS = [
  { id: "settings", key: "settings.actionSettings", default: "Ctrl+S" },
  { id: "refresh", key: "settings.actionRefresh", default: "Ctrl+R" },
  { id: "customize", key: "settings.actionCustomize", default: "Ctrl+E" },
  { id: "theme", key: "settings.actionTheme", default: "Ctrl+L" },
  { id: "expiring", key: "settings.actionExpiring", default: "T" },
  { id: "period", key: "settings.actionPeriod", default: "Shift" },
] as const;
type LocalShortcutAction = (typeof LOCAL_SHORTCUTS)[number]["id"];
function localShortcut(action: LocalShortcutAction): string {
  return config.localShortcuts?.[action] ?? LOCAL_SHORTCUTS.find((row) => row.id === action)!.default;
}

const SHORTCUT_MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);
const SHORTCUT_DEFAULTS: Record<string, string> = {
  wake: "Alt+2",
  category: "Shift+1",
  ...Object.fromEntries(LOCAL_SHORTCUTS.map((row) => [row.id, row.default])),
};

function shortcutRows(): { id: string; labelKey: string }[] {
  return [
    { id: "wake", labelKey: "settings.wakeShortcut" },
    { id: "category", labelKey: "settings.categoryShortcut" },
    ...LOCAL_SHORTCUTS.map((row) => ({ id: row.id as string, labelKey: row.key as string })),
  ];
}

function shortcutValue(id: string): string {
  if (id === "wake") return config.shortcut ?? "";
  if (id === "category") return config.categoryShortcut || "";
  return localShortcut(id as LocalShortcutAction);
}

/// Binding → key-cap chips ("Ctrl+S" → [Ctrl] + [S]); empty = 未设置.
function shortcutChips(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return `<span class="sc-unset">${escapeHtml(t("settings.shortcutUnset"))}</span>`;
  return trimmed
    .split("+")
    .map((part) => `<kbd>${escapeHtml(part.trim())}</kbd>`)
    .join('<span class="sc-plus">+</span>');
}

/// One renderer for every row (wake + category + local actions): identical
/// markup means identical alignment, and the recorder UX follows the
/// mainstream pattern (VS Code / Raycast): click to record, Esc cancels,
/// Backspace clears, the ↺ button restores the built-in default, and the ×
/// clears the binding so a fresh one can be recorded. Conflicts surface as
/// the row's red state text.
function renderShortcutSettings(): void {
  const root = document.querySelector<HTMLElement>("#local-shortcuts");
  if (!root) return;
  root.innerHTML = shortcutRows()
    .map((row) => {
      const value = shortcutValue(row.id).trim();
      const canReset = value !== (SHORTCUT_DEFAULTS[row.id] ?? "");
      return `<div class="setting-row shortcut-row" data-sc-row="${row.id}">
        <label>${escapeHtml(t(row.labelKey))}</label>
        <span class="shortcut-state" role="status" aria-live="polite"></span>
        ${canReset ? `<button type="button" class="mini-btn sc-icon" data-sc-reset="${row.id}" title="${escapeHtml(t("settings.shortcutResetTip"))}" aria-label="${escapeHtml(t("settings.shortcutResetTip"))}">${uiIcon("arrowsClockwise")}</button>` : ""}
        <button type="button" class="shortcut-field" data-sc-field="${row.id}">${shortcutChips(value)}</button>
        ${value ? `<button type="button" class="mini-btn sc-icon" data-sc-clear="${row.id}" title="${escapeHtml(t("settings.shortcutClearTip"))}" aria-label="${escapeHtml(t("settings.shortcutClearTip"))}">${uiIcon("x")}</button>` : ""}
      </div>`;
    })
    .join("");
  root.querySelectorAll<HTMLButtonElement>("[data-sc-reset]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const id = btn.dataset.scReset!;
      void applyShortcut(id, SHORTCUT_DEFAULTS[id] ?? "");
    });
  });
  root.querySelectorAll<HTMLButtonElement>("[data-sc-clear]").forEach((btn) => {
    btn.addEventListener("click", () => void applyShortcut(btn.dataset.scClear!, ""));
  });
  root.querySelectorAll<HTMLButtonElement>("[data-sc-field]").forEach((field) => {
    field.addEventListener("click", () => startShortcutRecording(field));
  });
}

function startShortcutRecording(field: HTMLButtonElement): void {
  if (field.classList.contains("recording")) return;
  const id = field.dataset.scField!;
  const row = field.closest<HTMLElement>(".shortcut-row");
  const state = row?.querySelector<HTMLElement>(".shortcut-state") ?? null;
  field.classList.add("recording");
  field.innerHTML = `<span class="sc-recording">${escapeHtml(t("settings.shortcutRecording"))}</span>`;
  if (state) {
    state.textContent = "";
    state.className = "shortcut-state";
  }
  let finished = false;
  const finish = (next: string | null) => {
    if (finished) return;
    finished = true;
    document.removeEventListener("keydown", onKey, true);
    field.classList.remove("recording");
    if (next === null) {
      // Cancel: restore the chips in place (no re-render, so a click that
      // is already headed for another row still lands correctly).
      field.innerHTML = shortcutChips(shortcutValue(id));
      return;
    }
    void applyShortcut(id, next);
  };
  const onKey = (event: KeyboardEvent) => {
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      finish(null);
      return;
    }
    if (event.key === "Backspace" || event.key === "Delete") {
      finish("");
      return;
    }
    if (SHORTCUT_MODIFIER_KEYS.has(event.key)) {
      // A lone modifier is legal only for the cycle key (its default IS Shift).
      if (id === "period" && event.key === "Shift") finish("Shift");
      return;
    }
    const captured = formatCapturedShortcut(event);
    if (captured) finish(captured);
  };
  document.addEventListener("keydown", onKey, true);
  field.addEventListener("blur", () => finish(null), { once: true });
}

const shortcutCanonical = (binding: string) =>
  binding.toLowerCase().split("+").map((p) => p.trim()).filter(Boolean).sort().join("+");

async function applyShortcut(id: string, value: string): Promise<void> {
  const trimmed = value.trim();
  const row = document.querySelector<HTMLElement>(`[data-sc-row="${id}"]`);
  const state = row?.querySelector<HTMLElement>(".shortcut-state") ?? null;
  const status = document.querySelector("#status");
  if (trimmed) {
    const clash = shortcutRows().some((other) => {
      if (other.id === id) return false;
      const otherValue = shortcutValue(other.id).trim();
      return otherValue && shortcutCanonical(otherValue) === shortcutCanonical(trimmed);
    });
    if (clash) {
      if (state) {
        state.textContent = t("settings.shortcutConflict");
        state.className = "shortcut-state conflict";
      }
      renderShortcutSettings();
      return;
    }
  }
  if (state) {
    state.textContent = t("settings.shortcutChecking");
    state.className = "shortcut-state checking";
  }
  try {
    if (id === "wake") {
      await invoke("set_shortcut", { shortcut: trimmed });
      await patchConfig({ shortcut: trimmed });
    } else if (id === "category") {
      config.categoryShortcut = trimmed;
      await patchConfig({ categoryShortcut: trimmed });
    } else {
      await patchConfig({ localShortcuts: { ...config.localShortcuts, [id]: trimmed } });
    }
    if (status) status.textContent = trimmed ? t("footer.shortcutSaved") : t("footer.shortcutCleared");
    renderShortcutSettings();
    const fresh = document.querySelector<HTMLElement>(`[data-sc-row="${id}"] .shortcut-state`);
    if (fresh && trimmed) {
      fresh.textContent = t("settings.shortcutAvailable");
      fresh.className = "shortcut-state available";
    }
  } catch (err) {
    if (status) status.textContent = String(err);
    renderShortcutSettings();
    const fresh = document.querySelector<HTMLElement>(`[data-sc-row="${id}"] .shortcut-state`);
    if (fresh) {
      fresh.textContent = t("settings.shortcutConflict");
      fresh.className = "shortcut-state conflict";
    }
  }
}

function isDashboardActive(): boolean {
  return document.hasFocus() && document.visibilityState === "visible";
}

function cycleOverviewCategory(): void {
  const index = OVERVIEW_CATEGORIES.indexOf(overviewCategory);
  const next = OVERVIEW_CATEGORIES[(index + 1) % OVERVIEW_CATEGORIES.length];
  switchOverviewCategory(next);
}

function formatCapturedShortcut(event: KeyboardEvent): string | null {
  if (event.key === "Control" || event.key === "Alt" || event.key === "Shift" || event.key === "Meta") return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Meta");
  parts.push(shortcutKeyToken(event));
  return parts.join("+");
}

function setupOverviewGroupDrag(root: HTMLElement): void {
  let pending: { id: string; pointerId: number; x: number; y: number; tile: HTMLElement } | null = null;
  let timer = 0;
  let ghost: HTMLElement | null = null;
  let dock: HTMLElement | null = null;
  let hot: HTMLElement | null = null;
  let suppressClickUntil = 0;
  const clear = () => {
    window.clearTimeout(timer);
    if (ghost) suppressClickUntil = performance.now() + 400;
    pending?.tile.classList.remove("overview-drag-source");
    ghost?.remove();
    dock?.remove();
    hot?.classList.remove("overview-drop-hot");
    pending = null;
    ghost = dock = hot = null;
    document.body.classList.remove("overview-drag-active");
  };
  const moveGhost = (x: number, y: number) => {
    if (!ghost) return;
    ghost.style.left = `${Math.max(0, Math.min(x + 12, window.innerWidth - ghost.offsetWidth))}px`;
    ghost.style.top = `${Math.max(0, Math.min(y + 12, window.innerHeight - ghost.offsetHeight))}px`;
  };
  const targetAt = (x: number, y: number) => document.elementFromPoint(x, y)?.closest<HTMLElement>(
    "[data-overview-cat], [data-overview-drop-group]",
  ) ?? null;
  root.addEventListener("pointerdown", (event) => {
    if (event.button !== 0 || event.isPrimary === false || (event.target as Element).closest("button")) return;
    const tile = (event.target as Element).closest<HTMLElement>(
      ".overview-item[data-jump-provider], .overview-bar-item[data-jump-provider]",
    );
    if (!tile?.dataset.jumpProvider) return;
    clear();
    pending = { id: tile.dataset.jumpProvider, pointerId: event.pointerId, x: event.clientX, y: event.clientY, tile };
    timer = window.setTimeout(() => {
      if (!pending || !tile.isConnected) { clear(); return; }
      ghost = tile.cloneNode(true) as HTMLElement;
      ghost.classList.add("overview-drag-ghost");
      ghost.removeAttribute("data-jump-provider");
      ghost.querySelector(".overview-move-btn")?.remove();
      ghost.style.width = `${tile.getBoundingClientRect().width}px`;
      document.body.appendChild(ghost);
      tile.classList.add("overview-drag-source");
      document.body.classList.add("overview-drag-active");
      dock = document.createElement("div");
      dock.className = "overview-drop-dock";
      dock.innerHTML = `<div class="overview-drop-title">${escapeHtml(t("overview.dragMoveHint"))}</div>
        <div class="overview-drop-options">${OVERVIEW_CATEGORIES.map((category) => `<div data-overview-cat="${category}">${escapeHtml(t(`category.${category}`))}</div>`).join("")}</div>
        <div class="overview-drop-options"><div data-overview-drop-group="">${escapeHtml(t("customize.groupNone"))}</div>${cardGroups().map((group) => `<div data-overview-drop-group="${escapeHtml(group.id)}">${escapeHtml(group.name)}</div>`).join("")}</div>`;
      document.body.appendChild(dock);
      moveGhost(pending.x, pending.y);
    }, 450);
  });
  window.addEventListener("pointermove", (event) => {
    if (!pending || pending.pointerId !== event.pointerId) return;
    if (!ghost) {
      if (Math.hypot(event.clientX - pending.x, event.clientY - pending.y) > 8) clear();
      return;
    }
    event.preventDefault();
    moveGhost(event.clientX, event.clientY);
    hot?.classList.remove("overview-drop-hot");
    hot = targetAt(event.clientX, event.clientY);
    hot?.classList.add("overview-drop-hot");
  }, { passive: false });
  window.addEventListener("pointerup", (event) => {
    if (!pending || pending.pointerId !== event.pointerId) return;
    const id = pending.id;
    const target = ghost ? targetAt(event.clientX, event.clientY) : null;
    const category = target?.dataset.overviewCat;
    const group = target?.dataset.overviewDropGroup;
    clear();
    if (category && (OVERVIEW_CATEGORIES as readonly string[]).includes(category)) {
      setFamilyCategory(providerFamily(id), category as OverviewCategory);
    } else if (group !== undefined) {
      setCardGroup(id, group);
    }
  });
  window.addEventListener("pointercancel", clear);
  window.addEventListener("blur", clear);
  root.addEventListener("contextmenu", clear);
  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && ghost) {
      event.preventDefault();
      event.stopImmediatePropagation();
      clear();
    }
  }, true);
  root.addEventListener("click", (event) => {
    if (performance.now() < suppressClickUntil) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  }, true);
}

function overviewMenuSide(tile: HTMLElement): "left" | "middle" | "right" {
  const rect = tile.getBoundingClientRect();
  const grid = tile.closest<HTMLElement>(".overview-grid");
  if (grid?.firstElementChild) {
    const firstTop = grid.firstElementChild.getBoundingClientRect().top;
    const columns = Array.from(grid.children)
      .map((child) => child.getBoundingClientRect())
      .filter((child) => Math.abs(child.top - firstTop) < 3)
      .sort((a, b) => a.left - b.left);
    if (columns.length > 1) {
      const center = (rect.left + rect.right) / 2;
      let index = 0;
      columns.forEach((column, i) => {
        if (Math.abs((column.left + column.right) / 2 - center) <
            Math.abs((columns[index].left + columns[index].right) / 2 - center)) index = i;
      });
      if (index === 0) return "left";
      if (index === columns.length - 1) return "right";
      return "middle";
    }
  }
  const parentRect = (grid ?? tile.parentElement)?.getBoundingClientRect();
  const fraction = parentRect?.width ? ((rect.left + rect.right) / 2 - parentRect.left) / parentRect.width : 0.5;
  return fraction < 1 / 3 ? "left" : fraction > 2 / 3 ? "right" : "middle";
}

function groupMenuPosition(rect: DOMRect, width: number, height: number, side: "left" | "middle" | "right", viewportWidth: number, viewportHeight: number): { x: number; y: number } {
  const desiredX = side === "left" ? rect.right + 4 : side === "right" ? rect.left - width - 4 : (rect.left + rect.right - width) / 2;
  return {
    x: Math.max(8, Math.min(desiredX, viewportWidth - width - 8)),
    y: Math.max(8, Math.min(rect.bottom + 4, viewportHeight - height - 8)),
  };
}

function openGroupMenu(
  cardId: string,
  anchor: HTMLElement,
  includeGrouping = true,
  contextPoint?: { x: number; y: number; target?: Element; atCursor?: boolean },
): void {
  document.querySelector(".group-menu-overlay")?.remove();
  const current = cardGroupId(cardId);
  const groups = cardGroups();
  const fam = providerFamily(cardId);
  const contextTarget = contextPoint?.target?.closest<HTMLElement>("[data-card-account], [data-card-pin]");
  const contextAccount = contextTarget?.dataset.cardAccount?.split("|")[1]
    ?? contextTarget?.dataset.cardPin?.split("|")[1];
  const shownId = contextAccount ?? anchor.closest<HTMLElement>("[data-shown-account]")?.dataset.shownAccount ?? cardId;
  const accounts = accountsCache.get(fam) ?? [];
  const accountIndex = accounts.findIndex((entry, index) => entry.id === shownId || (shownId === fam && index === 0 && !isParallelAccountFamily(fam)));
  // Archive/restore rows: the archive target is the account this menu was
  // opened on (the ⚙ context or the card's shown account); the family's
  // parked accounts follow below for one-click restore.
  const familyArchived = includeGrouping ? [] : (archivedAccountsCache.get(fam) ?? []);
  const curCat = effectiveCategory(fam);
  const item = (gid: string, label: string, checked = false) =>
    `<button class="group-menu-item${checked ? " on" : ""}" data-group-pick="${escapeHtml(gid)}">
       <span class="group-menu-check">${checked ? "✓" : ""}</span>${escapeHtml(label)}
     </button>`;
  const catItem = (cat: OverviewCategory) =>
    `<button class="group-menu-item${curCat === cat ? " on" : ""}" data-category-pick="${cat}">
       <span class="group-menu-check">${curCat === cat ? "✓" : ""}</span>${escapeHtml(
         t(`category.${cat}`),
       )}
     </button>`;
  const fallbackName = providerDisplayName(fam) || cardId;
  const groupingHtml = includeGrouping
    ? `${OVERVIEW_CATEGORIES.map((c) => catItem(c)).join("")}
      <div class="group-menu-sep"></div>
      ${item("", t("customize.groupNone"), current === "")}
      ${groups.map((g) => item(g.id, g.name, current === g.id)).join("")}
      <div class="group-menu-sep"></div>
      ${item("__new__", `${t("customize.groupNew")}…`)}`
    : "";
  const overlay = document.createElement("div");
  overlay.className = "group-menu-overlay";
  overlay.innerHTML = `
    <div class="group-menu${includeGrouping ? " overview-move-menu" : ""}" role="menu">
      <div class="group-menu-title">${escapeHtml(t(includeGrouping ? "overview.moveGroup" : "customize.cardSettings"))}</div>
      ${groupingHtml}
      ${includeGrouping ? "" : `<button class="group-menu-item" data-card-config="${escapeHtml(fam)}"><span class="group-menu-check">＋</span>${escapeHtml(t(supportsExtraAccounts(fam) ? "customize.acctAdd" : "customize.addCredential"))}</button>`}
      <div class="group-menu-sep"></div>
      <button class="group-menu-item" data-card-note="${escapeHtml(cardId)}">
        <span class="group-menu-check">✎</span>${escapeHtml(t("customize.noteProviderMenu"))}
      </button>
      ${contextAccount ? `<button class="group-menu-item" data-card-acct-note="${escapeHtml(contextAccount)}">
        <span class="group-menu-check">✎</span>${escapeHtml(t("customize.noteAccountMenu"))} · ${escapeHtml(labelForAccount(contextAccount, accounts))}
      </button>` : ""}
      ${contextAccount ? `<button class="group-menu-item" data-card-pin-acct="${escapeHtml(contextAccount)}"><span class="group-menu-check">★</span>${escapeHtml(t(providerLayout(fam).pinnedAccount === contextAccount ? "customize.acctUnpin" : "customize.acctPin"))} · ${escapeHtml(labelForAccount(contextAccount, accounts))}</button>` : ""}
      ${includeGrouping ? "" : `<button class="group-menu-item" data-card-help="${escapeHtml(fam)}"><span class="group-menu-check">?</span>${escapeHtml(t("customize.helpMenu"))}</button><button class="group-menu-item" data-card-test="${escapeHtml(shownId)}"><span class="group-menu-check">⚡</span>${escapeHtml(t("customize.testConnection"))}</button>`}
      ${includeGrouping ? "" : `<div class="group-menu-sep"></div>
      ${accountIndex >= 0 ? `<button class="group-menu-item" data-card-acct-archive="${accountIndex}"><span class="group-menu-check">⬇</span>${escapeHtml(t("customize.acctArchive"))} · ${escapeHtml(labelForAccount(shownId, accounts))}</button>` : ""}
      ${familyArchived.length ? `${familyArchived
          .map(
            (entry) =>
              `<button class="group-menu-item" data-card-acct-restore="${escapeHtml(fam)}|${escapeHtml(entry.card_id)}"><span class="group-menu-check">⤴</span>${escapeHtml(t("customize.acctRestore"))} · ${escapeHtml(entry.label || entry.card_id)}</button>`,
          )
          .join("")}<div class="group-menu-sep"></div>` : ""}
      <div class="group-menu-sep"></div>
      ${accountIndex >= 0 ? `<button class="group-menu-item danger" data-card-account-remove="${accountIndex}"><span class="group-menu-check">×</span>${escapeHtml(t("customize.acctDelete"))} · ${escapeHtml(labelForAccount(shownId, accounts))}</button>` : ""}
      <button class="group-menu-item danger" data-card-remove="${escapeHtml(cardId)}">
        <span class="group-menu-check">×</span>${escapeHtml(cardId.includes("@") ? t("customize.acctDelete") : t("customize.providerDelete"))}
      </button>`}
    </div>`;
  const close = () => {
    overlay.remove();
    document.removeEventListener("keydown", onKey, true);
  };
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) {
      close();
      return;
    }
    const accountRemove = (e.target as HTMLElement).closest<HTMLElement>("[data-card-account-remove]");
    if (accountRemove) {
      close(); void doAccountRemove(fam, Number(accountRemove.dataset.cardAccountRemove)); return;
    }
    const accountArchive = (e.target as HTMLElement).closest<HTMLElement>("[data-card-acct-archive]");
    if (accountArchive) {
      close();
      void doAccountArchive(fam, Number(accountArchive.dataset.cardAcctArchive));
      return;
    }
    const accountRestore = (e.target as HTMLElement).closest<HTMLElement>("[data-card-acct-restore]");
    if (accountRestore) {
      const [restoreFam, restoreCardId] = accountRestore.dataset.cardAcctRestore!.split("|");
      close();
      void doAccountRestore(restoreFam, restoreCardId);
      return;
    }
    const helpBtn = (e.target as HTMLElement).closest<HTMLElement>("[data-card-help]");
    if (helpBtn) {
      close();
      openProviderHelp(helpBtn.dataset.cardHelp!);
      return;
    }
    const configure = (e.target as HTMLElement).closest<HTMLElement>("[data-card-config]");
    if (configure) {
      close();
      if (fam === "cursor") openCursorAccountDialog();
      else if (supportsExtraAccounts(fam) && supportsApiKey(fam)) openAccountDialog(fam);
      else {
        skinMarketOpen = false; custConfigOpen = fam; setDrawer(true); renderDrawerBody();
        document.querySelector<HTMLElement>(`#drawer-body [data-cust-provider="${CSS.escape(fam)}"]`)?.scrollIntoView({ block: "start" });
      }
      return;
    }
    const removeBtn = (e.target as HTMLElement).closest<HTMLElement>("[data-card-remove]");
    if (removeBtn) {
      close();
      const name = notedName(cardId, providerDisplayName(fam) || cardId);
      void appConfirm({
        title: cardId.includes("@") ? t("customize.acctCardDeleteTitle") : t("customize.providerDeleteTitle"),
        message: cardId.includes("@") ? t("customize.acctCardDeleteConfirm", { name }) : t("customize.providerDeleteConfirm", { name }),
        confirmLabel: cardId.includes("@") ? t("customize.acctDelete") : t("customize.providerDelete"),
        danger: true,
      }).then((ok) => {
        if (ok) removeProviderCard(cardId);
      });
      return;
    }
    const noteBtn = (e.target as HTMLElement).closest<HTMLElement>("[data-card-note]");
    if (noteBtn) {
      // Provider-level note: drives the card title everywhere.
      const noteId = noteBtn.dataset.cardNote!;
      close();
      void appPrompt({
        title: t("customize.notePrompt", { name: fallbackName }),
        placeholder: t("customize.notePh"),
        initial: cardNote(noteId),
        confirmLabel: t("settings.save"),
        allowEmpty: true,
      }).then((note) => {
        if (note === null) return;
        setCardNote(noteId, note);
      });
      return;
    }
    const acctNoteBtn = (e.target as HTMLElement).closest<HTMLElement>("[data-card-acct-note]");
    if (acctNoteBtn) {
      // Account-level note: rides the account's own key — the default (bare)
      // account parks under family@__default__ so it never retitles the
      // provider (see accountNoteKey).
      const acctId = acctNoteBtn.dataset.cardAcctNote!;
      const key = accountNoteKey(acctId);
      const name = labelForAccount(acctId, accountsCache.get(fam) ?? []);
      close();
      void appPrompt({
        title: t("customize.notePrompt", { name }),
        placeholder: t("customize.notePh"),
        initial: cardNote(key),
        confirmLabel: t("settings.save"),
        allowEmpty: true,
      }).then((note) => {
        if (note === null) return;
        setCardNote(key, note);
      });
      return;
    }
    const pinAcct = (e.target as HTMLElement).closest<HTMLElement>("[data-card-pin-acct]");
    if (pinAcct) {
      close();
      setPinnedAccount(fam, pinAcct.dataset.cardPinAcct!);
      return;
    }
    const testBtn = (e.target as HTMLElement).closest<HTMLElement>("[data-card-test]");
    if (testBtn) {
      const testId = testBtn.dataset.cardTest!;
      close();
      const status = document.querySelector("#status")!;
      status.textContent = t("card.testing");
      void invoke<string>("test_provider", { providerId: testId })
        .then((result) => {
          const ms = result.split(":").pop() ?? "";
          status.textContent = result.startsWith("ok:")
            ? t("card.testOk", { ms })
            : t("card.testFail", { err: result.slice(0, result.lastIndexOf(":")) });
          void refresh(true, true);
        })
        .catch((err) => {
          status.textContent = t("card.testFail", { err: String(err) });
        });
      return;
    }
    const catBtn = includeGrouping
      ? (e.target as HTMLElement).closest<HTMLElement>("[data-category-pick]")
      : null;
    if (catBtn) {
      close();
      setFamilyCategory(fam, catBtn.dataset.categoryPick as OverviewCategory);
      return;
    }
    const pick = includeGrouping
      ? (e.target as HTMLElement).closest<HTMLElement>("[data-group-pick]")
      : null;
    if (!pick) return;
    const choice = pick.dataset.groupPick!;
    close();
    void applyGroupChoice(cardId, choice);
  });
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
      document.removeEventListener("keydown", onKey, true);
    }
  };
  document.addEventListener("keydown", onKey, true);
  document.body.appendChild(overlay);
  // Overview buttons use their whole tile as the anchor, matching right-click.
  const menu = overlay.querySelector<HTMLElement>(".group-menu")!;
  const tile = includeGrouping ? anchor.closest<HTMLElement>(".overview-item, .overview-bar-item, .expiring-row") : null;
  const accountAnchor = contextPoint?.target?.closest<HTMLElement>("[data-card-account], [data-card-pin]");
  const rect = (accountAnchor ?? tile ?? anchor).getBoundingClientRect();
  if (contextPoint?.atCursor) {
    // Right-click menus belong at the pointer. Drop below the cursor, flip
    // above when the menu would overflow the bottom, clamp inside the window.
    const belowY = contextPoint.y + 6;
    const aboveY = contextPoint.y - menu.offsetHeight - 6;
    const desiredY = belowY + menu.offsetHeight <= window.innerHeight - 8 ? belowY : aboveY;
    menu.style.left = `${Math.max(8, Math.min(contextPoint.x, window.innerWidth - menu.offsetWidth - 8))}px`;
    menu.style.top = `${Math.max(8, Math.min(desiredY, window.innerHeight - menu.offsetHeight - 8))}px`;
  } else if (tile) {
    const { x, y } = groupMenuPosition(rect, menu.offsetWidth, menu.offsetHeight, overviewMenuSide(tile), window.innerWidth, window.innerHeight);
    menu.style.left = `${x}px`;
    menu.style.top = `${y}px`;
  } else if (contextPoint) {
    // Context menus belong to the card that was pressed. Pick a lower corner
    // based on the press side, then flip above the card only when its bottom
    // would leave the viewport. This avoids menus detached at the window edge.
    const rightSide = contextPoint.x <= rect.left + rect.width / 2;
    const desiredX = rightSide ? rect.right - menu.offsetWidth : rect.left;
    const belowY = rect.bottom + 4;
    const aboveY = rect.top - menu.offsetHeight - 4;
    const desiredY = belowY + menu.offsetHeight <= window.innerHeight - 8 ? belowY : aboveY;
    menu.style.left = `${Math.max(8, Math.min(desiredX, window.innerWidth - menu.offsetWidth - 8))}px`;
    menu.style.top = `${Math.max(8, Math.min(desiredY, window.innerHeight - menu.offsetHeight - 8))}px`;
  } else {
    menu.style.left = `${Math.max(8, Math.min(rect.right - menu.offsetWidth, window.innerWidth - menu.offsetWidth - 8))}px`;
    menu.style.top = `${Math.max(8, Math.min(rect.bottom + 4, window.innerHeight - menu.offsetHeight - 8))}px`;
  }
}

/// "HH:MM" from minutes past midnight.
function peakClock(min: number): string {
  return `${String(Math.floor(min / 60)).padStart(2, "0")}:${String(min % 60).padStart(2, "0")}`;
}

/// Human window text for a peak rule: "Weekdays 14:00–18:00",
/// "Weekdays 09:00–12:00 & 14:00–18:00", "Daily 08:00–22:00".
function peakWindowText(rule: PeakRule): string {
  const days = rule.windows.every((w) => w.days.length === 7)
    ? t("peak.daily")
    : t("peak.weekdays");
  return `${days} ${rule.windows.map((w) => `${peakClock(w.fromMin)}–${peakClock(w.toMin)}`).join(" & ")}`;
}

/// The ? beside the 高峰 chip: a popup listing every ENABLED provider that
/// has peak-hour billing, its window, and whether it is in peak right now.
/// Purely informational — closes on outside click / Escape.
function openPeakHelp(anchor: HTMLElement): void {
  document.querySelector(".group-menu-overlay")?.remove();
  const seen = new Set<string>();
  const rows = orderedSnapshots()
    .map((s) => providerFamily(s.id))
    .filter((fam) => {
      if (!PEAK_RULES[fam] || seen.has(fam)) return false;
      seen.add(fam);
      return true;
    })
    .map((fam) => {
      const visual = providerVisual(fam);
      const icon = visual?.iconSvg ?? "";
      const name = notedName(fam, providerDisplayName(fam));
      const inPeak = isProviderInPeak(fam);
      return `
      <div class="peak-help-row${inPeak ? " in-peak" : ""}">
        <span class="peak-help-icon">${icon}</span>
        <span class="peak-help-name">${escapeHtml(name)}</span>
        <span class="peak-help-window">${escapeHtml(peakWindowText(PEAK_RULES[fam]))}</span>
        ${inPeak ? `<span class="peak-help-now">${escapeHtml(t("peak.nowTag"))}</span>` : ""}
      </div>`;
    })
    .join("");
  const overlay = document.createElement("div");
  overlay.className = "group-menu-overlay";
  overlay.innerHTML = `
    <div class="group-menu peak-help" role="dialog">
      <div class="group-menu-title">${escapeHtml(t("peak.helpTitle"))}</div>
      ${rows || `<div class="peak-help-empty">${escapeHtml(t("peak.helpEmpty"))}</div>`}
    </div>`;
  const close = () => {
    overlay.remove();
    document.removeEventListener("keydown", onKey, true);
  };
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay || !(e.target as HTMLElement).closest(".peak-help")) close();
  });
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
    }
  };
  document.addEventListener("keydown", onKey, true);
  document.body.appendChild(overlay);
  // Anchor under the ? button, clamped into the viewport.
  const menu = overlay.querySelector<HTMLElement>(".peak-help")!;
  const rect = anchor.getBoundingClientRect();
  const mw = menu.offsetWidth;
  const x = Math.max(8, Math.min(rect.left + rect.width / 2 - mw / 2, window.innerWidth - mw - 8));
  menu.style.left = `${x}px`;
  menu.style.top = `${Math.min(rect.bottom + 6, window.innerHeight - menu.offsetHeight - 8)}px`;
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
    // renders — bars, quota bars, the trend, and the On Demand section
    // when it's open — copies as-is. Only interactive chrome (buttons,
    // links, carets, grips) never belongs in an image. (The old "compact
    // composition" for collapsed cards is retired: it dropped the visible
    // trend and quota bars, which read as missing data in the copy.)
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

/// Chromium (WebView2 included) silently turns programmatic
/// behavior:"smooth" scrolls into no-ops when the OS "animate controls"
/// setting is off — the scroll never starts, so overview jumps to cards
/// below the fold dead-click. Probe once at boot; every jump site falls
/// back to instant scrolling when smooth is unavailable.
let smoothScrollWorks = true;
function probeSmoothScroll(): void {
  const probe = document.createElement("div");
  probe.setAttribute("aria-hidden", "true");
  probe.style.cssText =
    "position:fixed;left:-9999px;top:0;width:40px;height:40px;overflow-y:scroll;";
  probe.innerHTML = "<div style='height:120px'></div>";
  document.body.appendChild(probe);
  probe.scrollTo({ top: 60, behavior: "smooth" });
  window.setTimeout(() => {
    smoothScrollWorks = probe.scrollTop > 0;
    probe.remove();
  }, 150);
}
function scrollBehavior(): ScrollBehavior {
  return smoothScrollWorks ? "smooth" : "auto";
}

function overviewJumpBehavior(): ScrollBehavior {
  return config.jumpAnimation === "instant" ? "auto" : "smooth";
}

let overviewJumpFrame = 0;
function scrollOverviewCard(card: HTMLElement): void {
  const scroller = document.querySelector<HTMLElement>("#providers");
  if (!scroller) return;
  const scrollerRect = scroller.getBoundingClientRect();
  const cardRect = card.getBoundingClientRect();
  const target = Math.max(
    0,
    scroller.scrollTop + cardRect.top - scrollerRect.top - (scroller.clientHeight - cardRect.height) / 2,
  );
  cancelAnimationFrame(overviewJumpFrame);
  if (overviewJumpBehavior() === "auto") {
    scroller.scrollTop = target;
    return;
  }
  const start = scroller.scrollTop;
  const distance = target - start;
  const started = performance.now();
  const duration = Math.min(520, Math.max(260, Math.abs(distance) * 0.55));
  const tick = (now: number) => {
    const progress = Math.min(1, (now - started) / duration);
    const eased = progress < 0.5
      ? 4 * progress * progress * progress
      : 1 - Math.pow(-2 * progress + 2, 3) / 2;
    scroller.scrollTop = start + distance * eased;
    if (progress < 1) overviewJumpFrame = requestAnimationFrame(tick);
  };
  overviewJumpFrame = requestAnimationFrame(tick);
}

function applyReduceMotion(): void {
  document.body.classList.toggle("reduce-anim", config.reduceAnimations === true);
}

let lensReady = false;

function initLiquidLens(): void {
  if (config.glassEffects === false || lensReady) return;
  lensReady = true;
  const surfaces: [string, string, HTMLElement | null][] = [
    // The provider rail is intentionally solid; keep the lens only on the
    // footer surface where the glass treatment remains useful.
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
  // WebView2's native UI (the <select> popup menu, scrollbars) is painted by
  // the host's PreferredColorScheme, which page CSS cannot reach. An explicit
  // theme pins it to Pane's choice; "system" passes null to release it back to
  // Auto — pinning there would pollute prefers-color-scheme and stall the
  // follow-the-system change listener below.
  void getCurrentWebviewWindow()
    .setTheme(config.appearance === "system" ? null : (mode as "light" | "dark"))
    .catch(() => {});
  document.documentElement.dataset.density = config.density;
  const btn = document.querySelector<HTMLElement>("#theme-btn");
  if (btn) {
    btn.textContent = mode === "light" ? "☾" : "☀";
    btn.title = mode === "light" ? t("sidebar.themeToDark") : t("sidebar.themeToLight");
    delete btn.dataset.tip;
  }
  applyUiFont();
}

/// Settings font picker: a custom family replaces only the head of the
/// stock stack (kept in sync with the var() fallback in styles.css :root),
/// so missing glyphs still fall back. Cleared = stock font. String() guards
/// against a hand-edited config.json carrying a non-string uiFont.
function applyUiFont(): void {
  const root = document.documentElement;
  const font = String(config.uiFont ?? "").trim();
  if (!font) {
    root.style.removeProperty("--app-font");
    requestAnimationFrame(fitProviderNames);
    return;
  }
  const safe = font.replace(/[\\"]/g, "\\$&");
  root.style.setProperty(
    "--app-font",
    `"${safe}", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif`,
  );
  requestAnimationFrame(fitProviderNames);
}

// Magpie-style top-level settings navigation for the floating window. The
// existing accordion markup remains the source of truth; tabs only switch
// which section is visible, so no setting control or listener is duplicated.
function setupFloatingSettingsTabs(): void {
  const settings = document.querySelector<HTMLElement>("#settings");
  const head = settings?.querySelector<HTMLElement>(".panel-head");
  if (!settings || !head || settings.querySelector(".settings-tabs")) return;
  const definitions = [
    ["general", "settings.tabGeneral"],
    ["usage", "settings.tabUsage"],
    ["shortcuts", "settings.tabShortcuts"],
    ["notifications", "settings.tabNotifications"],
    ["privacy", "settings.tabPrivacy"],
    ["network", "settings.tabNetwork"],
    ["about", "settings.tabAbout"],
  ] as const;
  const groups = Array.from(settings.querySelectorAll<HTMLElement>(".acc-group"));
  const tabs = document.createElement("div");
  tabs.className = "settings-tabs";
  tabs.setAttribute("role", "tablist");
  tabs.setAttribute("aria-label", t("settings.title"));
  const activate = (id: string) => {
    groups.forEach((group, index) => {
      const section = definitions[index]?.[0] ?? "general";
      group.hidden = section !== id;
    });
    tabs.querySelectorAll<HTMLButtonElement>("button").forEach((button) => {
      const active = button.dataset.settingsTab === id;
      button.classList.toggle("active", active);
      button.setAttribute("aria-selected", String(active));
    });
  };
  definitions.forEach(([id, labelKey], index) => {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.settingsTab = id;
    button.dataset.i18n = labelKey;
    button.textContent = t(labelKey);
    button.setAttribute("role", "tab");
    button.addEventListener("click", () => activate(id));
    tabs.append(button);
    if (groups[index]) groups[index].dataset.settingsSection = id;
  });
  head.after(tabs);
  activate("general");
}

/// Settings font picker internals. The menu is a body-level fixed layer (the
/// accordion clips absolutely-positioned children) styled with theme vars;
/// each option renders its own name in its own font.
let systemFontFamilies: string[] | null = null; // null until loaded; reset on failure = retry next open
let fontMenu: HTMLDivElement | null = null;
let fontMenuList: string[] = [];
let fontMenuFocused = -1;

/// Quote a family name for use inside a CSS "..." string.
function cssFontName(name: string): string {
  return name.replace(/[\\"]/g, "\\$&");
}

function fontStack(name: string): string {
  return `"${cssFontName(name)}", "Segoe UI Variable", "Segoe UI", system-ui, sans-serif`;
}

async function ensureSystemFonts(): Promise<void> {
  if (systemFontFamilies) return;
  try {
    systemFontFamilies = await invoke<string[]>("list_system_fonts");
  } catch {
    // Leave null so the next focus retries the enumeration.
  }
}

function closeFontMenu(): void {
  fontMenu?.remove();
  fontMenu = null;
  fontMenuFocused = -1;
  document.removeEventListener("pointerdown", onFontMenuOutside);
  // Discard typed-but-unapplied text so the field always mirrors config.
  const input = document.querySelector<HTMLInputElement>("#ui-font");
  if (input && String(config.uiFont ?? "") !== input.value) {
    input.value = String(config.uiFont ?? "");
    input.style.fontFamily = config.uiFont ? fontStack(config.uiFont) : "";
  }
}

function onFontMenuOutside(e: PointerEvent): void {
  const target = e.target as Node;
  const input = document.querySelector<HTMLInputElement>("#ui-font");
  if (fontMenu && !fontMenu.contains(target) && target !== input) closeFontMenu();
}

function renderFontOptions(filter: string): void {
  const menu = fontMenu;
  if (!menu) return;
  const all = systemFontFamilies ?? [];
  const q = filter.trim().toLowerCase();
  fontMenuList = q ? all.filter((f) => f.toLowerCase().includes(q)) : all;
  const current = String(config.uiFont ?? "").trim().toLowerCase();
  fontMenuFocused = fontMenuList.length ? 0 : -1;
  menu.textContent = "";
  if (!fontMenuList.length) {
    const empty = document.createElement("div");
    empty.className = "font-empty";
    empty.textContent = t("settings.fontNone");
    menu.appendChild(empty);
    return;
  }
  fontMenuList.forEach((name, i) => {
    const opt = document.createElement("div");
    opt.className =
      `font-option${name.toLowerCase() === current ? " selected" : ""}` +
      `${i === fontMenuFocused ? " focused" : ""}`;
    opt.textContent = name;
    opt.style.fontFamily = fontStack(name);
    menu.appendChild(opt);
  });
}

function openFontMenu(): void {
  closeFontMenu();
  const input = document.querySelector<HTMLInputElement>("#ui-font");
  if (!input) return;
  const menu = document.createElement("div");
  menu.className = "font-menu";
  menu.addEventListener("click", (e) => {
    const opt = (e.target as HTMLElement).closest<HTMLElement>(".font-option");
    if (opt?.textContent) selectFont(opt.textContent);
  });
  document.body.appendChild(menu);
  fontMenu = menu;
  const rect = input.getBoundingClientRect();
  menu.style.left = `${rect.left}px`;
  menu.style.top = `${rect.bottom + 4}px`;
  menu.style.width = `${Math.max(rect.width, 220)}px`;
  void ensureSystemFonts().then(() => renderFontOptions(input.value));
  document.addEventListener("pointerdown", onFontMenuOutside);
}

function selectFont(name: string): void {
  const input = document.querySelector<HTMLInputElement>("#ui-font");
  if (input) {
    input.value = name;
    input.style.fontFamily = name ? fontStack(name) : "";
  }
  closeFontMenu();
  void patchConfig({ uiFont: name }).then(applyAppearance);
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

// Which provider's inline config panel is open (one at a time).
// Session-only, like custExpanded.
let custConfigOpen: string | null = null;
/// Whether the drawer footer's removed-provider list is expanded.
let custRemovedOpen = false;
// Single-key providers show the saved credential as a compact account row.
// The input form is an explicit add/edit action, so opening settings never
// drops the user into a blank form over an existing credential.
const custKeyEditing = new Set<string>();

// Which provider's read-only credential-info panel ("?" button) is open.
// One panel at a time, and opening one side closes the other.
let custInfoOpen: string | null = null;

// "Stored key / env key / local sign-in" answers from get_credential_status,
// cached so a background re-render of the drawer doesn't blank the panels.
interface CredStatus {
  storedKey: boolean;
  maskedKey?: string | null;
  envKey: boolean;
  localCli: string | null;
  // Account label of Pane's own OAuth login (codex/grok), null when none.
  oauth: string | null;
  // Kimi only: the source selected by the backend's refresh precedence.
  activeSource?: "api_key" | "oauth" | null;
  // Subscription/membership badge for local sign-ins (Cursor reads its
  // own local state DB; cockpit badges accounts the same way).
  membership?: string | null;
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
      if (custConfigOpen === id && !custKeyEditing.has(id)) renderDrawerBody();
    })
    .catch(() => {
      credStatusCache.set(id, {
        storedKey: false,
        envKey: false,
        localCli: null,
        oauth: null,
        activeSource: null,
      });
      paintCredStatus(id);
      if (custConfigOpen === id && !custKeyEditing.has(id)) renderDrawerBody();
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
const KEY_PROVIDERS = new Set(
  providerCatalog
    .filter((definition) => supportsApiKey(definition.familyId))
    .map((definition) => definition.familyId),
);

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
  copilot: { auto: "customize.cred.copilot", methods: ["local", "oauth"] },
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
  kimi: { auto: "customize.cred.kimi", methods: ["paste", "oauth"] },
  stepfun: { auto: "customize.cred.stepfun", methods: ["paste"] },
  "stepfun-plan": { auto: "customize.cred.stepfunPlan", methods: ["paste"] },
  clinepass: { auto: "customize.cred.clinepass", methods: ["paste"] },
  sensenova: { auto: "customize.cred.sensenova", methods: ["paste"] },
  apigoto: { auto: "customize.cred.apigoto", methods: ["paste"] },
  "amp": { auto: "customize.cred.amp", methods: ["paste"] },
  "bedrock": { auto: "customize.cred.bedrock", methods: ["local"] },
  "chutes": { auto: "customize.cred.chutes", methods: ["paste"] },
  "deepgram": { auto: "customize.cred.deepgram", methods: ["paste"] },
  "kiro": { auto: "customize.cred.kiro", methods: ["local"] },
  "openai-api": { auto: "customize.cred.openaiApi", methods: ["paste"] },
  "poe": { auto: "customize.cred.poe", methods: ["paste"] },
  "venice": { auto: "customize.cred.venice", methods: ["paste"] },
  "vertexai": { auto: "customize.cred.vertexai", methods: ["local"] },
  "warp": { auto: "customize.cred.warp", methods: ["paste"] },
  "mimo": { auto: "customize.cred.mimo", methods: ["paste"] },
  "trae": { auto: "customize.cred.trae", methods: ["local"] },
  "qoder": { auto: "customize.cred.qoder", methods: ["local"] },
  "zed": { auto: "customize.cred.zed", methods: ["local"] },
  "factory": { auto: "customize.cred.factory", methods: ["paste"] },
  "jetbrains": { auto: "customize.cred.jetbrains", methods: ["local"] },
  "groq": { auto: "customize.cred.groq", methods: ["paste"] },
  "huggingface": { auto: "customize.cred.huggingface", methods: ["paste"] },
  "longcat": { auto: "customize.cred.longcat", methods: ["paste"] },
  "sub2api": { auto: "customize.cred.sub2api", methods: ["paste"] },
  "mistral": { auto: "customize.cred.mistral", methods: ["paste"] },
  "perplexity": { auto: "customize.cred.perplexity", methods: ["paste"] },
  "volcengine": { auto: "customize.cred.volcengine", methods: ["paste"] },
  siliconflow: { auto: "customize.cred.siliconflow", methods: ["paste"] },
  novita: { auto: "customize.cred.novita", methods: ["paste"] },
  relaybalance: { auto: "customize.cred.relaybalance", methods: ["paste"] },
  qodercn: { auto: "customize.cred.qodercn", methods: ["local"] },
  traecn: { auto: "customize.cred.traecn", methods: ["local"] },
  shandianshuo: { auto: "customize.cred.shandianshuo", methods: ["local"] },
};

/// The "?" panel's read-only fact sheet: an ordered list of how this
/// provider can be read (local config file, API key, OAuth callback), not
/// a config surface — all actions live in the ⚙ panel.
function renderCustInfo(id: string): string {
  const info = PROVIDER_CRED_INFO[providerFamily(id)];
  const auto = info ? escapeHtml(t(info.auto)) : "";
  const methods = info
    ? info.methods
        .map((m) => `<li>${escapeHtml(t(`customize.credMethod.${m}`))}</li>`)
        .join("")
    : `<li class="dim">${escapeHtml(t("customize.credMethodNone"))}</li>`;
  return `<div class="cust-config cust-info">
      <p><span class="cust-info-label">${escapeHtml(t("customize.credAutoLabel"))}</span>${auto}</p>
      <ol class="cust-info-methods">
        ${methods}
      </ol>
      <p><span class="cust-info-label">${escapeHtml(t("customize.credStatusLabel"))}</span><span class="dim" data-cred-status="${escapeHtml(id)}">${escapeHtml(t("customize.credStatusLoading"))}</span></p>
    </div>`;
}

/// Kimi's active credential source: the backend's stated precedence, or
/// the same fallback order when it hasn't stated one. Shared by the "?"
/// status line and the ⚙ chips so the two can never drift apart.
function kimiActiveSource(status: CredStatus): "api_key" | "oauth" | null {
  return (
    status.activeSource ??
    (status.storedKey || status.envKey ? "api_key" : status.localCli ? "oauth" : null)
  );
}

/// One status line ("?" panel): for Kimi, show only the credential source
/// selected by the backend. Other providers retain their source inventory.
/// The subscription/membership tier, when known, is appended — that is the
/// "当前状态" the user asked for: source + tier, not just "已保存".
function credStatusLine(id: string): string {
  const status = credStatusCache.get(id);
  if (!status) return escapeHtml(t("customize.credStatusLoading"));
  const membership = status.membership
    ? ` · <span class="cred-chip tier">${escapeHtml(status.membership)}</span>`
    : "";
  if (providerFamily(id) === "kimi") {
    const source = kimiActiveSource(status);
    if (source === "api_key") {
      return escapeHtml(
        t(status.storedKey ? "customize.credStored" : "customize.credKimiEnv"),
      ) + membership;
    }
    if (source === "oauth" && status.localCli) {
      return escapeHtml(t("customize.chipKimiOAuth", { x: status.localCli })) + membership;
    }
    return escapeHtml(t("customize.credNotStored")) + membership;
  }
  const parts: string[] = [];
  parts.push(
    status.storedKey
      ? escapeHtml(t("customize.credStored"))
      : escapeHtml(t("customize.credNotStored")),
  );
  if (status.envKey) parts.push(escapeHtml(t("customize.credEnv")));
  if (status.localCli) parts.push(escapeHtml(status.localCli));
  return parts.join(" · ") + membership;
}

/// The gear panel's live status chips: one green chip for Kimi's active
/// credential source, or the existing source inventory for other providers.
function credChipsHtml(id: string): string {
  const status = credStatusCache.get(id);
  if (!status) return `<span class="dim">${escapeHtml(t("customize.credStatusLoading"))}</span>`;
  if (providerFamily(id) === "kimi") {
    const source = kimiActiveSource(status);
    if (source === "api_key" && status.storedKey) {
      return `<span class="cred-chip ok">${escapeHtml(t("customize.chipStoredKey"))}</span>`;
    }
    if (source === "api_key" && status.envKey) {
      return `<span class="cred-chip ok">${escapeHtml(t("customize.chipKimiEnvKey"))}</span>`;
    }
    if (source === "oauth" && status.localCli) {
      return `<span class="cred-chip ok">${escapeHtml(t("customize.chipKimiOAuth", { x: status.localCli }))}</span>`;
    }
    return `<span class="cred-chip none">${escapeHtml(t("customize.chipNone"))}</span>`;
  }
  const chips: string[] = [];
  if (status.storedKey)
    chips.push(`<span class="cred-chip ok">${escapeHtml(t("customize.chipStoredKey"))}</span>`);
  if (status.envKey)
    chips.push(`<span class="cred-chip ok">${escapeHtml(t("customize.chipEnvKey"))}</span>`);
  if (status.localCli)
    chips.push(
      `<span class="cred-chip ok">${escapeHtml(t("customize.chipLocal", { x: status.localCli }))}</span>`,
    );
  // Subscription/membership badge for local sign-ins: Free vs Pro vs
  // Ultra. The quota card may not be live (e.g. Free accounts), so the
  // tier is reported independently of usage data.
  if (status.membership)
    chips.push(`<span class="cred-chip tier">${escapeHtml(status.membership)}</span>`);
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
const OAUTH_PROVIDERS = new Set(["codex", "grok", "copilot"]);
/// Browser-PKCE families (the backend runs the loopback callback and its
/// own `<family>_login_*` commands).
const BROWSER_LOGIN_PROVIDERS = new Set([
  "codebuddy",
  "codex",
  "kiro",
  "qoder",
  "trae",
  "windsurf",
  "zed",
]);

// Relay families whose saved credential also carries a user-chosen base
// URL (relaybalance) — its gear panel and account dialog show
// the extra URL field.
const RELAY_BASE_URL_FAMILIES = new Set(["relaybalance", "sub2api"]);

// ---------------------------------------------------------------------------
// Extra API-key accounts (Phase 3.2) — deepseek/kimi/stepfun/siliconflow/
// novita/relaybalance. The gear panel's single-key field stays the family's main
// card; each entry below adds a stable <provider>@<fingerprint> card on the
// dashboard.
// ---------------------------------------------------------------------------

/// One account_list row: the masked key ("sk-…abcd") is all that comes
/// back — the full key never leaves the backend.
interface AccountEntry {
  id?: string;
  label: string;
  email?: string;
  maskedKey: string;
  baseUrl?: string | null;
}

// Cached account lists so a drawer re-render doesn't blank the panels,
// same trade-off as credStatusCache.
const accountsCache = new Map<string, AccountEntry[]>();

// The account editor is a real modal, not an inline expansion inside the
// provider row. Keep one dismissal hook so opening another provider or
// closing Customize cannot leave a stale editor behind.
let dismissAccountDialog: (() => void) | null = null;

// A live probe must only unlock the exact input value it tested. A request
// can finish after the user edits the form (or after a newer probe), so keep
// a small generation ledger instead of trusting promise completion order.
const testGenerations = new Map<string, number>();

function bumpTestGeneration(scope: "cust" | "acct", id: string): number {
  const key = `${scope}:${id}`;
  const next = (testGenerations.get(key) ?? 0) + 1;
  testGenerations.set(key, next);
  return next;
}

function isCurrentTestGeneration(scope: "cust" | "acct", id: string, generation: number): boolean {
  return testGenerations.get(`${scope}:${id}`) === generation;
}

function fetchAccounts(family: string): void {
  if (accountsCache.has(family)) return;
  refreshAccounts(family);
}

/// Archived account tombstones per family (archived_accounts.json via the
/// backend). Kept beside accountsCache so the card menu can offer restore
/// rows synchronously.
interface ArchivedAccountRow {
  provider: string;
  card_id: string;
  label: string;
  archived_at: number;
}
const archivedAccountsCache = new Map<string, ArchivedAccountRow[]>();

function refreshAccounts(family: string): void {
  void invoke<ArchivedAccountRow[]>("archived_accounts", { provider: family })
    .then((list) => {
      archivedAccountsCache.set(family, list);
      // The archive list changes independently of account_list. Repaint the
      // menu source immediately so a freshly archived account is offered in
      // Restore without requiring another quota refresh.
      if (customizeOpen) renderDrawerBody();
      else if (lastSnapshots.length) renderIfVisible();
    })
    .catch(() => {
      archivedAccountsCache.set(family, []);
    });
  void invoke<AccountEntry[]>("account_list", { provider: family })
    .then((list) => {
      accountsCache.set(family, list);
      const layoutChanged = reconcileAccountLayout(family, list);
      // The list feeds both the Customize child rows and the dashboard's
      // merged-card tabs, so both surfaces need the fresh labels.
      if (customizeOpen) renderDrawerBody();
      else if (layoutChanged || lastSnapshots.length) renderIfVisible();
    })
    .catch(() => {
      accountsCache.set(family, []);
      if (customizeOpen) renderDrawerBody();
    });
}

/// Returns the display label for an account id. Priority:
/// 1. saved label (user-set name)
/// 2. email (for imported accounts with a known email)
/// 3. fingerprint suffix (id after the @)
/// 4. bare id as last resort
function labelForAccount(id: string, list: AccountEntry[]): string {
  // A per-account note (renamed from the account's own context menu) beats
  // the stored label — that's the whole point of renaming one capsule.
  const note = accountNote(id);
  if (note) return note;
  const entry = list.find((e) => e.id === id);
  if (entry?.label) return entry.label;
  if (entry?.email) return entry.email;
  return id.split("@")[1]?.slice(0, 8) ?? id;
}

/// Cursor add-account dialog: three tabs mirroring cockpit's import paths —
/// OAuth browser login (PKCE deep link), token/refresh paste, and JSON
/// import (cockpit-compatible field aliases).
function openCursorAccountDialog(): void {
  dismissAccountDialog?.();
  const overlay = document.createElement("div");
  overlay.id = "account-overlay";
  overlay.innerHTML = `
    <section class="account-dialog" role="dialog" aria-modal="true" aria-labelledby="cursor-account-title">
      <div class="account-dialog-head">
        <h3 id="cursor-account-title">${escapeHtml(t("customize.cursorAddTitle"))}</h3>
        <button class="account-dialog-close" data-acct-close type="button" aria-label="${escapeHtml(t("dialog.cancel"))}">${uiIcon("x")}</button>
      </div>
      <div class="cursor-add-tabs">
        <button class="cursor-tab on" data-cursor-tab="oauth">${escapeHtml(t("customize.cursorTabOAuth"))}</button>
        <button class="cursor-tab" data-cursor-tab="token">${escapeHtml(t("customize.cursorTabToken"))}</button>
        <button class="cursor-tab" data-cursor-tab="json">${escapeHtml(t("customize.cursorTabJson"))}</button>
      </div>
      <div class="cursor-tab-body" data-cursor-tab-body="oauth">
        <p class="account-dialog-help">${escapeHtml(t("customize.cursorOAuthHelp"))}</p>
        <button class="mini-btn" data-cursor-oauth-start>${escapeHtml(t("customize.cursorOAuthStart"))}</button>
        <span class="cust-test-result" data-cursor-oauth-result></span>
      </div>
      <div class="cursor-tab-body" data-cursor-tab-body="token" hidden>
        <label class="account-field">
          <span>${escapeHtml(t("customize.cursorTokenLabel"))}</span>
          <input type="password" data-cursor-token autocomplete="new-password" spellcheck="false" />
        </label>
        <label class="account-field">
          <span>${escapeHtml(t("customize.cursorRefreshLabel"))}</span>
          <input type="password" data-cursor-refresh autocomplete="new-password" spellcheck="false" />
        </label>
        <label class="account-field">
          <span>${escapeHtml(t("customize.acctNoteLabel"))}</span>
          <input type="text" data-cursor-token-label autocomplete="off" spellcheck="false" />
        </label>
        <button class="mini-btn" data-cursor-token-import>${escapeHtml(t("customize.cursorTokenImport"))}</button>
        <span class="cust-test-result" data-cursor-token-result></span>
      </div>
      <div class="cursor-tab-body" data-cursor-tab-body="json" hidden>
        <p class="account-dialog-help">${escapeHtml(t("customize.cursorJsonHelp"))}</p>
        <textarea class="cursor-json-input" data-cursor-json rows="8" spellcheck="false"></textarea>
        <button class="mini-btn" data-cursor-json-import>${escapeHtml(t("customize.cursorJsonImport"))}</button>
        <span class="cust-test-result" data-cursor-json-result></span>
      </div>
    </section>`;

  let closed = false;
  let activeLoginId: string | null = null;
  const done = () => {
    if (closed) return;
    closed = true;
    stopOauthPoll();
    document.removeEventListener("keydown", onKey, true);
    overlay.remove();
    if (dismissAccountDialog === done) dismissAccountDialog = null;
  };
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      done();
    }
  };
  dismissAccountDialog = done;
  document.addEventListener("keydown", onKey, true);
  document.body.appendChild(overlay);

  // Shared success tail for all three import paths: refresh the account
  // list, close the dialog, force a quota refresh.
  const finishImport = () => {
    refreshAccounts("cursor");
    window.setTimeout(() => {
      done();
      void forceUsageRefreshAttempt(false).then(requestTraySync);
    }, 900);
  };

  // Tab switching.
  overlay.querySelectorAll<HTMLElement>("[data-cursor-tab]").forEach((tab) => {
    tab.addEventListener("click", () => {
      const name = tab.dataset.cursorTab!;
      overlay.querySelectorAll(".cursor-tab").forEach((t) => t.classList.toggle("on", t === tab));
      overlay.querySelectorAll<HTMLElement>("[data-cursor-tab-body]").forEach((body) => {
        body.hidden = body.dataset.cursorTabBody !== name;
      });
    });
  });

  // OAuth flow: start → open browser → poll (one backend tick per call)
  // until done. Closing the dialog cancels the pending backend session.
  let pollTimer: number | undefined;
  const stopOauthPoll = () => {
    if (pollTimer !== undefined) window.clearInterval(pollTimer);
    pollTimer = undefined;
    if (activeLoginId) {
      void invoke("cursor_oauth_cancel", { loginId: activeLoginId }).catch(() => {});
      activeLoginId = null;
    }
  };
  const oauthResult = overlay.querySelector<HTMLElement>("[data-cursor-oauth-result]")!;
  const oauthStart = overlay.querySelector<HTMLElement>("[data-cursor-oauth-start]")!;
  oauthStart.addEventListener("click", () => {
    oauthStart.setAttribute("disabled", "");
    oauthResult.textContent = t("customize.cursorOAuthStarting");
    oauthResult.classList.remove("ok", "err");
    void invoke<{ loginId: string; verificationUri: string }>("cursor_oauth_start", {})
      .then(async (started) => {
        activeLoginId = started.loginId;
        void invoke("open_link", { url: started.verificationUri }).catch(() => {});
        oauthResult.textContent = t("customize.cursorOAuthWaiting");
        stopOauthPoll();
        pollTimer = window.setInterval(async () => {
          try {
            const poll = await invoke<{
              done: boolean;
              error: string | null;
              account: { email: string } | null;
            }>("cursor_oauth_poll", { loginId: started.loginId });
            if (!poll.done && !poll.error) return;
            stopOauthPoll();
            activeLoginId = null;
            if (poll.error) {
              oauthResult.textContent = `${t("customize.testFailed")}: ${poll.error}`;
              oauthResult.classList.add("err");
              oauthStart.removeAttribute("disabled");
            } else {
              oauthResult.textContent = t("customize.cursorOAuthDone", {
                email: poll.account?.email || "",
              });
              oauthResult.classList.add("ok");
              finishImport();
            }
          } catch (err) {
            stopOauthPoll();
            oauthResult.textContent = `${t("customize.testFailed")}: ${String(err)}`;
            oauthResult.classList.add("err");
            oauthStart.removeAttribute("disabled");
          }
        }, 2000);
      })
      .catch((err) => {
        oauthResult.textContent = `${t("customize.testFailed")}: ${String(err)}`;
        oauthResult.classList.add("err");
        oauthStart.removeAttribute("disabled");
      });
  });

  // Token import.
  overlay.querySelector<HTMLElement>("[data-cursor-token-import]")?.addEventListener("click", () => {
    const access = overlay.querySelector<HTMLInputElement>("[data-cursor-token]")!.value.trim();
    if (!access) return;
    const refresh = overlay.querySelector<HTMLInputElement>("[data-cursor-refresh]")!.value.trim();
    const label = overlay.querySelector<HTMLInputElement>("[data-cursor-token-label]")!.value.trim();
    const result = overlay.querySelector<HTMLElement>("[data-cursor-token-result]")!;
    void appConfirm({
      title: t("customize.cursorTabToken"),
      message: t("customize.cursorTokenConfirm"),
      confirmLabel: t("customize.cursorTokenImport"),
    }).then((ok) => {
      if (!ok) return;
      void invoke<number>("cursor_import", {
        jsonContent: JSON.stringify({
          access_token: access,
          refresh_token: refresh || undefined,
          name: label || undefined,
        }),
      })
        .then(() => {
          result.textContent = t("customize.cursorImportDone");
          result.classList.add("ok");
          finishImport();
        })
        .catch((err) => {
          result.textContent = `${t("customize.testFailed")}: ${String(err)}`;
          result.classList.add("err");
        });
    });
  });

  // JSON import.
  overlay.querySelector<HTMLElement>("[data-cursor-json-import]")?.addEventListener("click", () => {
    const text = overlay.querySelector<HTMLTextAreaElement>("[data-cursor-json]")!.value;
    const result = overlay.querySelector<HTMLElement>("[data-cursor-json-result]")!;
    void appConfirm({
      title: t("customize.cursorTabJson"),
      message: t("customize.cursorTokenConfirm"),
      confirmLabel: t("customize.cursorJsonImport"),
    }).then((ok) => {
      if (!ok) return;
      void invoke<number>("cursor_import", { jsonContent: text })
        .then((n) => {
          result.textContent = t("customize.cursorImportCount", { n });
          result.classList.add("ok");
          finishImport();
        })
        .catch((err) => {
          result.textContent = `${t("customize.testFailed")}: ${String(err)}`;
          result.classList.add("err");
        });
    });
  });

  // Backdrop / close click.
  overlay.addEventListener("click", (event) => {
    if (event.target === overlay || (event.target as HTMLElement).closest("[data-acct-close]")) {
      done();
    }
  });
}

function openAccountDialog(family: string): void {
  dismissAccountDialog?.();
  const overlay = document.createElement("div");
  overlay.id = "account-overlay";
  overlay.innerHTML = `
    <section class="account-dialog" data-account-dialog="${escapeHtml(family)}" role="dialog" aria-modal="true" aria-labelledby="account-dialog-title">
      <div class="account-dialog-head">
        <h3 id="account-dialog-title">${escapeHtml(t("customize.acctDialogTitle", { name: providerDisplayName(family) }))}</h3>
        <button class="account-dialog-close" data-acct-close type="button" aria-label="${escapeHtml(t("dialog.cancel"))}">${uiIcon("x")}</button>
      </div>
      <p class="account-dialog-help">${escapeHtml(t("customize.acctDialogHelp"))}</p>
      ${
        getApiKeyLink(family)
          ? `<p class="account-dialog-getkey"><button class="mini-btn" data-acct-getkey="${escapeHtml(getApiKeyLink(family)!)}" type="button">${escapeHtml(t("customize.getApiKey"))}</button></p>`
          : ""
      }
      <div class="account-dialog-form">
        <label class="account-field">
          <span>${escapeHtml(t("customize.acctKeyLabel"))}</span>
          <input type="password" data-acct-key="${escapeHtml(family)}" placeholder="${escapeHtml(t("settings.keyPlaceholder"))}" autocomplete="new-password" spellcheck="false" />
        </label>
        <label class="account-field">
          <span>${escapeHtml(t("customize.acctNoteLabel"))}</span>
          <input type="text" data-acct-label="${escapeHtml(family)}" placeholder="${escapeHtml(t("customize.acctLabelPh"))}" autocomplete="off" spellcheck="false" required />
        </label>
        ${
          RELAY_BASE_URL_FAMILIES.has(family)
            ? `<label class="account-field">
          <span>${escapeHtml(t("settings.relayBaseUrl"))}</span>
          <input type="text" data-acct-baseurl="${escapeHtml(family)}" placeholder="https://api.example.com" spellcheck="false" />
        </label>`
            : ""
        }
        <div class="account-dialog-test-row">
          <button class="mini-btn" data-acct-test="${escapeHtml(family)}" type="button" disabled>${escapeHtml(t("customize.test"))}</button>
          <span class="cust-test-result" data-acct-result="${escapeHtml(family)}"></span>
        </div>
        <div class="account-dialog-footer">
          <button class="mini-btn account-dialog-cancel" data-acct-close type="button">${escapeHtml(t("dialog.cancel"))}</button>
          <button class="mini-btn account-dialog-save" data-acct-add="${escapeHtml(family)}" type="button" disabled title="${escapeHtml(t("customize.saveAfterTest"))}">${escapeHtml(t("customize.acctSaveBtn"))}</button>
        </div>
      </div>
    </section>`;

  let closed = false;
  const done = () => {
    if (closed) return;
    closed = true;
    document.removeEventListener("keydown", onKey, true);
    overlay.remove();
    if (dismissAccountDialog === done) dismissAccountDialog = null;
  };
  const onKey = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      done();
    }
  };
  overlay.addEventListener("click", (event) => {
    const target = event.target as HTMLElement;
    if (event.target === overlay || target.closest("[data-acct-close]")) {
      done();
      return;
    }
    const getKey = target.closest<HTMLElement>("[data-acct-getkey]");
    if (getKey) {
      void invoke("open_link", { url: getKey.dataset.acctGetkey }).catch((err) => {
        const status = document.querySelector("#status");
        if (status) status.textContent = t("footer.openLinkFailed", { err: String(err) });
      });
      return;
    }
    if (target.closest("[data-acct-test]")) {
      void runAccountKeyTest(family);
      return;
    }
    if (target.closest("[data-acct-add]")) void doAccountAdd(family);
  });
  overlay.addEventListener("input", (event) => {
    const target = event.target as HTMLInputElement;
    if (target.matches("[data-acct-key], [data-acct-baseurl], [data-acct-label]")) {
      resetAcctTestState(overlay.querySelector<HTMLElement>(".account-dialog-form"));
    }
  });
  dismissAccountDialog = done;
  document.addEventListener("keydown", onKey, true);
  document.body.appendChild(overlay);
  const form = overlay.querySelector<HTMLElement>(".account-dialog-form");
  resetAcctTestState(form);
  overlay.querySelector<HTMLInputElement>("[data-acct-key]")?.focus();
}

/// "Test" inside the Add-account dialog: the same probe the main key field
/// uses, but it unlocks the dialog's Save button instead of saving anything.
async function runAccountKeyTest(family: string): Promise<void> {
  const block = document.querySelector<HTMLElement>(
    `#account-overlay [data-account-dialog="${CSS.escape(family)}"]`,
  );
  const keyInp = block?.querySelector<HTMLInputElement>("[data-acct-key]");
  const labelInp = block?.querySelector<HTMLInputElement>("[data-acct-label]");
  const testBtn = block?.querySelector<HTMLButtonElement>("[data-acct-test]");
  const result = block?.querySelector<HTMLElement>("[data-acct-result]");
  const addBtn = block?.querySelector<HTMLButtonElement>("[data-acct-add]");
  if (!keyInp || !labelInp || !result) return;
  const generation = bumpTestGeneration("acct", family);
  const show = (text: string, ok: boolean | null) => {
    result.textContent = text;
    result.classList.toggle("ok", ok === true);
    result.classList.toggle("err", ok === false);
  };
  const key = keyInp.value.trim();
  const label = labelInp.value.trim();
  if (!key) {
    show(t("customize.testEmpty"), false);
    if (testBtn) testBtn.disabled = true;
    if (addBtn) addBtn.disabled = true;
    return;
  }
  if (!label) {
    show(t("customize.acctLabelRequired"), false);
    if (testBtn) testBtn.disabled = true;
    if (addBtn) addBtn.disabled = true;
    return;
  }
  if (testBtn) testBtn.disabled = true;
  if (addBtn) addBtn.disabled = true;
  show(t("customize.testing"), null);
  try {
    const baseUrl =
      block?.querySelector<HTMLInputElement>("[data-acct-baseurl]")?.value.trim() ?? "";
    const r = await invoke<{ ok: boolean; metrics: number; message: string }>("test_api_key", {
      provider: family,
      key,
      baseUrl: baseUrl || null,
    });
    if (!isCurrentTestGeneration("acct", family, generation)) return;
    if (testBtn) testBtn.disabled = false;
    if (r.ok) {
      show(t("customize.testOk", { n: r.metrics }), true);
      if (addBtn) addBtn.disabled = false;
    } else {
      show(`${t("customize.testFailed")}: ${r.message}`, false);
    }
  } catch (err) {
    if (!isCurrentTestGeneration("acct", family, generation)) return;
    if (testBtn) testBtn.disabled = false;
    show(`${t("customize.testFailed")}: ${String(err)}`, false);
  }
}

/// Remove saved layout/disabled entries for accounts that no longer exist.
/// The backend returns stable non-secret card ids, including disabled rows;
/// that lets this cleanup distinguish a deleted account from one temporarily
/// missing a live snapshot.
function reconcileAccountLayout(family: string, list: AccountEntry[]): boolean {
  if (!config.layout) return false;
  const active = new Set(list.map((entry) => entry.id).filter((id): id is string => Boolean(id)));
  const isFamilyAccount = (id: string): boolean =>
    id.includes("@") && providerFamily(id) === family;
  let changed = false;
  const order = config.layout.providerOrder.filter((id) => !isFamilyAccount(id) || active.has(id));
  if (order.length !== config.layout.providerOrder.length) {
    config.layout.providerOrder = order;
    changed = true;
  }
  for (const id of Object.keys(config.layout.providers)) {
    if (isFamilyAccount(id) && !active.has(id)) {
      delete config.layout.providers[id];
      changed = true;
    }
  }
  const disabled = config.disabled.filter((id) => !isFamilyAccount(id) || active.has(id));
  if (disabled.length !== config.disabled.length) {
    config.disabled = disabled;
    changed = true;
  }
  const prevSnapLen = lastSnapshots.length;
  lastSnapshots = lastSnapshots.filter((snap) => !isFamilyAccount(snap.id) || active.has(snap.id));
  if (lastSnapshots.length !== prevSnapLen) {
    changed = true;
  }
  if (changed) void patchConfig({ layout: config.layout, disabled: config.disabled }).catch(() => {});
  return changed;
}

/// Appends the tested account; the new stable <provider>@<fingerprint> card appears on the
/// follow-up refresh, like a saved main key does.
async function doAccountAdd(family: string): Promise<void> {
  const block = document.querySelector<HTMLElement>(
    `#account-overlay [data-account-dialog="${CSS.escape(family)}"]`,
  );
  const keyInp = block?.querySelector<HTMLInputElement>("[data-acct-key]");
  const labelInp = block?.querySelector<HTMLInputElement>("[data-acct-label]");
  if (!keyInp?.value.trim() || !labelInp?.value.trim()) return;
  const status = document.querySelector("#status")!;
  const result = block?.querySelector<HTMLElement>("[data-acct-result]");
  try {
    await invoke("account_add", {
      provider: family,
      label: labelInp.value.trim(),
      apiKey: keyInp.value.trim(),
      baseUrl:
        block?.querySelector<HTMLInputElement>("[data-acct-baseurl]")?.value.trim() || null,
    });
    // An added account says "show me this provider" — pull the family out
    // of Disabled exactly like pasting a key does, or first-run parking
    // keeps hiding the account card that was just created.
    recentlyKeyed.set(family, refreshGeneration);
    const enableGeneration = config.disabled.includes(family)
      ? markProviderEnablePending(family)
      : null;
    if (enableGeneration !== null) {
      await patchConfig({
        disabled: config.disabled.filter((id) => id !== family),
      }).catch(() => {});
    }
    refreshAccounts(family);
    status.textContent = t("customize.acctAdded", { name: providerDisplayName(family) });
    dismissAccountDialog?.();
    void forceUsageRefreshAttempt(false).then(() => {
      if (enableGeneration !== null) finishProviderEnable(family, enableGeneration);
      requestTraySync();
    });
  } catch (err) {
    status.textContent = t("customize.acctAddFailed", { err: String(err) });
    if (result) {
      result.textContent = `${t("customize.testFailed")}: ${String(err)}`;
      result.classList.remove("ok");
      result.classList.add("err");
    }
  }
}

/// Deletes an account after a confirm; its card vanishes on the refresh
/// this triggers (fetch_usage simply stops spawning it).
async function doAccountRemove(family: string, index: number): Promise<void> {
  const list = accountsCache.get(family) ?? [];
  const label = list[index]?.label || t("customize.acctDefaultName", { n: index + 1 });
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
    userSelectedAccountFor.delete(family);
    refreshAccounts(family);
    status.textContent = t("customize.acctRemoved");
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    status.textContent = t("customize.acctRemoveFailed", { err: String(err) });
  }
}

async function doAccountArchive(family: string, index: number): Promise<void> {
  const list = accountsCache.get(family) ?? [];
  const label = list[index]?.label || t("customize.acctDefaultName", { n: index + 1 });
  const ok = await appConfirm({
    title: t("customize.acctArchive"),
    message: t("customize.acctArchiveBody", { label }),
    confirmLabel: t("customize.acctArchive"),
    danger: false,
  });
  if (!ok) return;
  try {
    await invoke("account_archive", { provider: family, index });
    userSelectedAccountFor.delete(family);
    refreshAccounts(family);
    document.querySelector("#status")!.textContent = t("customize.acctArchived");
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    document.querySelector("#status")!.textContent = t("customize.acctAddFailed", { err: String(err) });
  }
}

/// Restores an archived account: the credential comes back from the archive
/// store, the card reappears with its original stable id (usage history and
/// layout reattach automatically).
async function doAccountRestore(family: string, cardId: string): Promise<void> {
  const status = document.querySelector("#status")!;
  try {
    await invoke("account_restore", { provider: family, cardId });
    userSelectedAccountFor.delete(family);
    refreshAccounts(family);
    status.textContent = t("customize.acctRestored");
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    status.textContent = t("customize.acctAddFailed", { err: String(err) });
  }
}

/// Makes the account at `index` the default: it moves to position 0 in the
/// accounts file and publishes under the bare family id on the next fetch.
/// Its old <provider>@<fingerprint> card folds away into the main card.
async function doAccountSetDefault(family: string, index: number): Promise<void> {
  const status = document.querySelector("#status")!;
  try {
    await invoke("account_set_default", { provider: family, index });
    userSelectedAccountFor.delete(family);
    refreshAccounts(family);
    status.textContent = t("customize.acctDefaultSet", { name: providerDisplayName(family) });
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    status.textContent = t("customize.acctDefaultFailed", { err: String(err) });
  }
}

/// Saves an edited note name. Labels are display-only, so no cache or
/// layout churn — but the account card's title carries the label, so a
/// non-default card needs the follow-up fetch to retitle.
async function doAccountRename(family: string, index: number, label: string): Promise<void> {
  const status = document.querySelector("#status")!;
  try {
    await invoke("account_rename", { provider: family, index, label });
    refreshAccounts(family);
    status.textContent = t("customize.acctRenamed");
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    status.textContent = t("customize.acctRenameFailed", { err: String(err) });
  }
}

/// Captures the Antigravity IDE's current Google login into a monitored
/// slot (label can be edited afterwards in the slot's ⚙ panel).
async function doAntigravityCapture(family: string): Promise<void> {
  const ok = await appConfirm({
    title: t("customize.agCapture"),
    message: t("customize.agCaptureConfirm"),
    confirmLabel: t("customize.agCapture"),
  });
  if (!ok) return;
  const status = document.querySelector("#status")!;
  try {
    await invoke("antigravity_capture_account", { label: "" });
    refreshAccounts(family);
    status.textContent = t("customize.agCaptured");
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    status.textContent = t("customize.agCaptureFailed", { err: String(err) });
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
/// providers get an API-key field (Custom Balance and Linkso also their base
/// URL), a "Test" button that validates the pasted key without saving it, and
/// Save — disabled until a test passes (an empty field stays savable:
/// that path clears the stored key).
///
/// Multi-account providers (deepseek/kimi/stepfun/siliconflow/novita/
/// relaybalance) are account-modeled: every key lives in the accounts
/// list, so the action section is just "Add account" + the account list +
/// a "Get API key" link — the standalone key field would be a second,
/// confusing save path for the same identity (phase 2, user report).
/// The rest sign in through their own CLI or desktop login, so their
/// action section is that provider's login hint. The "?" button on the row
/// stays: it shows the static facts, this panel the live detection.
/// The ⚙ panel's group-tag section: a select of existing groups plus a
/// "new group…" option that prompts for a name; existing groups can be
/// renamed/deleted inline. Rendered for every family card regardless of
/// credential kind — grouping is orthogonal to how the card connects.
function groupPickerHtml(id: string): string {
  const current = cardGroupId(id);
  const currentName = current === "" ? t("customize.groupNone") : (cardGroup(current)?.name ?? t("customize.groupNone"));
  const manage =
    current === ""
      ? ""
      : `<button class="mini-btn" data-group-rename="${escapeHtml(current)}" title="${escapeHtml(t("customize.groupRenameTip"))}">${escapeHtml(t("customize.groupRename"))}</button>
         <button class="mini-btn danger" data-group-delete="${escapeHtml(current)}" title="${escapeHtml(t("customize.groupDeleteTip"))}">${escapeHtml(t("customize.groupDelete"))}</button>`;
  return `<div class="form-field">
      <span class="form-label">${escapeHtml(t("customize.groupLabel"))}</span>
      <div class="form-actions">
        <button type="button" class="form-input cust-group-trigger" data-group-open="${escapeHtml(id)}" aria-haspopup="menu"><span>${escapeHtml(currentName)}</span><span class="cust-group-chev">⌄</span></button>
        ${manage}
      </div>
      <div class="form-help">${escapeHtml(t("customize.groupHelp"))}</div>
    </div>`;
}

/// Commit a group choice for one card: "__new__" prompts for a name first.
/// Shared by the card's own group menu and the ⚙ panel's group picker.
async function applyGroupChoice(cardId: string, choice: string): Promise<void> {
  if (choice === "__new__") {
    const name = await appPrompt({
      title: t("customize.groupNewPrompt"),
      placeholder: t("customize.groupNew"),
      confirmLabel: t("dialog.ok"),
    });
    if (!name) return;
    const gid = newGroupId();
    upsertCardGroup(gid, name);
    setCardGroup(cardId, gid);
    return;
  }
  setCardGroup(cardId, choice);
}

/// The ⚙ panel's group picker opens a self-drawn menu instead of a native
/// `<select>`: WebView2 paints the native popup from the host's
/// PreferredColorScheme, which ignores the page's dark theme and renders
/// light text on white — unreadable (user report 2026-10-08). The overlay
/// reuses the card group menu's styles, so it follows the theme everywhere.
function openGroupPicker(cardId: string, anchor: HTMLElement): void {
  document.querySelector(".group-menu-overlay")?.remove();
  const current = cardGroupId(cardId);
  const item = (gid: string, label: string, checked = false) =>
    `<button class="group-menu-item${checked ? " on" : ""}" data-group-pick="${escapeHtml(gid)}"><span class="group-menu-check">${checked ? "✓" : ""}</span>${escapeHtml(label)}</button>`;
  const overlay = document.createElement("div");
  overlay.className = "group-menu-overlay";
  overlay.innerHTML = `<div class="group-menu" role="menu">
      <div class="group-menu-title">${escapeHtml(t("customize.groupLabel"))}</div>
      ${item("", t("customize.groupNone"), current === "")}
      ${cardGroups()
        .map((g) => item(g.id, g.name, current === g.id))
        .join("")}
      <div class="group-menu-sep"></div>
      ${item("__new__", `${t("customize.groupNew")}…`)}
    </div>`;
  const close = () => overlay.remove();
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) {
      close();
      return;
    }
    const pick = (e.target as HTMLElement).closest<HTMLElement>("[data-group-pick]");
    if (!pick) return;
    const choice = pick.dataset.groupPick!;
    close();
    void applyGroupChoice(cardId, choice);
  });
  document.body.appendChild(overlay);
  const menu = overlay.querySelector<HTMLElement>(".group-menu")!;
  const rect = anchor.getBoundingClientRect();
  menu.style.left = `${Math.max(8, Math.min(rect.left, window.innerWidth - menu.offsetWidth - 8))}px`;
  menu.style.top = `${Math.max(8, Math.min(rect.bottom + 4, window.innerHeight - menu.offsetHeight - 8))}px`;
}

function renderCompactKeySummary(id: string): string {
  const status = credStatusCache.get(id);
  const label = cardNote(id) || providerDisplayName(providerFamily(id));
  const key = status?.maskedKey || (status?.storedKey ? "API key" : t("customize.noCredential"));
  return `<div class="cust-credential-bar">
      <div class="cust-credential-main">
        <strong>${escapeHtml(label)}</strong>
        <span>${escapeHtml(key)}</span>
      </div>
      <div class="cust-credential-meta">${credChipsHtml(id)}</div>
      <button class="mini-btn" data-cust-edit="${escapeHtml(id)}">${escapeHtml(t(status?.storedKey ? "customize.editCredential" : "customize.addCredential"))}</button>
    </div>`;
}

function custNoteField(id: string): string {
  return `<div class="form-field cust-note-field">
      <span class="form-label">${escapeHtml(t("customize.noteLabel"))}</span>
      <input class="form-input" type="text" data-cust-note="${escapeHtml(id)}" value="${escapeHtml(cardNote(id))}" placeholder="${escapeHtml(t("customize.notePh"))}" maxlength="48" />
      <div class="form-help">${escapeHtml(t("customize.noteHelp"))}</div>
    </div>`;
}

function custApplyHtml(id: string): string {
  return `<div class="form-actions cust-apply-actions">
      <button class="mini-btn primary" data-cust-apply="${escapeHtml(id)}">${escapeHtml(t("customize.saveRefresh"))}</button>
      <span class="cust-test-result" data-cust-apply-result="${escapeHtml(id)}"></span>
    </div>`;
}

/// Delete row for a provider's gear panel: full removal from Customize and
/// every dashboard surface, with the drawer footer as the way back.
/// Account rows (id !== family) keep their own menu-driven hide.
function custDangerZone(id: string): string {
  if (id !== providerFamily(id)) return "";
  return `<div class="cust-danger">
      <button class="mini-btn danger" data-remove-provider="${escapeHtml(id)}">${escapeHtml(t("customize.providerDelete"))}</button>
      <span class="form-help">${escapeHtml(t("customize.providerDeleteHint"))}</span>
    </div>`;
}

function renderCustConfig(id: string): string {
  const status = renderCustStatus(id);
  const fam = providerFamily(id);
  // The extra-account section only renders on the family's own row.
  const isFamilyRow = id === fam;
  // Multi-account family row: the account model owns every key. The old
  // paste-key path (gear input + Test + Save) is gone — add via dialog.
  // Checked BEFORE the KEY_PROVIDERS gate so non-key families like
  // Antigravity (captured OAuth slots) land here too.
  if (supportsExtraAccounts(id) && isFamilyRow) {
    // One/New API manages its sites in the family row's own account section
    // (onaFamilySection) — no generic add-account dialog here.
    if (id === ONA_FAMILY) {
      return `<div class="cust-config cust-form">
        <div class="form-field">
          <span class="form-label">${escapeHtml(t("customize.connLabel"))}</span>
          <div data-cred-chips="${escapeHtml(id)}">${credChipsHtml(id)}</div>
        </div>
        <p class="settings-note">${escapeHtml(t("customize.onaAccountsHint"))}</p>
        ${custNoteField(id)}
        ${custApplyHtml(id)}
        ${groupPickerHtml(id)}
      </div>`;
    }
    const getKey = getApiKeyLink(id);
    const linkLink = getKey
      ? `<button class="mini-btn cust-get-key" data-link="${escapeHtml(getKey)}">${escapeHtml(t("customize.getApiKey"))}</button>`
      : "";
    // Antigravity slots are captured OAuth snapshots; Cursor accounts are
    // imported via OAuth login / token / JSON — neither uses the pasted-key
    // dialog.
    let primary: string;
    if (id === "antigravity") {
      primary = `<button class="mini-btn cust-account-toggle" data-ag-capture="${id}">${escapeHtml(t("customize.agCapture"))}</button>`;
    } else if (id === "cursor") {
      primary = `<button class="mini-btn cust-account-toggle" data-cursor-account="${id}">${escapeHtml(t("customize.acctAdd"))}</button>`;
    } else {
      primary = `<button class="mini-btn cust-account-toggle" data-acct-toggle="${id}">${escapeHtml(t("customize.acctAdd"))}</button>`;
    }
    // The account list itself lives as child rows under the family row;
    // this panel is only the connection method + the add/get-key actions.
    const customHint =
      id === "relaybalance"
        ? `<p class="settings-note">${escapeHtml(t("customize.customRelayHint"))}</p>`
        : "";
    return `<div class="cust-config cust-form">
        <div class="form-field">
          <span class="form-label">${escapeHtml(t("customize.connLabel"))}</span>
          <div data-cred-chips="${escapeHtml(id)}">${credChipsHtml(id)}</div>
        </div>
        ${customHint}
        <div class="form-actions">
          ${primary}
          ${linkLink}
        </div>
        ${custNoteField(id)}
        ${custApplyHtml(id)}
        ${groupPickerHtml(id)}
      </div>`;
  }
  if (id === "stepfun-plan") {
    return `<div class="cust-config cust-form">
      ${status}
      <div class="form-actions">
        <button class="mini-btn" data-stepfun-login="https://platform.stepfun.com/account-overview">${escapeHtml(t("customize.stepfunPlanLogin"))}</button>
      </div>
      <p class="settings-note">${escapeHtml(t("customize.stepfunPlanLoginHelp"))}</p>
      <div class="form-field">
        <span class="form-label">${escapeHtml(t("customize.stepfunPlanUserLabel"))}</span>
        <input class="form-input" type="text" data-cust-user="${id}" placeholder="${escapeHtml(t("customize.stepfunPlanUserPh"))}" autocomplete="username" spellcheck="false" />
      </div>
      <div class="form-field">
        <span class="form-label">${escapeHtml(t("customize.stepfunPlanPassLabel"))}</span>
        <input class="form-input" type="password" data-cust-pass="${id}" placeholder="${escapeHtml(t("customize.stepfunPlanPassPh"))}" autocomplete="new-password" spellcheck="false" />
      </div>
      <div class="form-field">
        <span class="form-label">${escapeHtml(t("customize.stepfunPlanTokenLabel"))}</span>
        <input class="form-input" type="password" data-cust-key="${id}" placeholder="${escapeHtml(t("customize.stepfunPlanTokenPh"))}" autocomplete="new-password" spellcheck="false" />
        <div class="form-help">${escapeHtml(t("customize.formKeyHelp"))}</div>
      </div>
      <div class="form-actions">
        <button class="mini-btn" data-cust-test="${id}">${escapeHtml(t("customize.test"))}</button>
        <button class="mini-btn" data-cust-save="${id}" title="${escapeHtml(t("customize.saveAfterTest"))}">${escapeHtml(t("settings.save"))}</button>
        <span class="cust-test-result" data-cust-result="${id}"></span>
      </div>
      ${custNoteField(id)}
      ${custApplyHtml(id)}
      ${groupPickerHtml(id)}
    </div>`;
  }
  if (id === "sensenova") {
    // One-time browser sign-in: the link is generated backend-side (PKCE),
    // then whatever the browser yields goes back through oauth_finish —
    // no long-lived token ever sits in an input.
    return `<div class="cust-config cust-form">
      ${status}
      <div class="form-actions">
        <button class="mini-btn" data-sensenova-oauth-start="1">${escapeHtml(t("customize.sensenovaStart"))}</button>
      </div>
      <p class="settings-note">${escapeHtml(t("customize.sensenovaStartHelp"))}</p>
      <div class="form-field">
        <span class="form-label">${escapeHtml(t("customize.sensenovaPasteLabel"))}</span>
        <input class="form-input" type="password" data-sensenova-code="${id}" placeholder="${escapeHtml(t("customize.sensenovaPastePh"))}" autocomplete="off" spellcheck="false" />
        <div class="form-help">${escapeHtml(t("customize.sensenovaPasteHelp"))}</div>
      </div>
      <div class="form-actions">
        <button class="mini-btn" data-sensenova-oauth-finish="${id}">${escapeHtml(t("customize.sensenovaFinish"))}</button>
        <span class="cust-test-result" data-sensenova-result="${id}"></span>
      </div>
      ${custNoteField(id)}
      ${custApplyHtml(id)}
      ${groupPickerHtml(id)}
    </div>`;
  }
  if (!KEY_PROVIDERS.has(id)) {
    // An account card (deepseek@<fingerprint>) gets its own small config
    // panel: the masked key, the live snapshot status (connectivity as of
    // the last fetch), and delete.
    if (id !== fam && supportsExtraAccounts(fam)) {
      return renderAccountConfig(id);
    }
    const hintKey = `customize.loginHint.${providerFamily(id)}`;
    const hint = t(hintKey) !== hintKey ? t(hintKey) : t("customize.cliLoginHint");
    return `<div class="cust-config">${status}${renderOAuthBlock(id)}<p class="settings-note">${escapeHtml(hint)}</p>${custNoteField(id)}${custApplyHtml(id)}${groupPickerHtml(id)}</div>`;
  }
  // Single-key providers: one stacked API-key form (DSH/cockpit style).
  const phKey = `settings.keyPh${id[0].toUpperCase()}${id.slice(1)}`;
  const ph = t(phKey) !== phKey ? t(phKey) : t("settings.keyPlaceholder");
  const baseUrlField =
    RELAY_BASE_URL_FAMILIES.has(id)
      ? `<div class="form-field">
          <span class="form-label">${escapeHtml(t("settings.relayBaseUrl"))}</span>
          <input class="form-input" type="text" data-cust-baseurl="${id}" placeholder="https://api.example.com" spellcheck="false" />
          <div class="form-help">${escapeHtml(t("customize.relayBaseUrlHelp"))}</div>
        </div>`
      : "";
  if (!custKeyEditing.has(id)) {
    return `<div class="cust-config cust-form">
      ${renderCompactKeySummary(id)}
      ${custNoteField(id)}
      ${custApplyHtml(id)}
      ${groupPickerHtml(id)}
    </div>`;
  }
  return `<div class="cust-config cust-form">
      ${status}
      ${custNoteField(id)}
      <div class="form-field">
        <span class="form-label">${escapeHtml(t("customize.acctKeyLabel"))}</span>
        <input class="form-input" type="password" data-cust-key="${id}" placeholder="${escapeHtml(ph)}" autocomplete="new-password" spellcheck="false" />
        <div class="form-help">${escapeHtml(t("customize.formKeyHelp"))}</div>
      </div>
      ${baseUrlField}
      <div class="form-actions">
        <button class="mini-btn" data-cust-test="${id}">${escapeHtml(t("customize.test"))}</button>
        <button class="mini-btn" data-cust-save="${id}" title="${escapeHtml(t("customize.saveAfterTest"))}">${escapeHtml(t("settings.save"))}</button>
        <span class="cust-test-result" data-cust-result="${id}"></span>
      </div>
      ${custApplyHtml(id)}
      ${groupPickerHtml(id)}
    </div>`;
}

/// The ⚙ panel for one account row: an editable note name (saved through
/// account_rename), the masked key, the account's live connectivity as of
/// the last fetch, and delete. The key itself never leaves the backend.
function renderAccountConfig(id: string): string {
  const fam = providerFamily(id);
  const list = accountsCache.get(fam) ?? [];
  const index = list.findIndex((a) => a.id === id);
  if (index < 0) {
    return `<div class="cust-config"><p class="dim">${escapeHtml(t("customize.credStatusLoading"))}</p></div>`;
  }
  const entry = list[index];
  // The default account publishes under the bare family id, so that's
  // where its live snapshot lives. Antigravity/Cursor accounts are
  // independent cards (the family card is the logged-in account), so
  // their own id IS the card.
  const snapId = isParallelAccountFamily(fam) ? id : index === 0 ? fam : id;
  const snap = lastSnapshots.find((s) => s.id === snapId);
  const statusText = snap
    ? snap.status === "ok"
      ? escapeHtml(t("customize.connOk"))
      : snap.status === "no_credentials"
        ? escapeHtml(t("customize.connNoCred"))
        : escapeHtml(t("customize.connError", { err: snap.error ?? "" }))
    : escapeHtml(t("customize.connUnknown"));
  const stale = snap?.stale
    ? `<span class="stale" title="${escapeHtml(staleHelp(snap))}">${escapeHtml(t("card.outdated"))}</span>`
    : "";
  return `<div class="cust-config cust-form account-config">
      <div class="form-field">
        <span class="form-label">${escapeHtml(t("customize.acctNoteLabel"))}</span>
        <input class="form-input" type="text" data-acct-label-edit="${escapeHtml(fam)}|${index}" value="${escapeHtml(entry.label)}" autocomplete="off" spellcheck="false" />
      </div>
      <div class="form-field">
        <span class="form-label">${escapeHtml(t("customize.acctKeyLabel"))}</span>
        <div class="form-help">${escapeHtml(entry.maskedKey)} · ${escapeHtml(t("customize.acctKeyLocalOnly"))}</div>
      </div>
      <div class="form-field">
        <span class="form-label">${escapeHtml(t("customize.connLabel"))}</span>
        <div class="form-help">${statusText}${stale}</div>
      </div>
      <div class="form-actions">
        <button class="mini-btn" data-acct-rename="${escapeHtml(fam)}|${index}">${escapeHtml(t("settings.save"))}</button>
        <button class="mini-btn" data-acct-archive="${escapeHtml(fam)}|${index}" title="${escapeHtml(t("customize.acctArchive"))}">${escapeHtml(t("customize.acctArchive"))}</button>
        <button class="mini-btn danger" data-acct-del="${escapeHtml(fam)}|${index}" title="${escapeHtml(t("customize.acctDelete"))}">${escapeHtml(t("customize.acctDelete"))}</button>
      </div>
    </div>`;
}

/// The account child rows hanging under a multi-account family row in
/// Customize: [label] [★ default star] [spacer] [?] [⚙]. The ? and ⚙ act
/// like the family row's but scoped to that account (cred status + the
/// account's own API-key panel). The default account (index 0) shows a
/// filled star; the others a hollow one that makes it default on click.
function accountChildRows(family: string): string {
  const list = accountsCache.get(family);
  if (!list) {
    fetchAccounts(family);
    return `<p class="dim cust-account-loading">${escapeHtml(t("customize.credStatusLoading"))}</p>`;
  }
  if (!list.length) return "";
  // Pin (置顶) controls exist only with 2+ accounts — a single account has
  // nothing to order against. Applies to EVERY multi-account family.
  const showPin = list.length >= 2;
  return `<div class="cust-account-children" data-accounts-children="${escapeHtml(family)}">${list
    .map((a, i) => {
      const acctId = a.id ?? "";
      const label = labelForAccount(acctId, list);
      // Antigravity slots and Cursor imported accounts are parallel
      // accounts — no default/star concept (the bare family card is always
      // the locally logged-in account).
      const star =
        !showPin || isParallelAccountFamily(family)
          ? ""
          : i === 0
            ? `<button class="star on acct-child-star" data-acct-setdef="${family}|${i}" title="${escapeHtml(t("customize.acctDefault"))}">${uiIcon("star")}</button>`
            : `<button class="star acct-child-star" data-acct-setdef="${family}|${i}" title="${escapeHtml(t("customize.acctMakeDefault"))}">${uiIcon("star")}</button>`;
      return `<div class="cust-account-child" data-acct-child="${escapeHtml(family)}|${i}">
        <span class="acct-child-label">${escapeHtml(label)}</span>
        <span class="dim acct-child-key">${escapeHtml(a.maskedKey)}</span>
        <span class="spacer"></span>
        ${star}
        <button class="mini-btn" data-info="${escapeHtml(acctId) || escapeHtml(family)}" title="${escapeHtml(t("customize.credInfo"))}">${uiIcon("question")}</button>
        <button class="mini-btn" data-config="${escapeHtml(acctId)}" title="${escapeHtml(t("customize.configure"))}">${uiIcon("gear")}</button>
        ${custInfoOpen === acctId ? renderCustInfo(acctId) : ""}
        ${custConfigOpen === acctId ? renderAccountConfig(acctId) : ""}
      </div>`;
    })
    .join("")}</div>`;
}

function renderSkinMarket(): string {
  const selected = activeSkin()?.id;
  const preview = skinPreviewId ? SKINS.find((skin) => skin.id === skinPreviewId) : undefined;
  if (preview) {
    return `<section class="skin-market skin-detail" aria-label="Skin preview">
      <button class="skin-back" type="button" data-skin-back>← Skin market</button>
      <div class="skin-detail-hero" style="--skin-preview:${preview.wallpaper}">
        <div class="skin-detail-mascot">${preview.mascot}</div>
        <div><p class="skin-kicker">SELECTED SKIN</p><h2>${escapeHtml(preview.name)}</h2><p>${escapeHtml(preview.tagline)}</p></div>
      </div>
      <div class="skin-detail-actions"><button class="mini-btn primary" data-skin-select="${preview.id}">${selected === preview.id ? "✓ Applied" : "Use this skin"}</button><button class="mini-btn" data-skin-reset>Use native</button><span class="skin-detail-note">Wallpaper + mascot appear together when Pane wakes.</span></div>
    </section>`;
  }
  return `<section class="skin-market" aria-label="Skin market">
    <div class="skin-market-head"><div><p class="skin-kicker">PANE SKIN MARKET</p><h2>Make the popover yours</h2><p class="skin-market-copy">Choose a wallpaper and its companion mascot. Your native light/dark theme stays intact.</p></div><button class="mini-btn" data-skin-close>Done</button></div>
    <div class="skin-market-actions"><button class="mini-btn" data-skin-reset>Use native / reset</button><span class="skin-detail-note">Native wallpaper and theme remain unchanged.</span></div>
    <div class="skin-grid">${SKINS.map((skin) => `<button class="skin-card${selected === skin.id ? " selected" : ""}" type="button" data-skin-preview="${skin.id}" style="--skin-preview:${skin.wallpaper}"><span class="skin-card-art"><span class="skin-card-mascot">${skin.mascot}</span></span><span class="skin-card-copy"><strong>${escapeHtml(skin.name)}</strong><small>${escapeHtml(skin.tagline)}</small></span>${selected === skin.id ? '<span class="skin-selected">Applied</span>' : ""}</button>`).join("")}</div>
  </section>`;
}

/// Footer registry of providers deleted from Customize — the one-line way
/// back that keeps a mis-click from being permanent.
function removedProvidersHtml(): string {
  const removed = config.removedProviders;
  if (!removed.length) return "";
  const rows = removed
    .map((fam) => {
      const name = ALL_PROVIDERS.find(([pid]) => pid === fam)?.[1] ?? fam;
      return `<div class="cust-removed-row">
          <span class="cust-label">${escapeHtml(name)}</span>
          <button class="mini-btn" data-restore-provider="${escapeHtml(fam)}">${escapeHtml(t("customize.providerRestore"))}</button>
        </div>`;
    })
    .join("");
  return `<div class="cust-removed${custRemovedOpen ? " open" : ""}">
      <button class="cust-removed-head" data-removed-toggle>
        <span>${escapeHtml(t("customize.removedHead", { n: removed.length }))}</span>
        <span class="chev">${uiIcon("caretDown")}</span>
      </button>
      ${custRemovedOpen ? `<div class="cust-removed-list">${rows}</div>` : ""}
    </div>`;
}

function renderCustomize(): string {
  if (skinMarketOpen) return renderSkinMarket();
  // A-Z by English display name, locale-independent. Card order is owned by
  // dragging cards on the main view; the drawer is for enabling,
  // configuring and per-row management, so a stable sorted list reads best.
  const nameOf = (id: string): string =>
    ALL_PROVIDERS.find(([pid]) => pid === id)?.[1] ??
    lastSnapshots.find((s) => s.id === id)?.name ??
    id;
  const ids = [...(config.layout?.providerOrder ?? ALL_PROVIDERS.map(([id]) => id))]
    .filter((id) => {
      const snapshot = lastSnapshots.find((s) => s.id === id);
      // Multi-account API-key families render their account rows as
      // children of the family row in Customize, so the account cards
      // themselves (kimi@<fp>) must not appear as independent rows.
      const fam = providerFamily(id);
      if (id !== fam && supportsExtraAccounts(fam)) {
        // Antigravity slot cards (antigravity@<fp>) also hang under the
        // family row as child rows — same treatment.
        return false;
      }
      // A family deleted from the catalog must not haunt the drawer even when
      // a saved layout still lists it (old providerOrder / disabled entries):
      // no snapshot AND no catalog entry means the provider no longer exists.
      // A merely disabled family has no snapshot either — famKnown keeps its
      // re-enable toggle alive.
      const famKnown =
        ALL_PROVIDERS.some(([pid]) => pid === fam) || fam === ONA_FAMILY || supportsExtraAccounts(fam);
      if (!snapshot && !famKnown) return false;
      // Deleted providers leave the drawer entirely — a re-enable toggle
      // for them would defeat the point; restore lives in the footer.
      if (config.removedProviders.includes(providerFamily(id))) {
        return false;
      }
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
            ${starrable ? `<button class="star${starred ? " on" : ""}" data-star="${id}|${escapeHtml(key)}" title="${escapeHtml(t("customize.star"))}">${uiIcon("star")}</button>` : ""}
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
      const accountRows =
        id === providerFamily(id) && supportsExtraAccounts(id)
          ? id === ONA_FAMILY
            ? onaFamilySection()
            : accountChildRows(id)
          : "";
      // Same brand mark the dashboard/trail use (One/New API site cards
      // resolve their per-host icon through the snapshot origin);
      // families without an SVG get a "?" placeholder — the user supplies
      // real icons as SVG files later.
      const visual = providerVisual(id, snapshot?.dashboard_url ?? undefined);
      const icon = visual?.iconSvg ?? `<span class="icon-fallback">?</span>`;
      return {
        id,
        html: `
        <article class="provider customize-block${enabled ? "" : " muted"}${open ? " open" : ""}" data-cust-provider="${id}" data-letter="${letter}" data-name="${escapeHtml(name.toLowerCase())}">
          <div class="provider-head">
            <button class="cust-expand" data-cust-expand="${id}" title="${open ? t("customize.collapse") : t("customize.expand")}">
              <span class="cust-head-icon" aria-hidden="true">${icon}</span>
              <span class="provider-name">${escapeHtml(name)}</span>
              <span class="chev">${uiIcon("caretDown")}</span>
            </button>
            <span class="spacer"></span>
            <button class="mini-btn cust-info-btn${custInfoOpen === id ? " on" : ""}" data-info="${id}" title="${escapeHtml(t("customize.credInfo"))}">${uiIcon("question")}</button>
            <button class="mini-btn cust-config-btn${custConfigOpen === id ? " on" : ""}" data-config="${id}" title="${escapeHtml(t("customize.configure"))}">${uiIcon("gear")}</button>
            <button class="mini-btn icon-btn reset-layout-btn" data-reset="${id}" title="${escapeHtml(t("customize.resetLayoutTip"))}" aria-label="${escapeHtml(t("customize.resetLayoutTip"))}">${uiIcon("arrowsClockwise")}</button>
            <label class="toggle mini" title="${escapeHtml(t("customize.enable"))}"><input type="checkbox" data-enable="${id}"${enabled ? " checked" : ""} /></label>
          </div>
          ${accountRows}
          ${custConfigOpen === id ? renderCustConfig(id) + custDangerZone(id) : ""}
          ${custInfoOpen === id ? renderCustInfo(id) : ""}
          <div class="acc-body"><div class="acc-inner cust-rows">${rows}</div></div>
        </article>`,
      };
    });

  // Strict A–Z across the whole drawer (user requirement 2026-10-08): the
  // dashboard's card groups (custom buckets like 鸡蛋/羊毛) order the MAIN
  // view only — in the drawer they are ignored entirely, so enabling and
  // configuring always happens in one predictable alphabetical list.
  const blocksHtml = blocks.map((b) => b.html).join("");

  const starCount = Object.values(config.layout?.providers ?? {}).reduce((n, l) => n + l.starred.length, 0);
  // The skin market rides the experimental-features gate: entry hidden
  // entirely unless the user opted in from Settings.
  const skinEntry = config.experimentalFeatures === true
    ? `<div class="customize-skin-entry"><div><strong>Skin market</strong><span>Wallpaper · mascot · atmosphere</span></div><button class="mini-btn" data-skin-open>${uiIcon("palette", "Open skin market")}</button></div>`
    : "";
  return `
    ${skinEntry}
    <div class="customize-bar glass-bar">
      <div class="customize-heading">
        <button class="dock-btn" data-customize-close>${escapeHtml(t("customize.done"))}</button>
        <div class="customize-heading-copy">
          <strong>${escapeHtml(t("customize.title"))}</strong>
        </div>
      </div>
      <span class="detail customize-summary">${escapeHtml(t("customize.starred", { n: starCount }))}</span>
      <button class="dock-btn danger" data-reset-all title="${escapeHtml(t("customize.resetAllTip"))}">${escapeHtml(t("customize.resetAll"))}</button>
    </div>
    <nav class="cust-az">${letters
      .map((l) => `<button data-az="${l}">${l}</button>`)
      .join("")}</nav>
    <div class="cust-search-wrap">
      <span class="cust-search-icon">${uiIcon("magnifyingGlass")}</span>
      <input id="cust-search" class="cust-search" type="search" autocomplete="off"
        placeholder="${escapeHtml(t("customize.searchPlaceholder"))}" />
    </div>
    ${blocksHtml}
    ${removedProvidersHtml()}`;
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
        <button class="share-btn welcome-close" data-welcome-close title="${escapeHtml(t("welcome.dismiss"))}">${uiIcon("x")}</button>
      </div>
      <p class="placeholder" style="margin:2px 0 8px">
        ${escapeHtml(t("welcome.body"))}
      </p>
      <button class="mini-btn" data-welcome-customize>${escapeHtml(t("welcome.open"))}</button>
    </article>`;
}

/// Cards laid out by top-level category first (Coding Agent / productivity),
/// then the group layout inside each category: ungrouped cards flat, then
/// one section per used group in the groups' own order. Categories without
/// cards render nothing.
function renderGroupedCards(): string {
  const snaps = orderedSnapshots();
  if (snaps.length === 0) return "";
  const groups = usedCardGroups();
  const head = (category: OverviewCategory) =>
    `<div class="category-head"><span class="category-name">${escapeHtml(
      t(`category.${category}`),
    )}</span><span class="category-count">${
      snaps.filter((s) => effectiveCategory(providerFamily(s.id)) === category).length
    }</span></div>`;
  return OVERVIEW_CATEGORIES.map((category) => {
    const members = snaps.filter((s) => effectiveCategory(providerFamily(s.id)) === category);
    return members.length ? head(category) + renderCategoryCards(members, groups) : "";
  }).join("");
}

/// The pre-category wall layout: ungrouped cards first (flat), then one
/// collapsible section per used group. Within a section the normal
/// providerOrder sort applies. Sections are folded by the group's own
/// collapsed flag.
function renderCategoryCards(snaps: Snapshot[], groups: CardGroup[]): string {
  if (groups.length === 0) return snaps.map(renderCard).join("");
  const usedIds = new Set(groups.map((g) => g.id));
  const byGroup = new Map<string, Snapshot[]>();
  const flat: Snapshot[] = [];
  for (const s of snaps) {
    const gid = cardGroupId(s.id);
    // Live-group members bucket by id; ungrouped cards and stale tags
    // (group deleted but the card still points at it) render flat.
    if (!gid || !usedIds.has(gid)) {
      flat.push(s);
      continue;
    }
    const list = byGroup.get(gid) ?? [];
    list.push(s);
    byGroup.set(gid, list);
  }
  const sections: string[] = flat.map(renderCard);
  for (const g of groups) {
    const members = byGroup.get(g.id) ?? [];
    if (members.length === 0) continue;
    const chevron = g.collapsed ? "›" : "⌄";
    sections.push(
      `<div class="card-group-head" data-group-toggle="${escapeHtml(g.id)}" role="button" tabindex="0">
        <span class="card-group-chevron">${chevron}</span>
        <span class="card-group-name">${escapeHtml(g.name)}</span>
        <span class="card-group-count">${members.length}</span>
        <button class="card-group-remove" data-group-remove="${escapeHtml(g.id)}" title="${escapeHtml(t("customize.groupDeleteTip"))}" aria-label="${escapeHtml(t("customize.groupDelete"))}">${uiIcon("x")}</button>
      </div>`,
    );
    for (const s of members) {
      sections.push(
        g.collapsed ? `<div class="card-group-fold" hidden>${renderCard(s)}</div>` : renderCard(s),
      );
    }
  }
  return sections.join("");
}

function renderAll(): void {
  hideSpendPop();
  document.querySelector(".spend-detail-overlay")?.remove();
  const el = document.querySelector("#providers")!;
  el.innerHTML =
    renderWelcome() +
    renderTotalSpend() +
    renderQuotaOverview() +
    renderGroupedCards();
  if (customizeOpen) renderDrawerBody();
  if (spendDetailOpen) document.body.insertAdjacentHTML("beforeend", renderSpendDetailOverlay());
  rebuildTrail();
  requestAnimationFrame(fitProviderNames);
}

/// Jumps from a quota-overview item to its real provider card. Grouped cards
/// may live inside a hidden `.card-group-fold`; reveal that group first, then
/// resolve the card again because renderAll() rebuilds the provider DOM.
function jumpToProviderCard(providerId: string): void {
  const selector = `article.provider[data-provider="${CSS.escape(providerId)}"]`;
  const card = document.querySelector<HTMLElement>(selector);
  if (!card) return;

  const hiddenGroup = card.closest<HTMLElement>(".card-group-fold[hidden]");
  if (hiddenGroup) {
    const group = cardGroup(cardGroupId(providerId));
    if (group) {
      group.collapsed = false;
      void patchConfig({ layout: config.layout });
      renderAll();
      requestAnimationFrame(() => jumpToProviderCard(providerId));
      return;
    }
  }

  scrollOverviewCard(card);
  card.classList.add("card-highlight");
  setTimeout(() => card.classList.remove("card-highlight"), 1200);
}

function renderDrawerBody(): void {
  const body = document.querySelector<HTMLElement>("#drawer-body");
  if (!body) return;
  body.innerHTML = renderCustomize();
  if (custSearchQuery) {
    const inp = body.querySelector<HTMLInputElement>("#cust-search");
    if (inp) inp.value = custSearchQuery;
    applyCustSearch(custSearchQuery);
  }
}

/// Fuzzy-ish filter for the Customize drawer: case-insensitive substring on
/// the display name or family id, so prefixes, suffixes and middles all hit.
/// Rows hide in place (no re-render) to keep the input's focus and caret.
let custSearchQuery = "";
function applyCustSearch(raw: string): void {
  custSearchQuery = raw;
  const query = raw.trim().toLowerCase();
  let visible = 0;
  for (const blk of document.querySelectorAll<HTMLElement>(".customize-block")) {
    const hit =
      !query ||
      (blk.dataset.name ?? "").includes(query) ||
      (blk.dataset.custProvider ?? "").toLowerCase().includes(query);
    blk.style.display = hit ? "" : "none";
    if (hit) visible++;
  }
  const body = document.querySelector<HTMLElement>("#drawer-body");
  document.querySelector(".cust-search-empty")?.remove();
  if (!visible && body) {
    const empty = document.createElement("p");
    empty.className = "placeholder cust-search-empty";
    empty.textContent = t("customize.searchEmpty");
    body.appendChild(empty);
  }
}

// ---------------------------------------------------------------------------
// Render root
// ---------------------------------------------------------------------------

function setDrawer(open: boolean): void {
  customizeOpen = open;
  if (!open) {
    dismissAccountDialog?.();
    skinMarketOpen = false;
    skinPreviewId = null;
  }
  if (open) {
    renderDrawerBody();
    // Local JSON list — cheap, and required if Customize opens before Settings.
    void loadOneNewApiSites();
  }
  document.body.classList.toggle("drawer-open", open);
  document.querySelector("#customize-btn")?.classList.toggle("active", open);
}

// ---------------------------------------------------------------------------
// Navigation trail: a slim rail of ticks — one per provider mark — that shows
// where you are in the scroll and jumps to a card on click. Cards sharing the
// same mark (parallel accounts, extra relay keys) collapse into one tick.
// ---------------------------------------------------------------------------

function trailCards(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>("#providers > article"));
}

/// One trail tick per visual identity; `indices` lists every card the tick
/// stands for (DOM order) and clicking cycles through them.
type TrailEntry = { key: string; indices: number[]; names: string[]; cursor: number };
let trailEntries: TrailEntry[] = [];
// Click-cycle position per visual key, kept across rebuilds so a background
// refresh doesn't snap a merged tick back to its first card.
const trailCursorMemory = new Map<string, number>();

const DOT_SEVERITY: Record<string, number> = { gray: 0, green: 1, yellow: 2, red: 3, error: 4 };

function rebuildTrail(): void {
  const trail = document.querySelector<HTMLElement>("#trail")!;
  const cards = trailCards();
  trailEntries = [];
  if (!cards.length) {
    trail.innerHTML = "";
    trail.hidden = true;
    return;
  }
  trail.hidden = false;
  const byKey = new Map<string, TrailEntry>();
  cards.forEach((card, i) => {
    const id = card.dataset.provider ?? "";
    const family = id ? providerFamily(id) : "";
    const visual = id === "__overview__" ? undefined : providerVisual(id || family, card.dataset.origin || undefined);
    const key =
      id === "__overview__"
        ? "__overview__"
        : visual
          ? `icon:${visual.iconKey}`
          : `family:${family || `#${i}`}`;
    let entry = byKey.get(key);
    if (!entry) {
      entry = { key, indices: [], names: [], cursor: trailCursorMemory.get(key) ?? 0 };
      byKey.set(key, entry);
      trailEntries.push(entry);
    }
    entry.indices.push(i);
    entry.names.push(card.querySelector(".provider-name")?.textContent ?? `Card ${i + 1}`);
  });
  trail.innerHTML = trailEntries
    .map((entry, j) => {
      const card = cards[entry.indices[0]];
      const id = card.dataset.provider ?? "";
      const family = id ? providerFamily(id) : "";
      const title = escapeHtml(
        entry.names.length > 1 ? `${entry.names[0]} (+${entry.names.length - 1})` : entry.names[0],
      );
      if (entry.key === "__overview__") {
        return `<button class="trail-tick trail-icon" data-trail="${j}" title="${title}"><span class="trail-icon-inner"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><circle cx="12" cy="12" r="9"/><polyline points="12 7 12 12 15 15"/></svg></span></button>`;
      }
      const visual = providerVisual(id || family, card.dataset.origin || undefined);
      const icon = visual?.iconSvg;
      let dot = "";
      if (icon) {
        // A merged tick shows the worst health across the cards it stands for.
        dot = entry.indices
          .map((idx) => {
            const cid = cards[idx].dataset.provider ?? "";
            const fam = cid ? providerFamily(cid) : "";
            return isParallelAccountFamily(fam)
              ? peakTintedDot(fam, accountHealthDot(cid))
              : (fam ? peakTintedDot(fam, familyHealthDot(fam)) : "");
          })
          .reduce(
            (worst, d) => ((DOT_SEVERITY[d] ?? -1) > (DOT_SEVERITY[worst] ?? -1) ? d : worst),
            "",
          );
      }
      const dotHtml = dot ? `<span class="trail-badge ${dot}"></span>` : "";
      if (icon) {
        const extra = [
          visual?.recolorOnTray ? " trail-recolor" : "",
          visual?.invertOnDarkTray ? " trail-invert-dark" : "",
        ].join("");
        return `<button class="trail-tick trail-icon${extra}" data-trail="${j}" title="${title}"><span class="trail-icon-inner">${icon}</span>${dotHtml}</button>`;
      }
      return `<button class="trail-tick" data-trail="${j}" title="${title}"></button>`;
    })
    .join("");
  // Minimap feel: tick width follows the card's height, like Codex's rail.
  // Icon ticks keep a fixed square box instead — the mark itself is the
  // height signal.
  const ticks = trail.querySelectorAll<HTMLElement>(".trail-tick");
  trailEntries.forEach((entry, j) => {
    const tick = ticks[j];
    if (!tick || tick.classList.contains("trail-icon")) return;
    const h = Math.max(...entry.indices.map((idx) => cards[idx]?.offsetHeight ?? 80));
    tick.style.width = `${Math.max(7, Math.min(16, Math.round(5 + h / 45)))}px`;
  });
  updateTrailActive();
  updateTrailLayout();
}

function updateTrailLayout(): void {
  const trail = document.querySelector<HTMLElement>("#trail");
  const wrap = document.querySelector<HTMLElement>("#trail-wrap");
  if (!trail || !wrap) return;

  const count = trail.querySelectorAll<HTMLElement>(".trail-tick").length;
  if (!count) return;

  // Measure available vertical space in #trail-wrap
  const availH = wrap.clientHeight || 360;

  const MAX_SIZE = 22;
  const MIN_SIZE = 12;
  const MIN_GAP = 2;
  const MAX_GAP = 14;
  // Fill the rail with evenly scaled spacing: the icon size shrinks
  // proportionally as the count grows, then the leftover vertical slack is
  // handed to the gaps — bounded, so a sparse rail never spreads edge to
  // edge. What the cap cannot absorb stays as a small even margin around
  // the centered stack. Scrolling remains the overflow valve.
  const FILL = 0.94;

  const effectiveUnits = count + (count - 1) * 0.22;
  const fitSize = Math.floor((availH * FILL) / effectiveUnits);
  const size = Math.max(MIN_SIZE, Math.min(MAX_SIZE, fitSize));
  const gapFit = Math.max(MIN_GAP, Math.min(6, Math.round(size * 0.22)));
  const slack = availH * FILL - count * size;
  const gap = Math.max(
    gapFit,
    Math.min(MAX_GAP, Math.floor(slack / Math.max(1, count - 1))),
  );

  const innerSize = Math.max(9, Math.min(14, Math.round(size * 0.64)));
  const dotSize = size <= 16 ? 3 : 4;

  trail.style.setProperty("--trail-icon-size", `${size}px`);
  trail.style.setProperty("--trail-inner-size", `${innerSize}px`);
  trail.style.setProperty("--trail-gap", `${gap}px`);
  trail.style.setProperty("--trail-dot-size", `${dotSize}px`);

  updateTrailScrollIndicators();
}

function updateTrailScrollIndicators(): void {
  const trail = document.querySelector<HTMLElement>("#trail");
  const moreTop = document.querySelector<HTMLElement>("#trail-more-top");
  const moreBottom = document.querySelector<HTMLElement>("#trail-more-bottom");
  if (!trail || !moreTop || !moreBottom) return;

  const isOverflowing = trail.scrollHeight > trail.clientHeight + 2;
  if (!isOverflowing) {
    moreTop.hidden = true;
    moreBottom.hidden = true;
    trail.classList.remove("is-overflowing");
    return;
  }

  trail.classList.add("is-overflowing");
  moreTop.hidden = trail.scrollTop <= 4;
  moreBottom.hidden = trail.scrollTop + trail.clientHeight >= trail.scrollHeight - 4;
}

function setupTrailScroll(): void {
  const trail = document.querySelector<HTMLElement>("#trail");
  const sidebar = document.querySelector<HTMLElement>(".sidebar-right");
  const moreTop = document.querySelector<HTMLElement>("#trail-more-top");
  const moreBottom = document.querySelector<HTMLElement>("#trail-more-bottom");

  if (trail) {
    trail.addEventListener("scroll", updateTrailScrollIndicators, { passive: true });
  }

  if (sidebar && trail) {
    sidebar.addEventListener(
      "wheel",
      (e) => {
        if (trail.scrollHeight > trail.clientHeight) {
          trail.scrollTop += e.deltaY;
          e.preventDefault();
        }
      },
      { passive: false },
    );
  }

  moreTop?.addEventListener("click", () => {
    trail?.scrollBy({ top: -50, behavior: scrollBehavior() });
  });

  moreBottom?.addEventListener("click", () => {
    trail?.scrollBy({ top: 50, behavior: scrollBehavior() });
  });

  window.addEventListener("resize", () => {
    updateTrailLayout();
    requestAnimationFrame(fitProviderNames);
  });

  document.addEventListener("input", (e) => {
    const inp = (e.target as HTMLElement | null)?.closest?.("#cust-search");
    if (inp) applyCustSearch((inp as HTMLInputElement).value);
  });
}

/// Codex-style magnetic rail: ticks near the cursor stretch and brighten
/// with a smooth falloff; everything settles back when the mouse leaves.
/// Icon ticks scale uniformly (the mark grows) instead of stretching and
/// skip the background wash, which would paint over the artwork.
function setupTrailFisheye(): void {
  const sidebar = document.querySelector<HTMLElement>(".sidebar-right")!;
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
          tick.style.transform = `scale(${(1 + 0.4 * g).toFixed(3)})`;
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
  const trail = document.querySelector<HTMLElement>("#trail");
  const ticks = document.querySelectorAll<HTMLElement>("#trail .trail-tick");
  const activeEntry = trailEntries.findIndex((entry) => entry.indices.includes(active));
  ticks.forEach((tick, j) => {
    const isAct = j === activeEntry;
    tick.classList.toggle("active", isAct);
    if (isAct && trail && trail.scrollHeight > trail.clientHeight) {
      tick.scrollIntoView({ block: "nearest", behavior: scrollBehavior() });
    }
  });
  updateTrailScrollIndicators();
}

// ---------------------------------------------------------------------------
// Spend row model tooltip
// ---------------------------------------------------------------------------

/// Tooltip for one Usage Trend bar: date + the day's value — dollars or
/// tokens for local-log spends (follows the metric toggle), sampled
/// used-percent for quota history, "no data" for unsampled days.
function showTrendTip(el: HTMLElement): void {
  const tip = document.querySelector<HTMLElement>("#model-tip")!;
  const [id, idxStr] = (el.dataset.trend ?? "").split("|");
  const spend = lastSpend.find((s) => s.id === id);
  // Mirror trendSourceFor's precedence: credits-billed cards render the
  // credit series, so the hover must read it too.
  const credits = spend ? undefined : lastCreditTrend[id];
  const useCredits = !spend && credits?.some((v) => v != null && v > 0);
  const sampled = spend || useCredits ? undefined : lastQuotaTrend[id];
  if (!spend && !sampled && !useCredits) return;
  const i = Number(idxStr);
  if (Number.isNaN(i)) return;
  const date = new Date(Date.now() - (29 - i) * 86_400_000).toLocaleDateString(localeTag(), {
    weekday: "short",
    month: "short",
    day: "numeric",
  });

  let lines: string;
  if (spend) {
    const metricCost = config.spendMetric === "cost";
    const series = metricCost ? spend.trend_cost : spend.trend;
    const dayVal = series[i] ?? 0;
    const total = series.reduce((a, b) => a + b, 0);
    const share = total > 0 ? (dayVal / total) * 100 : 0;
    lines = `
    <div class="tip-line"><span class="tip-name">${escapeHtml(date)}</span><span>${
      dayVal > 0 ? escapeHtml(metricCost ? fmtMoney(dayVal) : t("card.tokens", { n: fmtTokens(dayVal) })) : escapeHtml(t("spend.noUsage"))
    }</span></div>
    ${dayVal > 0 ? `<div class="tip-line detail"><span>${escapeHtml(t("spend.of30", { n: share < 1 ? "<1" : share.toFixed(0) }))}</span></div>` : ""}`;
  } else if (useCredits && credits) {
    const dayVal = credits[i];
    lines = `
    <div class="tip-line"><span class="tip-name">${escapeHtml(date)}</span><span>${
      dayVal == null ? escapeHtml(t("spend.noDataDay")) : dayVal > 0 ? escapeHtml(t("spend.creditBarTip", { n: fmtCredits(dayVal) })) : escapeHtml(t("spend.noUsage"))
    }</span></div>`;
  } else {
    const pct = sampled?.[i];
    lines = `
    <div class="tip-line"><span class="tip-name">${escapeHtml(date)}</span><span>${
      pct == null ? escapeHtml(t("spend.noDataDay")) : pct > 0 ? escapeHtml(t("spend.quotaBarTip", { n: Math.round(pct) })) : escapeHtml(t("spend.noUsage"))
    }</span></div>`;
  }
  tip.innerHTML = lines;

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
  scheduleRender();
}

/// One refresh can ask to paint several times (usage, history, spend,
/// each extra-account list). Coalesce those onto the next frame so the
/// tree is built once.
let renderFrame = 0;
function scheduleRender(): void {
  if (renderFrame) return;
  renderFrame = requestAnimationFrame(() => {
    renderFrame = 0;
    if (document.hidden) {
      pendingRender = true;
      return;
    }
    renderAll();
    populatePinnedOptions();
  });
}

/// 30s tick: rewrite countdown text in place. A full `renderAll` would
/// throw away the whole tree (scroll, hover, focus) just to change
/// "Resets in 3h 41m" → "3h 40m".
function tickCountdowns(): void {
  if (document.hidden) {
    pendingRender = true;
    return;
  }
  if (customizeOpen || !lastSnapshots.length) return;
  const now = Date.now();
  let expired = false;
  document.querySelectorAll<HTMLElement>("[data-reset-at]").forEach((el) => {
    const at = Number(el.dataset.resetAt);
    if (!Number.isFinite(at)) return;
    const remain = at - now;
    if (remain <= 0) {
      expired = true;
      return;
    }
    if (el.dataset.flip === "reset") {
      if (config.resetExact) return;
      el.textContent = remain < 60_000 ? t("card.resetsSoon") : t("card.resetsIn", { time: fmtDuration(remain) });
      return;
    }
    el.textContent = fmtDuration(remain);
  });
  // A window that just closed needs a real refresh — bars and tones change.
  if (expired) renderIfVisible();
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

async function refresh(
  force = false,
  usageOnly = false,
  interactive = false,
): Promise<void> {
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
  const spendHistoryPromise = usageOnly
    ? Promise.resolve<{ d7: HistorySpend[]; all: HistorySpend[] } | null>(null)
    : spendPromise.then(() =>
        Promise.all([
          invoke<HistorySpend[]>("fetch_spend_history", { rangeDays: 7 }).catch(() => []),
          invoke<HistorySpend[]>("fetch_spend_history", { rangeDays: null }).catch(() => []),
        ]).then(([d7, all]) => ({ d7, all })),
      );
  const spendDailyPromise = usageOnly
    ? Promise.resolve<DailySpendRow[] | null>(null)
    : spendPromise.then(() => invoke<DailySpendRow[]>("fetch_spend_daily", { rangeDays: 365 }).catch(() => []));
  // The sampled quota history is one tiny JSON read — fetch it even for
  // usage-only refreshes so account cards keep their trend bars fresh.
  const quotaTrendPromise = invoke<Record<string, (number | null)[]>>("fetch_usage_history").catch(
    () => null,
  );
  // Credits-billed cards (Qoder CN / Trae CN): daily credit consumption,
  // kept apart from the percent-based quota trend and any token totals.
  const creditTrendPromise = invoke<Record<string, (number | null)[]>>("fetch_credit_history").catch(
    () => null,
  );
  try {
    await unparkRecentlyKeyed();
    let snapshots = await invoke<Snapshot[]>("fetch_usage", {
      disabled: [...config.disabled],
      // Only explicit clicks (Refresh button, Ctrl+R, overview ⟳) may
      // clear ordinary-error benches; timer/refocus passes stay polite.
      clearBenches: interactive,
    });
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
  const spendHistory = await spendHistoryPromise;
  const spendDaily = await spendDailyPromise;
  const quotaTrend = await quotaTrendPromise;
  if (quotaTrend) {
    lastQuotaTrend = quotaTrend;
    // A usage-only refresh rendered before the history landed; account
    // cards gaining their first trend need the layout patched + repainted.
    if (usageOnly && lastSnapshots.length) {
      ensureLayout();
      if (!customizeOpen) renderIfVisible();
    }
  }
  const creditTrend = await creditTrendPromise;
  if (creditTrend) {
    lastCreditTrend = creditTrend;
    if (usageOnly && lastSnapshots.length) {
      ensureLayout();
      if (!customizeOpen) renderIfVisible();
    }
  }
  if (usageOnly) return;
  if (spendHistory && myGen >= lastAppliedSpendGen) {
    lastSpendHistory = spendHistory;
  }
  if (spendDaily && myGen >= lastAppliedSpendGen) {
    lastSpendDaily = spendDaily;
  }
  spendLoaded = true;
  // Overlapping scans are allowed now that Refresh unlocks before spend
  // finishes. Keep the newest successful result — a later failed scan
  // (null) must not discard dollars an older pass already computed.
  if (spend && myGen >= lastAppliedSpendGen) {
    lastSpend = spend;
    lastAppliedSpendGen = myGen;
  }
  if (lastSnapshots.length) ensureLayout();
  // The merged account tabs on the dashboard need the account labels; load
  // the lists now (cached after the first pass).
  for (const def of providerCatalog) {
    if (def.supportsExtraAccounts) fetchAccounts(def.familyId);
  }
  if (!customizeOpen && lastSnapshots.length) renderIfVisible();
}

function scheduleAutoRefresh(): void {
  if (refreshTimer !== undefined) window.clearInterval(refreshTimer);
  const minutes = Math.max(1, config.refreshMinutes || 5);
  refreshTimer = window.setInterval(() => {
    // A hidden WebView2 throttles intervals to a halt; the Rust-side
    // refresh loop owns fetching then and pushes "usage-updated". This
    // timer only covers the visible window.
    if (document.hidden) return;
    void refresh();
  }, minutes * 60 * 1000);
}

const logoPixels = new Map<string, number[]>();

async function rasterizeLogo(id: string): Promise<number[] | null> {
  const cached = logoPixels.get(id);
  if (cached) return cached;
  const svg = providerVisual(id)?.iconSvg;
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
    // Raster-embedded logos have no fill/stroke to whiten; the filter turns
    // any draw into a white silhouette (a no-op for already-white vectors).
    ctx.filter = "brightness(0) invert(1)";
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
  // Panel form: the popover window owns the tray surfaces. Syncing from
  // here before any snapshot arrived would push an empty strip over the
  // main window's — only join in once usage-updated has warmed the cache.
  if (IS_PANEL_FORM && !lastSnapshots.length) return;
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

async function handleCustomizeClick(target: HTMLElement): Promise<boolean> {
  const skinOpen = target.closest<HTMLElement>("[data-skin-open]");
  if (skinOpen) {
    skinMarketOpen = true;
    skinPreviewId = null;
    renderDrawerBody();
    return true;
  }
  const skinClose = target.closest<HTMLElement>("[data-skin-close]");
  if (skinClose) {
    setDrawer(false);
    return true;
  }
  const skinPreview = target.closest<HTMLElement>("[data-skin-preview]");
  if (skinPreview) {
    skinPreviewId = skinPreview.dataset.skinPreview as SkinId;
    renderDrawerBody();
    return true;
  }
  const skinBack = target.closest<HTMLElement>("[data-skin-back]");
  if (skinBack) {
    skinPreviewId = null;
    renderDrawerBody();
    return true;
  }
  const skinSelect = target.closest<HTMLElement>("[data-skin-select]");
  if (skinSelect) {
    selectSkin(skinSelect.dataset.skinSelect as SkinId);
    return true;
  }
  const skinReset = target.closest<HTMLElement>("[data-skin-reset]");
  if (skinReset) {
    selectSkin(null);
    renderDrawerBody();
    return true;
  }
  // One/New API site manager (relocated from Settings): expand/edit/delete
  // actions live inside the family row's account section.
  const onaHandled = handleOneNewApiClick(target);
  if (onaHandled) return true;
  const clearToken = target.closest<HTMLElement>("[data-ona-clear-token]");
  if (clearToken) {
    void clearOneNewApiToken(clearToken.dataset.onaClearToken!);
    return true;
  }
  const delKey = target.closest<HTMLElement>("[data-ona-delete-key]");
  if (delKey) {
    const siteId = delKey.closest<HTMLElement>("[data-ona-site]")?.dataset.onaSite;
    if (siteId) void deleteOneNewApiKey(siteId, delKey.dataset.onaDeleteKey!);
    return true;
  }
  const del = target.closest<HTMLElement>("[data-ona-delete]");
  if (del) {
    void deleteOneNewApiSite(del.dataset.onaDelete!);
    return true;
  }
  const link = target.closest<HTMLElement>("[data-link]");
  if (link) {
    void invoke("open_link", { url: link.dataset.link }).catch((err) => {
      const status = document.querySelector("#status");
      if (status) status.textContent = t("footer.openLinkFailed", { err: String(err) });
    });
    return true;
  }
  const groupOpen = target.closest<HTMLElement>("[data-group-open]");
  if (groupOpen) {
    openGroupPicker(groupOpen.dataset.groupOpen!, groupOpen);
    return true;
  }
  const groupRename = target.closest<HTMLElement>("[data-group-rename]");
  if (groupRename) {
    const gid = groupRename.dataset.groupRename!;
    const g = cardGroup(gid);
    if (g) {
      const name = await appPrompt({
        title: t("customize.groupRenamePrompt"),
        initial: g.name,
        confirmLabel: t("dialog.ok"),
      });
      if (name && name !== g.name) {
        upsertCardGroup(gid, name);
        renderAll(); // the name shows on dashboard banners + overview pills too
      }
    }
    return true;
  }
  const groupDelete = target.closest<HTMLElement>("[data-group-delete]");
  if (groupDelete) {
    const gid = groupDelete.dataset.groupDelete!;
    const g = cardGroup(gid);
    if (g) {
      const count = cardGroupMemberCount(gid);
      if (count > 0) {
        await appConfirm({ title: t("overview.groupDeleteBlockedTitle"), message: t("overview.groupDeleteBlockedBody", { name: g.name, n: count }), confirmLabel: t("dialog.ok") });
        return true;
      }
      if (await appConfirm({
        title: t("customize.groupDelete"),
        message: t("customize.groupDeleteConfirm", { name: g.name }),
        confirmLabel: t("customize.groupDelete"),
        danger: true,
      })) {
        deleteCardGroup(gid); // its renderAll also refreshes the drawer body
      }
    }
    return true;
  }
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
  const removeBtn = target.closest<HTMLElement>("[data-remove-provider]");
  if (removeBtn) {
    const id = removeBtn.dataset.removeProvider!;
    const name = ALL_PROVIDERS.find(([pid]) => pid === id)?.[1] ?? id;
    void appConfirm({
      title: t("customize.providerDeleteTitle"),
      message: t("customize.providerDeleteConfirm", { name }),
      confirmLabel: t("customize.providerDelete"),
    }).then((ok) => {
      if (ok) removeProviderCard(id);
    });
    return true;
  }
  const restoreBtn = target.closest<HTMLElement>("[data-restore-provider]");
  if (restoreBtn) {
    restoreProviderFamily(restoreBtn.dataset.restoreProvider!);
    return true;
  }
  const removedToggle = target.closest<HTMLElement>("[data-removed-toggle]");
  if (removedToggle) {
    custRemovedOpen = !custRemovedOpen;
    renderDrawerBody();
    return true;
  }
  const cfgBtn = target.closest<HTMLElement>("[data-config]");
  if (cfgBtn) {
    const id = cfgBtn.dataset.config!;
    if (custConfigOpen === id) custKeyEditing.delete(id);
    custConfigOpen = custConfigOpen === id ? null : id;
    custInfoOpen = null; // one panel at a time per row
    dismissAccountDialog?.();
    renderDrawerBody();
    if (custConfigOpen) {
      fetchCredStatus(id); // the status section's live chips
      fetchAccounts(providerFamily(id)); // the account section's saved list
      const block = document.querySelector<HTMLElement>(
        `#drawer-body [data-cust-provider="${CSS.escape(custConfigOpen)}"]`,
      );
      block?.querySelector<HTMLInputElement>("[data-cust-key]")?.focus();
      // Custom Balance / Linkso: pre-fill the relay base URL saved with the key.
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
  const stepfunLogin = target.closest<HTMLElement>("[data-stepfun-login]");
  if (stepfunLogin) {
    void invoke("open_link", { url: stepfunLogin.dataset.stepfunLogin! }).catch((err) => {
      const status = document.querySelector("#status");
      if (status) status.textContent = t("footer.openLinkFailed", { err: String(err) });
    });
    return true;
  }
  const sensenovaStart = target.closest<HTMLElement>("[data-sensenova-oauth-start]");
  if (sensenovaStart) {
    void runSensenovaOauthStart();
    return true;
  }
  const sensenovaFinish = target.closest<HTMLElement>("[data-sensenova-oauth-finish]");
  if (sensenovaFinish) {
    void runSensenovaOauthFinish();
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
    openAccountDialog(acctToggle.dataset.acctToggle!);
    return true;
  }
  const acctDel = target.closest<HTMLElement>("[data-acct-del]");
  if (acctDel) {
    const [family, idxStr] = acctDel.dataset.acctDel!.split("|");
    const index = Number(idxStr);
    if (family && Number.isInteger(index)) void doAccountRemove(family, index);
    return true;
  }
  const acctArchive = target.closest<HTMLElement>("[data-acct-archive]");
  if (acctArchive) {
    const [family, idxStr] = acctArchive.dataset.acctArchive!.split("|");
    const index = Number(idxStr);
    if (family && Number.isInteger(index)) void doAccountArchive(family, index);
    return true;
  }
  const acctSetdef = target.closest<HTMLElement>("[data-acct-setdef]");
  if (acctSetdef) {
    const [family, idxStr] = acctSetdef.dataset.acctSetdef!.split("|");
    const index = Number(idxStr);
    if (family && Number.isInteger(index)) void doAccountSetDefault(family, index);
    return true;
  }
  const acctRename = target.closest<HTMLElement>("[data-acct-rename]");
  if (acctRename) {
    const [family, idxStr] = acctRename.dataset.acctRename!.split("|");
    const index = Number(idxStr);
    const input = acctRename
      .closest(".cust-config")
      ?.querySelector<HTMLInputElement>("[data-acct-label-edit]");
    if (family && Number.isInteger(index) && input) {
      void doAccountRename(family, index, input.value);
    }
    return true;
  }
  const agCapture = target.closest<HTMLElement>("[data-ag-capture]");
  if (agCapture) {
    void doAntigravityCapture(agCapture.dataset.agCapture!);
    return true;
  }
  const cursorAccount = target.closest<HTMLElement>("[data-cursor-account]");
  if (cursorAccount) {
    openCursorAccountDialog();
    return true;
  }
  const custSave = target.closest<HTMLElement>("[data-cust-save]");
  if (custSave) {
    const id = custSave.dataset.custSave!;
    const panel = custSave.closest(".cust-config");
    const keyInp = panel?.querySelector<HTMLInputElement>("[data-cust-key]");
    if (id === "stepfun-plan") {
      // Triple-credential save: the command validates the account with a
      // real login before storing, so a typo fails here, not later.
      const username = panel?.querySelector<HTMLInputElement>("[data-cust-user]")?.value.trim() ?? "";
      const password = panel?.querySelector<HTMLInputElement>("[data-cust-pass]")?.value ?? "";
      const result = panel?.querySelector<HTMLElement>("[data-cust-result]");
      void invoke("stepfun_plan_save", {
        token: keyInp?.value.trim() ?? "",
        username,
        password,
      })
        .then(() => {
          if (result) {
            result.textContent = t("customize.stepfunPlanSaved");
            result.classList.toggle("ok", true);
            result.classList.toggle("err", false);
          }
          const passInp = panel?.querySelector<HTMLInputElement>("[data-cust-pass]");
          if (passInp) passInp.value = ""; // never leave the password in the DOM
          void forceUsageRefreshAttempt(false).then(requestTraySync);
        })
        .catch((err: unknown) => {
          if (result) {
            result.textContent = `${t("customize.testFailed")}: ${String(err)}`;
            result.classList.toggle("ok", false);
            result.classList.toggle("err", true);
          }
        });
      return true;
    }
    if (keyInp) {
      void saveApiKey(id, {
        key: keyInp,
        baseUrl: panel?.querySelector<HTMLInputElement>("[data-cust-baseurl]") ?? null,
      });
    }
    const note = panel?.querySelector<HTMLInputElement>(`[data-cust-note="${CSS.escape(id)}"]`);
    if (note) setCardNote(id, note.value.trim());
    return true;
  }
  const custEdit = target.closest<HTMLElement>("[data-cust-edit]");
  if (custEdit) {
    const id = custEdit.dataset.custEdit!;
    custKeyEditing.add(id);
    renderDrawerBody();
    document
      .querySelector<HTMLElement>(`#drawer-body [data-cust-provider="${CSS.escape(id)}"] [data-cust-key]`)
      ?.focus();
    return true;
  }
  const custApply = target.closest<HTMLElement>("[data-cust-apply]");
  if (custApply) {
    const id = custApply.dataset.custApply!;
    const panel = custApply.closest<HTMLElement>(".cust-config");
    const note = panel?.querySelector<HTMLInputElement>("[data-cust-note]");
    if (note) setCardNote(id, note.value.trim());
    void forceUsageRefreshAttempt(false).then(requestTraySync);
    return true;
  }
  const az = target.closest<HTMLElement>("[data-az]");
  if (az) {
    document
      .querySelector<HTMLElement>(`#drawer-body [data-letter="${az.dataset.az}"]:not([hidden])`)
      ?.scrollIntoView({ behavior: scrollBehavior(), block: "start" });
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
    config.layout.providers[id] = defaultProviderLayout(
      snapshot,
      spend,
      Boolean(trendSourceFor(id)),
      false,
    );
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

function setSensenovaResult(text: string, ok: boolean | null): void {
  const result = document
    .querySelector<HTMLElement>(
      `#drawer-body [data-cust-provider="${CSS.escape("sensenova")}"]`,
    )
    ?.querySelector<HTMLElement>("[data-sensenova-result]");
  if (!result) return;
  result.textContent = text;
  result.classList.toggle("ok", ok === true);
  result.classList.toggle("err", ok === false);
}

/// SenseNova sign-in step 1: build the PKCE link backend-side and open it.
async function runSensenovaOauthStart(): Promise<void> {
  try {
    const url = await invoke<string>("sensenova_oauth_start");
    await invoke("open_link", { url });
    setSensenovaResult(t("customize.sensenovaOpened"), true);
  } catch (err) {
    setSensenovaResult(`${t("customize.testFailed")}: ${String(err)}`, false);
  }
}

/// SenseNova sign-in step 2: submit whatever the browser yielded —
/// redirect URL, token payload, bare token, or code.
async function runSensenovaOauthFinish(): Promise<void> {
  const panel = document.querySelector<HTMLElement>(
    `#drawer-body [data-cust-provider="${CSS.escape("sensenova")}"]`,
  );
  const input = panel?.querySelector<HTMLInputElement>("[data-sensenova-code]");
  const value = input?.value.trim() ?? "";
  if (!value) {
    setSensenovaResult(t("customize.sensenovaPasteEmpty"), false);
    return;
  }
  setSensenovaResult(t("customize.testing"), null);
  try {
    const snap = await invoke<{ metrics: unknown[] }>("sensenova_oauth_finish", { input: value });
    setSensenovaResult(t("customize.testOk", { n: snap.metrics.length }), true);
    if (input) input.value = "";
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    setSensenovaResult(`${t("customize.testFailed")}: ${String(err)}`, false);
  }
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
  const generation = bumpTestGeneration("cust", id);
  const show = (text: string, ok: boolean | null) => {
    result.textContent = text;
    result.classList.toggle("ok", ok === true);
    result.classList.toggle("err", ok === false);
  };
  const key = keyInp.value.trim();
  if (id === "stepfun-plan") {
    // Step Plan takes account+password (auto-renewing session) or a pasted
    // token pair — the generic single-key probe doesn't apply.
    const username = panel?.querySelector<HTMLInputElement>("[data-cust-user]")?.value.trim() ?? "";
    const password = panel?.querySelector<HTMLInputElement>("[data-cust-pass]")?.value ?? "";
    if (!key && !(username && password)) {
      show(t("customize.testEmpty"), false);
      if (saveBtn) saveBtn.disabled = false;
      return;
    }
    if (saveBtn) saveBtn.disabled = true;
    show(t("customize.testing"), null);
    try {
      const r = await invoke<{ ok: boolean; metrics: number; message: string }>("stepfun_plan_test", {
        token: key,
        username,
        password,
      });
      if (!isCurrentTestGeneration("cust", id, generation)) return;
      if (r.ok) {
        show(t("customize.testOk", { n: r.metrics }), true);
        if (saveBtn) saveBtn.disabled = false;
      } else {
        show(`${t("customize.testFailed")}: ${r.message}`, false);
      }
    } catch (err) {
      if (!isCurrentTestGeneration("cust", id, generation)) return;
      show(`${t("customize.testFailed")}: ${String(err)}`, false);
    }
    return;
  }
  if (!key) {
    show(t("customize.testEmpty"), false);
    if (saveBtn) saveBtn.disabled = false; // empty = clear the stored key
    return;
  }
  if (saveBtn) saveBtn.disabled = true;
  show(t("customize.testing"), null);
  try {
    const baseUrl =
      panel?.querySelector<HTMLInputElement>("[data-cust-baseurl]")?.value.trim() ?? "";
    const r = await invoke<{ ok: boolean; metrics: number; message: string }>("test_api_key", {
      provider: id,
      key,
      baseUrl: baseUrl || null,
    });
    if (!isCurrentTestGeneration("cust", id, generation)) return;
    if (r.ok) {
      show(t("customize.testOk", { n: r.metrics }), true);
      if (saveBtn) saveBtn.disabled = false;
    } else {
      show(`${t("customize.testFailed")}: ${r.message}`, false);
    }
  } catch (err) {
    if (!isCurrentTestGeneration("cust", id, generation)) return;
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
  const id = keyInp?.dataset.custKey;
  if (id) bumpTestGeneration("cust", id);
  if (saveBtn) saveBtn.disabled = keyInp?.value.trim() ? true : false;
  if (result) {
    result.textContent = "";
    result.classList.remove("ok", "err");
  }
}

function syncAccountTestButton(form: HTMLElement | null): void {
  if (!form) return;
  const key = form.querySelector<HTMLInputElement>("[data-acct-key]")?.value.trim();
  const label = form.querySelector<HTMLInputElement>("[data-acct-label]")?.value.trim();
  const testBtn = form.querySelector<HTMLButtonElement>("[data-acct-test]");
  if (testBtn) testBtn.disabled = !(key && label);
}

/// Any edit to the account dialog's inputs invalidates its passing test:
/// Save re-locks until the new values are tested again.
function resetAcctTestState(form: HTMLElement | null): void {
  if (!form) return;
  const addBtn = form.querySelector<HTMLButtonElement>("[data-acct-add]");
  const result = form.querySelector<HTMLElement>("[data-acct-result]");
  const family = form.querySelector<HTMLInputElement>("[data-acct-key]")?.dataset.acctKey;
  if (family) bumpTestGeneration("acct", family);
  syncAccountTestButton(form);
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

async function handleCustomizeChange(target: HTMLInputElement): Promise<void> {
  if (target.dataset.enable !== undefined) {
    const id = target.dataset.enable;
    const enable = target.checked;
    const enableGeneration = enable ? markProviderEnablePending(id) : null;
    if (!enable) pendingProviderEnables.delete(id);
    pendingToggles.push({ id, enable });
    config.disabled = withPendingToggles(config.disabled); // optimistic
    renderAll(); // disabled cards vanish from the dashboard immediately
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
  has_access_token: boolean;
  user_id: string;
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
        <div class="ona-add-row">
          <input type="password" data-ona-edit-token value="" placeholder="${escapeHtml(site.has_access_token ? t("settings.onenewapiTokenSavedPh") : t("settings.onenewapiTokenPh"))}" autocomplete="new-password" />
          <button type="button" class="mini-btn" data-ona-clear-token="${escapeHtml(site.id)}">${escapeHtml(t("settings.onenewapiTokenClear"))}</button>
        </div>
        <input type="text" spellcheck="false" data-ona-edit-uid value="${escapeHtml(site.user_id)}" placeholder="${escapeHtml(t("settings.onenewapiUidPh"))}" />
        <p class="settings-note ona-key-hint">${escapeHtml(t("settings.onenewapiTokenHint"))}</p>
        <div class="ona-edit-actions">
          <button type="submit">${escapeHtml(t("settings.save"))}</button>
          <button type="button" data-ona-cancel>${escapeHtml(t("dialog.cancel"))}</button>
        </div>
      </form>`
    : `<p class="ona-site-url">${escapeHtml(site.base_url)}${
        site.has_access_token
          ? `<span class="ona-key-has" title="${escapeHtml(t("footer.onenewapiTokenSaved"))}">✓</span>`
          : ""
      }</p>`;
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
          <span class="chev">${uiIcon("caretDown")}</span>
        </button>
        <button type="button" class="mini-btn" data-ona-edit="${escapeHtml(site.id)}">${escapeHtml(t("settings.onenewapiEdit"))}</button>
        <button type="button" class="mini-btn danger" data-ona-delete="${escapeHtml(site.id)}">${escapeHtml(t("settings.onenewapiDelete"))}</button>
      </div>
      <div class="acc-body"><div class="acc-inner">${head}${keysHtml}${addKey}</div></div>
    </article>`;
}

/// One/New API family row children in Customize: the site manager that used
/// to live in Settings, now the family's account list — one row per relay
/// site (the account) with its access token and relay keys, plus the
/// add-site form. Rendered inline from the cached onaSites so drawer
/// re-renders never blank it.
function onaFamilySection(): string {
  if (!onaSitesLoaded) {
    void loadOneNewApiSites();
    return `<p class="dim cust-account-loading">${escapeHtml(t("customize.credStatusLoading"))}</p>`;
  }
  return `<div class="cust-account-children" data-accounts-children="${escapeHtml(ONA_FAMILY)}"><div id="onenewapi-sites">${onaSites.map(renderOneNewApiSite).join("")}</div></div>
    <form id="onenewapi-add" class="ona-add" autocomplete="off">
      <input id="ona-add-name" type="text" spellcheck="false" placeholder="${escapeHtml(t("settings.onenewapiNamePh"))}" />
      <input id="ona-add-url" type="text" spellcheck="false" placeholder="${escapeHtml(t("settings.onenewapiUrlPh"))}" />
      <div class="ona-add-row">
        <input id="ona-add-secret" type="password" placeholder="${escapeHtml(t("settings.onenewapiKeySecretPh"))}" autocomplete="new-password" />
        <button type="submit">${escapeHtml(t("settings.onenewapiAdd"))}</button>
      </div>
    </form>`;
}

function renderOneNewApiSettings(): void {
  // The sites live inside the Customize drawer now; a drawer rebuild
  // re-renders them inline, so only a standalone host needs a manual fill.
  if (customizeOpen) {
    renderDrawerBody();
    return;
  }
  const host = document.querySelector("#onenewapi-sites");
  if (host) host.innerHTML = onaSites.map(renderOneNewApiSite).join("");
}

function focusOneNewApiSite(id: string): void {
  onaExpanded.add(id);
  custExpanded.add(ONA_FAMILY);
  renderOneNewApiSettings();
  requestAnimationFrame(() => {
    document.querySelector(`[data-ona-site="${CSS.escape(id)}"]`)?.scrollIntoView({
      block: "nearest",
      behavior: scrollBehavior(),
    });
  });
}

async function loadOneNewApiSites(opts?: { focusId?: string }): Promise<void> {
  try {
    onaSites = await invoke<OneNewApiSiteDto[]>("onenewapi_list_sites");
    onaSitesLoaded = true;
  } catch (err) {
    onaSites = [];
    // Mark loaded even on failure — the render-triggered fetch must not
    // retry on every drawer repaint (the error already hit the footer).
    onaSitesLoaded = true;
    setOneNewApiStatus("footer.onenewapiFailed", { err: String(err) });
    if (opts?.focusId) {
      focusOneNewApiSite(opts.focusId);
    } else {
      renderOneNewApiSettings();
    }
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
    const tokenInput = block?.querySelector<HTMLInputElement>("[data-ona-edit-token]");
    const uidInput = block?.querySelector<HTMLInputElement>("[data-ona-edit-uid]");
    const accessToken = tokenInput?.value ?? "";
    const userId = uidInput?.value ?? "";
    let authChanged = false;
    if (accessToken.trim()) {
      try {
        await invoke("onenewapi_set_site_access_token", {
          siteId: id,
          accessToken,
          userId: userId.trim(),
        });
        if (tokenInput) tokenInput.value = "";
        authChanged = true;
      } catch (err) {
        // The site edit itself already applied; reload so a retry doesn't
        // re-confirm the (done) URL migration, and keep the token typed in.
        setOneNewApiCaughtError(err);
        onaExpanded.add(id);
        await loadOneNewApiSites();
        return;
      }
    } else if (userId.trim() !== (onaSites.find((s) => s.id === id)?.user_id ?? "")) {
      try {
        await invoke("onenewapi_set_site_access_token", { siteId: id, userId: userId.trim() });
        authChanged = true;
      } catch (err) {
        setOneNewApiCaughtError(err);
      }
    }
    onaEditingId = null;
    onaExpanded.add(id);
    setOneNewApiStatus(authChanged ? "footer.onenewapiTokenSaved" : "footer.onenewapiSaved");
    await loadOneNewApiSites();
    if (urlChanged || authChanged) await forceUsageRefreshAttempt();
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

async function clearOneNewApiToken(siteId: string): Promise<void> {
  if (onaBusy) return;
  onaBusy = true;
  try {
    await invoke("onenewapi_set_site_access_token", { siteId, accessToken: "", userId: null });
    setOneNewApiStatus("footer.onenewapiTokenCleared");
    await loadOneNewApiSites();
    await forceUsageRefreshAttempt();
    requestTraySync();
  } catch (err) {
    setOneNewApiStatus("footer.onenewapiFailed", { err: String(err) });
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

function handleOneNewApiClick(target: HTMLElement): boolean {
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
    return true;
  }
  const editKey = target.closest<HTMLElement>("[data-ona-edit-key]");
  if (editKey) {
    const keyId = editKey.dataset.onaEditKey!;
    if (onaEditingKeyId === keyId) return true;
    onaEditingKeyId = keyId;
    const siteId = editKey.closest<HTMLElement>("[data-ona-site]")?.dataset.onaSite;
    if (siteId) onaExpanded.add(siteId);
    renderOneNewApiSettings();
    requestAnimationFrame(() => {
      document
        .querySelector<HTMLInputElement>(`[data-ona-key="${CSS.escape(keyId)}"] [data-ona-key-label]`)
        ?.focus();
    });
    return true;
  }
  const edit = target.closest<HTMLElement>("[data-ona-edit]");
  if (edit) {
    const id = edit.dataset.onaEdit!;
    if (onaEditingId === id) return true;
    onaEditingId = id;
    onaExpanded.add(id);
    renderOneNewApiSettings();
    requestAnimationFrame(() => {
      document
        .querySelector<HTMLInputElement>(`[data-ona-site="${CSS.escape(id)}"] [data-ona-edit-name]`)
        ?.focus();
    });
    return true;
  }
  const cancelKey = target.closest<HTMLElement>("[data-ona-cancel-key]");
  if (cancelKey) {
    onaEditingKeyId = null;
    renderOneNewApiSettings();
    return true;
  }
  const cancel = target.closest<HTMLElement>("[data-ona-cancel]");
  if (cancel) {
    onaEditingId = null;
    renderOneNewApiSettings();
    return true;
  }
  return false;
}

async function unparkRecentlyKeyed(): Promise<void> {
  if (!config.disabled.some((id) => recentlyKeyed.has(id))) return;
  await patchConfig({
    disabled: config.disabled.filter((id) => !recentlyKeyed.has(id)),
  }).catch(() => {});
}

// Settings rows are gone (their key fields moved into each provider's
// Customize gear panel); the only caller passes its own inputs explicitly.
async function saveApiKey(
  provider: string,
  fields: { key: HTMLInputElement; baseUrl?: HTMLInputElement | null },
): Promise<void> {
  const input = fields.key;
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
    // Providers with a user-chosen endpoint (relaybalance) carry a
    // base URL input next to the key field; Save persists both together.
    const baseUrl = fields.baseUrl?.value.trim() || null;
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

interface KeyVaultRow {
  id: string;
  service: string;
  label: string;
  masked: string;
  note: string;
}

interface VaultStatus {
  has_password: boolean;
  unlocked: boolean;
  count: number;
  recovery_questions: number;
}

let vaultStatus: VaultStatus | null = null;
let lastVaultRows: KeyVaultRow[] = [];

// ── Master-password vault flows ────────────────────────────────────────────
// The vault bar mirrors keyvault.rs: no password yet → offer "set password"
// (everything still works, stored plaintext like before); password set →
// locked/unlocked state with unlock/lock buttons. Viewing or copying a key
// while locked routes through the unlock prompt first.

function renderVaultBar(): void {
  const bar = document.querySelector<HTMLElement>("#kv-vault-bar");
  if (!bar || !vaultStatus) return;
  bar.replaceChildren();
  const inner = document.createElement("div");
  inner.className = "kv-vault-bar-inner";
  const state = document.createElement("span");
  state.className = "kv-vault-state";
  const buttons: HTMLButtonElement[] = [];
  const button = (label: string, action: string, iconName: UiIconName) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "mini-btn";
    b.innerHTML = uiIcon(iconName) + escapeHtml(label);
    b.dataset.kvVaultAction = action;
    buttons.push(b);
  };
  if (!vaultStatus.has_password) {
    state.textContent = t("settings.kvVaultNoPassword");
    button(t("settings.kvSetPassword"), "set", "key");
  } else if (vaultStatus.unlocked) {
    state.textContent = t("settings.kvVaultUnlocked", { count: vaultStatus.count });
    button(t("settings.kvChange"), "change", "key");
    button(
      vaultStatus.recovery_questions > 0 ? t("settings.kvRecoveryEdit") : t("settings.kvRecoverySet"),
      "questions",
      "shield",
    );
    button(t("settings.kvLock"), "lock", "lock");
  } else {
    state.textContent = t("settings.kvVaultLocked", { count: vaultStatus.count });
    state.classList.add("locked");
    button(t("settings.kvUnlock"), "unlock", "lockOpen");
    if (vaultStatus.recovery_questions > 0) button(t("settings.kvForgot"), "forgot", "question");
  }
  inner.append(state, ...buttons);
  bar.append(inner);
}

async function setVaultPassword(): Promise<void> {
  const pw = await appPrompt({
    title: t("settings.kvSetPasswordTitle"),
    placeholder: t("settings.kvPasswordPlaceholder"),
    confirmLabel: t("settings.kvSetPassword"),
    secret: true,
  });
  if (pw === null) return;
  const again = await appPrompt({
    title: t("settings.kvPasswordConfirmTitle"),
    placeholder: t("settings.kvPasswordPlaceholder"),
    confirmLabel: t("settings.kvSetPassword"),
    secret: true,
  });
  if (again === null) return;
  if (pw !== again) {
    const status = document.querySelector("#status");
    if (status) status.textContent = t("settings.kvPasswordMismatch");
    return;
  }
  try {
    vaultStatus = await invoke<VaultStatus>("keyvault_set_password", { password: pw });
    await loadKeyvault();
    // Fresh password in hand — walk straight into recovery-question setup
    // (skip = no forgot-password path; the vault bar keeps the entry).
    renderRecoveryEditor(pw);
  } catch (err) {
    const status = document.querySelector("#status");
    if (status) status.textContent = String(err);
  }
}

// ── Q&A recovery (forgot master password) ───────────────────────────────────

let recoveryPresetPw: string | null = null;

const RECOVERY_PRESET_KEYS = [
  "settings.kvQFruit",
  "settings.kvQTool",
  "settings.kvQPet",
  "settings.kvQCartoon",
  "settings.kvQTravel",
  "settings.kvQUniversity",
  "settings.kvQPhone",
  "settings.kvQMovie",
];

function recoveryEditorRow(): HTMLElement {
  const row = document.createElement("div");
  row.className = "kv-recovery-row";
  row.innerHTML = `
    <input type="text" class="rc-q" list="kv-recovery-presets" maxlength="60"
      placeholder="${escapeHtml(t("settings.kvRecoveryQPh"))}" spellcheck="false" />
    <input type="password" class="rc-a" maxlength="40" autocomplete="off"
      placeholder="${escapeHtml(t("settings.kvRecoveryAPh"))}" />
    <input type="password" class="rc-a2" maxlength="40" autocomplete="off"
      placeholder="${escapeHtml(t("settings.kvRecoveryA2Ph"))}" />
    <button type="button" class="mini-btn" data-rc-del>−</button>`;
  return row;
}

/// The set/change-questions form. `pw` is the just-collected password when
/// the flow follows set-password (submit reuses it); null → submit prompts.
function renderRecoveryEditor(pw: string | null): void {
  recoveryPresetPw = pw;
  const panel = document.querySelector<HTMLElement>("#kv-recovery-panel");
  if (!panel) return;
  const presets = document.createElement("datalist");
  presets.id = "kv-recovery-presets";
  presets.innerHTML = RECOVERY_PRESET_KEYS.map(
    (k) => `<option value="${escapeHtml(t(k))}"></option>`,
  ).join("");
  const wrap = document.createElement("div");
  wrap.className = "kv-recovery";
  wrap.innerHTML = `
    <p class="kv-hint">${escapeHtml(t("settings.kvRecoveryHint"))}</p>
    <div class="kv-recovery-rows"></div>
    <div class="kv-recovery-actions">
      <button type="button" class="mini-btn" data-rc-add>${escapeHtml(t("settings.kvRecoveryAdd"))}</button>
      <span class="spacer"></span>
      <button type="button" class="mini-btn" data-rc-cancel>${escapeHtml(t("dialog.cancel"))}</button>
      <button type="button" class="mini-btn" data-rc-save>${escapeHtml(t("settings.kvRecoverySave"))}</button>
    </div>`;
  panel.replaceChildren(presets, wrap);
  const rows = wrap.querySelector<HTMLElement>(".kv-recovery-rows")!;
  // Default and recommendation: two questions.
  rows.append(recoveryEditorRow(), recoveryEditorRow());
}

async function submitRecoveryEditor(): Promise<void> {
  const status = document.querySelector("#status");
  const panel = document.querySelector<HTMLElement>("#kv-recovery-panel");
  if (!panel) return;
  const rows = [...panel.querySelectorAll<HTMLElement>(".kv-recovery-row")];
  const questions: [string, string][] = [];
  for (const r of rows) {
    const q = (r.querySelector(".rc-q") as HTMLInputElement).value.trim();
    const a = (r.querySelector(".rc-a") as HTMLInputElement).value;
    const a2 = (r.querySelector(".rc-a2") as HTMLInputElement).value;
    if (!q || !a.trim()) {
      if (status) status.textContent = t("settings.kvRecoveryFillAll");
      return;
    }
    if (a !== a2) {
      if (status) status.textContent = t("settings.kvRecoveryMismatch");
      return;
    }
    questions.push([q, a]);
  }
  let pw = recoveryPresetPw;
  recoveryPresetPw = null;
  if (pw === null) {
    pw = await appPrompt({
      title: t("settings.kvUnlockTitle"),
      placeholder: t("settings.kvPasswordPlaceholder"),
      confirmLabel: t("settings.kvUnlock"),
      secret: true,
    });
    if (pw === null) return;
  }
  try {
    vaultStatus = await invoke<VaultStatus>("keyvault_set_recovery", { password: pw, questions });
    if (status) status.textContent = t("settings.kvRecoverySaved", { n: questions.length });
    panel.replaceChildren();
    await loadKeyvault();
  } catch (err) {
    if (status) status.textContent = String(err);
  }
}

/// The forgot-password challenge: every stored question + one answer each;
/// all answers must verify before a new master password is accepted.
async function renderRecoveryChallenge(): Promise<void> {
  const panel = document.querySelector<HTMLElement>("#kv-recovery-panel");
  if (!panel) return;
  let qs: [string, string][] = [];
  try {
    qs = await invoke<[string, string][]>("keyvault_recovery_questions");
  } catch {
    qs = [];
  }
  if (!qs.length) return;
  const wrap = document.createElement("div");
  wrap.className = "kv-recovery";
  const qRows = qs
    .map(
      ([, q], i) => `
    <div class="kv-recovery-row">
      <span class="rc-q-text">${escapeHtml(q)}</span>
      <input type="password" class="rc-answer" data-rc-ans="${i}" maxlength="40" autocomplete="off"
        placeholder="${escapeHtml(t("settings.kvRecoveryAPh"))}" />
    </div>`,
    )
    .join("");
  wrap.innerHTML = `
    <p class="kv-hint">${escapeHtml(t("settings.kvForgotHint"))}</p>
    ${qRows}
    <div class="kv-recovery-actions">
      <span class="spacer"></span>
      <button type="button" class="mini-btn" data-rc-cancel>${escapeHtml(t("dialog.cancel"))}</button>
      <button type="button" class="mini-btn" data-rc-reset>${escapeHtml(t("settings.kvForgotSubmit"))}</button>
    </div>`;
  panel.replaceChildren(wrap);
}

async function submitRecoveryChallenge(): Promise<void> {
  const status = document.querySelector("#status");
  const panel = document.querySelector<HTMLElement>("#kv-recovery-panel");
  if (!panel) return;
  const inputs = [...panel.querySelectorAll<HTMLInputElement>(".rc-answer")];
  if (inputs.some((i) => !i.value.trim())) {
    if (status) status.textContent = t("settings.kvRecoveryFillAll");
    return;
  }
  const np = await appPrompt({
    title: t("settings.kvNewPwTitle"),
    placeholder: t("settings.kvPasswordPlaceholder"),
    confirmLabel: t("settings.kvChangeNext"),
    secret: true,
  });
  if (np === null) return;
  const again = await appPrompt({
    title: t("settings.kvPasswordConfirmTitle"),
    placeholder: t("settings.kvPasswordPlaceholder"),
    confirmLabel: t("settings.kvForgotSubmit"),
    secret: true,
  });
  if (again === null) return;
  if (np !== again) {
    if (status) status.textContent = t("settings.kvPasswordMismatch");
    return;
  }
  try {
    vaultStatus = await invoke<VaultStatus>("keyvault_recovery_reset", {
      answers: inputs.map((i) => i.value),
      newPassword: np,
    });
    if (status) status.textContent = t("settings.kvRecoveryDone");
    panel.replaceChildren();
    await loadKeyvault();
  } catch (err) {
    if (status) status.textContent = String(err);
    // keep the challenge panel so the user can retry (or read the cooldown)
  }
}

/// Returns true when the vault ended up unlocked (so callers can retry the
/// gated action that triggered the prompt).
async function unlockVault(): Promise<boolean> {
  const pw = await appPrompt({
    title: t("settings.kvUnlockTitle"),
    placeholder: t("settings.kvPasswordPlaceholder"),
    confirmLabel: t("settings.kvUnlock"),
    secret: true,
  });
  if (pw === null) return false;
  try {
    vaultStatus = await invoke<VaultStatus>("keyvault_unlock", { password: pw });
    await loadKeyvault();
    return true;
  } catch {
    const status = document.querySelector("#status");
    if (status) status.textContent = t("settings.kvWrongPassword");
    return false;
  }
}

async function lockVault(): Promise<void> {
  vaultStatus = await invoke<VaultStatus>("keyvault_lock");
  await loadKeyvault();
}

/// Change the master password: verify the old one server-side, then set and
/// confirm the new one. The whole vault is re-sealed on success.
async function changeVaultPassword(): Promise<void> {
  const status = document.querySelector("#status");
  const old = await appPrompt({
    title: t("settings.kvChange"),
    placeholder: t("settings.kvChangeOldPh"),
    confirmLabel: t("settings.kvChangeNext"),
    secret: true,
  });
  if (old === null) return;
  const next = await appPrompt({
    title: t("settings.kvChangeNewTitle"),
    placeholder: t("settings.kvChangeNewPh"),
    confirmLabel: t("settings.kvChangeNext"),
    secret: true,
  });
  if (next === null) return;
  const again = await appPrompt({
    title: t("settings.kvPasswordConfirmTitle"),
    placeholder: t("settings.kvChangeRepeatPh"),
    confirmLabel: t("settings.kvChange"),
    secret: true,
  });
  if (again === null) return;
  if (next !== again) {
    if (status) status.textContent = t("settings.kvPasswordMismatch");
    return;
  }
  try {
    vaultStatus = await invoke<VaultStatus>("change_master_password", { old, new: next });
    if (status) status.textContent = t("settings.kvChanged");
    await loadKeyvault();
  } catch (err) {
    if (status) {
      status.textContent = String(err).includes("wrong master password")
        ? t("settings.kvWrongPassword")
        : String(err);
    }
  }
}

/// Copy a stored provider/account key straight to the clipboard — same
/// gate as viewing (open session first, master-password prompt when
/// locked), but the plaintext never renders anywhere.
async function copySettingsKey(
  family: string,
  entryId: string | null,
  button: HTMLButtonElement,
): Promise<void> {
  const status = document.querySelector("#status");
  const call = (password: string) =>
    entryId
      ? invoke<string>("reveal_account_key", { provider: family, id: entryId, password })
      : invoke<string>("reveal_provider_key", { provider: family, password });
  try {
    let raw: string;
    try {
      raw = await call("");
    } catch (err) {
      const msg = String(err);
      if (msg.includes("no master password is set")) {
        if (status) status.textContent = t("settings.kvSetFirst");
        return;
      }
      if (!msg.includes("wrong master password")) throw err;
      const pw = await appPrompt({
        title: t("settings.kvUnlockTitle"),
        placeholder: t("settings.kvPasswordPlaceholder"),
        confirmLabel: t("settings.kvUnlock"),
        secret: true,
      });
      if (pw === null) return;
      raw = await call(pw);
    }
    await navigator.clipboard.writeText(raw);
    const old = button.textContent;
    button.innerHTML = uiIcon("copy") + escapeHtml(t("settings.kvCopied"));
    window.setTimeout(() => {
      button.innerHTML = uiIcon("copy") + escapeHtml(old || t("settings.kvCopy"));
    }, 1400);
  } catch (err) {
    if (status) status.textContent = String(err);
  }
}

/// Reveal a stored provider/account key in place: probe with the open
/// session (an empty password answers when unlocked), fall back to the
/// master-password prompt, toggle back on the second click. One password
/// and one lock cover the vault and every key — matching the backend gate.
async function revealSettingsKey(
  family: string,
  entryId: string | null,
  button: HTMLButtonElement,
  stateEl: HTMLElement,
): Promise<void> {
  const status = document.querySelector("#status");
  if (button.dataset.revealed === "1") {
    if (button.dataset.masked) stateEl.textContent = button.dataset.masked;
    delete button.dataset.revealed;
    button.innerHTML = uiIcon("eye") + escapeHtml(t("settings.kvReveal"));
    return;
  }
  const call = (password: string) =>
    entryId
      ? invoke<string>("reveal_account_key", { provider: family, id: entryId, password })
      : invoke<string>("reveal_provider_key", { provider: family, password });
  try {
    let raw: string;
    try {
      raw = await call("");
    } catch (err) {
      const msg = String(err);
      if (msg.includes("no master password is set")) {
        if (status) status.textContent = t("settings.kvSetFirst");
        return;
      }
      if (!msg.includes("wrong master password")) {
        if (status) status.textContent = msg;
        return;
      }
      const pw = await appPrompt({
        title: t("settings.kvUnlockTitle"),
        placeholder: t("settings.kvPasswordPlaceholder"),
        confirmLabel: t("settings.kvUnlock"),
        secret: true,
      });
      if (pw === null) return;
      raw = await call(pw);
    }
    button.dataset.masked = stateEl.textContent || "";
    stateEl.textContent = raw;
    button.dataset.revealed = "1";
    button.innerHTML = uiIcon("eyeSlash") + escapeHtml(t("settings.kvHide"));
  } catch (err) {
    const msg = String(err);
    if (status) {
      status.textContent = msg.includes("wrong master password")
        ? t("settings.kvWrongPassword")
        : msg;
    }
  }
}

async function revealKeyvaultEntry(id: string, button: HTMLButtonElement): Promise<void> {
  const item = button.closest<HTMLElement>(".kv-item");
  const code = item?.querySelector<HTMLElement>(".kv-masked");
  if (!item || !code) return;
  if (item.dataset.kvShown === "1") {
    code.textContent = item.dataset.kvMasked || "";
    code.classList.remove("kv-revealed");
    item.dataset.kvShown = "0";
    button.innerHTML = uiIcon("eye") + escapeHtml(t("settings.kvReveal"));
    return;
  }
  try {
    const raw = await invoke<string>("keyvault_reveal", { id });
    item.dataset.kvMasked = code.textContent || "";
    code.textContent = raw;
    code.classList.add("kv-revealed");
    item.dataset.kvShown = "1";
    button.innerHTML = uiIcon("eyeSlash") + escapeHtml(t("settings.kvHide"));
  } catch (err) {
    if (String(err).includes("locked") && (await unlockVault())) {
      return revealKeyvaultEntry(id, button);
    }
    const status = document.querySelector("#status");
    if (status && !String(err).includes("locked")) status.textContent = String(err);
  }
}

async function editKeyvaultNote(id: string): Promise<void> {
  const row = lastVaultRows.find((r) => r.id === id);
  const note = await appPrompt({
    title: t("settings.kvNoteTitle"),
    placeholder: t("settings.kvNotePlaceholder"),
    initial: row?.note ?? "",
    confirmLabel: t("settings.kvSaveNote"),
    allowEmpty: true,
  });
  if (note === null) return;
  try {
    renderKeyvault(await invoke<KeyVaultRow[]>("keyvault_set_note", { id, note }));
  } catch (err) {
    const status = document.querySelector("#status");
    if (status) status.textContent = String(err);
  }
}

// ---------------------------------------------------------------------------
// Settings panel: provider credential management (the keys page)
// ---------------------------------------------------------------------------

/// Which family's inline key editor or add-account form is open. Panel-local
/// (its window owns the DOM), re-rendered in place on every change.
const skeyEditing = new Set<string>();
const skeyAddingAccount = new Set<string>();

/// Every provider shown on the Keys & quota page: the whole catalog minus
/// the MCP/search families (their keys live in the MCP card) and the
/// One/New API relay (it has its own site manager). Key-capable families get
/// key controls; OAuth / local sign-in families show their live sign-in
/// state and the "?" explainer instead — the page states what each provider
/// needs, even where there is no key to paste.
const CRED_PAGE_SKIP = new Set(["bocha", "tavily", "firecrawl", "brave", "onenewapi"]);
function settingsCredentialFamilies(): string[] {
  return providerCatalog
    .map((definition) => definition.familyId)
    .filter(
      (family) => !CRED_PAGE_SKIP.has(family) && !config.removedProviders.includes(family),
    );
}

/// One provider's fresh credential facts (used by both the bulk load and
/// single-row refreshes after a write).
async function refreshSettingsProviderKey(family: string): Promise<void> {
  try {
    credStatusCache.set(
      family,
      await invoke<CredStatus>("get_credential_status", { provider: family }),
    );
  } catch {
    // A failed probe must not blank a row into a wrong "not set" state.
  }
  if (supportsExtraAccounts(family)) {
    try {
      accountsCache.set(
        family,
        await invoke<AccountEntry[]>("account_list", { provider: family }),
      );
    } catch {
      // Keep the previous list on failure.
    }
  }
}

async function loadSettingsProviderKeys(): Promise<void> {
  await Promise.all(settingsCredentialFamilies().map((family) => refreshSettingsProviderKey(family)));
  renderSettingsProviderKeys();
}

function renderSettingsProviderKeys(): void {
  const root = document.querySelector<HTMLElement>("#settings-keys-rows");
  if (!root) return;
  root.replaceChildren();
  // Configured rows first, then A–Z by display name.
  const families = settingsCredentialFamilies().sort((a, b) => {
    const configured = (f: string) =>
      Number(
        credStatusCache.get(f)?.storedKey === true ||
          (accountsCache.get(f)?.length ?? 0) > 0,
      );
    return (
      configured(b) - configured(a) ||
      providerDisplayName(a).localeCompare(providerDisplayName(b), "en")
    );
  });
  for (const family of families) root.append(settingsKeyRow(family));
}

function settingsKeyRow(family: string): HTMLElement {
  const status = credStatusCache.get(family);
  const accounts = accountsCache.get(family) ?? [];
  const multi = supportsExtraAccounts(family);
  const configured = multi ? accounts.length > 0 : status?.storedKey === true;

  const item = document.createElement("div");
  item.className = "skey-item";
  item.dataset.skeyFamily = family;

  const head = document.createElement("div");
  head.className = "skey-head";
  const icon = document.createElement("span");
  icon.className = "skey-icon";
  icon.innerHTML = providerVisual(family)?.iconSvg ?? uiIcon("key");
  const name = document.createElement("span");
  name.className = "skey-name";
  name.textContent = providerDisplayName(family);
  const state = document.createElement("span");
  state.className = `skey-state${configured ? " ok" : ""}`;
  // Families without a key are read from a local sign-in (Claude Code,
  // Cursor, gcloud, …): their state is that sign-in, and there is nothing
  // to paste — the "?" explains what the provider actually needs.
  const keyable = providerDefinition(family)?.supportsApiKey ?? false;
  const local = status?.membership || status?.oauth || status?.localCli;
  // Locally connected (IDE sign-in / Pane OAuth / membership): say so and
  // name the exact source — the mechanism table's files — whether or not
  // the family also accepts a pasted key.
  let detail: HTMLElement | null = null;
  const connected = !configured && Boolean(local);
  if (connected) {
    const mech = MECHANISMS[family];
    const parts = [String(local)];
    if (mech?.reads?.length) parts.push(t("settings.credReads", { files: mech.reads.join(" · ") }));
    detail = document.createElement("div");
    detail.className = "skey-detail";
    detail.textContent = `${t("settings.credConnected")} · ${parts.join(" · ")}`;
  }
  if (!keyable) {
    state.textContent = local ? t("settings.credConnected") : t("settings.credLocalMissing");
    state.classList.toggle("ok", Boolean(local));
  } else if (multi) {
    state.textContent = accounts.length
      ? t("settings.acctCount", { n: accounts.length })
      : t("settings.keyNotSet");
  } else if (configured) {
    state.textContent = status?.maskedKey || t("settings.keyConfigured");
  } else if (connected) {
    state.textContent = t("settings.credConnected");
    state.classList.add("ok");
  } else {
    state.textContent = status?.envKey ? t("settings.keyEnv") : t("settings.keyNotSet");
  }
  const actions = document.createElement("span");
  actions.className = "skey-actions";
  head.append(icon, name, state, actions);
  item.append(head);
  if (detail) item.append(detail);

  // "?" first: it sits left of the row's own buttons (查看 / 添加 …) and is
  // the only control a non-key family has.
  const help = document.createElement("button");
  help.type = "button";
  help.className = "mini-btn skey-help";
  help.innerHTML = uiIcon("question");
  help.title = t("customize.helpMenu");
  help.setAttribute("aria-label", t("customize.helpMenu"));
  help.addEventListener("click", () => openProviderHelp(family));
  actions.append(help);
  if (!keyable) return item;

  const button = (label: string, extraClass = "", iconName?: UiIconName) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = `mini-btn${extraClass}`;
    if (iconName) b.innerHTML = uiIcon(iconName) + escapeHtml(label);
    else b.textContent = label;
    return b;
  };

  if (multi) {
    const open = skeyAddingAccount.has(family);
    const add = button(open ? t("dialog.cancel") : t("customize.acctAdd"), "", open ? undefined : "plus");
    add.addEventListener("click", () => {
      open ? skeyAddingAccount.delete(family) : skeyAddingAccount.add(family);
      renderSettingsProviderKeys();
    });
    actions.append(add);
  } else if (skeyEditing.has(family)) {
    const cancel = button(t("dialog.cancel"));
    cancel.addEventListener("click", () => {
      skeyEditing.delete(family);
      renderSettingsProviderKeys();
    });
    actions.append(cancel);
  } else {
    if (configured) {
      const view = button(t("settings.kvReveal"), "", "eye");
      view.addEventListener("click", () => {
        void revealSettingsKey(family, null, view, state);
      });
      actions.append(view);
      const copy = button(t("settings.kvCopy"), " ok", "copy");
      copy.addEventListener("click", () => {
        void copySettingsKey(family, null, copy);
      });
      actions.append(copy);
    }
    const edit = button(configured ? t("settings.keyReplace") : t("settings.keyEdit"), "", "pencil");
    edit.addEventListener("click", () => {
      skeyEditing.add(family);
      renderSettingsProviderKeys();
    });
    actions.append(edit);
    if (configured) {
      const remove = button(t("settings.keyRemove"), " danger", "trash");
      remove.addEventListener("click", () => {
        void appConfirm({
          title: t("settings.keyRemove"),
          message: t("settings.keyRemoveConfirm", { name: providerDisplayName(family) }),
          confirmLabel: t("settings.kvRemove"),
        }).then(async (ok) => {
          if (!ok) return;
          try {
            await invoke("set_api_key", { provider: family, key: "" });
          } catch (err) {
            console.error("remove api key failed:", err);
          }
          await refreshSettingsProviderKey(family);
          renderSettingsProviderKeys();
        });
      });
      actions.append(remove);
    }
  }

  if (!multi && skeyEditing.has(family)) item.append(settingsKeyEditor(family));
  if (multi) {
    if (accounts.length) {
      const list = document.createElement("div");
      list.className = "skey-accounts";
      accounts.forEach((entry, index) => list.append(settingsAccountRow(family, entry, index)));
      item.append(list);
    }
    if (skeyAddingAccount.has(family)) item.append(settingsAccountEditor(family));
  }
  return item;
}

/// Test-first save gate for the key editors: key-based families talk to
/// preset endpoints, so their pasted credential must pass a live probe
/// (test_api_key — a pure read; nothing is written) before Save unlocks.
/// Families with no live probe fall back to a direct save. Editing the
/// input after a pass re-arms the gate.
function attachKeyTest(
  family: string,
  keyInput: HTMLInputElement,
  urlInput: HTMLInputElement | null,
  save: HTMLButtonElement,
  err: HTMLElement,
): HTMLButtonElement {
  const test = document.createElement("button");
  test.type = "button";
  test.className = "mini-btn";
  test.innerHTML = uiIcon("lightning") + escapeHtml(t("settings.keyTest"));
  let passed = false;
  const arm = () => {
    passed = false;
    save.disabled = true;
    test.classList.remove("ok");
    test.innerHTML = uiIcon("lightning") + escapeHtml(t("settings.keyTest"));
  };
  save.disabled = true;
  keyInput.addEventListener("input", () => {
    if (passed) arm();
  });
  urlInput?.addEventListener("input", () => {
    if (passed) arm();
  });
  test.addEventListener("click", () => {
    const value = keyInput.value.trim();
    if (!value) {
      err.textContent = t("settings.keyRequired");
      return;
    }
    test.disabled = true;
    test.innerHTML = uiIcon("lightning") + escapeHtml(t("customize.helpTesting"));
    void invoke<{ ok: boolean; metrics: number; message: string }>("test_api_key", {
      provider: family,
      key: value,
      baseUrl: urlInput?.value.trim() || null,
    })
      .then((res) => {
        if (res.ok) {
          passed = true;
          save.disabled = false;
          err.textContent = "";
          test.classList.add("ok");
          test.innerHTML = uiIcon("lightning") + escapeHtml(t("settings.keyTestOk"));
        } else {
          save.disabled = true;
          err.textContent = res.message || t("settings.keyTestFail");
        }
      })
      .catch((e) => {
        const msg = String(e);
        if (msg.includes("unknown provider")) {
          // No live probe exists for this family — direct save, no gate.
          test.remove();
          save.disabled = false;
          return;
        }
        save.disabled = true;
        err.textContent = msg;
      })
      .finally(() => {
        test.disabled = false;
        if (!passed) test.innerHTML = uiIcon("lightning") + escapeHtml(t("settings.keyTest"));
      });
  });
  return test;
}

/// Inline key editor: password field (+ base URL for relay families) with
/// Save/Cancel. Saves through the exact command Customize uses, so the
/// card picks the key up on its next refresh.
function settingsKeyEditor(family: string): HTMLElement {
  const form = document.createElement("div");
  form.className = "skey-editor";
  const key = document.createElement("input");
  key.type = "password";
  key.className = "form-input";
  key.placeholder = t("settings.keyPlaceholder");
  key.autocomplete = "off";
  key.spellcheck = false;
  form.append(key);
  let url: HTMLInputElement | null = null;
  if (RELAY_BASE_URL_FAMILIES.has(family)) {
    url = document.createElement("input");
    url.type = "text";
    url.className = "form-input";
    url.placeholder = "https://api.example.com";
    url.spellcheck = false;
    void invoke<string | null>("get_base_url", { provider: family })
      .then((v) => {
        if (v && url) url.value = v;
      })
      .catch(() => {});
    form.append(url);
  }
  const err = document.createElement("div");
  err.className = "skey-error";
  const row = document.createElement("div");
  row.className = "skey-editor-actions";
  const save = document.createElement("button");
  save.type = "button";
  save.className = "mini-btn primary";
  save.textContent = t("settings.save");
  save.addEventListener("click", () => {
    const value = key.value.trim();
    if (!value) {
      err.textContent = t("settings.keyRequired");
      return;
    }
    save.disabled = true;
    void invoke("set_api_key", {
      provider: family,
      key: value,
      baseUrl: url?.value.trim() || null,
    })
      .then(async () => {
        skeyEditing.delete(family);
        await refreshSettingsProviderKey(family);
        renderSettingsProviderKeys();
      })
      .catch((e) => {
        err.textContent = String(e);
        save.disabled = false;
      });
  });
  // Test-first: Save unlocks only after a passing probe (attachKeyTest).
  row.append(attachKeyTest(family, key, url, save, err), save);
  form.append(row, err);
  return form;
}

function settingsAccountRow(family: string, entry: AccountEntry, index: number): HTMLElement {
  const row = document.createElement("div");
  row.className = "skey-account";
  const label = document.createElement("span");
  label.className = "skey-acct-label";
  label.textContent = entry.label || entry.email || entry.maskedKey;
  const masked = document.createElement("span");
  masked.className = "skey-acct-masked";
  masked.textContent = entry.maskedKey;
  const actions = document.createElement("span");
  actions.className = "skey-account-actions";
  if (entry.id) {
    const view = document.createElement("button");
    view.type = "button";
    view.className = "mini-btn";
    view.innerHTML = uiIcon("eye") + escapeHtml(t("settings.kvReveal"));
    view.addEventListener("click", () => {
      void revealSettingsKey(family, entry.id!, view, masked);
    });
    actions.append(view);
    const copy = document.createElement("button");
    copy.type = "button";
    copy.className = "mini-btn ok";
    copy.innerHTML = uiIcon("copy") + escapeHtml(t("settings.kvCopy"));
    copy.addEventListener("click", () => {
      void copySettingsKey(family, entry.id!, copy);
    });
    actions.append(copy);
  }
  const rename = document.createElement("button");
  rename.type = "button";
  rename.className = "mini-btn";
  rename.innerHTML = uiIcon("pencil") + escapeHtml(t("settings.acctRename"));
  rename.addEventListener("click", () => {
    void appPrompt({
      title: t("settings.acctRename"),
      initial: entry.label,
      confirmLabel: t("settings.save"),
    }).then(async (next) => {
      if (next == null || next === entry.label) return;
      try {
        await invoke("account_rename", { provider: family, index, label: next });
      } catch (e) {
        console.error("account rename failed:", e);
      }
      await refreshSettingsProviderKey(family);
      renderSettingsProviderKeys();
    });
  });
  const remove = document.createElement("button");
  remove.type = "button";
  remove.className = "mini-btn danger";
  remove.innerHTML = uiIcon("trash") + escapeHtml(t("settings.kvRemove"));
  remove.addEventListener("click", () => {
    void appConfirm({
      title: t("settings.acctRemoveConfirm", { name: entry.label || entry.maskedKey }),
      message: t("settings.acctRemoveHint"),
      confirmLabel: t("customize.acctDelete"),
    }).then(async (ok) => {
      if (!ok) return;
      try {
        await invoke("account_remove", { provider: family, index });
      } catch (e) {
        console.error("account remove failed:", e);
      }
      await refreshSettingsProviderKey(family);
      renderSettingsProviderKeys();
    });
  });
  actions.append(rename, remove);
  row.append(label, masked, actions);
  return row;
}

function settingsAccountEditor(family: string): HTMLElement {
  const form = document.createElement("div");
  form.className = "skey-editor skey-editor-account";
  const label = document.createElement("input");
  label.type = "text";
  label.className = "form-input";
  label.placeholder = t("settings.acctLabelPh");
  label.spellcheck = false;
  const key = document.createElement("input");
  key.type = "password";
  key.className = "form-input";
  key.placeholder = t("settings.keyPlaceholder");
  key.autocomplete = "off";
  key.spellcheck = false;
  form.append(label, key);
  let url: HTMLInputElement | null = null;
  if (RELAY_BASE_URL_FAMILIES.has(family)) {
    url = document.createElement("input");
    url.type = "text";
    url.className = "form-input";
    url.placeholder = "https://api.example.com";
    url.spellcheck = false;
    form.append(url);
  }
  const err = document.createElement("div");
  err.className = "skey-error";
  const row = document.createElement("div");
  row.className = "skey-editor-actions";
  const save = document.createElement("button");
  save.type = "button";
  save.className = "mini-btn primary";
  save.textContent = t("settings.save");
  save.addEventListener("click", () => {
    const value = key.value.trim();
    if (!value) {
      err.textContent = t("settings.keyRequired");
      return;
    }
    save.disabled = true;
    void invoke("account_add", {
      provider: family,
      label: label.value.trim(),
      apiKey: value,
      baseUrl: url?.value.trim() || null,
    })
      .then(async () => {
        skeyAddingAccount.delete(family);
        await refreshSettingsProviderKey(family);
        renderSettingsProviderKeys();
      })
      .catch((e) => {
        err.textContent = String(e);
        save.disabled = false;
      });
  });
  // Test-first: Save unlocks only after a passing probe (attachKeyTest).
  row.append(attachKeyTest(family, key, url, save, err), save);
  form.append(row, err);
  return form;
}

function renderKeyvault(rows: KeyVaultRow[]): void {
  lastVaultRows = rows;
  const root = document.querySelector<HTMLElement>("#keyvault-rows");
  if (!root) return;
  root.replaceChildren();
  if (!rows.length) {
    const empty = document.createElement("p");
    empty.className = "settings-note kv-empty";
    empty.textContent = t("settings.kvEmpty");
    root.append(empty);
    return;
  }
  for (const row of rows) {
    const item = document.createElement("div");
    item.className = "kv-item";
    item.dataset.kvId = row.id;

    const info = document.createElement("div");
    info.className = "kv-info";
    const icon = document.createElement("span");
    icon.className = "kv-icon";
    // Free-text service names resolve to a brand mark when one exists
    // (tavily, firecrawl); everything else falls back to the key glyph.
    icon.innerHTML = providerVisual(row.service.trim().toLowerCase())?.iconSvg ?? uiIcon("key");
    const service = document.createElement("span");
    service.className = "kv-service-badge";
    service.textContent = row.service;
    const label = document.createElement("span");
    label.className = "kv-label";
    label.textContent = row.label || row.note || row.service;
    const masked = document.createElement("code");
    masked.className = "kv-masked";
    masked.textContent = row.masked;
    info.append(icon, service, label, masked);
    if (row.note) {
      const note = document.createElement("span");
      note.className = "kv-note-text";
      note.textContent = row.note;
      info.append(note);
    }

    const actions = document.createElement("div");
    actions.className = "kv-actions";
    const reveal = document.createElement("button");
    reveal.type = "button";
    reveal.className = "mini-btn";
    reveal.dataset.kvReveal = row.id;
    reveal.innerHTML = uiIcon("eye") + escapeHtml(t("settings.kvReveal"));
    const noteBtn = document.createElement("button");
    noteBtn.type = "button";
    noteBtn.className = "mini-btn";
    noteBtn.dataset.kvNote = row.id;
    noteBtn.innerHTML = uiIcon("pencil") + escapeHtml(t("settings.kvEditNote"));
    const copy = document.createElement("button");
    copy.type = "button";
    copy.className = "mini-btn ok";
    copy.dataset.kvCopy = row.id;
    copy.innerHTML = uiIcon("copy") + escapeHtml(t("settings.kvCopy"));
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "mini-btn danger";
    remove.dataset.kvRemove = row.id;
    remove.innerHTML = uiIcon("trash") + escapeHtml(t("settings.kvRemove"));
    actions.append(reveal, noteBtn, copy, remove);
    item.append(info, actions);
    root.append(item);
  }
}

async function loadKeyvault(): Promise<void> {
  try {
    const [status, rows] = await Promise.all([
      invoke<VaultStatus>("keyvault_status"),
      invoke<KeyVaultRow[]>("keyvault_list"),
    ]);
    vaultStatus = status;
    renderVaultBar();
    renderKeyvault(rows);
  } catch (err) {
    const root = document.querySelector<HTMLElement>("#keyvault-rows");
    if (root) {
      root.replaceChildren();
      const error = document.createElement("p");
      error.className = "settings-note kv-error";
      error.textContent = `${t("settings.kvLoadFailed")}: ${String(err)}`;
      root.append(error);
    }
  }
}

async function addKeyvaultEntry(): Promise<void> {
  const service = document.querySelector<HTMLInputElement>("#kv-service");
  const key = document.querySelector<HTMLInputElement>("#kv-key");
  const note = document.querySelector<HTMLInputElement>("#kv-note");
  if (!service || !key || !service.value.trim() || !key.value.trim()) return;
  const button = document.querySelector<HTMLButtonElement>("#kv-add-btn");
  if (button) button.disabled = true;
  try {
    const rows = await invoke<KeyVaultRow[]>("keyvault_add", {
      service: service.value.trim(),
      label: service.value.trim(),
      key: key.value.trim(),
      note: note?.value.trim() ?? "",
    });
    service.value = "";
    key.value = "";
    if (note) note.value = "";
    renderKeyvault(rows);
  } catch (err) {
    const status = document.querySelector("#status");
    if (status) status.textContent = String(err);
  } finally {
    if (button) button.disabled = false;
  }
}

async function removeKeyvaultEntry(id: string): Promise<void> {
  const ok = await appConfirm({
    title: t("settings.kvRemoveTitle"),
    message: t("settings.kvRemoveBody"),
    confirmLabel: t("settings.kvRemove"),
    danger: true,
  });
  if (!ok) return;
  try {
    renderKeyvault(await invoke<KeyVaultRow[]>("keyvault_remove", { id }));
  } catch (err) {
    const status = document.querySelector("#status");
    if (status) status.textContent = String(err);
  }
}

async function copyKeyvaultEntry(id: string, button: HTMLButtonElement): Promise<void> {
  try {
    const rawKey = await invoke<string>("keyvault_copy", { id });
    await navigator.clipboard.writeText(rawKey);
    const old = button.textContent;
    button.innerHTML = uiIcon("copy") + escapeHtml(t("settings.kvCopied"));
    window.setTimeout(() => {
      button.innerHTML = uiIcon("copy") + escapeHtml(old || t("settings.kvCopy"));
    }, 1400);
  } catch (err) {
    if (String(err).includes("locked") && (await unlockVault())) {
      return copyKeyvaultEntry(id, button);
    }
    const status = document.querySelector("#status");
    if (status && !String(err).includes("locked")) status.textContent = String(err);
  }
}

function applyLocale(): void {
  config.locale = normalizeLocalePref(config.locale);
  try {
    localStorage.setItem("pane.locale", config.locale);
  } catch {
    /* non-fatal */
  }
  setActiveLocale(resolveLocale(config.locale));
  applyStaticI18n();
  renderOneNewApiSettings();
  if (document.body.classList.contains("settings-open") || IS_PANEL_FORM) void loadKeyvault();
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
  // Auth-center rows paint with t() at render time — re-render so a locale
  // switch retranslates them (static [data-i18n] is already handled above).
  if (IS_PANEL_FORM && authViewActive) renderAuthCenter();
  populatePinnedOptions();
  renderBuildInfo();
}

async function initSettings(): Promise<void> {
  config = await invoke<Config>("get_config");
  markConfigLoaded();
  config.localShortcuts = config.localShortcuts ?? {};
  pruneEmptyCardGroups();
  config.locale = normalizeLocalePref(config.locale);
  // Panel form: build the settings shell after config lands (controls read
  // initial values from it) but before applyLocale, so applyStaticI18n
  // translates the new DOM in the same pass.
  if (IS_PANEL_FORM) initPanelForm();
  document.querySelector<HTMLButtonElement>("#settings-save-all")?.addEventListener("click", () => {
    void applySettingsAndReload();
  });
  try {
    const sys = await invoke<string>("system_ui_locale");
    setSystemLocale(sys === "zh" || sys === "ru" ? sys : "en");
  } catch {
    // Dev / missing command — fall back to navigator.language.
  }
  applyLocale();
  if (["today", "yesterday", "last30"].includes(config.spendTab)) {
    spendTab = config.spendTab;
    rangeSelected = spendTab === "last30";
  }
  if (config.overviewTab === "5h" || config.overviewTab === "week" || config.overviewTab === "month") {
    overviewTab = config.overviewTab;
  }
  if ((OVERVIEW_CATEGORIES as readonly string[]).includes(config.overviewCategory)) {
    overviewCategory = config.overviewCategory;
  }
  if (config.overviewStyle !== "rings" && config.overviewStyle !== "bars") {
    config.overviewStyle = "rings";
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
    ["#notify-resetsoon", "notifyResetSoon"],
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

  // Spend-bolt tiers: built-in ladder or three custom daily thresholds (the
  // inputs hold M tokens; config stores raw token counts).
  const tierMode = document.querySelector<HTMLSelectElement>("#spend-tier-mode")!;
  const tierCustom = document.querySelector<HTMLElement>("#spend-tier-custom")!;
  const tierInputs = {
    medium: document.querySelector<HTMLInputElement>("#spend-tier-medium")!,
    high: document.querySelector<HTMLInputElement>("#spend-tier-high")!,
    max: document.querySelector<HTMLInputElement>("#spend-tier-max")!,
  };
  const paintTierInputs = () => {
    const current = config.spendIconTiers ?? { medium: 100_000_000, high: 250_000_000, max: 500_000_000 };
    tierInputs.medium.value = String(Math.round(current.medium / 1_000_000));
    tierInputs.high.value = String(Math.round(current.high / 1_000_000));
    tierInputs.max.value = String(Math.round(current.max / 1_000_000));
    tierCustom.hidden = tierMode.value !== "custom";
  };
  const saveTierInputs = () => {
    const medium = Math.max(1, Number(tierInputs.medium.value) || 100) * 1_000_000;
    const high = Math.max(medium, Number(tierInputs.high.value) || 250) * 1_000_000;
    const max = Math.max(high, Number(tierInputs.max.value) || 500) * 1_000_000;
    config.spendIconTiers = { medium, high, max };
    void patchConfig({ spendIconTiers: config.spendIconTiers }).then(renderAll);
  };
  tierMode.value = config.spendIconTiers ? "custom" : "builtin";
  paintTierInputs();
  tierMode.addEventListener("change", () => {
    if (tierMode.value === "builtin") {
      config.spendIconTiers = null;
      void patchConfig({ spendIconTiers: null }).then(renderAll);
      paintTierInputs();
    } else {
      paintTierInputs();
      saveTierInputs();
    }
  });
  for (const input of Object.values(tierInputs)) {
    input.addEventListener("change", () => {
      if (tierMode.value === "custom") saveTierInputs();
    });
  }

  // Experimental features: off = entries don't render; enabling asks first
  // (unstable/compat risk); disabling hot-unplugs back to native.
  const experimental = document.querySelector<HTMLInputElement>("#experimental-features")!;
  experimental.checked = config.experimentalFeatures === true;
  experimental.addEventListener("change", () => {
    if (experimental.checked) {
      void appConfirm({
        title: t("settings.experimental"),
        message: t("settings.experimentalWarn"),
        confirmLabel: t("dialog.ok"),
      }).then((ok) => {
        if (!ok) {
          experimental.checked = false;
          return;
        }
        config.experimentalFeatures = true;
        void patchConfig({ experimentalFeatures: true });
        applyExperimental();
      });
    } else {
      config.experimentalFeatures = false;
      void patchConfig({ experimentalFeatures: false });
      applyExperimental();
    }
  });
  applyExperimental();

  const catFull = document.querySelector<HTMLInputElement>("#overview-cat-full")!;
  catFull.checked = config.overviewCatFull === true;
  catFull.addEventListener("change", () => {
    config.overviewCatFull = catFull.checked;
    void patchConfig({ overviewCatFull: catFull.checked }).then(renderAll);
  });

  // Qoder CN daily check-in: off by default — it claims benefits with the
  // user's credential, so it only runs when explicitly enabled here.
  const qoderCheckin = document.querySelector<HTMLInputElement>("#qoder-checkin")!;
  qoderCheckin.checked = config.qoderCheckin === true;
  qoderCheckin.addEventListener("change", () => {
    config.qoderCheckin = qoderCheckin.checked;
    void patchConfig({ qoderCheckin: qoderCheckin.checked });
  });

  applyAppearance();
  const appearance = document.querySelector<HTMLSelectElement>("#appearance")!;
  appearance.value = config.appearance;
  appearance.addEventListener("change", () => {
    void patchConfig({ appearance: appearance.value as Config["appearance"] }).then(applyAppearance);
  });

  const uiFont = document.querySelector<HTMLInputElement>("#ui-font")!;
  uiFont.value = config.uiFont ?? "";
  if (config.uiFont) uiFont.style.fontFamily = fontStack(config.uiFont);
  uiFont.addEventListener("focus", () => {
    if (!fontMenu) openFontMenu();
  });
  uiFont.addEventListener("click", () => {
    if (!fontMenu) openFontMenu();
  });
  uiFont.addEventListener("input", () => {
    if (fontMenu) renderFontOptions(uiFont.value);
  });
  uiFont.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && fontMenu) {
      closeFontMenu();
      return;
    }
    if (!fontMenu) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!fontMenuList.length) return;
      const delta = e.key === "ArrowDown" ? 1 : -1;
      fontMenuFocused = (fontMenuFocused + delta + fontMenuList.length) % fontMenuList.length;
      renderFontOptions(uiFont.value);
      fontMenu?.children[fontMenuFocused]?.scrollIntoView({ block: "nearest" });
    } else if (e.key === "Enter" && fontMenuList[fontMenuFocused]) {
      e.preventDefault();
      selectFont(fontMenuList[fontMenuFocused]);
    }
  });
  uiFont.addEventListener("change", () => {
    void patchConfig({ uiFont: uiFont.value.trim() }).then(applyAppearance);
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

  const startupAnim = document.querySelector<HTMLInputElement>("#startup-anim")!;
  startupAnim.checked = config.startupAnimation !== false;
  startupAnim.addEventListener("change", () => {
    void patchConfig({ startupAnimation: startupAnim.checked });
  });
  document.querySelector("#splash-replay")?.addEventListener("click", () => {
    // Close the settings panel so the splash is actually visible. Mirrors
    // setSettings(false) — that helper is scoped to the boot wiring below.
    document.body.classList.remove("settings-open");
    document.querySelector("#settings-btn")?.classList.remove("active");
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    playSplash();
  });

  const jumpAnimation = document.querySelector<HTMLSelectElement>("#jump-animation")!;
  jumpAnimation.value = config.jumpAnimation === "instant" ? "instant" : "smooth";
  pendingJumpAnimation = null;
  jumpAnimation.addEventListener("change", () => {
    pendingJumpAnimation = jumpAnimation.value === "instant" ? "instant" : "smooth";
    setSettingsDirty(true);
  });

  const hideShare = document.querySelector<HTMLInputElement>("#hide-while-sharing")!;
  hideShare.checked = config.hideUsageWhileSharing === true;
  hideShare.addEventListener("change", () => {
    void patchConfig({ hideUsageWhileSharing: hideShare.checked }).catch(() => {});
    requestTraySync();
  });

  const showTrendEl = document.querySelector<HTMLInputElement>("#show-trend")!;
  showTrendEl.checked = config.showTrend === true;
  showTrendEl.addEventListener("change", () => {
    void patchConfig({ showTrend: showTrendEl.checked }).then(renderAll);
  });

  renderShortcutSettings();
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

  populatePinnedOptions();

  document.querySelector("#reset-all-settings")!.addEventListener("click", () => {
    void resetAllSettings();
  });

  document.querySelector("#kv-add-btn")?.addEventListener("click", () => {
    void addKeyvaultEntry();
  });
  document.querySelector("#keyvault-rows")?.addEventListener("click", (event) => {
    const target = event.target as HTMLElement;
    const reveal = target.closest<HTMLButtonElement>("[data-kv-reveal]");
    if (reveal?.dataset.kvReveal) {
      void revealKeyvaultEntry(reveal.dataset.kvReveal, reveal);
      return;
    }
    const noteBtn = target.closest<HTMLButtonElement>("[data-kv-note]");
    if (noteBtn?.dataset.kvNote) {
      void editKeyvaultNote(noteBtn.dataset.kvNote);
      return;
    }
    const copy = target.closest<HTMLButtonElement>("[data-kv-copy]");
    if (copy?.dataset.kvCopy) {
      void copyKeyvaultEntry(copy.dataset.kvCopy, copy);
      return;
    }
    const remove = target.closest<HTMLButtonElement>("[data-kv-remove]");
    if (remove?.dataset.kvRemove) void removeKeyvaultEntry(remove.dataset.kvRemove);
  });

  document.querySelector("#kv-vault-bar")?.addEventListener("click", (event) => {
    const btn = (event.target as HTMLElement).closest<HTMLButtonElement>("[data-kv-vault-action]");
    if (!btn) return;
    if (btn.dataset.kvVaultAction === "set") void setVaultPassword();
    else if (btn.dataset.kvVaultAction === "unlock") void unlockVault();
    else if (btn.dataset.kvVaultAction === "lock") void lockVault();
    else if (btn.dataset.kvVaultAction === "change") void changeVaultPassword();
    else if (btn.dataset.kvVaultAction === "questions") renderRecoveryEditor(null);
    else if (btn.dataset.kvVaultAction === "forgot") void renderRecoveryChallenge();
  });

  // Q&A recovery: add/remove rows, submit the editor, or submit the
  // forgot-password challenge. All inline under the vault bar.
  document.querySelector("#kv-recovery-panel")?.addEventListener("click", (event) => {
    const target = event.target as HTMLElement;
    const status = document.querySelector("#status");
    const panel = document.querySelector<HTMLElement>("#kv-recovery-panel");
    if (!panel) return;
    if (target.closest("[data-rc-add]")) {
      if (panel.querySelectorAll(".kv-recovery-row").length >= 5) {
        if (status) status.textContent = t("settings.kvRecoveryMax");
        return;
      }
      panel.querySelector(".kv-recovery-rows")?.append(recoveryEditorRow());
    } else if (target.closest("[data-rc-del]")) {
      const rowsEl = panel.querySelectorAll(".kv-recovery-row");
      if (rowsEl.length <= 1) return; // one is the floor
      void (async () => {
        // Dropping to a single question gets a nudge back to two: one
        // answer is far easier to guess or dig out of social profiles.
        if (rowsEl.length === 2) {
          const keep = await appConfirm({
            title: t("settings.kvRecoverySingleTitle"),
            message: t("settings.kvRecoverySingleBody"),
            confirmLabel: t("settings.kvRecoveryKeepOne"),
          });
          if (!keep) return;
        }
        rowsEl[rowsEl.length - 1].remove();
      })();
    } else if (target.closest("[data-rc-cancel]")) {
      recoveryPresetPw = null;
      panel.replaceChildren();
    } else if (target.closest("[data-rc-save]")) {
      void submitRecoveryEditor();
    } else if (target.closest("[data-rc-reset]")) {
      void submitRecoveryChallenge();
    }
  });

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
    telemetry: true,
    notifyAlmostOut: true,
    notifyCuttingClose: true,
    notifyWillRunOut: true,
    notifyResetSoon: true,
    spendTab: "today",
    overviewTab: "5h",
    overviewCategory: "coding",
    overviewStyle: "rings",
    categoryOverrides: {},
    spendMetric: "cost",
    spendGrouping: "tool",
    showUsed: false,
    showTrend: true,
    resetExact: false,
    timeFormat: "auto",
    layout: null,
    appearance: "dark",
    density: "compact",
    glassEffects: true,
    shortcut: "",
    categoryShortcut: "Shift+1",
    localShortcuts: {},
    proxy: { enabled: false, url: "" },
    showTotalSpend: true,
    reduceAnimations: false,
    jumpAnimation: "smooth",
    hideUsageWhileSharing: false,
    locale: "auto",
    windowForm: "floating",
    silentStart: false,
  }).catch(() => {});
  spendTab = "today";
  rangeSelected = false;
  rangeTab = "d30";
  overviewTab = "5h";
  overviewCategory = "coding";
  if (config.spendGrouping !== "tool" && config.spendGrouping !== "model") {
    config.spendGrouping = "tool";
  }
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
  setSelect("#timeformat", config.timeFormat);
  setSelect("#locale", config.locale);
  setCheck("#notify-almost", config.notifyAlmostOut);
  setCheck("#notify-close", config.notifyCuttingClose);
  setCheck("#notify-runout", config.notifyWillRunOut);
  setCheck("#notify-resetsoon", config.notifyResetSoon === true);
  setCheck("#telemetry", config.telemetry);
  setCheck("#hide-while-sharing", config.hideUsageWhileSharing === true);
  setCheck("#show-trend", config.showTrend === true);
  setCheck("#show-total-spend", config.showTotalSpend);
  setSelect("#appearance", config.appearance);
  setCheck("#density", config.density === "compact");
  setCheck("#glass", config.glassEffects !== false);
  setCheck("#reduce-anim", config.reduceAnimations === true);
  setSelect("#jump-animation", config.jumpAnimation === "instant" ? "instant" : "smooth");
  renderShortcutSettings();
  setCheck("#proxy-enabled", config.proxy?.enabled ?? false);
  setNum("#proxy-url", config.proxy?.url ?? "");
  const autostart = document.querySelector<HTMLInputElement>("#autostart");
  if (autostart) autostart.checked = true;
  // Panel-form controls (absent in the popover window).
  const silentStart = document.querySelector<HTMLInputElement>("#st-silent-start");
  if (silentStart) silentStart.checked = config.silentStart === true;
  const form = config.windowForm === "panel" ? "panel" : "floating";
  const formRadio = document.querySelector<HTMLInputElement>(
    `input[name="window-form"][value="${form}"]`,
  );
  if (formRadio) formRadio.checked = true;
  populatePinnedOptions();
}

// ---------------------------------------------------------------------------
// Panel form ("panel" window): the settings large panel
// ---------------------------------------------------------------------------
// Dual-form M2 (cc-switch borrow): the second window presents a settings
// panel instead of the floating dashboard. Layout re-creates cc-switch's
// four SettingsLayout primitives in plain HTML/CSS — a block title
// (15px/600) over a bordered card of hairline-divided rows, label left and
// control right (the .st-* classes in styles.css carry the numbers). The
// popover's own accordion settings page is untouched; the two coexist.
//
// Most controls are RE-PARENTED from the popover settings DOM rather than
// rebuilt: initSettings wires them by id, and moving an element keeps its
// listeners, so the key vault, shortcut capture, and font picker work here
// with no duplicated logic. The popover chrome they leave behind is
// display:none in this window; the main window's own DOM is unaffected.

function panelRow(labelKey: string, ...controls: HTMLElement[]): HTMLElement {
  const row = document.createElement("div");
  row.className = "setting-row";
  const label = document.createElement("label");
  label.dataset.i18n = labelKey;
  label.textContent = t(labelKey);
  row.append(label, ...controls);
  return row;
}

function panelSwitchRow(labelKey: string, tipKey: string, input: HTMLInputElement): HTMLElement {
  const row = document.createElement("div");
  row.className = "setting-row";
  const toggle = document.createElement("label");
  toggle.className = "toggle";
  toggle.title = t(tipKey);
  toggle.dataset.i18nTitle = tipKey;
  const span = document.createElement("span");
  span.dataset.i18n = labelKey;
  span.textContent = t(labelKey);
  toggle.append(input, span);
  row.append(toggle);
  return row;
}

function panelBlock(id: string, titleKey: string): { section: HTMLElement; body: HTMLElement } {
  const section = document.createElement("section");
  section.className = "st-block";
  section.id = id;
  const head = document.createElement("div");
  head.className = "st-block-head";
  const h2 = document.createElement("h2");
  h2.dataset.i18n = titleKey;
  h2.textContent = t(titleKey);
  head.append(h2);
  const body = document.createElement("div");
  body.className = "st-card";
  section.append(head, body);
  return { section, body };
}

async function panelCheckForUpdate(btn: HTMLButtonElement): Promise<void> {
  btn.disabled = true;
  btn.textContent = t("update.check");
  try {
    // Same command the footer's update entry uses; a hit goes through the
    // existing prompt + install_update chain.
    const v = await invoke<string | null>("check_update");
    if (v) {
      updateVersion = v;
      maybePromptUpdate(v);
      btn.textContent = t("update.to", { version: v });
    } else {
      btn.textContent = t("settings.updateLatest");
    }
  } catch (err) {
    btn.textContent = t("footer.updateFailed", { err: String(err) });
  } finally {
    btn.disabled = false;
    window.setTimeout(() => {
      if (!updateVersion) btn.textContent = t("settings.checkUpdate");
    }, 2600);
  }
}

function buildPanelShell(): void {
  if (document.querySelector("#panel-shell")) return;
  const shell = document.createElement("div");
  shell.id = "panel-shell";

  // Left icon nav — settings + auth center (M3); a Dashboard entry can
  // join here later.
  const nav = document.createElement("nav");
  nav.id = "panel-nav";
  const logo = document.createElement("span");
  logo.id = "panel-logo";
  logo.className = "app-logo";
  logo.innerHTML = `<img src="${paneLogo}" alt="Pane" />`;
  const settingsItem = document.createElement("button");
  settingsItem.type = "button";
  settingsItem.className = "panel-nav-item active";
  settingsItem.title = t("settings.title");
  settingsItem.dataset.i18nTitle = "settings.title";
  settingsItem.dataset.panelView = "settings";
  settingsItem.innerHTML = uiIcon("gear", t("settings.title"));
  const authItem = document.createElement("button");
  authItem.type = "button";
  authItem.className = "panel-nav-item";
  authItem.title = t("auth.title");
  authItem.dataset.i18nTitle = "auth.title";
  authItem.dataset.panelView = "auth";
  authItem.innerHTML = uiIcon("key", t("auth.title"));
  settingsItem.addEventListener("click", () => setPanelView("settings"));
  authItem.addEventListener("click", () => setPanelView("auth"));
  nav.append(logo, settingsItem, authItem);

  const main = document.createElement("div");
  main.id = "panel-main";
  const head = document.createElement("header");
  head.id = "panel-head";
  const h1 = document.createElement("h1");
  h1.dataset.i18n = "settings.title";
  h1.textContent = t("settings.title");
  const save = document.createElement("button");
  save.type = "button";
  save.className = "mini-btn settings-save-all";
  save.dataset.i18n = "settings.save";
  save.textContent = t("settings.save");
  save.disabled = true;
  save.addEventListener("click", () => void applySettingsAndReload());
  head.append(h1, save);
  const scroll = document.createElement("div");
  scroll.id = "panel-scroll";
  main.append(head, scroll);

  // Two switchable views share the scroll area (M3): settings and the auth
  // center. setPanelView toggles them and retitles the header.
  const settingsView = document.createElement("div");
  settingsView.id = "panel-view-settings";
  const authView = document.createElement("div");
  authView.id = "panel-view-auth";
  authView.hidden = true;
  authView.addEventListener("click", (e) => {
    void handleAuthCenterClick(e.target as HTMLElement);
  });
  scroll.append(settingsView, authView);

  // Magpie-style top directory bar: one pill per settings category, shared
  // vocabulary with the floating form's tabs. Switching pages only hides
  // blocks — every control stays mounted so its wiring never re-runs.
  const PANEL_PAGES: ReadonlyArray<readonly [string, string, string, UiIconName]> = [
    ["general", "st-general", "settings.tabGeneral", "gear"],
    ["usage", "st-keyvault", "settings.tabUsage", "key"],
    ["shortcuts", "st-shortcuts", "settings.tabShortcuts", "keyboard"],
    ["notifications", "st-notifications", "settings.tabNotifications", "bell"],
    ["privacy", "st-privacy", "settings.tabPrivacy", "shield"],
    ["network", "st-network", "settings.tabNetwork", "globe"],
    ["about", "st-about", "settings.tabAbout", "info"],
  ];
  const panelTabs = document.createElement("div");
  panelTabs.className = "settings-tabs panel-tabs";
  panelTabs.setAttribute("role", "tablist");
  panelTabs.setAttribute("aria-label", t("settings.title"));
  const activatePanelPage = (pageId: string) => {
    for (const [id, blockId] of PANEL_PAGES) {
      const block = document.querySelector<HTMLElement>(`#${blockId}`);
      if (block) block.hidden = id !== pageId;
    }
    panelTabs.querySelectorAll<HTMLButtonElement>("button").forEach((button) => {
      const active = button.dataset.settingsTab === pageId;
      button.classList.toggle("active", active);
      button.setAttribute("aria-selected", String(active));
    });
  };
  for (const [id, , labelKey, iconName] of PANEL_PAGES) {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.settingsTab = id;
    button.setAttribute("role", "tab");
    const label = document.createElement("span");
    label.dataset.i18n = labelKey;
    label.textContent = t(labelKey);
    button.innerHTML = uiIcon(iconName);
    button.append(label);
    button.addEventListener("click", () => activatePanelPage(id));
    panelTabs.append(button);
  }
  settingsView.append(panelTabs);

  // Rows/notes moved from the popover markup (each window owns its own
  // document — the floating settings keeps its copy).
  const moveToCard = (card: HTMLElement, ...selectors: string[]) => {
    for (const selector of selectors) {
      const el = document.querySelector<HTMLElement>(selector);
      if (!el) continue;
      const row = el.closest<HTMLElement>(".setting-row") ?? el;
      card.append(row);
    }
  };

  // --- General: re-parented popover rows + the new dual-form controls ----
  const general = panelBlock("st-general", "settings.general");
  const generalCard = general.body;
  const moveRow = (controlId: string) => {
    const row = document.querySelector(`#${controlId}`)?.closest<HTMLElement>(".setting-row");
    if (row) generalCard.append(row);
  };
  moveRow("locale");
  moveRow("appearance");
  moveRow("ui-font");
  moveRow("density");
  moveRow("glass");
  moveRow("reduce-anim");
  moveRow("jump-animation");
  moveRow("interval");
  moveRow("pinned");
  moveRow("timeformat");
  const shortcutEntry = document.createElement("button");
  shortcutEntry.type = "button";
  shortcutEntry.className = "mini-btn";
  shortcutEntry.dataset.i18n = "settings.manage";
  shortcutEntry.textContent = t("settings.manage");
  shortcutEntry.addEventListener("click", () => {
    document
      .querySelector("#st-shortcuts")
      ?.scrollIntoView({ behavior: reduceMotion() ? "auto" : scrollBehavior() });
  });
  generalCard.append(panelRow("settings.shortcutManage", shortcutEntry));
  moveRow("autostart");
  // Stored-only for now: the popover already launches hidden, so the flag
  // carries no extra behavior until the panel form grows auto-open-at-login
  // semantics. The tooltip says exactly that.
  const silentStart = document.createElement("input");
  silentStart.id = "st-silent-start";
  silentStart.type = "checkbox";
  silentStart.checked = config.silentStart === true;
  silentStart.addEventListener("change", () => {
    void patchConfig({ silentStart: silentStart.checked }).catch(() => {});
  });
  generalCard.append(panelSwitchRow("settings.silentStart", "settings.silentStartTip", silentStart));
  const formGroup = document.createElement("div");
  formGroup.className = "st-radio-group";
  for (const value of ["floating", "panel"] as const) {
    const label = document.createElement("label");
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = "window-form";
    radio.value = value;
    radio.checked = (config.windowForm ?? "floating") === value;
    const span = document.createElement("span");
    const key = value === "panel" ? "settings.windowFormPanel" : "settings.windowFormFloating";
    span.dataset.i18n = key;
    span.textContent = t(key);
    label.append(radio, span);
    formGroup.append(label);
  }
  formGroup.addEventListener("change", () => {
    const value =
      formGroup.querySelector<HTMLInputElement>("input:checked")?.value === "panel"
        ? "panel"
        : "floating";
    // set_window_form persists; the local mirror keeps the next patchConfig
    // snapshot from writing the old value back.
    config.windowForm = value;
    void invoke("set_window_form", { form: value }).catch(() => {});
    // Immediate effect: "panel" focuses (or opens) the panel window;
    // "floating" closes it — a self-close when picked from inside it.
    if (value === "panel") void invoke("open_panel_window").catch(() => {});
    else void invoke("close_panel_window").catch(() => {});
  });
  generalCard.append(panelRow("settings.windowForm", formGroup));
  settingsView.append(general.section);

  // --- Keys page: provider credentials + the MCP/search vault ------------
  // Two cards under one page id so the tab toggle keeps working: the
  // provider-keys card covers every API-key provider (the same catalog set
  // Customize manages), the second carries the popover's generic vault UI
  // for MCP/search keys.
  const keyPage = document.createElement("div");
  keyPage.id = "st-keyvault";
  // Master-password entry lives at the page's top-right (the vault bar with
  // add/switch/unlock + the Q&A recovery panel); the MCP card below keeps
  // only its key rows.
  const kvHead = document.createElement("div");
  kvHead.className = "st-kv-head";
  for (const selector of ["#kv-vault-bar", "#kv-recovery-panel"]) {
    const el = document.querySelector<HTMLElement>(selector);
    if (el) kvHead.append(el);
  }
  const providerKeys = panelBlock("st-kv-providers", "settings.keysProviders");
  const pkPad = document.createElement("div");
  pkPad.className = "st-pad";
  const pkNote = document.createElement("p");
  pkNote.className = "settings-note st-kv-note";
  pkNote.dataset.i18n = "settings.keysProvidersHint";
  pkNote.textContent = t("settings.keysProvidersHint");
  const pkRows = document.createElement("div");
  pkRows.id = "settings-keys-rows";
  pkRows.className = "skey-list";
  pkPad.append(pkNote, pkRows);
  providerKeys.body.append(pkPad);
  const mcpKeys = panelBlock("st-kv-mcp", "settings.keysMcp");
  const kvPad = document.createElement("div");
  kvPad.className = "st-pad";
  for (const selector of ["#keyvault-rows", ".kv-add-row", ".kv-hint"]) {
    const el = document.querySelector<HTMLElement>(selector);
    if (el) kvPad.append(el);
  }
  mcpKeys.body.append(kvPad);
  keyPage.append(kvHead, providerKeys.section, mcpKeys.section);
  settingsView.append(keyPage);

  // --- Shortcuts: wake + category + local bindings, moved in whole -------
  const shortcuts = panelBlock("st-shortcuts", "settings.shortcutManage");
  const scPad = document.createElement("div");
  scPad.className = "st-pad";
  const shortcutInner = document.querySelector("#local-shortcuts")?.closest(".acc-inner");
  if (shortcutInner) {
    while (shortcutInner.firstChild) scPad.append(shortcutInner.firstChild as HTMLElement);
  }
  shortcuts.body.append(scPad);
  settingsView.append(shortcuts.section);

  // --- About ---------------------------------------------------------------
  const about = panelBlock("st-about", "settings.about");
  const versionValue = document.createElement("span");
  versionValue.id = "st-version";
  versionValue.className = "st-version-foot";
  versionValue.textContent = "…";
  const checkBtn = document.createElement("button");
  checkBtn.type = "button";
  checkBtn.className = "mini-btn";
  checkBtn.dataset.i18n = "settings.checkUpdate";
  checkBtn.textContent = t("settings.checkUpdate");
  checkBtn.addEventListener("click", () => {
    void panelCheckForUpdate(checkBtn);
  });
  about.body.append(panelRow("settings.update", checkBtn));
  const changelogBtn = document.querySelector<HTMLElement>("#changelog-btn");
  if (changelogBtn) {
    const row = document.createElement("div");
    row.className = "setting-row";
    row.append(changelogBtn);
    about.body.append(row);
  }
  const resetBtn = document.querySelector<HTMLElement>("#reset-all-settings");
  if (resetBtn) {
    const row = document.createElement("div");
    row.className = "setting-row";
    row.append(resetBtn);
    about.body.append(row);
  }
  const ack = document.createElement("p");
  ack.className = "settings-note st-pad";
  ack.dataset.i18n = "settings.ackBody";
  ack.textContent = t("settings.ackBody");
  about.body.append(ack);
  // Version is a quiet footer in the blank area under the last rows —
  // the user asked for it not to be a standalone setting row.
  about.body.append(versionValue);
  settingsView.append(about.section);

  // --- Notifications / Privacy / Network: re-parented 1:1 from the popover
  // markup, so the panel window carries the same seven pages as the
  // floating sheet instead of a subset.
  const notifications = panelBlock("st-notifications", "settings.notifications");
  moveToCard(
    notifications.body,
    '[data-i18n="settings.notifyNote"]',
    "#notify-almost",
    "#notify-close",
    "#notify-runout",
    "#notify-resetsoon",
  );
  settingsView.append(notifications.section);

  const privacy = panelBlock("st-privacy", "settings.privacy");
  moveToCard(
    privacy.body,
    '[data-i18n="settings.privacyNote"]',
    "#telemetry",
    "#hide-while-sharing",
    "#show-trend",
  );
  settingsView.append(privacy.section);

  const network = panelBlock("st-network", "settings.network");
  moveToCard(network.body, "#proxy-enabled", "#proxy-url", '[data-i18n="settings.networkNote"]');
  settingsView.append(network.section);

  // About also takes the entries the floating sheet keeps outside its
  // groups: changelog + reset.
  moveToCard(about.body, "#changelog-btn", "#reset-all-settings");

  // General rows that joined the floating sheet after this shell was first
  // written — keep the two forms at parity.
  moveRow("startup-anim");
  moveRow("show-total-spend");
  moveRow("spend-tier-mode");
  moveRow("spend-tier-custom");
  moveRow("experimental-features");
  moveRow("overview-cat-full");

  activatePanelPage("general");

  shell.append(nav, main);
  document.body.appendChild(shell);
}

function initPanelForm(): void {
  buildPanelShell();
  // No accordion gating here — the vault is visible as soon as the panel
  // opens (the popover only loads it while its settings page is open).
  void loadKeyvault();
  void loadSettingsProviderKeys();
  void getVersion().then((v) => {
    const el = document.querySelector("#st-version");
    if (el) el.textContent = `v${v} · build ${__BUILD_STAMP__}`;
  });
}

// ---------------------------------------------------------------------------
// Auth center (panel window, M3 — cc-switch borrow)
// ---------------------------------------------------------------------------
// The second panel view: account rows grouped by family (antigravity /
// codex / copilot / cursor / grok-as-xAI), laid out like cc-switch's
// ManagedAccountsGroup — avatar + name + second line on the left, a quota
// column on the right, a ⋯ menu at the edge. All data comes from existing
// chains: auth_center_list shapes the stores the accounts modules already
// own, the quota column reads the same snapshot cache the dashboard cards
// use, and add/remove/relogin ride the existing OAuth / capture / import /
// account_remove commands — no new credential logic lives here.

/// One auth_center_list row.
interface AuthAccountRow {
  id: string;
  label: string;
  email?: string;
  maskedKey: string;
  capturedAt?: number;
  kind: "slot" | "account" | "oauth";
}

interface AuthFamilyGroup {
  family: string;
  accounts: AuthAccountRow[];
}

let authGroups: AuthFamilyGroup[] | null = null;
let authViewActive = false;

/// A device-code login between "Add account" and completion/cancel,
/// rendered as the four-state block inside the family card (starting →
/// polling → success | error), mirroring GroupLoginFlow's shape.
interface AuthFlow {
  phase: "starting" | "polling" | "browser" | "success" | "error";
  deviceAuthId: string;
  /** Codex browser login id (loopback flow); absent for device-code flows. */
  loginId?: string;
  userCode: string;
  error: string | null;
  label: string | null;
  timer?: number;
}
const authFlows = new Map<string, AuthFlow>();

function stopAuthFlow(family: string): void {
  const flow = authFlows.get(family);
  if (flow?.timer !== undefined) window.clearInterval(flow.timer);
  if (flow?.loginId) {
    // Every browser-login family (codex/kiro) has its own cancel command.
    void invoke(`${family}_login_cancel`, { loginId: flow.loginId }).catch(() => {});
  }
}

function authFlowBusy(): boolean {
  for (const flow of authFlows.values()) {
    if (flow.phase === "starting" || flow.phase === "polling" || flow.phase === "browser") return true;
  }
  return false;
}

function setPanelView(view: "settings" | "auth"): void {
  authViewActive = view === "auth";
  document.querySelectorAll<HTMLElement>("#panel-nav .panel-nav-item").forEach((el) => {
    el.classList.toggle("active", el.dataset.panelView === view);
  });
  const h1 = document.querySelector<HTMLElement>("#panel-head h1");
  if (h1) {
    h1.dataset.i18n = view === "auth" ? "auth.title" : "settings.title";
    h1.textContent = t(h1.dataset.i18n);
  }
  const settingsView = document.querySelector<HTMLElement>("#panel-view-settings");
  const authView = document.querySelector<HTMLElement>("#panel-view-auth");
  if (settingsView) settingsView.hidden = view !== "settings";
  if (authView) authView.hidden = view !== "auth";
  if (view === "auth") void loadAuthCenter();
}

async function loadAuthCenter(): Promise<void> {
  const container = document.querySelector<HTMLElement>("#panel-view-auth");
  if (!container) return;
  if (!authGroups) {
    container.innerHTML = `<div class="auth-empty">${escapeHtml(t("auth.loading"))}</div>`;
  }
  try {
    authGroups = await invoke<AuthFamilyGroup[]>("auth_center_list");
  } catch (err) {
    container.innerHTML = `<div class="auth-empty"><span>${escapeHtml(t("auth.loadFailed", { err: String(err) }))}</span><button class="mini-btn" type="button" data-auth-retry>${escapeHtml(t("auth.retry"))}</button></div>`;
    return;
  }
  // The panel skips the boot fetch, so the first visit can precede the
  // backend loop's first broadcast — seed the quota column from the
  // persisted snapshot cache, exactly like the popover's first paint.
  if (!lastSnapshots.length) {
    try {
      const cached = await invoke<Snapshot[]>("cached_usage");
      if (cached.length && !lastSnapshots.length) lastSnapshots = cached;
    } catch {
      // No cache: the quota column shows its dash until the first tick.
    }
  }
  renderAuthCenter();
}

function renderAuthCenter(): void {
  const container = document.querySelector<HTMLElement>("#panel-view-auth");
  if (!container || !authGroups) return;
  container.innerHTML = authGroups.map(authGroupHtml).join("");
}

function authRowName(family: string, acct: AuthAccountRow): string {
  return (
    acct.label.trim() ||
    acct.email ||
    acct.id.split("@")[1]?.slice(0, 8) ||
    providerDisplayName(family)
  );
}

/// The snapshot behind a row: the account's own card id, which for OAuth
/// single-login families IS the bare family id.
function authRowSnapshot(acct: AuthAccountRow): Snapshot | undefined {
  return lastSnapshots.find((s) => s.id === acct.id);
}

/// Same sign-in-failure classification the ⚠ Outdated tooltip uses.
function authNeedsReauth(snap: Snapshot | undefined): string | null {
  const err = (snap?.error ?? snap?.warning ?? "").trim();
  if (!err) return null;
  return /http 40[13]|invalid_grant|expired|no refresh token|sign[- ]?in|log ?in|credentials/i.test(err)
    ? err
    : null;
}

function authQuotaHtml(snap: Snapshot | undefined, reauth: string | null): string {
  if (reauth || !snap || snap.status !== "ok") {
    return `<span class="auth-quota dim">—</span>`;
  }
  const bar = snap.metrics.find((m) => m.kind === "progress" && m.used_percent !== null);
  if (bar) {
    return `<span class="auth-quota" title="${escapeHtml(bar.detail ?? "")}">${escapeHtml(displayMetricLabel(bar.label))} ${Math.round(bar.used_percent!)}%</span>`;
  }
  const text = snap.metrics.find((m) => m.value);
  if (text) {
    return `<span class="auth-quota">${escapeHtml(displayMetricLabel(text.label))} ${escapeHtml(text.value!)}</span>`;
  }
  return `<span class="auth-quota dim">—</span>`;
}

function authRowHtml(family: string, acct: AuthAccountRow, index: number): string {
  const snap = authRowSnapshot(acct);
  const reauth = authNeedsReauth(snap);
  const name = authRowName(family, acct);
  let sub: string;
  if (reauth) {
    // cc-switch swaps the second line for the reauth reason.
    sub = `<div class="auth-sub reauth" title="${escapeHtml(reauth)}">${escapeHtml(reauth)}</div>`;
  } else {
    const details: string[] = [];
    if (acct.capturedAt) {
      const date = new Date(acct.capturedAt * 1000).toLocaleDateString(localeTag(), {
        year: "numeric",
        month: "short",
        day: "numeric",
      });
      details.push(t("auth.captured", { date }));
    }
    details.push(acct.maskedKey);
    details.push(snap ? t("auth.monitoring") : t("auth.noCard"));
    sub = `<div class="auth-sub">${escapeHtml(details.join(" · "))}</div>`;
  }
  return `<div class="auth-row" data-auth-row="${escapeHtml(acct.id)}">
    <span class="auth-avatar" aria-hidden="true">${escapeHtml(name.charAt(0) || "?")}</span>
    <div class="auth-id">
      <div class="auth-name-line">
        <span class="auth-name" title="${escapeHtml(name)}">${escapeHtml(name)}</span>
        ${reauth ? `<span class="auth-pill reauth">${escapeHtml(t("auth.needsReauth"))}</span>` : ""}
      </div>
      ${sub}
    </div>
    ${authQuotaHtml(snap, reauth)}
    <button class="mini-btn" type="button" data-auth-menu="${escapeHtml(family)}|${index}" title="${escapeHtml(t("auth.more"))}" aria-label="${escapeHtml(t("auth.more"))}">⋯</button>
  </div>`;
}

function authFlowHtml(family: string): string {
  const flow = authFlows.get(family);
  if (!flow) return "";
  if (flow.phase === "error") {
    return `<div class="auth-flow err" role="status">
      <div class="auth-flow-head"><span>${escapeHtml(t("auth.flow.failed", { err: flow.error ?? "" }))}</span>
        <button class="mini-btn" type="button" data-auth-flow-retry="${escapeHtml(family)}">${escapeHtml(t("auth.retry"))}</button>
        <button class="mini-btn" type="button" data-auth-flow-close="${escapeHtml(family)}">${escapeHtml(t("auth.flow.close"))}</button></div>
    </div>`;
  }
  if (flow.phase === "success") {
    return `<div class="auth-flow" role="status">
      <div class="auth-flow-head"><span>${escapeHtml(t("auth.flow.success", { label: flow.label ?? "" }))}</span></div>
    </div>`;
  }
  if (flow.phase === "browser") {
    // Codex loopback login: the browser tab owns the interaction, this card
    // just watches the callback.
    return `<div class="auth-flow" role="status">
      <div class="auth-flow-head"><span>${escapeHtml(t("auth.flow.browser"))}</span>
        <button class="mini-btn" type="button" data-auth-flow-cancel="${escapeHtml(family)}">${escapeHtml(t("auth.flow.cancel"))}</button></div>
      <div class="auth-note">${escapeHtml(t("auth.flow.browserNote"))}</div>
    </div>`;
  }
  const codeRow = flow.userCode
    ? `<div class="auth-flow-row"><span>${escapeHtml(t("auth.flow.enterCode"))}</span>
        <code class="auth-flow-code">${escapeHtml(flow.userCode)}</code>
        <button class="mini-btn" type="button" data-auth-flow-copy="${escapeHtml(family)}">${escapeHtml(t("auth.flow.copy"))}</button></div>
       <div class="auth-note">${escapeHtml(t("auth.flow.waiting"))}</div>`
    : "";
  return `<div class="auth-flow" role="status">
    <div class="auth-flow-head"><span>${escapeHtml(t("auth.flow.starting"))}</span>
      <button class="mini-btn" type="button" data-auth-flow-cancel="${escapeHtml(family)}">${escapeHtml(t("auth.flow.cancel"))}</button></div>
    ${codeRow}
  </div>`;
}

function authGroupHtml(group: AuthFamilyGroup): string {
  const family = group.family;
  const name = providerDisplayName(family);
  const icon = providerVisual(family)?.iconSvg ?? "";
  // Every OAuth family (codex/copilot/grok) accumulates one account per
  // completed sign-in in its own store, so they all behave like the
  // multi-account families here: rows + an "add account" (login) button.
  const rows = group.accounts.map((acct, i) => authRowHtml(family, acct, i)).join("");
  const empty =
    group.accounts.length === 0
      ? `<div class="auth-empty"><span>${escapeHtml(t("auth.empty"))}</span>
        <button class="mini-btn" type="button" data-auth-add="${escapeHtml(family)}">${escapeHtml(t("auth.addAccount"))}</button></div>`
      : "";
  const addBtn =
    group.accounts.length > 0
      ? `<button class="mini-btn" type="button" data-auth-add="${escapeHtml(family)}">${escapeHtml(t("auth.addAccount"))}</button>`
      : "";
  return `<section class="st-block" data-auth-group="${escapeHtml(family)}">
    <div class="st-block-head auth-group-head">
      <span class="auth-group-icon" aria-hidden="true">${icon}</span>
      <h2>${escapeHtml(name)}</h2>
      ${addBtn}
    </div>
    <div class="st-card">${rows}${empty}${authFlowHtml(family)}</div>
  </section>`;
}

function handleAuthCenterClick(target: HTMLElement): void {
  if (target.closest("[data-auth-retry]")) {
    authGroups = null;
    void loadAuthCenter();
    return;
  }
  const add = target.closest<HTMLElement>("[data-auth-add]");
  if (add) {
    authAddAccount(add.dataset.authAdd!);
    return;
  }
  const menu = target.closest<HTMLElement>("[data-auth-menu]");
  if (menu) {
    const [family, index] = menu.dataset.authMenu!.split("|");
    openAuthRowMenu(family, Number(index), menu);
    return;
  }
  const cancel = target.closest<HTMLElement>("[data-auth-flow-cancel]");
  if (cancel) {
    const family = cancel.dataset.authFlowCancel!;
    stopAuthFlow(family);
    authFlows.delete(family);
    renderAuthCenter();
    return;
  }
  const retry = target.closest<HTMLElement>("[data-auth-flow-retry]");
  if (retry) {
    void startAuthOauthFlow(retry.dataset.authFlowRetry!);
    return;
  }
  const close = target.closest<HTMLElement>("[data-auth-flow-close]");
  if (close) {
    authFlows.delete(close.dataset.authFlowClose!);
    renderAuthCenter();
    return;
  }
  const copy = target.closest<HTMLElement>("[data-auth-flow-copy]");
  if (copy) {
    const code = authFlows.get(copy.dataset.authFlowCopy!)?.userCode;
    if (code) {
      void navigator.clipboard.writeText(code).then(() => {
        copy.textContent = t("auth.flow.copied");
      });
    }
  }
}

/// "Add account" per family, riding the existing login chains — nothing
/// here re-implements an OAuth flow.
function authAddAccount(family: string): void {
  if (authFlowBusy()) return;
  if (family === "antigravity") {
    void doAntigravityCapture(family).then(() => loadAuthCenter());
    return;
  }
  if (family === "cursor") {
    // The existing three-tab dialog (OAuth / token / JSON); its success
    // path forces a usage refresh, whose broadcast reloads this view.
    openCursorAccountDialog();
    return;
  }
  if (OAUTH_PROVIDERS.has(family) || BROWSER_LOGIN_PROVIDERS.has(family)) {
    void startAuthOauthFlow(family);
  }
}

/// Login entry point for the sign-in families. Browser-PKCE families
/// (codex/kiro) open the auth URL and poll the backend's local-callback
/// state; device-code families (copilot/grok, and codex's fallback)
/// ride oauth_start/oauth_poll.
async function startAuthOauthFlow(family: string): Promise<void> {
  stopAuthFlow(family);
  if (BROWSER_LOGIN_PROVIDERS.has(family)) {
    // Browser PKCE first (the IDEs' own loopback flows). Codex's CLI owns
    // port 1455 while it runs, so a busy port falls through to device code;
    // kiro has no device-code ladder and stops on failure.
    authFlows.set(family, { phase: "starting", deviceAuthId: "", userCode: "", error: null, label: null });
    renderAuthCenter();
    try {
      const started = await invoke<{ loginId: string; authUrl: string }>(`${family}_login_start`);
      void invoke("open_link", { url: started.authUrl }).catch(() => {});
      const flow: AuthFlow = {
        phase: "browser",
        deviceAuthId: "",
        loginId: started.loginId,
        userCode: "",
        error: null,
        label: null,
      };
      authFlows.set(family, flow);
      renderAuthCenter();
      flow.timer = window.setInterval(() => void pollBrowserLogin(family), 2000);
      return;
    } catch (err) {
      if (family !== "codex") {
        authFlows.set(family, { phase: "error", deviceAuthId: "", userCode: "", error: String(err), label: null });
        renderAuthCenter();
        return;
      }
      // Port busy or unavailable — fall through to the device-code flow.
    }
  }
  authFlows.set(family, { phase: "starting", deviceAuthId: "", userCode: "", error: null, label: null });
  renderAuthCenter();
  let started: { device_auth_id: string; user_code: string; verify_url: string };
  try {
    started = await invoke("oauth_start", { provider: family });
  } catch (err) {
    authFlows.set(family, { phase: "error", deviceAuthId: "", userCode: "", error: String(err), label: null });
    renderAuthCenter();
    return;
  }
  void invoke("open_link", { url: started.verify_url }).catch(() => {});
  const flow: AuthFlow = {
    phase: "polling",
    deviceAuthId: started.device_auth_id,
    userCode: started.user_code,
    error: null,
    label: null,
  };
  authFlows.set(family, flow);
  renderAuthCenter();
  flow.timer = window.setInterval(() => void pollAuthOauth(family), 3000);
  void pollAuthOauth(family);
}

/// One poll tick for a browser login (codex/kiro): the callback lands in
/// the backend, this just collects the outcome.
async function pollBrowserLogin(family: string): Promise<void> {
  const flow = authFlows.get(family);
  if (!flow || flow.phase !== "browser" || !flow.loginId) return;
  let r: { done: boolean; label: string | null; error: string | null };
  try {
    r = await invoke(`${family}_login_poll`, { loginId: flow.loginId });
  } catch (err) {
    stopAuthFlow(family);
    flow.phase = "error";
    flow.error = String(err);
    renderAuthCenter();
    return;
  }
  if (!r.done && !r.error) return; // still waiting for the callback
  stopAuthFlow(family);
  if (r.error) {
    flow.phase = "error";
    flow.error = r.error;
    renderAuthCenter();
    return;
  }
  flow.phase = "success";
  flow.label = r.label;
  renderAuthCenter();
  credStatusCache.delete(family);
  window.setTimeout(() => {
    if (authFlows.get(family) === flow) authFlows.delete(family);
    void loadAuthCenter();
  }, 1600);
  void forceUsageRefreshAttempt(false).then(requestTraySync);
}

/// One poll tick; the backend paces itself against the server-asked
/// interval, so a fixed 3 s timer here is safe (same as the drawer's).
async function pollAuthOauth(family: string): Promise<void> {
  const flow = authFlows.get(family);
  if (!flow || flow.phase !== "polling" || !flow.deviceAuthId) return;
  let r: { done: boolean; label: string | null; error: string | null };
  try {
    r = await invoke("oauth_poll", { provider: family, deviceAuthId: flow.deviceAuthId });
  } catch (err) {
    stopAuthFlow(family);
    flow.phase = "error";
    flow.error = String(err);
    renderAuthCenter();
    return;
  }
  if (!r.done && !r.error) return; // still waiting for the user
  stopAuthFlow(family);
  if (r.error) {
    flow.phase = "error";
    flow.error = r.error;
    renderAuthCenter();
    return;
  }
  flow.phase = "success";
  flow.label = r.label;
  renderAuthCenter();
  credStatusCache.delete(family);
  // Brief success beat, then the fresh account row replaces the block.
  window.setTimeout(() => {
    if (authFlows.get(family) === flow) authFlows.delete(family);
    void loadAuthCenter();
  }, 1600);
  void forceUsageRefreshAttempt(false).then(requestTraySync);
}

/// The row's ⋯ menu, styled on the dashboard's group menu. Removing an
/// account row drops just that login; signing in another account is the
/// group's "add account" button. Copy-token is deliberately absent —
/// account keys never leave the backend (only the masked form does), so
/// there is no existing copy chain to ride. Parallel families have no
/// default concept and OAuth families are per-login accounts, so "set
/// default" has no semantics to call either.
function openAuthRowMenu(family: string, index: number, anchor: HTMLElement): void {
  document.querySelector(".group-menu-overlay")?.remove();
  const acct = authGroups?.find((g) => g.family === family)?.accounts[index];
  if (!acct) return;
  const name = authRowName(family, acct);
  const overlay = document.createElement("div");
  overlay.className = "group-menu-overlay";
  overlay.innerHTML = `<div class="group-menu" role="menu">
    <div class="group-menu-title">${escapeHtml(name)}</div>
    <button class="group-menu-item danger" type="button" data-auth-menu-remove="${escapeHtml(family)}|${index}"><span class="group-menu-check">×</span>${escapeHtml(t("auth.remove"))}</button>
  </div>`;
  const close = () => {
    overlay.remove();
    document.removeEventListener("keydown", onKey, true);
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
    }
  };
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) {
      close();
      return;
    }
    const remove = (e.target as HTMLElement).closest<HTMLElement>("[data-auth-menu-remove]");
    if (remove) {
      const [fam, idx] = remove.dataset.authMenuRemove!.split("|");
      close();
      void authRemoveAccount(fam, Number(idx));
    }
  });
  document.addEventListener("keydown", onKey, true);
  document.body.appendChild(overlay);
  // Anchor to the ⋯ button, opening below-right and flipping up at the
  // window's bottom edge, same recipe as the card context menu.
  const menu = overlay.querySelector<HTMLElement>(".group-menu")!;
  const rect = anchor.getBoundingClientRect();
  const belowY = rect.bottom + 4;
  const aboveY = rect.top - menu.offsetHeight - 4;
  const desiredY = belowY + menu.offsetHeight <= window.innerHeight - 8 ? belowY : aboveY;
  menu.style.left = `${Math.max(8, Math.min(rect.right - menu.offsetWidth, window.innerWidth - menu.offsetWidth - 8))}px`;
  menu.style.top = `${Math.max(8, desiredY)}px`;
}

/// Remove rides the existing account_remove chain for every family (slot,
/// import, and login-store families alike).
async function authRemoveAccount(family: string, index: number): Promise<void> {
  const acct = authGroups?.find((g) => g.family === family)?.accounts[index];
  if (!acct) return;
  const name = authRowName(family, acct);
  const ok = await appConfirm({
    title: t("auth.removeTitle"),
    message: t("auth.removeBody", { label: name }),
    confirmLabel: t("auth.remove").replace(/…$/, ""),
    danger: true,
  });
  if (!ok) return;
  const status = document.querySelector("#status");
  try {
    await invoke("account_remove", { provider: family, index });
    credStatusCache.delete(family);
    if (status) status.textContent = t("auth.removed");
    await loadAuthCenter();
    void forceUsageRefreshAttempt(false).then(requestTraySync);
  } catch (err) {
    if (status) status.textContent = t("auth.removeFailed", { err: String(err) });
  }
}

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Startup splash — plays the first time the popover is shown after each OS
// boot (config.startupAnimation off = never). The gate lives in config's
// lastStartupBootId; launching hidden to the tray must not burn the one
// showing, so the trigger is "popover shown", not "webview loaded".
// ---------------------------------------------------------------------------

let splashGateChecked = false;
let splashTimer: number | undefined;
let splashTl: ReturnType<typeof gsap.timeline> | undefined;
let markConfigLoaded: () => void = () => {};
const configLoaded = new Promise<void>((resolve) => {
  markConfigLoaded = resolve;
});

function hideSplash(): void {
  window.clearTimeout(splashTimer);
  splashTl?.pause();
  document.querySelector("#splash")?.classList.add("hide");
}

function playSplash(): void {
  const splash = document.querySelector<HTMLElement>("#splash");
  if (!splash) return;
  window.clearTimeout(splashTimer);
  splashTl?.kill();
  splash.classList.remove("hide");

  // Brand beat: tile settles in, the quota arc draws itself, the needle
  // sweeps to its reading, ripples breathe out, then the wordmark rises.
  const tl = gsap.timeline({ onComplete: hideSplash });
  splashTl = tl;
  tl.fromTo(
    ".sa-mark",
    { scale: 0.8, opacity: 0, transformOrigin: "50% 50%" },
    { scale: 1, opacity: 1, duration: 0.6, ease: "power3.out" },
    0,
  )
    .fromTo(
      "#sa-arc",
      { strokeDashoffset: 360 },
      { strokeDashoffset: 145, duration: 1.0, ease: "power2.inOut" },
      0.18,
    )
    .fromTo(
      "#sa-needle",
      { rotation: -70, opacity: 0, svgOrigin: "256 256" },
      { rotation: 0, opacity: 1, duration: 0.7, ease: "power2.out" },
      0.5,
    )
    .fromTo(
      "#sa-hub",
      { scale: 0, svgOrigin: "256 256" },
      { scale: 1, duration: 0.5, ease: "back.out(2.5)" },
      0.62,
    )
    .fromTo(
      ".sa-rip",
      { scale: 0.7, opacity: 0.55 },
      { scale: 1.7, opacity: 0, duration: 1.2, ease: "power2.out", stagger: 0.25 },
      0.55,
    )
    .fromTo(
      ".sa-word span",
      { y: 16, opacity: 0 },
      { y: 0, opacity: 1, duration: 0.55, ease: "power3.out", stagger: 0.06 },
      0.78,
    )
    .fromTo(
      ".sa-sub",
      { y: 8, opacity: 0 },
      { y: 0, opacity: 1, duration: 0.5, ease: "power2.out" },
      1.05,
    )
    .to(
      ".sa-col",
      { scale: 1.05, opacity: 0, duration: 0.45, ease: "power2.in" },
      2.45,
    );

  if (reduceMotion()) {
    // No motion, but the composed frame still shows: park just before the
    // exit tween and let the normal dismiss path run.
    tl.pause();
    tl.progress(2.45 / tl.duration());
    splashTimer = window.setTimeout(hideSplash, 1200);
    return;
  }
  // Backstop for a timeline stalled by boot-time tab throttling.
  splashTimer = window.setTimeout(hideSplash, 4500);
}

async function maybePlayStartupAnimation(): Promise<void> {
  if (splashGateChecked || IS_PANEL_FORM) return;
  // popover-shown can race the config load — the gate value lives in config.
  await configLoaded;
  if (config.startupAnimation === false) return;
  splashGateChecked = true;
  const bootId = await invoke<number | null>("get_boot_id").catch(() => null);
  // boot_id is derived (wall now − uptime) and truncated to whole seconds on
  // both ends, so an NTP step right after boot jitters it by ~1s. Compare with
  // a tolerance or the splash replays inside the same boot session.
  const seenBoot = config.lastStartupBootId;
  if (bootId != null && seenBoot != null && Math.abs(bootId - seenBoot) <= 60) return;
  if (bootId != null) {
    config.lastStartupBootId = bootId;
    void patchConfig({ lastStartupBootId: bootId });
  }
  playSplash();
}

window.addEventListener("DOMContentLoaded", () => {
  // First-paint locale: the last applied preference is cached locally and
  // restored synchronously here, so the static DOM never flashes in English
  // while get_config resolves; applyLocale reconciles it a moment later.
  try {
    setSystemLocale(detectSystemLocale());
    const cached = localStorage.getItem("pane.locale");
    if (cached === "en" || cached === "zh" || cached === "ru" || cached === "auto") {
      config.locale = cached;
      setActiveLocale(resolveLocale(cached));
    } else {
      setActiveLocale(resolveLocale(config.locale));
    }
    applyStaticI18n();
  } catch {
    /* translation retry happens in applyLocale */
  }
  document.documentElement.classList.remove("i18n-boot");
  const appLogo = document.querySelector<HTMLElement>("#app-logo")!;
  appLogo.innerHTML = `<img src="${paneLogo}" alt="Pane" />`;
  document.querySelector("#splash")?.addEventListener("click", hideSplash);
  document.querySelector<HTMLElement>("#skin-btn")!.innerHTML = uiIcon("palette", "Open skin market");
  applySkin();
  setupFloatingSettingsTabs();
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
  setupTrailScroll();
  setupTooltips();
  // No lens init here: applyGlass() (via initSettings, after the saved
  // config arrives) owns it — a fixed timer raced the config load and
  // built the maps even for users who turned glass off.
  // Local shortcuts are handled only by this focused dashboard window. The
  // wake shortcut is registered by Rust as the sole global shortcut.
  let shiftAlone = false;
  const canCyclePeriod = () => isDashboardActive() && !customizeOpen &&
    !document.body.classList.contains("settings-open") &&
    !document.querySelector(".group-menu-overlay, #confirm-overlay") &&
    !isTypingTarget(document.activeElement);
  window.addEventListener("keydown", (e) => {
    if (e.key === "Shift") {
      shiftAlone = localShortcut("period") === "Shift" && !e.repeat && !e.ctrlKey && !e.altKey && !e.metaKey &&
        !e.isComposing && canCyclePeriod();
    } else {
      shiftAlone = false;
    }
    konamiListen(e);
    if (!e.repeat && !e.isComposing && !isTypingTarget(document.activeElement) && isDashboardActive()) {
      if (shortcutMatches(e, localShortcut("customize"))) {
        e.preventDefault(); setSettings(false); setDrawer(!customizeOpen); return;
      }
      if (shortcutMatches(e, localShortcut("theme"))) {
        e.preventDefault(); document.querySelector<HTMLElement>("#theme-btn")?.click(); return;
      }
      if (localShortcut("period") !== "Shift" && shortcutMatches(e, localShortcut("period")) && canCyclePeriod()) {
        e.preventDefault(); switchOverviewTab(OVERVIEW_TABS[(OVERVIEW_TABS.indexOf(overviewTab) + 1) % OVERVIEW_TABS.length]); return;
      }
    }
    if (
      isDashboardActive() &&
      shortcutMatches(e, config.categoryShortcut) &&
      !e.repeat &&
      !customizeOpen &&
      !document.body.classList.contains("settings-open") &&
      !e.isComposing &&
      e.keyCode !== 229 &&
      !isTypingTarget(document.activeElement)
    ) {
      e.preventDefault();
      cycleOverviewCategory();
      return;
    }
    if (e.ctrlKey && e.key.toLowerCase() === "z" && customizeOpen) {
      e.preventDefault();
      undoLayout();
    }
    // Esc backs out of Customize/Settings (Mac parity); with nothing open
    // it hides the popover, like clicking empty space.
    if (e.key === "Escape") {
      // IME: Esc cancels an in-flight composition (candidate window) — it
      // must not double as "close the panel" / "hide the window".
      if (e.isComposing || e.keyCode === 229) return;
      if (spendDetailOpen) {
        spendDetailOpen = false;
        document.querySelector(".spend-detail-overlay")?.remove();
        return;
      }
      if (skinPreviewId) {
        skinPreviewId = null;
        renderDrawerBody();
        return;
      }
      if (skinMarketOpen) {
        skinMarketOpen = false;
        renderDrawerBody();
        return;
      }
      if (customizeOpen || document.body.classList.contains("settings-open")) {
        setDrawer(false);
        setSettings(false);
        return;
      }
      void invoke("hide_popover").catch(() => {});
    }
    // Ctrl+R refreshes data — and must NOT reload the webview.
    if (shortcutMatches(e, localShortcut("refresh")) && !isTypingTarget(document.activeElement)) {
      e.preventDefault();
      void refresh(true, false, true);
    }
    // Ctrl+S opens Settings — same semantics as the ⚙ button: the standalone
    // panel window (and must NOT trigger the webview "save page" dialog).
    if (shortcutMatches(e, localShortcut("settings")) && !isTypingTarget(document.activeElement)) {
      e.preventDefault();
      setDrawer(false);
      void invoke("open_panel_window").catch(() => {});
    }
    // Ctrl+Shift+P opens the big panel window (dual-form PoC).
    if (
      e.ctrlKey && e.shiftKey && !e.altKey && e.code === "KeyP" &&
      !e.repeat && !isTypingTarget(document.activeElement)
    ) {
      e.preventDefault();
      void invoke("open_panel_window").catch(() => {});
    }
    // Bare T flips the Quota Overview to its soonest-reset (expiring)
    // list — the follow-up key after the global popover shortcut (Alt+2
    // shows the popover, T then shows what runs out soonest; pressing T
    // again goes back to the sectioned board). Same guards as the
    // bare-Shift board toggle: plain key only, no typing targets, no
    // Customize. A collapsed overview is unfolded and scrolled into view
    // so the list is actually on screen.
    if (
      shortcutMatches(e, localShortcut("expiring")) &&
      !e.repeat &&
      !e.isComposing &&
      e.keyCode !== 229 &&
      !customizeOpen &&
      !isTypingTarget(document.activeElement)
    ) {
      overviewExpiringOpen = !overviewExpiringOpen;
      if (config.layout && isOverviewCollapsed()) {
        config.layout.overviewCollapsed = false;
        saveLayout(false);
      }
      renderAll();
      document
        .querySelector<HTMLElement>(".quota-overview")
        ?.scrollIntoView({ block: "nearest" });
    }
  });
  window.addEventListener("keyup", (e) => {
    if (e.key !== "Shift") return;
    const cycle = shiftAlone;
    shiftAlone = false;
    if (cycle && !e.ctrlKey && !e.altKey && !e.metaKey && !e.isComposing && canCyclePeriod()) {
      switchOverviewTab(OVERVIEW_TABS[(OVERVIEW_TABS.indexOf(overviewTab) + 1) % OVERVIEW_TABS.length]);
    }
  });
  window.addEventListener("blur", () => { shiftAlone = false; });
  void getVersion().then((v) => {
    appVersion = v;
    buildText = `v${v} · build ${__BUILD_STAMP__}`;
    renderBuildInfo();
    void checkForUpdate();
  });
  document.querySelector("#refresh")!.addEventListener("click", () => void refresh(true, false, true));

  const setSettings = (open: boolean) => {
    document.body.classList.toggle("settings-open", open);
    document.querySelector("#settings-btn")?.classList.toggle("active", open);
    if (!open && document.activeElement instanceof HTMLElement) {
      // Native <select> popups can outlive the sliding panel in WebView2;
      // blur the control before hiding the surface so it cannot remain over
      // the dashboard or steal the next click.
      document.activeElement.blur();
    }
    if (open) void loadKeyvault();
  };
  // Settings is one system: the popover ⚙ opens the same standalone panel
  // window the tray's 设置 entry opens — no in-popover sheet anymore.
  document.querySelector("#settings-btn")!.addEventListener("click", () => {
    setDrawer(false);
    void invoke("open_panel_window").catch(() => {});
  });
  document.querySelector("#settings-close")!.addEventListener("click", () => setSettings(false));
  document.querySelector("#changelog-btn")!.addEventListener("click", () => {
    setSettings(false);
    showChangelogDialog(t("dialog.changelog"), parseChangelog());
  });
  // Magpie-style settings: sections are static caption + card, always
  // expanded — the top pill tabs switch pages, nothing folds anymore.
  document.querySelectorAll<HTMLElement>(".acc-group").forEach((group) => {
    group.classList.add("open");
  });
  document.querySelector("#customize-btn")!.addEventListener("click", () => {
    setSettings(false);
    setDrawer(!customizeOpen);
  });
  document.querySelector("#skin-btn")!.addEventListener("click", () => {
    if (config.experimentalFeatures !== true) return;
    setSettings(false);
    skinMarketOpen = true;
    skinPreviewId = null;
    setDrawer(true);
  });
  const drawerBody = document.querySelector<HTMLElement>("#drawer-body")!;
  drawerBody.addEventListener("click", (e) => {
    void handleCustomizeClick(e.target as HTMLElement);
  });
  // One/New API site manager forms (add-site / edit-site / relay keys) —
  // submit delegation, since the drawer body re-renders innerHTML.
  drawerBody.addEventListener("submit", (e) => {
    const target = e.target as HTMLElement;
    if (target.id === "onenewapi-add") {
      e.preventDefault();
      void createOneNewApiSite();
      return;
    }
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
  drawerBody.addEventListener("change", (e) => {
    void handleCustomizeChange(e.target as HTMLInputElement);
  });
  drawerBody.addEventListener("input", (e) => {
    const el = e.target as HTMLInputElement;
    if (el.matches("[data-cust-key], [data-cust-baseurl]")) {
      resetCustTestState(el.closest(".cust-config"));
    }
  });
  setupCustomizeDnD(drawerBody);

  const providersEl = document.querySelector<HTMLElement>("#providers")!;
  setupOverviewGroupDrag(providersEl);
  // WebView2 can swallow wheel input before it reaches a nested flex
  // scroller. Capture at the window and hit-test the pointer instead, then
  // move only the provider list. This also works when the pointer is over a
  // card child whose own handler calls preventDefault().
  window.addEventListener("wheel", (event) => {
    if (!event.deltaY) return;
    // Only reroute wheel events that actually land on the card list. When an
    // overlay surface (customize drawer, skin market, settings, menus) is
    // open it covers #providers' rect; without this check the capture below
    // scrolled the hidden list and ate the event, leaving every overlay a
    // dead, unscrollable page.
    const hit = event.target as HTMLElement | null;
    if (!hit || !providersEl.contains(hit)) return;
    const rect = providersEl.getBoundingClientRect();
    if (event.clientX < rect.left || event.clientX > rect.right ||
        event.clientY < rect.top || event.clientY > rect.bottom) return;
    const max = providersEl.scrollHeight - providersEl.clientHeight;
    if (max <= 0) return;
    const before = providersEl.scrollTop;
    providersEl.scrollTop = Math.max(0, Math.min(max, before + event.deltaY));
    if (providersEl.scrollTop !== before) event.preventDefault();
  }, { passive: false, capture: true });
  // Group banners are focusable (role="button") — Enter/Space folds them
  // like a click would.
  providersEl.addEventListener("keydown", (e) => {
    if (e.key !== "Enter" && e.key !== " ") return;
    const banner = (e.target as HTMLElement).closest<HTMLElement>("[data-group-toggle]");
    if (!banner) return;
    e.preventDefault();
    banner.click();
  });
  // The donut center toggles what the card meters: dollars ⇄ raw tokens.
  // Left or right click both work; the choice persists.
  const toggleSpendMetric = (back = false) => {
    config.spendMetric = nextSpendMetric(back);
    void patchConfig({ spendMetric: config.spendMetric });
    renderAll();
  };
  providersEl.addEventListener("click", (e) => {
    const button = (e.target as Element).closest<HTMLElement>("[data-spend-metric]");
    if (button) {
      const metric = button.dataset.spendMetric;
      if (metric !== "cost" && metric !== "tokens") return;
      config.spendMetric = metric;
      void patchConfig({ spendMetric: metric });
      renderAll();
      return;
    }
    const groupBtn = (e.target as Element).closest<HTMLElement>("[data-spend-group]");
    if (groupBtn) {
      const grouping = groupBtn.dataset.spendGroup;
      if (grouping !== "tool" && grouping !== "model") return;
      if (config.spendGrouping === grouping) return;
      config.spendGrouping = grouping;
      void patchConfig({ spendGrouping: grouping });
      renderAll();
      return;
    }
    // Total Spend header: the green arrow shows up only while a version is
    // announced; clicking it pushes the update right away (the popover-open
    // check and the footer entry keep working unchanged). Progress state
    // lives at module level so card re-renders mid-download keep the ring.
    const pushBtn = (e.target as Element).closest<HTMLElement>("[data-update-push]");
    if (pushBtn) {
      if (updatePushing) return;
      updatePushing = true;
      updatePct = null;
      updateSeen = 0;
      syncPushProgress();
      // On success the app restarts into the new version; only the failure
      // path needs in-place feedback.
      void invoke("install_update").catch((err) => {
        updatePushing = false;
        updatePct = null;
        syncPushProgress();
        const status = document.querySelector("#status");
        if (status) status.textContent = t("footer.updateFailed", { err: String(err) });
      });
      return;
    }
  });
  providersEl.addEventListener("contextmenu", (e) => {
    if ((e.target as Element).closest?.(".donut-wrap")) {
      e.preventDefault();
      toggleSpendMetric(true); // right-click cycles backward
      return;
    }
    // Right-click an overview cell = the same group menu the card's ⚙
    // opens, so re-grouping never needs scrolling to the cards below.
    const ovCell = (e.target as Element).closest?.<HTMLElement>(
      ".overview-item[data-jump-provider], .overview-bar-item[data-jump-provider], .expiring-row[data-jump-provider]",
    );
    if (ovCell?.dataset.jumpProvider) {
      e.preventDefault();
      // Anchor at the cursor: the tile-side anchoring was designed for the ⚙
      // button on a wide layout — in the popover's ~285px CSS viewport it
      // flings the ~200px menu to the opposite edge of the window.
      openGroupMenu(ovCell.dataset.jumpProvider, ovCell, true, {
        x: e.clientX,
        y: e.clientY,
        atCursor: true,
        target: e.target as Element,
      });
      return;
    }
    // Right-click on a card = the group menu, same as the head's ⚙.
    const card = (e.target as Element).closest?.<HTMLElement>("article[data-provider]");
    if (card && card.dataset.provider && card.dataset.provider !== "__overview__") {
      e.preventDefault();
      openGroupMenu(card.dataset.provider, card, false, {
        x: e.clientX,
        y: e.clientY,
        target: e.target as Element,
      });
    }
  });

  document.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;
    const details = target.closest<HTMLElement>("[data-spend-details]");
    if (details) {
      e.preventDefault();
      spendDetailOpen = true;
      spendDetailDay = spendDetailDay || localDayKey(new Date());
      renderAll();
      return;
    }
    if (target.closest<HTMLElement>("[data-spend-detail-close]") || target.classList.contains("spend-detail-overlay")) {
      spendDetailOpen = false;
      document.querySelector(".spend-detail-overlay")?.remove();
      return;
    }
    const day = target.closest<HTMLElement>("[data-spend-detail-day]");
    if (day && spendDetailOpen) {
      spendDetailDay = day.dataset.spendDetailDay ?? spendDetailDay;
      renderAll();
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
    if (t) {
      setDonutHot(t.dataset.pid ?? null);
      showSpendPop(t);
    }
  });
  providersEl.addEventListener("mouseout", (e) => {
    if ((e.target as Element).closest?.(".total-spend [data-pid]")) {
      setDonutHot(null);
      hideSpendPop();
    }
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
    // DOM order changed — rebuild so merged-tick card indices stay truthful.
    rebuildTrail();
  };
  providersEl.addEventListener("drop", (e) => {
    e.preventDefault();
    endCardDrag();
  });
  providersEl.addEventListener("dragend", endCardDrag);

  providersEl.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;

    const overviewAcctFold = target.closest<HTMLElement>("[data-overview-acct-fold]");
    if (overviewAcctFold) {
      const family = overviewAcctFold.dataset.overviewAcctFold!;
      const expanded = [...(config.overviewExpanded ?? [])];
      const idx = expanded.indexOf(family);
      if (idx >= 0) expanded.splice(idx, 1);
      else expanded.push(family);
      config.overviewExpanded = expanded;
      void patchConfig({ overviewExpanded: expanded });
      renderAll();
      return;
    }

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
    const groupBtn = target.closest<HTMLElement>("[data-card-group-menu]");
    if (groupBtn) {
      openGroupMenu(groupBtn.dataset.cardGroupMenu!, groupBtn, false);
      return;
    }
    const pinBtn = target.closest<HTMLElement>("[data-card-pin]");
    if (pinBtn) {
      const raw = pinBtn.dataset.cardPin ?? "";
      const splitAt = raw.indexOf("|");
      if (splitAt > 0) setPinnedAccount(raw.slice(0, splitAt), raw.slice(splitAt + 1));
      return;
    }
    const acctTab = target.closest<HTMLElement>("[data-card-account]");
    if (acctTab) {
      const [family, acctId] = acctTab.dataset.cardAccount!.split("|");
      userSelectedAccountFor.set(family, acctId);
      renderAll();
      return;
    }
    const cardRefresh = target.closest<HTMLElement>("[data-card-refresh]");
    if (cardRefresh) {
      const id = cardRefresh.dataset.cardRefresh!;
      const btn = cardRefresh;
      if (btn.classList.contains("spinning")) return;
      btn.classList.add("spinning");
      void invoke<Snapshot>("refresh_provider", { providerId: id })
        .then((snap) => {
          const idx = lastSnapshots.findIndex((s) => s.id === id);
          if (idx >= 0) lastSnapshots[idx] = snap;
          else lastSnapshots.push(snap);
          renderAll();
        })
        .catch(() => {
          // On failure the button just stops spinning; the card keeps
          // showing its last-known values (or stale badge if cached).
        })
        .finally(() => {
          btn.classList.remove("spinning");
        });
      return;
    }
    const cardFold = target.closest<HTMLElement>("[data-card-fold]");
    if (cardFold) {
      const id = cardFold.dataset.cardFold!;
      const L = providerLayout(id);
      // Flip whatever the card currently shows — the default is expanded,
      // so the first click folds.
      L.collapsed = !isCardCollapsed(id);
      saveLayout(false);
      renderAll();
      return;
    }
    const ovFold = target.closest<HTMLElement>("[data-overview-fold]");
    if (ovFold) {
      if (config.layout) {
        config.layout.overviewCollapsed = !isOverviewCollapsed();
        saveLayout(false);
        renderAll();
      }
      return;
    }
    const spendFold = target.closest<HTMLElement>("[data-spend-fold]");
    if (spendFold) {
      config.layout ??= { providerOrder: [], providers: {} };
      config.layout.spendCollapsed = !isSpendFolded();
      saveLayout(false);
      renderAll();
      return;
    }
    const ovTab = target.closest<HTMLElement>("[data-overview-tab]");
    if (ovTab) {
      const next = ovTab.dataset.overviewTab;
      if (next === "5h" || next === "week" || next === "month") switchOverviewTab(next);
      return;
    }
    const ovCatMore = target.closest<HTMLElement>("[data-overview-cat-more]");
    if (ovCatMore) {
      openGroupManagementPanel();
      return;
    }
    const ovCat = target.closest<HTMLElement>("[data-overview-cat]");
    if (ovCat) {
      const next = ovCat.dataset.overviewCat ?? "";
      if ((OVERVIEW_CATEGORIES as readonly string[]).includes(next)) {
        switchOverviewCategory(next as OverviewCategory);
      }
      return;
    }
    const ovStyle = target.closest<HTMLElement>("[data-overview-style]");
    if (ovStyle) {
      const style = ovStyle.dataset.overviewStyle;
      if (style !== "rings" && style !== "bars") return;
      if (config.overviewStyle === style) return;
      config.overviewStyle = style;
      void patchConfig({ overviewStyle: style });
      renderAll();
      return;
    }
    const ovExpiring = target.closest<HTMLElement>("[data-overview-expiring]");
    if (ovExpiring) {
      overviewExpiringOpen = !overviewExpiringOpen;
      renderAll();
      return;
    }
    const groupManage = target.closest<HTMLElement>("[data-overview-group-manage]");
    if (groupManage) {
      openGroupManagementPanel();
      return;
    }
    const peakHelp = target.closest<HTMLElement>("[data-overview-peak-help]");
    if (peakHelp) {
      openPeakHelp(peakHelp);
      return;
    }
    const groupRemove = target.closest<HTMLElement>("[data-group-remove]");
    if (groupRemove) {
      const gid = groupRemove.dataset.groupRemove!;
      const g = cardGroup(gid);
      if (g) {
        const count = cardGroupMemberCount(gid);
        if (count > 0) {
          void appConfirm({ title: t("overview.groupDeleteBlockedTitle"), message: t("overview.groupDeleteBlockedBody", { name: g.name, n: count }), confirmLabel: t("dialog.ok") });
          return;
        }
        void appConfirm({
          title: t("customize.groupDelete"),
          message: t("customize.groupDeleteConfirm", { name: g.name }),
          confirmLabel: t("customize.groupDelete"),
          danger: true,
        }).then((ok) => {
          if (ok) deleteCardGroup(gid);
        });
      }
      return;
    }
    const groupToggle = target.closest<HTMLElement>("[data-group-toggle]");
    if (groupToggle) {
      const g = cardGroup(groupToggle.dataset.groupToggle!);
      if (g) {
        g.collapsed = !g.collapsed;
        void patchConfig({ layout: config.layout });
        renderAll();
      }
      return;
    }
    const ovRefresh = target.closest<HTMLElement>("[data-overview-refresh]");
    if (ovRefresh) {
      const btn = ovRefresh;
      if (!btn.classList.contains("spinning")) {
        btn.classList.add("spinning");
        void refresh(true, false, true).finally(() => btn.classList.remove("spinning"));
      }
      return;
    }
    const jump = target.closest<HTMLElement>("[data-jump-provider]");
    if (jump) {
      const pid = jump.dataset.jumpProvider;
      if (pid) {
        jumpToProviderCard(pid);
      }
      return;
    }
    if (target.closest(".donut-wrap")) {
      const range = target.closest<HTMLElement>("[data-range-tab]");
      if (range) {
        const next = range.dataset.rangeTab;
        if (next === "d7" || next === "d30" || next === "all") {
          rangeTab = next;
          rangeSelected = true;
          renderAll();
        }
        return;
      }
      const period = target.closest<HTMLElement>("[data-tab]");
      if (period) {
        switchSpendTab(period.dataset.tab as SpendTab);
        return;
      }
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
    const range = target.closest<HTMLElement>("[data-range-tab]");
    if (range) {
      const next = range.dataset.rangeTab;
      if (next === "d7" || next === "d30" || next === "all") {
        rangeTab = next;
        rangeSelected = true;
        renderAll();
      }
      return;
    }
    const caret = target.closest<HTMLElement>("[data-caret]");
    if (caret) {
      const id = caret.dataset.caret!;
      const L = providerLayout(id);
      L.expanded = !L.expanded;
      saveLayout(true);
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
      return;
    }
    // Folded card: clicking anywhere outside its controls (fold chevron,
    // refresh, links… all handled above) opens the detail — expand, then
    // center + highlight it like an overview jump.
    const foldedCard = target.closest<HTMLElement>("article.provider[data-provider]");
    if (foldedCard) {
      const pid = foldedCard.dataset.provider!;
      if (isCardCollapsed(pid)) {
        const L = providerLayout(pid);
        L.collapsed = false;
        saveLayout(false);
        renderAll();
        requestAnimationFrame(() => jumpToProviderCard(pid));
      }
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
  let scrollSaveTimer: number | undefined;
  providersEl.addEventListener("scroll", () => {
    tip.hidden = true;
    cancelAnimationFrame(scrollRaf);
    scrollRaf = requestAnimationFrame(updateTrailActive);
    // Dashboard memory: the scroll offset persists (debounced) so a
    // re-summoned popover lands on the same panel — spend card vs quota
    // overview — instead of snapping back to the top.
    window.clearTimeout(scrollSaveTimer);
    scrollSaveTimer = window.setTimeout(() => {
      config.mainScrollTop = providersEl.scrollTop;
      void patchConfig({ mainScrollTop: providersEl.scrollTop });
    }, 400);
  });

  document.querySelector("#trail")!.addEventListener("click", (e) => {
    const tick = (e.target as HTMLElement).closest<HTMLElement>("[data-trail]");
    if (!tick) return;
    const entry = trailEntries[Number(tick.dataset.trail)];
    if (!entry) return;
    // One tick can stand for several accounts of the same provider — every
    // click advances to the next card in the group, wrapping around.
    const idx = entry.indices[entry.cursor % entry.indices.length];
    entry.cursor = (entry.cursor + 1) % entry.indices.length;
    trailCursorMemory.set(entry.key, entry.cursor);
    trailCards()[idx]?.scrollIntoView({ behavior: reduceMotion() ? "auto" : scrollBehavior(), block: "start" });
  });

  // The 4-hourly background checker feeds the same footer button.
  void listen<string>("update-available", (e) => {
    updateVersion = e.payload;
    renderIfVisible();
    renderBuildInfo();
    maybePromptUpdate(e.payload);
  });

  // Byte progress for the push indicator: the green ring fills as chunks
  // arrive; a card re-render reads the same module state.
  void listen<{ chunk: number; total: number | null }>("update-progress", (e) => {
    if (!updatePushing) return;
    updateSeen += e.payload.chunk;
    updatePct = e.payload.total
      ? Math.min(100, Math.round((updateSeen / e.payload.total) * 100))
      : null;
    syncPushProgress();
  });

  void listen("tray-strip-restore", () => {
    requestTraySync();
  });

  // Cross-window config sync: the other window persisted a write — reload
  // and re-apply (self-writes are filtered inside).
  void listen("config-updated", () => {
    void reloadConfigFromBackend();
  });

  // The backend's background loop refreshes even while this window is
  // hidden (where setInterval is throttled dead). Adopt its results;
  // rendering still defers to the next open via renderIfVisible().
  void listen<Snapshot[]>("usage-updated", (e) => {
    if (refreshing || !Array.isArray(e.payload)) return;
    lastFetch = Date.now();
    lastSnapshots = hideFoldedMoonshot(e.payload);
    ensureLayout();
    renderIfVisible();
    requestTraySync();
    // Auth center: fresh quota columns and account lists (a login/import
    // finishing elsewhere also lands here). A live login flow owns its
    // card block, so don't re-render under it.
    if (IS_PANEL_FORM && authViewActive && !authFlowBusy()) void loadAuthCenter();
  });

  // Back from hidden: pull the backend's latest right away instead of
  // waiting out the rest of the refresh interval. The panel form skips
  // this — it renders no cards, and usage-updated keeps its cache warm.
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden && !IS_PANEL_FORM) void refresh(true);
  });

  void listen("popover-shown", () => {
    void checkForUpdate();
    void maybePlayStartupAnimation();
    // Always reopen on the main page, at the top — leftover Customize/
    // Settings panels, a stale confirm dialog, or a stale scroll position
    // from the previous visit feel like the app is stuck mid-page.
    setDrawer(false);
    setSettings(false);
    dismissConfirm?.();
    dismissWhatsNew?.();
    userSelectedAccountFor.clear();
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
    } else if (lastSnapshots.length) {
      renderAll();
    }
    // Land where the user left off: the persisted scroll offset decides
    // whether the spend card or the quota overview is in view. Fold states
    // (overview / spend) already persist in layout, so both halves of the
    // "same page as last time" contract hold across ESC and restarts.
    // Deferred one frame so the freshly rendered list has its height.
    const rememberedScroll = config.mainScrollTop ?? 0;
    requestAnimationFrame(() => {
      providersEl.scrollTop = rememberedScroll;
    });
    rebuildTrail();
    updateTrailActive();
    if (lastSnapshots.length && !customizeOpen) playReveal();
    requestTraySync();
    const mascot = document.querySelector<HTMLElement>("#skin-mascot");
    mascot?.classList.remove("wake");
    if (activeSkin()) {
      void mascot?.offsetWidth;
      mascot?.classList.add("wake");
    }
    void refresh();
  });
  void initSettings().then(() => {
    // Capability probe runs in both window forms: overview jumps and trail
    // scrolls live in the popover, but the settings panel scrolls too.
    probeSmoothScroll();
    if (IS_PANEL_FORM) {
      // Settings-only surface: the backend auto-refresh loop already fetches
      // and broadcasts usage-updated — no boot fetch, no refresh timer, no
      // What's-new popup in this window.
      return;
    }
    scheduleAutoRefresh();
    void paintCachedSnapshots();
    void refresh(true);
    // Manual launch with the window already visible: popover-shown never
    // fires, so the splash gate runs here too.
    void getCurrentWebviewWindow()
      .isVisible()
      .then((visible) => {
        if (visible) void maybePlayStartupAnimation();
      })
      .catch(() => {});
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
    if (lastSnapshots.length && !customizeOpen) tickCountdowns();
  }, 30_000);
});
