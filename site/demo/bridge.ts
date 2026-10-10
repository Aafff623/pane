// Browser transport for the real Pane frontend. No native IPC, vendor requests,
// account files, API keys, telemetry or loopback connections are used here.
// 'pane-source/providerCatalog' is aliased by build-demo.mjs to the catalog of
// the project passed on the command line, so the demo always mirrors it.
import { providerCatalog } from 'pane-source/providerCatalog';

declare const __SOURCE_VERSION__: string;

const families = providerCatalog.map(provider => provider.familyId);
const params = new URLSearchParams(location.search);
const locale = params.get('lang') === 'en' ? 'en' : 'zh';
const version = __SOURCE_VERSION__;
let keyboardDemoEnabled = false;

// The demo mirrors the author's real floating window: same enabled families,
// groups (🥚鸡蛋 / 🐑羊毛), notes, account pool, regular density and MV Boli UI font.
const DISABLED = ['devin', 'openrouter', 'elevenlabs', 'ollama', 'codebuff', 'kilo', 'aihubmix', 'qwen', 'hermes', 'opencode', 'deepseek', 'claude', 'moonshot', 'grok', 'cursor', 'siliconflow', 'novita', 'relaybalance', 'clawsgo', 'stepfun', 'onenewapi', 'traecn'];
const LAYOUT_ORDER = ['codex', 'opencode', 'copilot', 'minimax', 'zai', 'antigravity', 'deepseek', 'stepfun-plan', 'qodercn', 'doubao', 'clawsgo', 'shandianshuo', 'tavily', 'kimi', 'stepfun', 'commandcode@63592739d4022730c40f340504638dfd', 'bocha', 'firecrawl', 'brave', 'clinepass', 'clinepass@67ae0d8e0f651d5c85389d55b439010d', 'clinepass@061fa48ec8ce213347f645f9ba6e85b6', 'clinepass@37364f9058253ce13c44730f4761c3e0', 'clinepass@6dd83003c55ed8d3b87dbe698f82a6e6', 'clinepass@bf604e26e7d5a98163f51d4148e714e8', 'sensenova', 'apigoto', 'claude', 'traecn', 'onenewapi@BjhOUJrUH3GNZO0milB15Q', 'commandcode', 'cursor', 'grok', 'devin', 'openrouter', 'moonshot', 'elevenlabs', 'ollama', 'codebuff', 'kilo', 'aihubmix', 'onenewapi', 'qwen', 'hermes', 'siliconflow', 'novita', 'relaybalance', 'mcode', 'qoder'];
const NOTES: Record<string, string> = {
  antigravity: 'Antigravity', clinepass: 'ClinePass', codex: 'GPT', commandcode: 'Command Code',
  copilot: 'Copilot', doubao: 'Doubao', kimi: 'Kimi', minimax: 'MiniMax', qodercn: 'Qoder CN',
  'stepfun-plan': 'stepfun', zai: 'GLM',
};
const POOL_NOTES: Record<string, string> = {
  'clinepass@67ae0d8e0f651d5c85389d55b439010d': 'ls', 'clinepass@061fa48ec8ce213347f645f9ba6e85b6': 'ls',
  'clinepass@37364f9058253ce13c44730f4761c3e0': 'njf', 'clinepass@6dd83003c55ed8d3b87dbe698f82a6e6': 'lzl',
  'clinepass@bf604e26e7d5a98163f51d4148e714e8': 'njf2',
};
const GROUPS: Record<string, string> = { antigravity: 'g6', codex: 'g6', commandcode: 'g6', kimi: 'g6', minimax: 'g6', zai: 'g6', copilot: 'g5', doubao: 'g5' };

let config: Record<string, any> = {
  refreshMinutes: 5, disabled: [...DISABLED],
  pinned: null, trayProviders: [], telemetry: false,
  notifyAlmostOut: false, notifyCuttingClose: false, notifyWillRunOut: false, notifyResetSoon: true,
  spendTab: 'today', overviewTab: '5h', overviewCategory: 'coding', overviewStyle: 'bars',
  categoryOverrides: {}, spendMetric: 'tokens', spendGrouping: 'model', showUsed: false, showTrend: false,
  resetExact: false, timeFormat: 'auto', layout: {
    providerOrder: [...LAYOUT_ORDER],
    groups: [{ id: 'g5', name: '🥚鸡蛋', collapsed: false }, { id: 'g6', name: '🐑羊毛', collapsed: false }],
    providers: Object.fromEntries([...families, ...Object.keys(POOL_NOTES)].map(id => [id, {
      metricOrder: [], onDemand: [], hidden: [], starred: [], expanded: false,
      ...(NOTES[id] ? { note: NOTES[id] } : {}), ...(POOL_NOTES[id] ? { note: POOL_NOTES[id] } : {}), ...(GROUPS[id] ? { group: GROUPS[id] } : {}),
    }])),
  },
  appearance: params.get('theme') === 'light' ? 'light' : 'dark', density: 'regular', uiFont: 'MV Boli',
  glassEffects: true, shortcut: 'Alt+2', categoryShortcut: 'Shift+1', localShortcuts: {},
  proxy: { enabled: false, url: '' }, showTotalSpend: true, welcomeDismissed: true,
  lastSeenVersion: version, reduceAnimations: matchMedia('(prefers-reduced-motion: reduce)').matches,
  jumpAnimation: 'smooth', hideUsageWhileSharing: false, locale, windowForm: 'floating',
  silentStart: false, startupAnimation: false, lastStartupBootId: 1, overviewExpanded: [],
  experimentalFeatures: false, spendIconTiers: null, overviewCatFull: false,
  minimal: false, mainScrollTop: 0, spendHeadRange: 'today',
  // Keys the app expects to exist: the Customize drawer's "removed" section
  // reads config.removedProviders.includes(...) and crashes on undefined.
  qoderCheckin: false, removedProviders: [],
};

const copy = <T>(v: T): T => structuredClone(v);
const now = Date.now();
const hours = (h: number) => now + h * 3_600_000;
const quota = (used: number, resetHours: number, periodHours = 5) => ({
  label: 'Session', kind: 'progress', used_percent: used, detail: null, value: null,
  resets_at: resetHours == null ? null : hours(resetHours), period_ms: periodHours * 3_600_000,
});

// Per-family sample data shaped after the author's real panel screenshot:
// two exhausted cards (Qoder CN, Kimi), two in peak (GLM, Command Code), a
// ClinePass 6-account pool, "not started" cards and MCP no-data rows.
const snapshotDefs: Record<string, () => Record<string, unknown>> = {
  'stepfun-plan': () => ({ metrics: [quota(12, 10 * 24 + 18, 24 * 30)] }),
  qodercn: () => ({ metrics: [
    { label: 'Session', kind: 'progress', used_percent: 100, detail: '已用 2500 / 2500 额度', value: null, resets_at: hours(24 + 2), period_ms: 5 * 3_600_000 },
    { label: 'Add-on credits', kind: 'progress', used_percent: 100, detail: '已用 500 / 500 额度', value: null, resets_at: null, period_ms: null },
  ] }),
  clinepass: () => ({ metrics: [
    { label: '会话', kind: 'progress', used_percent: 0, detail: null, value: null, resets_at: null, period_ms: 5 * 3_600_000 },
    { label: '每周', kind: 'progress', used_percent: 16, detail: null, value: null, resets_at: hours(2 * 24 + 21), period_ms: 168 * 3_600_000 },
    { label: '每月', kind: 'progress', used_percent: 8, detail: null, value: null, resets_at: hours(25 * 24 + 21), period_ms: 24 * 30 * 3_600_000 },
  ] }),
  sensenova: () => ({ metrics: [quota(0, 2 + 50 / 60)] }),
  apigoto: () => ({ metrics: [quota(0, 26 * 24 + 22, 24 * 30)] }),
  copilot: () => ({ metrics: [quota(0, 24 * 24 + 15, 24 * 30)] }),
  doubao: () => ({ metrics: [quota(0, null, 24)] }),
  codex: () => ({ metrics: [
    { label: 'Session', kind: 'progress', used_percent: 48, detail: null, value: '1.1B tokens', resets_at: hours(1 + 14 / 60), period_ms: 5 * 3_600_000 },
    { label: 'Weekly', kind: 'progress', used_percent: 20, detail: null, value: '1.1B tokens', resets_at: hours(6 * 24 + 20), period_ms: 168 * 3_600_000 },
  ] }),
  minimax: () => ({ metrics: [quota(0, 3 + 26 / 60)] }),
  zai: () => ({ metrics: [quota(82, 1 + 42 / 60)] }),
  antigravity: () => ({ metrics: [quota(0, 4 + 54 / 60)] }),
  kimi: () => ({ metrics: [quota(100, 4 * 24 + 7)] }),
  commandcode: () => ({ metrics: [quota(12, 2 + 32 / 60)] }),
  shandianshuo: () => ({ metrics: [{ label: 'Monthly', kind: 'progress', used_percent: 4, detail: null, value: null, resets_at: hours(26 * 24 + 21), period_ms: 24 * 30 * 3_600_000 }] }),
  tavily: () => ({ metrics: [{ label: 'Requests', kind: 'progress', used_percent: 9, detail: null, value: null, resets_at: hours(30 * 24), period_ms: 24 * 30 * 3_600_000 }] }),
  bocha: () => ({ metrics: [{ label: 'Requests', kind: 'progress', used_percent: 0, detail: null, value: null, resets_at: hours(30 * 24), period_ms: 24 * 30 * 3_600_000 }] }),
  firecrawl: () => ({ metrics: [{ label: 'Requests', kind: 'progress', used_percent: 80, detail: null, value: null, resets_at: hours(29 * 24), period_ms: 24 * 30 * 3_600_000 }] }),
  brave: () => ({ metrics: [{ label: 'Requests', kind: 'progress', used_percent: 7, detail: null, value: null, resets_at: hours(30 * 24), period_ms: 24 * 30 * 3_600_000 }] }),
  mcode: () => ({ metrics: [{ label: 'Credits', kind: 'progress', used_percent: 0, detail: null, value: null, resets_at: null, period_ms: null }] }),
  qoder: () => ({ metrics: [{ label: 'Credits', kind: 'progress', used_percent: 0, detail: null, value: null, resets_at: null, period_ms: null }] }),
};

const enabledIds = families.filter(id => !DISABLED.includes(id));
const poolAccounts = [
  ['clinepass@67ae0d8e0f651d5c85389d55b439010d', 'ls', 0], ['clinepass@061fa48ec8ce213347f645f9ba6e85b6', 'ls', 0],
  ['clinepass@37364f9058253ce13c44730f4761c3e0', 'njf', 0], ['clinepass@6dd83003c55ed8d3b87dbe698f82a6e6', 'lzl', 0],
  ['clinepass@bf604e26e7d5a98163f51d4148e714e8', 'njf2', 100],
] as const;

const snapshots = [
  // Only providers with sample data: a catalog entry without a snapshotDefs
  // entry renders as an empty "no usage data" card, which makes the panel look
  // nothing like the real window (the desktop app hides credential-less
  // providers). Filtering here — rather than narrowing enabledIds — keeps the
  // blocklist semantics intact for everything else that reads it.
  ...enabledIds.filter(id => snapshotDefs[id]).map(id => ({
    id,
    name: NOTES[id] || providerCatalog.find(p => p.familyId === id)?.displayName || id,
    plan: 'Pro', status: 'ok', error: null, stale: false, warning: null,
    ...(snapshotDefs[id] ? snapshotDefs[id]() : { metrics: [] }),
    dashboard_url: null,
  })),
  ...poolAccounts.map(([id, , used]) => ({
    id, name: `ClinePass · ${id.split('@')[1].slice(0, 4)}`, plan: 'Pro', status: used >= 100 ? 'exhausted' : 'ok',
    error: null, stale: false, warning: null,
    metrics: [
      { label: '会话', kind: 'progress', used_percent: used, detail: null, value: null, resets_at: null, period_ms: 5 * 3_600_000 },
      { label: '每周', kind: 'progress', used_percent: Math.min(100, used + 16), detail: null, value: null, resets_at: hours(2 * 24 + 21), period_ms: 168 * 3_600_000 },
    ],
    dashboard_url: null,
  })),
];

const spendModels = [
  { model: 'deepseek/deepseek-v4.1-flash', cost: 8.13, tokens: 813_000_000 },
  { model: 'gemini-3.8-flash-n', cost: 1.36, tokens: 136_300_000 },
  { model: 'MiniMax-M3.1-Flash-Preview', cost: 1.06, tokens: 106_000_000 },
  { model: 'step-5-preview', cost: 0.87, tokens: 86_700_000 },
  { model: 'GLM-5.3', cost: 0.32, tokens: 32_100_000 },
  { model: 'gpt-6.1-sol', cost: 0.27, tokens: 26_700_000 },
  { model: 'Other', cost: 0.02, tokens: 428_700 },
];
// Token proportions mirror the supplied floating-panel screenshot. Dollar
// amounts are illustrative demo values, not model prices or live account data.
// Every period scales this same window for consistent totals and breakdowns.
const round2 = (n: number) => Math.round(n * 100) / 100;
const windowCost = () => spendModels.reduce((sum, m) => sum + m.cost, 0);
const windowTokens = () => spendModels.reduce((sum, m) => sum + m.tokens, 0);
const tokenWindow = (factor = 1) => ({
  cost: round2(windowCost() * factor),
  tokens: Math.round(windowTokens() * factor),
  models: spendModels.map(m => ({ ...m, cost: round2(m.cost * factor), tokens: Math.round(m.tokens * factor) })),
});
const spend = ['codex'].map((id, i) => ({
  id, name: NOTES[id] || id,
  today: tokenWindow(1), yesterday: tokenWindow(0.72), last30: tokenWindow(18),
  trend: [1, 3, 2, 6, 4, 7, 5].map(n => n * 24_000_000), trend_cost: [2, 3, 1, 5, 4, 6, 8],
  unpriced: 0, unpriced_models: [],
}));
const daily = Array.from({ length: 365 }, (_, dayIndex) => {
  const date = new Date(); date.setDate(date.getDate() - dayIndex);
  const day = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
  return spend.map((s, i) => ({ day, id: s.id, ...tokenWindow(dayIndex === 0 ? 1 : ((dayIndex * 13 + i * 7) % 19) / 12) }));
}).flat();

const nativeOnly = () => locale === 'zh'
  ? '请在桌面版 Pane 中连接账户。网页体验不会连接真实账户。'
  : 'Connect your account in the desktop app. This web experience uses no real accounts.';

// Demo key-vault rows (masked) so the settings vault section is operable.
let vaultRows = [
  { id: 'kv-glm', service: 'Z.ai', label: 'Z.ai', masked: 'sk-…9f2a', note: 'CN 区' },
  { id: 'kv-brave', service: 'Brave Search', label: 'Brave Search', masked: 'BSA…x7q', note: '' },
  { id: 'kv-tavily', service: 'Tavily', label: 'Tavily', masked: 'tvly…4c1', note: 'MCP 搜索' },
];
const maskDemo = (key: unknown) => {
  const s = String(key ?? '');
  return s.length > 6 ? `${s.slice(0, 4)}…${s.slice(-4)}` : 'sk-…demo';
};

export async function invoke<T>(command: string, args: Record<string, any> = {}): Promise<T> {
  let result: unknown;
  switch (command) {
    case 'get_config': result = config; break;
    case 'set_config':
      config = { ...config, ...args.patch };
      if (args.patch?.overviewTab) publishDemoPeriod();
      result = config; break;
    case 'cached_usage': case 'fetch_usage':
      // No network in the demo: answer immediately so the first paint is
      // instant instead of waiting on a simulated round-trip.
      result = snapshots; break;
    case 'refresh_provider': result = snapshots.find(s => s.id === args.providerId); break;
    case 'fetch_spend': result = spend; break;
    case 'fetch_spend_history': result = spend.map(s => ({ id: s.id, ...s.last30, active_days: 24 })); break;
    case 'fetch_spend_daily': result = daily; break;
    case 'fetch_usage_history': case 'fetch_credit_history': result = Object.fromEntries(families.map((id, i) => [id, [12, 18, 25, 22, 34, 30, 40].map(n => n + i * 4)])); break;
    case 'system_ui_locale': result = locale; break;
    case 'get_autostart': result = false; break;
    case 'get_boot_id': result = 1; break;
    case 'check_update': result = null; break;
    case 'list_system_fonts': result = ['Segoe UI', 'Arial', 'MV Boli']; break;
    case 'account_list': result = args.provider === 'clinepass'
      ? Object.entries(POOL_NOTES).map(([id, note]) => ({ id, label: note, maskedKey: `sk-…${id.slice(-4)}`, baseUrl: null }))
      : []; break;
    case 'archived_accounts': case 'onenewapi_list_sites': case 'auth_center_list': result = []; break;
    // Key vault: a real in-memory list so add / remove / note / copy flows
    // actually respond to clicks in the demo.
    case 'keyvault_status': result = { has_password: false, unlocked: true, count: vaultRows.length }; break;
    case 'keyvault_unlock': case 'keyvault_set_password': result = { has_password: false, unlocked: true, count: vaultRows.length }; break;
    case 'keyvault_lock': result = { has_password: false, unlocked: true, count: vaultRows.length }; break;
    case 'keyvault_list': result = vaultRows.map(r => ({ ...r })); break;
    case 'keyvault_add': {
      vaultRows.push({ id: `kv-${Date.now()}`, service: String(args.service ?? ''), label: String(args.label ?? ''), masked: maskDemo(args.key), note: String(args.note ?? '') });
      result = vaultRows.map(r => ({ ...r })); break;
    }
    case 'keyvault_remove': vaultRows = vaultRows.filter(r => r.id !== args.id); result = vaultRows.map(r => ({ ...r })); break;
    case 'keyvault_set_note': { const row = vaultRows.find(r => r.id === args.id); if (row) row.note = String(args.note ?? ''); result = vaultRows.map(r => ({ ...r })); break; }
    case 'keyvault_copy': case 'keyvault_reveal': result = 'sk-demo-0000000000000000000000000000'; break;
    case 'test_api_key': case 'test_provider': case 'stepfun_plan_test': result = { ok: true, metrics: 2, message: '连接成功 · 2 个指标' }; break;
    case 'get_credential_status': result = { source: 'demo', hasKey: false }; break;
    case 'get_base_url': result = null; break;
    case 'open_panel_window':
      // Desktop opens the standalone panel-form settings window. The web demo
      // has no window manager: ask the host page to reveal its overlay.
      if (params.get('settings') !== '1') parent.postMessage({ type: 'pane-demo', action: 'settings' }, location.origin);
      result = null; break;
    case 'close_panel_window': document.body.classList.remove('settings-open'); result = null; break;
    case 'open_link': {
      const url = new URL(args.url);
      if (url.protocol !== 'https:') throw new Error('Only HTTPS links are available.');
      window.open(url.href, '_blank', 'noopener,noreferrer'); result = null; break;
    }
    case 'hide_popover':
      if (keyboardDemoEnabled) parent.postMessage({ type: 'pane-demo-hide' }, location.origin);
      result = null; break;
    case 'sync_tray_surfaces': case 'set_window_form': result = null; break;
    default: throw new Error(nativeOnly());
  }
  return copy(result) as T;
}
export async function listen() { return () => {}; }
export async function getVersion() { return version; }
// The desktop settings surface is the "panel" window: the same bundle with
// label=panel renders the large settings panel (IS_PANEL_FORM branch).
export function getCurrentWebviewWindow() {
  return {
    label: params.get('settings') === '1' ? 'panel' : 'main',
    isVisible: async () => true,
    // The desktop app pins WebView2's native chrome (select popups, scrollbars)
    // to its own theme. The demo has no native chrome, but the call must still
    // resolve — a missing method aborts the app's boot at "Starting…".
    setTheme: async (_theme?: string | null) => {},
  };
}

function publishDemoPeriod() {
  if (params.get('settings') !== '1') parent.postMessage({ type: 'pane-demo-period', period: config.overviewTab }, location.origin);
}

// Alt+2 is a native global shortcut in Pane. The web demo forwards it to
// its host; Shift is left to the real frontend's press/release handler.
window.addEventListener('keydown', event => {
  if (!keyboardDemoEnabled || event.repeat || !event.altKey || event.ctrlKey || event.metaKey || event.code !== 'Digit2') return;
  event.preventDefault();
  event.stopImmediatePropagation();
  parent.postMessage({ type: 'pane-demo-toggle' }, location.origin);
}, true);

window.addEventListener('message', event => {
  if (event.source !== parent || event.origin !== location.origin || event.data?.type !== 'pane-demo') return;
  const action = event.data.action;
  if (action === 'keyboard-demo') { keyboardDemoEnabled = event.data.enabled === true; publishDemoPeriod(); }
  if (action === 'cycle-period') {
    const periods = ['5h', 'week', 'month'];
    const next = periods[(periods.indexOf(config.overviewTab) + 1) % periods.length];
    document.querySelector<HTMLElement>(`[data-overview-tab="${next}"]`)?.click();
    document.querySelector('.content')?.scrollTo({ top: 0, behavior: 'auto' });
  }
  if (action === 'settings') document.body.classList.add('settings-open');
  if (action === 'overview') {
    document.body.classList.remove('settings-open');
    document.querySelector('#settings-close')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    document.querySelector('[data-spend-detail-close]')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    document.querySelector('.content')?.scrollTo({ top: 0, behavior: 'auto' });
  }
  if (action === 'spend') {
    document.body.classList.remove('settings-open');
    document.querySelector('[data-spend-details]')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  }
  if (action === 'refresh') document.querySelector('#refresh')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  if (action === 'theme') document.querySelector('#theme-btn')?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  if (action === 'cycle-category') {
    const cats = ['coding', 'productivity', 'mcp'];
    const active = document.querySelector<HTMLElement>('[data-overview-cat].active')?.dataset.overviewCat || 'coding';
    const next = cats[(cats.indexOf(active) + 1) % cats.length];
    const btn = document.querySelector<HTMLElement>(`[data-overview-cat="${next}"]`);
    btn?.click();
    if (btn) parent.postMessage({ type: 'pane-demo-category', category: next, label: btn.textContent?.trim() || next }, location.origin);
  }
  if (action === 'customize') document.querySelector<HTMLElement>('#customize-btn')?.click();
  // T (expiring list) is keyboard-only in the app; replay it inside the demo.
  if (action === 'expiring') window.dispatchEvent(new KeyboardEvent('keydown', { key: 't', code: 'KeyT', bubbles: true }));
});

// A settings=1 query renders the standalone panel-form settings window,
// mirroring the desktop app's dedicated settings window (975×677 CSS).
// No body.settings-open: the IS_PANEL_FORM branch owns the whole page.

function collectAnnotationAnchors() {
  const anchor = (selector: string) => {
    const element = document.querySelector(selector);
    if (!element) return null;
    const rect = element.getBoundingClientRect();
    return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
  };
  return {
    anchors: {
      sample: anchor('.donut-wrap'), categories: anchor('[data-overview-cat="coding"]') || anchor('button.tab'),
      periods: anchor('[data-overview-tab="5h"]') || anchor('.overview-tabs'),
      left: anchor('#side-zone'), right: anchor('#side-zone-right'),
      spend: anchor('.total-spend .legend-row') || anchor('.total-spend svg'),
      accounts: anchor('[data-provider="clinepass"] .provider-account-badge') || anchor('[data-provider="clinepass"]'),
      pool: anchor('[data-provider="clinepass"] .provider-account-badge') || anchor('[data-provider="clinepass"]'),
      groups: anchor('.card-group-head') || anchor('.category-head'),
      settings: anchor('#side-zone') || anchor('.panel-head'),
      refresh: anchor('#status') || anchor('.panel-foot') || anchor('footer'),
      status: anchor('#status') || anchor('.panel-foot') || anchor('footer'),
    },
    height: innerHeight, width: innerWidth, settings: document.body.classList.contains('settings-open'),
  };
}
// Publish anchors on a change-gated rAF loop instead of a zoo of scroll /
// resize / mutation listeners: every scroll, entry animation and re-render
// is picked up next frame, and quiet frames cost one getBoundingClientRect
// round per anchor and a string compare.
window.addEventListener('DOMContentLoaded', () => {
  parent.postMessage({ type: 'pane-demo-ready' }, location.origin);
  let lastSignature = '';
  const tick = () => {
    const snapshot = collectAnnotationAnchors();
    const signature = JSON.stringify(snapshot);
    if (signature !== lastSignature) {
      lastSignature = signature;
      parent.postMessage({ type: 'pane-demo-anchors', ...snapshot }, location.origin);
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
});
