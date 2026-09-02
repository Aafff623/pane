// Frontend locale: Settings stores auto / en / zh / ru. Metric row labels from
// Rust stay English in config.layout (stars, pins, Customize keys); only
// the painted text switches.

export type LocalePref = "auto" | "en" | "zh" | "ru";
export type Locale = "en" | "zh" | "ru";

type Dict = Record<string, string>;

const en: Dict = {
  "sidebar.theme": "Light / dark mode",
  "sidebar.themeToDark": "Switch to dark mode",
  "sidebar.themeToLight": "Switch to light mode",
  "sidebar.refresh": "Refresh now",
  "sidebar.customize": "Customize",
  "sidebar.settings": "Settings",
  "sidebar.cards": "Card navigation",

  "footer.starting": "Starting…",
  "footer.refreshing": "Refreshing…",
  "footer.updated": "Updated {time}",
  "footer.refreshFailed": "Refresh failed: {err}",
  "footer.keySaved": "{name} key saved",
  "footer.keySaveFailed": "Could not save key: {err}",
  "footer.shortcutSaved": "Shortcut saved",
  "footer.shortcutCleared": "Shortcut cleared",
  "footer.proxySaved": "Proxy saved — takes effect after restart",
  "footer.autostartFailed": "Autostart failed: {err}",
  "footer.copied": "Copied to clipboard",
  "footer.shareFailed": "Share failed: {err}",
  "footer.updateFailed": "Update failed: {err}",
  "footer.openLinkFailed": "Could not open link: {err}",
  "footer.twoStars": "Up to 2 stars per provider",
  "footer.redeeming": "Redeeming reset credit…",
  "footer.redeemFailed": "Redeem failed: {err}",
  "footer.configSaveFailed": "Settings were not saved and may be lost after restart: {err}",
  "footer.traySyncFailed": "Tray display could not update; it will retry: {err}",
  "footer.onenewapiSaved": "Site saved",
  "footer.onenewapiDeleted": "Site deleted",
  "footer.onenewapiFailed": "One/New API: {err}",
  "footer.onenewapiNotCompatible": "This is not a OneAPI / NewAPI compatible site",
  "footer.onenewapiDuplicate": "That site is already registered",
  "footer.onenewapiProbeFailed": "Could not verify this site: {err}",
  "footer.onenewapiKeySaved": "Key saved",

  "update.check": "Checking for updates…",
  "update.to": "⬆ Update to v{version}",
  "update.installing": "Installing…",
  "update.retry": "⬆ Update to v{version} — retry",

  "settings.done": "← Done",
  "settings.title": "Settings",
  "settings.general": "General",
  "settings.language": "Language",
  "settings.langAuto": "Auto",
  "settings.langEn": "English",
  "settings.langZh": "中文",
  "settings.langRu": "Русский",
  "settings.refreshEvery": "Refresh every",
  "settings.min": "min",
  "settings.startWithWindows": "Start with Windows",
  "settings.pacing": "Always show pacing",
  "settings.trayShows": "Tray icon shows",
  "settings.pinAuto": "Auto (first live metric)",
  "settings.pinOption": "{name} — {label}",
  "settings.timeFormat": "Time format",
  "settings.timeAuto": "Auto",
  "settings.time12": "12-hour",
  "settings.time24": "24-hour",
  "settings.appearance": "Appearance",
  "settings.appearSystem": "System",
  "settings.appearLight": "Light",
  "settings.appearDark": "Dark",
  "settings.compact": "Compact layout",
  "settings.glass": "Liquid glass effects",
  "settings.glassTip":
    "Turn off on slower PCs — replaces the glass refraction with a simple solid look",
  "settings.reduceAnim": "Reduce animations",
  "settings.reduceAnimTip":
    "Skip card entrance motion and the day/night wipe. Windows' own 'Animation effects' setting is still respected.",
  "settings.showSpend": "Show Total Spend card",
  "settings.shortcut": "Global shortcut",
  "settings.shortcutPh": "e.g. Ctrl+Shift+U",

  "settings.notifications": "Notifications",
  "settings.notifyNote":
    "Windows toasts when a quota worsens — once per metric per reset period.",
  "settings.notifyAlmost": "Almost out (<10% left)",
  "settings.notifyClose": "Cutting it close",
  "settings.notifyRunout": "Will run out",

  "settings.privacy": "Privacy",
  "settings.privacyNote":
    'One anonymous "alive today" ping and per-provider success/failure counts, once a day, under a random ID attached to nothing. No usage amounts, no spend, no IPs stored. Full details in docs/privacy.md.',
  "settings.telemetry": "Share anonymous usage statistics",
  "settings.hideSharing": "Hide tray numbers while screen sharing",
  "settings.hideSharingTip":
    "During Presentation Settings, exclusive fullscreen, or remote control, tray percentages hide. The Pane icon and starred provider logos stay. A Teams/Zoom window share is not detected. Off by default.",

  "settings.network": "Network",
  "settings.useProxy": "Use proxy",
  "settings.proxyUrl": "Proxy URL",
  "settings.networkNote":
    "Applies after the app restarts. Local usage API always runs at http://127.0.0.1:6736/v1/usage",

  "settings.relayBalance": "Custom balance",
  "settings.relayBalanceNote":
    "For relay/gateway sites that expose the OpenAI billing endpoints (/dashboard/billing/subscription + /usage). Save stores both fields.",
  "settings.relayBaseUrl": "Base URL",
  "settings.relayApiKey": "API key",

  "settings.apiKeys": "API keys",
  "settings.apiKeysNote":
    "Stored only on this PC (%APPDATA%\\Pane). Leave empty and save to remove.",
  "settings.save": "Save",
  "settings.keyPlaceholder": "API key",
  "settings.keyPhMinimax": "API key (auto-detected from CLI)",
  "settings.keyPhMoonshot": "sk-… (platform.kimi.ai key)",
  "settings.keyPhKimi": "Kimi For Coding plan key (if you don't use kimi login)",
  "settings.keyPhCodebuff": "API key (auto-detected from CLI)",
  "settings.keyPhKilo": "API key (auto-detected from CLI)",
  "settings.keyPhAihubmix": "sk-… (auto-detected from OpenCode)",
  "settings.keyPhQwen": "sk-sp-… (auto-detected from env)",
  "settings.keyPhOpencode": "Go key (auto-detected from auth.json)",
  "settings.keyPhStepfun": "API key (platform.stepfun.com)",
  "settings.keyPhSiliconflow": "sk-… (siliconflow.cn)",
  "settings.keyPhNovita": "API key (novita.ai)",

  "settings.advanced": "Advanced",
  "settings.advancedNote":
    "Restores every preference to its default and re-detects installed tools. API keys and your usage history stay. Proxy changes still need a restart.",
  "settings.resetAll": "Reset all settings",
  "settings.changelog": "What's new · Changelog",
  "settings.customizeHint":
    "Providers, row order, and tray-strip stars live in Customize (☰ in the sidebar). Star up to 2 metrics per provider there to show them as tray icons.",

  "settings.resetTitle": "Reset all settings?",
  "settings.resetBody":
    "Theme, density, notifications, shortcut, proxy, pacing, tray stars, and card layouts go back to defaults. Installed tools are re-detected. API keys and usage history stay. A proxy change still needs a restart.",
  "settings.resetConfirm": "Reset all",

  "settings.onenewapi": "One/New API sites",
  "settings.onenewapiNote":
    "Register OneAPI / NewAPI compatible sites. Keys stay on this PC (%APPDATA%\\Pane\\onenewapi.json) and are sent only to that site. Empty sites stay here; cards appear after you add a key.",
  "settings.onenewapiFamily": "Show One/New API cards",
  "settings.onenewapiFamilyTip":
    "Turns off every key card at once. Per-key switches in Customize stay.",
  "settings.onenewapiAdd": "Add site",
  "settings.onenewapiNamePh": "Name (optional)",
  "settings.onenewapiUrlPh": "https://host or http://host",
  "settings.onenewapiUrlRequired": "Base URL is required",
  "settings.onenewapiEdit": "Edit",
  "settings.onenewapiDelete": "Delete",
  "settings.onenewapiNoKeys": "No keys yet",
  "settings.onenewapiDeleteTitle": "Delete this site?",
  "settings.onenewapiDeleteBody": "This removes the site and {n} API key(s). This cannot be undone.",
  "settings.onenewapiDeleteConfirm": "Delete site",
  "settings.onenewapiMigrateTitle": "Redirect this site's keys?",
  "settings.onenewapiMigrateBody":
    "{n} API key(s) will be sent to the new origin. Last-good quota for those cards will be dropped.",
  "settings.onenewapiMigrateConfirm": "Redirect keys",
  "settings.onenewapiAddKey": "Add key",
  "settings.onenewapiKeyLabelPh": "Label (optional)",
  "settings.onenewapiKeySecretPh": "API key",
  "settings.onenewapiSaveKey": "Save key",
  "settings.onenewapiDeleteKey": "Delete key",
  "settings.onenewapiKeyKeepHint": "Leave empty to keep the current key",

  "dialog.cancel": "Cancel",
  "dialog.gotIt": "Got it",
  "dialog.changelog": "Changelog",
  "dialog.whatsNew": "What's new in v{version}",

  "card.notConnected": "Not connected",
  "card.outdated": "⚠ Outdated",
  "card.showMore": "Show more",
  "card.showLess": "Show less",
  "card.share": "Copy card as image",
  "card.drag": "Drag to reorder",
  "card.notStarted": "Not started",
  "card.notStartedTip": "Sessions start after you send your first message.",
  "card.resetsSoon": "Resets soon",
  "card.resetsIn": "Resets in {time}",
  "card.resetsAt": "Resets {when}",
  "card.expires": "Expires {when}",
  "card.available": "Available",
  "card.use": "Use",
  "card.useTip": "Spend this credit to reset your Codex rate limits now",
  "card.creditDying": "This credit expires in {time} — use it or lose it.",
  "card.pctUsed": "{n}% used",
  "card.pctLeft": "{n}% left",
  "card.noData": "No data",
  "card.tokens": "{n} tokens",
  "card.tokensEst": "{cost} · {n} tokens · estimated",
  "card.tokensPlain": "{cost} · {n} tokens",

  "pace.limitReached": "🔥 Limit reached",
  "pace.limitReachedTitle": "Limit reached",
  "pace.limitAt": "Limit {when}",
  "pace.limitIn": "Limit in {time}",
  "pace.overReset": "~{n}% over limit at reset",
  "pace.fullReset": "~100% used at reset",
  "pace.spare": "~{n}% spare",
  "pace.usedReset": "~{n}% used at reset",
  "pace.leftReset": "~{n}% left at reset",

  "time.today": "today at {time}",
  "time.tomorrow": "tomorrow at {time}",
  "time.dateAt": "{date} at {time}",
  "time.daysHours": "{d}d {h}h",
  "time.hoursMins": "{h}h {m}m",
  "time.mins": "{m}m",

  "stale.lastFailed": "The last refresh failed",
  "stale.reloginDefault":
    "add the API key again in Settings (or sign in with the tool once)",
  "stale.fixRetry": "Pane keeps retrying automatically — nothing to do unless this persists.",
  "stale.fixDone": "Pane recovers automatically once that's done.",
  "stale.fixRelogin": "Fix: {how} — Pane picks it up on the next refresh.",
  "stale.fix429":
    "The vendor is rate-limiting; Pane waits exactly as long as it asked, then retries by itself.",
  "stale.fix5xx":
    "The vendor's API is having trouble; Pane retries automatically until it recovers.",
  "stale.fixNet":
    "Pane couldn't reach the vendor — check your internet connection (or the proxy in Settings).",
  "stale.tail": "Showing the last good data meanwhile.",
  "stale.relogin.claude": "run `claude` in a terminal and sign in",
  "stale.relogin.codex": "run `codex login` in a terminal",
  "stale.relogin.grok": "run `grok` in a terminal and sign in",
  "stale.relogin.copilot": "run `gh auth login` in a terminal",
  "stale.relogin.cursor": "open Cursor and sign in again",
  "stale.relogin.devin": "run `devin` in a terminal and sign in",
  "stale.relogin.opencode": "run `opencode auth login` in a terminal",
  "stale.relogin.antigravity": "open Antigravity and sign in again",
  "stale.relogin.ollama": "make sure Ollama is running",
  "stale.relogin.hermes":
    "open the Hermes desktop app once so it writes its local ledger",
  "stale.relogin.kimi": "run `kimi login` in a terminal",

  "unpriced.tip":
    "{n} requests ran on models with no public pricing ({models}). Their tokens are included, but they can't be turned into dollars — so the real cost is a little higher than shown.",

  "spend.title": "Total Spend",
  "spend.scanning": "Scanning session logs…",
  "spend.emptyFirst":
    "No spend data yet — appears once Claude Code, Codex, or another CLI logs some usage on this PC.",
  "spend.emptyPeriod": "No spend in this period.",
  "spend.emptyPeriodTip": "No spend recorded in this period.",
  "spend.info":
    "Fed by: {names}. All figures are local estimates from each tool's own logs.",
  "spend.clickTip":
    "{exact} — computed locally from session logs. Click to show {next}.",
  "spend.today": "Today",
  "spend.yesterday": "Yesterday",
  "spend.days30": "30 Days",
  "spend.last30": "Last 30 Days",
  "spend.trend": "Usage Trend",
  "spend.trendTip":
    "Last 30 days ({from} – {to}) · peak {tokens} tokens on {peak} · from local logs",
  "spend.others": "Others",
  "spend.underEach": "Under ${limit} each:",
  "spend.metric.cost": "dollars",
  "spend.metric.mtok": "cost per MTok",
  "spend.metric.tokens": "tokens",
  "spend.centerTokens": "tokens",
  "spend.noUsage": "No usage",
  "spend.of30": "{n}% of the last 30 days",
  "spend.noModelData": "No model data for this period.",

  "metric.session": "Session",
  "metric.weekly": "Weekly",
  "metric.monthly": "Monthly",
  "metric.daily": "Daily",
  "metric.usage": "Usage",
  "metric.credits": "Credits",
  "metric.creditsUsed": "Credits used",
  "metric.api": "API",
  "metric.balance": "Balance",
  "metric.vouchers": "Vouchers",
  "metric.cash": "Cash",
  "metric.limit": "Limit",
  "metric.used": "Used",
  "metric.onDemand": "On-demand",
  "metric.cursorModels": "Cursor Models",
  "metric.otherModels": "Other Models",
  "metric.totalUsage": "Total usage",
  "metric.bonus": "Bonus",
  "metric.extraUsage": "Extra usage",
  "metric.extraCredits": "Extra credits",
  "metric.resetCredit": "Reset credit",
  "metric.resetCreditNumbered": "Reset credit {n}",
  "metric.resetCredits": "Reset credits",
  "metric.extraBalance": "Extra balance",
  "metric.kiloPass": "Kilo Pass",
  "metric.reqToday": "Requests today",
  "metric.reqMonth": "Requests this month",
  "metric.reqCycle": "Requests this cycle",
  "metric.lastUsed": "Last used",
  "metric.recentModels": "Recent models",
  "metric.via": "Via",
  "metric.sessions": "Sessions",
  "metric.expiry": "Expiry",
  "metric.modelWeekly": "{model} weekly",

  "link.Status": "Status",
  "link.Dashboard": "Dashboard",
  "link.Usage": "Usage",
  "link.Credits": "Credits",
  "link.Platform": "Platform",
  "link.Activity": "Activity",
  "link.API Keys": "API Keys",
  "link.Console": "Console",
  "link.Coding Plan": "Coding Plan",
  "link.Library": "Library",
  "link.Site": "Site",
  "link.Quota": "Quota",
  "link.API": "API",

  "welcome.title": "Welcome 👋",
  "welcome.body":
    "You're set up with the AI tools found on this PC. Arrange cards, star tray metrics, and hide rows in Customize.",
  "welcome.open": "Open Customize",
  "welcome.dismiss": "Dismiss",

  "customize.done": "← Done",
  "customize.starred": "{n} starred · drag cards to reorder",
  "customize.resetAll": "↺ Reset all",
  "customize.resetAllTip":
    "Restore all cards' default layouts — does not touch your usage limits",
  "customize.resetLayout": "Reset layout",
  "customize.resetLayoutTip":
    "Restore this card's default layout — does not touch your usage limits",
  "customize.enable": "Enable provider",
  "customize.configure": "Configure",
  "customize.search": "Search providers",
  "customize.cliLoginHint":
    "This service gets its credentials from its own CLI or desktop sign-in — no API key to enter here.",
  "customize.expand": "Expand",
  "customize.collapse": "Collapse",
  "customize.dragRows": "Drag to reorder",
  "customize.star": "Star for tray strip (max 2)",
  "customize.onDemand": "On Demand — behind the card's caret",
  "customize.noData": "No data yet — refresh with this provider enabled first.",
  "customize.resetTitle": "Reset all layouts?",
  "customize.resetBody":
    "Order, stars, and hidden rows go back to defaults, and installed AI tools are re-detected. Your usage limits are not affected.",
  "customize.resetConfirm": "Reset all",
  "customize.credInfo": "Credential info",
  "customize.credAutoLabel": "Reads automatically:",
  "customize.credMethodsLabel": "Sign-in methods:",
  "customize.credStatusLabel": "Current status:",
  "customize.credMethod.paste": "Paste API key",
  "customize.credMethod.oauth": "OAuth sign-in",
  "customize.credMethod.local": "Local CLI / desktop sign-in",
  "customize.credMethodNone": "No credentials needed",
  "customize.credStored": "API key saved",
  "customize.credNotStored": "No key saved",
  "customize.credEnv": "env var detected",
  "customize.credStatusLoading": "Checking…",
  "customize.chipStoredKey": "API key saved",
  "customize.chipEnvKey": "Env var detected",
  "customize.chipLocal": "Local sign-in: {x}",
  "customize.chipNone": "Not configured",
  "customize.chipOAuth": "OAuth: {x}",
  "customize.oauth.login": "Sign in with browser",
  "customize.oauth.loginAlt": "Sign in with browser instead",
  "customize.oauth.code": "Enter this code on the opened page:",
  "customize.oauth.waiting": "Waiting for authorization…",
  "customize.oauth.cancel": "Cancel",
  "customize.oauth.logout": "Sign out",
  "customize.oauth.failed": "Sign-in failed: {err}",
  "customize.credSourcePane": "Pane settings",
  "customize.acctDefaultName": "Account {n}",
  "customize.acctNone": "No extra accounts",
  "customize.acctLabelPh": "Label (optional)",
  "customize.acctAdd": "Add account",
  "customize.acctClose": "Close",
  "customize.acctAddBtn": "Add",
  "customize.acctAdded": "Account added to {name}",
  "customize.acctAddFailed": "Could not add account: {err}",
  "customize.acctRemoved": "Account removed",
  "customize.acctRemoveFailed": "Could not remove account: {err}",
  "customize.acctDelTitle": "Remove this account?",
  "customize.acctDelBody": "The “{label}” card disappears on the next refresh.",
  "customize.acctDelConfirm": "Remove",
  "customize.acctDelete": "Remove account",
  "customize.acctCardHint": "Account card — manage accounts on the {name} row.",
  "customize.loginHint.claude": "Run `claude` in a terminal and log in.",
  "customize.loginHint.codex": "Run `codex login` in a terminal.",
  "customize.loginHint.cursor": "Open Cursor and log in.",
  "customize.loginHint.copilot": "Run `gh auth login`, or sign in to Copilot in an editor.",
  "customize.loginHint.grok": "Sign in with the Grok CLI once (it writes ~/.grok/auth.json).",
  "customize.loginHint.devin": "Sign in with the Devin CLI (it writes credentials.toml).",
  "customize.loginHint.antigravity": "Start Antigravity once and sign in.",
  "customize.loginHint.ollama": "Start the local Ollama service (the app, or `ollama serve`).",
  "customize.loginHint.hermes": "Use the Hermes desktop app — Pane reads its local ledger.",
  "customize.test": "Test",
  "customize.testing": "Testing…",
  "customize.testOk": "OK — {n} metrics",
  "customize.testFailed": "Failed",
  "customize.testEmpty": "Paste a key first",
  "customize.saveAfterTest": "Run a passing test before saving",
  "customize.cred.claude":
    "Claude Code sign-in (~\\.claude); extra config dirs become their own cards",
  "customize.cred.codex": "Codex CLI sign-in (CODEX_HOME, usually ~\\.codex), or Pane's browser sign-in",
  "customize.cred.cursor": "The Cursor editor's local sign-in",
  "customize.cred.opencode":
    "OpenCode's auth.json (opencode-go entry); a pasted Go key also works",
  "customize.cred.copilot": "GitHub Copilot sign-in (gh CLI or the Copilot app)",
  "customize.cred.grok": "Grok CLI sign-in (~\\.grok\\auth.json), or Pane's browser sign-in",
  "customize.cred.devin": "Devin CLI sign-in (%APPDATA%\\devin\\credentials.toml)",
  "customize.cred.minimax": "Saved key, MINIMAX_API_KEY, or the MiniMax Agent CLI config",
  "customize.cred.openrouter": "Saved key, or the OpenRouter key found in OpenCode's auth.json",
  "customize.cred.zai": "Saved key, ZAI_API_KEY / GLM_API_KEY, or the Z.ai CLI key file",
  "customize.cred.antigravity":
    "The Antigravity IDE's local language server / Windows Credential Manager",
  "customize.cred.deepseek": "Nothing on its own — paste a key (or set DEEPSEEK_API_KEY)",
  "customize.cred.moonshot":
    "Nothing on its own — paste a key (or set MOONSHOT_API_KEY / KIMI_API_KEY)",
  "customize.cred.elevenlabs": "Nothing on its own — paste a key (or set ELEVENLABS_API_KEY)",
  "customize.cred.ollama": "Talks to the local Ollama service — no credentials involved",
  "customize.cred.codebuff": "codebuff login credentials, or a pasted key",
  "customize.cred.kilo": "Kilo CLI login, or a pasted key",
  "customize.cred.aihubmix": "Saved key, or the AihubMix entry in OpenCode's auth.json",
  "customize.cred.qwen": "Saved key, or BAILIAN_TOKEN_PLAN_API_KEY / DASHSCOPE_API_KEY",
  "customize.cred.hermes": "Reads Hermes desktop's local ledger — no credentials involved",
  "customize.cred.kimi":
    "Kimi Code CLI OAuth sign-in first; a saved Kimi Coding API key takes over when it dies",
  "customize.cred.stepfun": "Nothing on its own — paste a key (or set STEPFUN_API_KEY)",
  "customize.cred.siliconflow": "Nothing on its own — paste a key (or set SILICONFLOW_API_KEY)",
  "customize.cred.novita": "Nothing on its own — paste a key (or set NOVITA_API_KEY)",
  "customize.cred.relaybalance": "Nothing on its own — paste the relay's base URL and key",

  "redeem.title": "Use a reset credit?",
  "redeem.body":
    "This resets your Codex rate-limit windows immediately and cannot be undone. The refreshed windows can take a couple of minutes to appear.",
  "redeem.confirm": "Use credit",

  "share.tagline": "Monitor Your AI Subscriptions with Pane",
  "tray.left": "{label}: {n}% left",

  "detail.unlimited": "Unlimited",
  "detail.moneyOfLeft": "{a} of {b} left",
  "detail.moneyOfLeftCredits": "{a} of {b} left · {n} credits",
  "detail.moneyLeftOf": "{a} left of {b}",
  "detail.moneyOfUsed": "{a} of {b} used",
  "detail.moneyOfLimit": "{a} of {b} limit",
  "detail.moneyOf": "{a} of {b}",
  "detail.moneyCredits": "{a} · {n} credits",
  "detail.countCreditsUsed": "{a} of {b} credits used",
  "detail.countOfLeft": "{a} of {b} left",
  "detail.countOfUsed": "{a} of {b} used",
};

const zh: Dict = {
  "sidebar.theme": "浅色 / 深色模式",
  "sidebar.themeToDark": "切换到深色模式",
  "sidebar.themeToLight": "切换到浅色模式",
  "sidebar.refresh": "立即刷新",
  "sidebar.customize": "自定义",
  "sidebar.settings": "设置",
  "sidebar.cards": "卡片导航",

  "footer.starting": "正在启动…",
  "footer.refreshing": "正在刷新…",
  "footer.updated": "已更新 {time}",
  "footer.refreshFailed": "刷新失败：{err}",
  "footer.keySaved": "已保存 {name} 密钥",
  "footer.keySaveFailed": "无法保存密钥：{err}",
  "footer.shortcutSaved": "快捷键已保存",
  "footer.shortcutCleared": "快捷键已清除",
  "footer.proxySaved": "代理已保存 — 重启后生效",
  "footer.autostartFailed": "开机启动失败：{err}",
  "footer.copied": "已复制到剪贴板",
  "footer.shareFailed": "分享失败：{err}",
  "footer.updateFailed": "更新失败：{err}",
  "footer.openLinkFailed": "无法打开链接：{err}",
  "footer.twoStars": "每个服务最多加星 2 项",
  "footer.redeeming": "正在兑换重置额度…",
  "footer.redeemFailed": "兑换失败：{err}",
  "footer.configSaveFailed": "配置未保存，重启后可能丢失：{err}",
  "footer.traySyncFailed": "托盘显示更新失败，将自动重试：{err}",
  "footer.onenewapiSaved": "站点已保存",
  "footer.onenewapiDeleted": "站点已删除",
  "footer.onenewapiFailed": "One/New API：{err}",
  "footer.onenewapiNotCompatible": "该站点不是兼容 OneAPI / NewAPI 的面板",
  "footer.onenewapiDuplicate": "该站点已注册",
  "footer.onenewapiProbeFailed": "无法验证该站点：{err}",
  "footer.onenewapiKeySaved": "密钥已保存",

  "update.check": "正在检查更新…",
  "update.to": "⬆ 更新到 v{version}",
  "update.installing": "正在安装…",
  "update.retry": "⬆ 更新到 v{version} — 重试",

  "settings.done": "← 完成",
  "settings.title": "设置",
  "settings.general": "常规",
  "settings.language": "语言",
  "settings.langAuto": "自动",
  "settings.langEn": "English",
  "settings.langZh": "中文",
  "settings.langRu": "Русский",
  "settings.refreshEvery": "刷新间隔",
  "settings.min": "分钟",
  "settings.startWithWindows": "开机启动",
  "settings.pacing": "始终显示消耗速度",
  "settings.trayShows": "托盘图标显示",
  "settings.pinAuto": "自动（第一个可用指标）",
  "settings.pinOption": "{name} — {label}",
  "settings.timeFormat": "时间格式",
  "settings.timeAuto": "自动",
  "settings.time12": "12 小时",
  "settings.time24": "24 小时",
  "settings.appearance": "外观",
  "settings.appearSystem": "跟随系统",
  "settings.appearLight": "浅色",
  "settings.appearDark": "深色",
  "settings.compact": "紧凑布局",
  "settings.glass": "液态玻璃效果",
  "settings.glassTip": "较慢的电脑可以关掉 — 会改成简单的纯色背景",
  "settings.reduceAnim": "减少动画",
  "settings.reduceAnimTip":
    "跳过卡片入场动画和日夜切换过渡。仍会遵守 Windows 自己的“动画效果”设置。",
  "settings.showSpend": "显示总花费卡片",
  "settings.shortcut": "全局快捷键",
  "settings.shortcutPh": "例如 Ctrl+Shift+U",

  "settings.notifications": "通知",
  "settings.notifyNote": "额度变差时弹出 Windows 提醒 — 每个指标在每个重置周期只提醒一次。",
  "settings.notifyAlmost": "即将用完（剩余不足 10%）",
  "settings.notifyClose": "余量紧张",
  "settings.notifyRunout": "将会用完",

  "settings.privacy": "隐私",
  "settings.privacyNote":
    "每天一次匿名的“今天还活着”心跳，以及各服务成功/失败次数，使用一个不绑定任何身份的随机 ID。不上传用量、花费或 IP。详情见 docs/privacy.md。",
  "settings.telemetry": "分享匿名使用统计",
  "settings.hideSharing": "共享屏幕时隐藏托盘数字",
  "settings.hideSharingTip":
    "演示设置、独占全屏或远程控制时，托盘百分比会隐藏。Pane 图标和已加星的服务标志仍在。检测不到 Teams/Zoom 的窗口共享。默认关闭。",

  "settings.network": "网络",
  "settings.useProxy": "使用代理",
  "settings.proxyUrl": "代理地址",
  "settings.networkNote":
    "重启应用后生效。本机用量接口始终运行在 http://127.0.0.1:6736/v1/usage",

  "settings.relayBalance": "自定义余额",
  "settings.relayBalanceNote":
    "适用于开放了 OpenAI 计费接口（/dashboard/billing/subscription + /usage）的中转站。保存时会同时存入两项。",
  "settings.relayBaseUrl": "接口地址",
  "settings.relayApiKey": "API 密钥",

  "settings.apiKeys": "API 密钥",
  "settings.apiKeysNote":
    "只存在这台电脑上（%APPDATA%\\Pane）。留空再保存即可删除。",
  "settings.save": "保存",
  "settings.keyPlaceholder": "API 密钥",
  "settings.keyPhMinimax": "API 密钥（可从 CLI 自动读取）",
  "settings.keyPhMoonshot": "sk-…（platform.kimi.ai 密钥）",
  "settings.keyPhKimi": "Kimi For Coding 订阅密钥（未用 kimi login 时填写）",
  "settings.keyPhCodebuff": "API 密钥（可从 CLI 自动读取）",
  "settings.keyPhKilo": "API 密钥（可从 CLI 自动读取）",
  "settings.keyPhAihubmix": "sk-…（可从 OpenCode 自动读取）",
  "settings.keyPhQwen": "sk-sp-…（可从环境变量自动读取）",
  "settings.keyPhOpencode": "Go 密钥（可从 auth.json 自动读取）",
  "settings.keyPhStepfun": "API 密钥（platform.stepfun.com）",
  "settings.keyPhSiliconflow": "sk-…（siliconflow.cn）",
  "settings.keyPhNovita": "API 密钥（novita.ai）",

  "settings.advanced": "高级",
  "settings.advancedNote":
    "把所有偏好恢复成默认值，并重新检测已安装的工具。API 密钥和用量记录会保留。代理更改仍需重启。",
  "settings.resetAll": "重置全部设置",
  "settings.changelog": "更新说明 · 更新日志",
  "settings.customizeHint":
    "服务开关、行顺序和托盘加星在「自定义」里（侧栏的 ☰）。每个服务最多加星 2 项，作为托盘图标。",

  "settings.resetTitle": "重置全部设置？",
  "settings.resetBody":
    "主题、密度、通知、快捷键、代理、消耗速度、托盘加星和卡片布局都会回到默认。会重新检测已安装的工具。API 密钥和用量记录保留。代理更改仍需重启。",
  "settings.resetConfirm": "全部重置",

  "settings.onenewapi": "One/New API 站点",
  "settings.onenewapiNote":
    "注册兼容 OneAPI / NewAPI 的站点。密钥只保存在这台电脑（%APPDATA%\\Pane\\onenewapi.json），并只发送到该站点。空站点留在这里；添加密钥后才会出现卡片。",
  "settings.onenewapiFamily": "显示 One/New API 卡片",
  "settings.onenewapiFamilyTip":
    "一次关掉所有密钥卡片。自定义里的逐密钥开关会保留。",
  "settings.onenewapiAdd": "添加站点",
  "settings.onenewapiNamePh": "名称（可选）",
  "settings.onenewapiUrlPh": "https://host 或 http://host",
  "settings.onenewapiUrlRequired": "需要填写 Base URL",
  "settings.onenewapiEdit": "编辑",
  "settings.onenewapiDelete": "删除",
  "settings.onenewapiNoKeys": "还没有密钥",
  "settings.onenewapiDeleteTitle": "删除这个站点？",
  "settings.onenewapiDeleteBody": "将删除该站点及其 {n} 个 API 密钥。此操作无法撤销。",
  "settings.onenewapiDeleteConfirm": "删除站点",
  "settings.onenewapiMigrateTitle": "把该站点的密钥迁到新地址？",
  "settings.onenewapiMigrateBody":
    "将把 {n} 个 API 密钥改发到新来源，并丢弃这些卡片上一次成功的额度数据。",
  "settings.onenewapiMigrateConfirm": "迁移密钥",
  "settings.onenewapiAddKey": "添加密钥",
  "settings.onenewapiKeyLabelPh": "备注（可选）",
  "settings.onenewapiKeySecretPh": "API 密钥",
  "settings.onenewapiSaveKey": "保存密钥",
  "settings.onenewapiDeleteKey": "删除密钥",
  "settings.onenewapiKeyKeepHint": "留空则保留现有密钥",

  "dialog.cancel": "取消",
  "dialog.gotIt": "知道了",
  "dialog.changelog": "更新日志",
  "dialog.whatsNew": "v{version} 有什么新内容",

  "card.notConnected": "未连接",
  "card.outdated": "⚠ 数据过时",
  "card.showMore": "显示更多",
  "card.showLess": "收起",
  "card.share": "复制卡片为图片",
  "card.drag": "拖动以排序",
  "card.notStarted": "尚未开始",
  "card.notStartedTip": "发送第一条消息后，会话窗口才会开始计时。",
  "card.resetsSoon": "即将重置",
  "card.resetsIn": "{time}后重置",
  "card.resetsAt": "{when} 重置",
  "card.expires": "{when} 过期",
  "card.available": "可用",
  "card.use": "使用",
  "card.useTip": "立刻用掉这张额度，重置 Codex 速率限制",
  "card.creditDying": "这张额度将在 {time}后过期 — 不用就作废。",
  "card.pctUsed": "已用 {n}%",
  "card.pctLeft": "剩余 {n}%",
  "card.noData": "暂无数据",
  "card.tokens": "{n} tokens",
  "card.tokensEst": "{cost} · {n} tokens · 估算",
  "card.tokensPlain": "{cost} · {n} tokens",

  "pace.limitReached": "🔥 已达上限",
  "pace.limitReachedTitle": "已达上限",
  "pace.limitAt": "{when} 达上限",
  "pace.limitIn": "{time}后达上限",
  "pace.overReset": "重置时大约超出上限 {n}%",
  "pace.fullReset": "重置时大约用满",
  "pace.spare": "大约剩 {n}% 余量",
  "pace.usedReset": "重置时大约用掉 {n}%",
  "pace.leftReset": "重置时大约剩 {n}%",

  "time.today": "今天 {time}",
  "time.tomorrow": "明天 {time}",
  "time.dateAt": "{date} {time}",
  "time.daysHours": "{d} 天 {h} 小时",
  "time.hoursMins": "{h} 小时 {m} 分",
  "time.mins": "{m} 分钟",

  "stale.lastFailed": "上次刷新失败",
  "stale.reloginDefault": "在设置里重新粘贴 API 密钥（或用该工具登录一次）",
  "stale.fixRetry": "Pane 会自动重试 — 除非一直失败，否则不用动手。",
  "stale.fixDone": "完成后 Pane 会自动恢复。",
  "stale.fixRelogin": "解决方法：{how} — 下次刷新时 Pane 会接上。",
  "stale.fix429": "对方在限流；Pane 会按对方要求的时间等待，然后自己重试。",
  "stale.fix5xx": "对方的接口出了问题；Pane 会自动重试直到恢复。",
  "stale.fixNet": "连不上对方 — 请检查网络（或设置里的代理）。",
  "stale.tail": "期间显示上次成功的数据。",
  "stale.relogin.claude": "在终端运行 `claude` 并登录",
  "stale.relogin.codex": "在终端运行 `codex login`",
  "stale.relogin.grok": "在终端运行 `grok` 并登录",
  "stale.relogin.copilot": "在终端运行 `gh auth login`",
  "stale.relogin.cursor": "打开 Cursor 并重新登录",
  "stale.relogin.devin": "在终端运行 `devin` 并登录",
  "stale.relogin.opencode": "在终端运行 `opencode auth login`",
  "stale.relogin.antigravity": "打开 Antigravity 并重新登录",
  "stale.relogin.ollama": "确认 Ollama 正在运行",
  "stale.relogin.hermes": "打开一次 Hermes 桌面应用，让它写本地账本",
  "stale.relogin.kimi": "在终端运行 `kimi login`",

  "unpriced.tip":
    "有 {n} 次请求用了没有公开定价的模型（{models}）。tokens 已计入，但无法换成美元 — 所以真实花费会比显示的略高。",

  "spend.title": "总花费",
  "spend.scanning": "正在扫描会话日志…",
  "spend.emptyFirst":
    "还没有花费数据 — 等这台电脑上的 Claude Code、Codex 或其他 CLI 记下用量后就会出现。",
  "spend.emptyPeriod": "这段时间没有花费。",
  "spend.emptyPeriodTip": "这段时间没有记录花费。",
  "spend.info": "数据来自：{names}。都是根据各工具本地日志估算的。",
  "spend.clickTip": "{exact} — 根据本地会话日志计算。点击可改为显示{next}。",
  "spend.today": "今天",
  "spend.yesterday": "昨天",
  "spend.days30": "30 天",
  "spend.last30": "最近 30 天",
  "spend.trend": "用量趋势",
  "spend.trendTip":
    "最近 30 天（{from} – {to}）· 峰值 {tokens} tokens，在 {peak} · 来自本地日志",
  "spend.others": "其他",
  "spend.underEach": "每项低于 ${limit}：",
  "spend.metric.cost": "美元",
  "spend.metric.mtok": "每百万 tokens 成本",
  "spend.metric.tokens": "tokens",
  "spend.centerTokens": "tokens",
  "spend.noUsage": "无用量",
  "spend.of30": "占最近 30 天的 {n}%",
  "spend.noModelData": "这段时间没有按模型拆分的数据。",

  "metric.session": "会话",
  "metric.weekly": "每周",
  "metric.monthly": "每月",
  "metric.daily": "每天",
  "metric.usage": "用量",
  "metric.credits": "额度",
  "metric.creditsUsed": "已用额度",
  "metric.api": "API",
  "metric.balance": "余额",
  "metric.vouchers": "代金券",
  "metric.cash": "现金",
  "metric.limit": "上限",
  "metric.used": "已用",
  "metric.onDemand": "按量",
  "metric.cursorModels": "Cursor 模型",
  "metric.otherModels": "其他模型",
  "metric.totalUsage": "总用量",
  "metric.bonus": "赠送",
  "metric.extraUsage": "额外用量",
  "metric.extraCredits": "额外额度",
  "metric.resetCredit": "重置额度",
  "metric.resetCreditNumbered": "重置额度 {n}",
  "metric.resetCredits": "重置额度",
  "metric.extraBalance": "额外余额",
  "metric.kiloPass": "Kilo Pass",
  "metric.reqToday": "今日请求",
  "metric.reqMonth": "本月请求",
  "metric.reqCycle": "本周期请求",
  "metric.lastUsed": "上次使用",
  "metric.recentModels": "最近使用的模型",
  "metric.via": "经由",
  "metric.sessions": "会话数",
  "metric.expiry": "到期",
  "metric.modelWeekly": "{model} 每周",

  "link.Status": "状态",
  "link.Dashboard": "控制台",
  "link.Usage": "用量",
  "link.Credits": "额度",
  "link.Platform": "平台",
  "link.Activity": "活动",
  "link.API Keys": "API 密钥",
  "link.Console": "控制台",
  "link.Coding Plan": "编程套餐",
  "link.Library": "模型库",
  "link.Site": "网站",
  "link.Quota": "配额",
  "link.API": "API",

  "welcome.title": "欢迎 👋",
  "welcome.body":
    "已根据这台电脑上的 AI 工具完成设置。可在「自定义」里排列卡片、给托盘指标加星，或隐藏某些行。",
  "welcome.open": "打开自定义",
  "welcome.dismiss": "关闭",

  "customize.done": "← 完成",
  "customize.starred": "已加星 {n} 项 · 在主页拖动卡片排序",
  "customize.resetAll": "↺ 全部重置",
  "customize.resetAllTip": "恢复所有卡片的默认布局 — 不影响你的用量上限",
  "customize.resetLayout": "重置布局",
  "customize.resetLayoutTip": "恢复这张卡片的默认布局 — 不影响你的用量上限",
  "customize.enable": "启用此服务",
  "customize.configure": "配置",
  "customize.search": "搜索服务",
  "customize.cliLoginHint": "该服务通过它自己的终端命令或桌面应用登录提供凭据，这里无需填写 API 密钥。",
  "customize.expand": "展开",
  "customize.collapse": "收起",
  "customize.dragRows": "拖动以排序",
  "customize.star": "加星到托盘（最多 2 项）",
  "customize.onDemand": "更多 — 藏在卡片的下拉箭头后面",
  "customize.noData": "还没有数据 — 先启用此服务并刷新。",
  "customize.resetTitle": "重置全部布局？",
  "customize.resetBody":
    "顺序、加星和隐藏的行会回到默认，并重新检测已安装的 AI 工具。你的用量上限不受影响。",
  "customize.resetConfirm": "全部重置",
  "customize.credInfo": "凭据说明",
  "customize.credAutoLabel": "默认自动读取：",
  "customize.credMethodsLabel": "支持的方式：",
  "customize.credStatusLabel": "当前状态：",
  "customize.credMethod.paste": "粘贴 API key",
  "customize.credMethod.oauth": "OAuth 登录",
  "customize.credMethod.local": "本地 CLI / 桌面端登录",
  "customize.credMethodNone": "无需凭据",
  "customize.credStored": "已保存 API key",
  "customize.credNotStored": "未保存 key",
  "customize.credEnv": "检测到环境变量",
  "customize.credStatusLoading": "查询中…",
  "customize.chipStoredKey": "已存 API key",
  "customize.chipEnvKey": "检测到环境变量",
  "customize.chipLocal": "检测到本地登录：{x}",
  "customize.chipNone": "未配置",
  "customize.chipOAuth": "OAuth 已登录：{x}",
  "customize.oauth.login": "在浏览器登录",
  "customize.oauth.loginAlt": "改用浏览器登录",
  "customize.oauth.code": "在打开的页面输入此代码：",
  "customize.oauth.waiting": "等待授权…",
  "customize.oauth.cancel": "取消",
  "customize.oauth.logout": "退出登录",
  "customize.oauth.failed": "登录失败：{err}",
  "customize.credSourcePane": "Pane 设置",
  "customize.acctDefaultName": "账号 {n}",
  "customize.acctNone": "还没有添加账号",
  "customize.acctLabelPh": "备注名（选填）",
  "customize.acctAdd": "添加账号",
  "customize.acctClose": "收起",
  "customize.acctAddBtn": "添加",
  "customize.acctAdded": "已添加 {name} 账号",
  "customize.acctAddFailed": "添加账号失败：{err}",
  "customize.acctRemoved": "账号已删除",
  "customize.acctRemoveFailed": "删除账号失败：{err}",
  "customize.acctDelTitle": "删除这个账号？",
  "customize.acctDelBody": "「{label}」卡片将在下次刷新时消失。",
  "customize.acctDelConfirm": "删除",
  "customize.acctDelete": "删除账号",
  "customize.acctCardHint": "账号卡片——请在 {name} 一行管理账号。",
  "customize.loginHint.claude": "在终端运行 `claude` 并登录。",
  "customize.loginHint.codex": "在终端运行 `codex login`。",
  "customize.loginHint.cursor": "打开 Cursor 并登录。",
  "customize.loginHint.copilot": "运行 `gh auth login`，或在编辑器里登录 Copilot。",
  "customize.loginHint.grok": "用 Grok CLI 登录一次（写入 ~/.grok/auth.json）。",
  "customize.loginHint.devin": "用 Devin CLI 登录（写入 credentials.toml）。",
  "customize.loginHint.antigravity": "启动 Antigravity 并登录一次。",
  "customize.loginHint.ollama": "启动本机 Ollama 服务（桌面应用或 `ollama serve`）。",
  "customize.loginHint.hermes": "使用 Hermes 桌面端 — Pane 读取它的本地账本。",
  "customize.test": "测试连接",
  "customize.testing": "测试中…",
  "customize.testOk": "成功 — {n} 项数据",
  "customize.testFailed": "失败",
  "customize.testEmpty": "请先粘贴 key",
  "customize.saveAfterTest": "测试成功后才能保存",
  "customize.cred.claude": "Claude Code 登录（~\\.claude）；额外的配置目录会各自成卡",
  "customize.cred.codex": "Codex CLI 登录（CODEX_HOME，通常是 ~\\.codex），或 Pane 浏览器登录",
  "customize.cred.cursor": "Cursor 编辑器的本地登录状态",
  "customize.cred.opencode": "OpenCode 的 auth.json（opencode-go 条目）；也可以粘贴 Go key",
  "customize.cred.copilot": "GitHub Copilot 登录（gh CLI 或 Copilot 应用）",
  "customize.cred.grok": "Grok CLI 登录（~\\.grok\\auth.json），或 Pane 浏览器登录",
  "customize.cred.devin": "Devin CLI 登录（%APPDATA%\\devin\\credentials.toml）",
  "customize.cred.minimax": "已存 key、MINIMAX_API_KEY 或 MiniMax Agent CLI 配置",
  "customize.cred.openrouter": "已存 key，或 OpenCode auth.json 里的 OpenRouter key",
  "customize.cred.zai": "已存 key、ZAI_API_KEY / GLM_API_KEY 或 Z.ai CLI 的 key 文件",
  "customize.cred.antigravity": "Antigravity IDE 本地 language server / Windows 凭据管理器",
  "customize.cred.deepseek": "不会自动读取 — 请粘贴 key（或设置 DEEPSEEK_API_KEY）",
  "customize.cred.moonshot": "不会自动读取 — 请粘贴 key（或设置 MOONSHOT_API_KEY / KIMI_API_KEY）",
  "customize.cred.elevenlabs": "不会自动读取 — 请粘贴 key（或设置 ELEVENLABS_API_KEY）",
  "customize.cred.ollama": "直接访问本机 Ollama 服务 — 无需任何凭据",
  "customize.cred.codebuff": "codebuff 登录凭据，或粘贴的 key",
  "customize.cred.kilo": "Kilo CLI 登录，或粘贴的 key",
  "customize.cred.aihubmix": "已存 key，或 OpenCode auth.json 里的 AihubMix 条目",
  "customize.cred.qwen": "已存 key，或 BAILIAN_TOKEN_PLAN_API_KEY / DASHSCOPE_API_KEY",
  "customize.cred.hermes": "读取 Hermes 桌面端的本地账本 — 无需任何凭据",
  "customize.cred.kimi": "优先用 Kimi Code CLI OAuth 登录；登录失效时已存的 Kimi Coding API key 接管",
  "customize.cred.stepfun": "不会自动读取 — 请粘贴 key（或设置 STEPFUN_API_KEY）",
  "customize.cred.siliconflow": "不会自动读取 — 请粘贴 key（或设置 SILICONFLOW_API_KEY）",
  "customize.cred.novita": "不会自动读取 — 请粘贴 key（或设置 NOVITA_API_KEY）",
  "customize.cred.relaybalance": "不会自动读取 — 请粘贴中转站的 base URL 和 key",

  "redeem.title": "使用一张重置额度？",
  "redeem.body":
    "这会立刻重置 Codex 的速率限制窗口，而且不能撤销。刷新后的窗口可能要一两分钟才显示出来。",
  "redeem.confirm": "使用额度",

  "share.tagline": "用 Pane 盯紧你的 AI 订阅",
  "tray.left": "{label}：剩余 {n}%",

  "detail.unlimited": "无限制",
  "detail.moneyOfLeft": "{a} / {b} 剩余",
  "detail.moneyOfLeftCredits": "{a} / {b} 剩余 · {n} 额度",
  "detail.moneyLeftOf": "{a} / {b} 剩余",
  "detail.moneyOfUsed": "已用 {a} / {b}",
  "detail.moneyOfLimit": "{a} / {b} 上限",
  "detail.moneyOf": "{a} / {b}",
  "detail.moneyCredits": "{a} · {n} 额度",
  "detail.countCreditsUsed": "已用 {a} / {b} 额度",
  "detail.countOfLeft": "{a} / {b} 剩余",
  "detail.countOfUsed": "已用 {a} / {b}",
};

const ru: Dict = {
  "sidebar.theme": "Светлая / тёмная тема",
  "sidebar.themeToDark": "Переключить на тёмную тему",
  "sidebar.themeToLight": "Переключить на светлую тему",
  "sidebar.refresh": "Обновить сейчас",
  "sidebar.customize": "Настройка",
  "sidebar.settings": "Настройки",
  "sidebar.cards": "Навигация по карточкам",

  "footer.starting": "Запуск…",
  "footer.refreshing": "Обновление…",
  "footer.updated": "Обновлено {time}",
  "footer.refreshFailed": "Ошибка обновления: {err}",
  "footer.keySaved": "Ключ {name} сохранён",
  "footer.keySaveFailed": "Не удалось сохранить ключ: {err}",
  "footer.shortcutSaved": "Ярлык сохранён",
  "footer.shortcutCleared": "Ярлык сброшен",
  "footer.proxySaved": "Прокси сохранён — заработает после перезапуска",
  "footer.autostartFailed": "Автозапуск не удался: {err}",
  "footer.copied": "Скопировано в буфер",
  "footer.shareFailed": "Не удалось поделиться: {err}",
  "footer.updateFailed": "Ошибка обновления: {err}",
  "footer.openLinkFailed": "Не удалось открыть ссылку: {err}",
  "footer.twoStars": "Не больше 2 звёзд на сервис",
  "footer.redeeming": "Применяем сброс лимита…",
  "footer.redeemFailed": "Не удалось применить сброс: {err}",
  "footer.configSaveFailed": "Настройки не сохранены и могут быть утеряны после перезапуска: {err}",
  "footer.traySyncFailed": "Не удалось обновить значок в трее; будет повторная попытка: {err}",

  "update.check": "Проверка обновлений…",
  "update.to": "⬆ Обновить до v{version}",
  "update.installing": "Установка…",
  "update.retry": "⬆ Обновить до v{version} — повторить",

  "settings.done": "← Готово",
  "settings.title": "Настройки",
  "settings.general": "Общие",
  "settings.language": "Язык",
  "settings.langAuto": "Авто",
  "settings.langEn": "English",
  "settings.langZh": "中文",
  "settings.langRu": "Русский",
  "settings.refreshEvery": "Обновлять каждые",
  "settings.min": "мин",
  "settings.startWithWindows": "Запускать с Windows",
  "settings.pacing": "Всегда показывать темп",
  "settings.trayShows": "Значок в трее показывает",
  "settings.pinAuto": "Авто (первый живой показатель)",
  "settings.pinOption": "{name} — {label}",
  "settings.timeFormat": "Формат времени",
  "settings.timeAuto": "Авто",
  "settings.time12": "12 часов",
  "settings.time24": "24 часа",
  "settings.appearance": "Оформление",
  "settings.appearSystem": "Как в системе",
  "settings.appearLight": "Светлая",
  "settings.appearDark": "Тёмная",
  "settings.compact": "Компактный вид",
  "settings.glass": "Эффект жидкого стекла",
  "settings.glassTip":
    "На слабых ПК лучше выключить — вместо стекла будет простой фон",
  "settings.reduceAnim": "Меньше анимации",
  "settings.reduceAnimTip":
    "Пропустить появление карточек и переход день/ночь. Настройка Windows «Эффекты анимации» всё равно учитывается.",
  "settings.showSpend": "Показывать карточку общих трат",
  "settings.shortcut": "Глобальная горячая клавиша",
  "settings.shortcutPh": "например Ctrl+Shift+U",

  "settings.notifications": "Уведомления",
  "settings.notifyNote":
    "Всплывающие уведомления Windows, когда квота ухудшается — один раз на показатель за период сброса.",
  "settings.notifyAlmost": "Почти кончилось (осталось <10%)",
  "settings.notifyClose": "Запас на исходе",
  "settings.notifyRunout": "Кончится до сброса",

  "settings.privacy": "Конфиденциальность",
  "settings.privacyNote":
    'Один анонимный сигнал «сегодня жив» и счётчики успеха/сбоя по сервисам, раз в день, под случайным ID без привязки. Без объёмов, трат и IP. Подробности в docs/privacy.md.',
  "settings.telemetry": "Делиться анонимной статистикой",
  "settings.hideSharing": "Скрывать цифры в трее при демонстрации экрана",
  "settings.hideSharingTip":
    "В режиме презентации, полноэкранном режиме или удалённом управлении проценты в трее скрываются. Значок Pane и логотипы со звёздами остаются. Демонстрация окна в Teams/Zoom не определяется. По умолчанию выключено.",

  "settings.network": "Сеть",
  "settings.useProxy": "Использовать прокси",
  "settings.proxyUrl": "Адрес прокси",
  "settings.networkNote":
    "Применяется после перезапуска. Локальный API всегда на http://127.0.0.1:6736/v1/usage",

  "settings.relayBalance": "Свой баланс",
  "settings.relayBalanceNote":
    "Для релеев/шлюзов с OpenAI-совместимыми биллинговыми эндпоинтами (/dashboard/billing/subscription + /usage). Кнопка сохраняет оба поля.",
  "settings.relayBaseUrl": "Базовый URL",
  "settings.relayApiKey": "Ключ API",

  "settings.apiKeys": "Ключи API",
  "settings.apiKeysNote":
    "Хранятся только на этом ПК (%APPDATA%\\Pane). Оставьте пустым и сохраните, чтобы удалить.",
  "settings.save": "Сохранить",
  "settings.keyPlaceholder": "Ключ API",
  "settings.keyPhMinimax": "Ключ API (можно взять из CLI)",
  "settings.keyPhMoonshot": "sk-… (ключ platform.kimi.ai)",
  "settings.keyPhKimi": "Ключ подписки Kimi For Coding (если не используете kimi login)",
  "settings.keyPhCodebuff": "Ключ API (можно взять из CLI)",
  "settings.keyPhKilo": "Ключ API (можно взять из CLI)",
  "settings.keyPhAihubmix": "sk-… (можно взять из OpenCode)",
  "settings.keyPhQwen": "sk-sp-… (можно взять из переменных среды)",
  "settings.keyPhOpencode": "Ключ Go (можно взять из auth.json)",
  "settings.keyPhStepfun": "Ключ API (platform.stepfun.com)",
  "settings.keyPhSiliconflow": "sk-… (siliconflow.cn)",
  "settings.keyPhNovita": "Ключ API (novita.ai)",

  "settings.advanced": "Дополнительно",
  "settings.advancedNote":
    "Вернёт все настройки к значениям по умолчанию и заново найдёт установленные инструменты. Ключи API и история использования останутся. Смена прокси по-прежнему требует перезапуска.",
  "settings.resetAll": "Сбросить все настройки",
  "settings.changelog": "Что нового · Журнал изменений",
  "settings.customizeHint":
    "Сервисы, порядок строк и звёзды трея — в «Настройка» (☰ в боковой панели). Там можно отметить до 2 показателей на сервис — они появятся значками в трее.",

  "settings.resetTitle": "Сбросить все настройки?",
  "settings.resetBody":
    "Тема, плотность, уведомления, ярлык, прокси, темп, звёзды трея и раскладки карточек вернутся к значениям по умолчанию. Установленные инструменты будут найдены заново. Ключи API и история использования останутся. Смена прокси по-прежнему требует перезапуска.",
  "settings.resetConfirm": "Сбросить всё",

  "dialog.cancel": "Отмена",
  "dialog.gotIt": "Понятно",
  "dialog.changelog": "Журнал изменений",
  "dialog.whatsNew": "Что нового в v{version}",

  "card.notConnected": "Не подключено",
  "card.outdated": "⚠ Данные устарели",
  "card.showMore": "Ещё",
  "card.showLess": "Свернуть",
  "card.share": "Скопировать карточку как изображение",
  "card.drag": "Перетащите, чтобы изменить порядок",
  "card.notStarted": "Ещё не началось",
  "card.notStartedTip": "Окно начнётся после первого сообщения.",
  "card.resetsSoon": "Скоро сброс",
  "card.resetsIn": "Сброс через {time}",
  "card.resetsAt": "Сброс {when}",
  "card.expires": "Истекает {when}",
  "card.available": "Доступно",
  "card.use": "Использовать",
  "card.useTip": "Потратить этот кредит, чтобы сразу сбросить лимиты Codex",
  "card.creditDying": "Этот кредит истечёт через {time} — используйте или пропадёт.",
  "card.pctUsed": "Использовано {n}%",
  "card.pctLeft": "Осталось {n}%",
  "card.noData": "Нет данных",
  "card.tokens": "{n} tokens",
  "card.tokensEst": "{cost} · {n} tokens · оценка",
  "card.tokensPlain": "{cost} · {n} tokens",

  "pace.limitReached": "🔥 Лимит исчерпан",
  "pace.limitReachedTitle": "Лимит исчерпан",
  "pace.limitAt": "Лимит {when}",
  "pace.limitIn": "Лимит через {time}",
  "pace.overReset": "~{n}% сверх лимита к сбросу",
  "pace.fullReset": "~100% к сбросу",
  "pace.spare": "запас ~{n}%",
  "pace.usedReset": "~{n}% будет использовано к сбросу",
  "pace.leftReset": "~{n}% останется к сбросу",

  "time.today": "сегодня в {time}",
  "time.tomorrow": "завтра в {time}",
  "time.dateAt": "{date} в {time}",
  "time.daysHours": "{d}д {h}ч",
  "time.hoursMins": "{h}ч {m}м",
  "time.mins": "{m}м",

  "stale.lastFailed": "Последнее обновление не удалось",
  "stale.reloginDefault":
    "снова вставьте ключ API в Настройках (или войдите в инструмент один раз)",
  "stale.fixRetry": "Pane сам повторяет попытки — ничего делать не нужно, пока это не затянется.",
  "stale.fixDone": "Pane восстановится сам, как только это будет сделано.",
  "stale.fixRelogin": "Что сделать: {how} — Pane подхватит это при следующем обновлении.",
  "stale.fix429":
    "Сервис ограничивает частоту запросов; Pane ждёт ровно столько, сколько просили, и повторяет сам.",
  "stale.fix5xx":
    "У сервиса сбой API; Pane повторяет попытки, пока тот не восстановится.",
  "stale.fixNet":
    "Pane не достучался до сервиса — проверьте интернет (или прокси в Настройках).",
  "stale.tail": "Пока показываем последние хорошие данные.",
  "stale.relogin.claude": "запустите `claude` в терминале и войдите",
  "stale.relogin.codex": "запустите `codex login` в терминале",
  "stale.relogin.grok": "запустите `grok` в терминале и войдите",
  "stale.relogin.copilot": "запустите `gh auth login` в терминале",
  "stale.relogin.cursor": "откройте Cursor и войдите снова",
  "stale.relogin.devin": "запустите `devin` в терминале и войдите",
  "stale.relogin.opencode": "запустите `opencode auth login` в терминале",
  "stale.relogin.antigravity": "откройте Antigravity и войдите снова",
  "stale.relogin.ollama": "убедитесь, что Ollama запущена",
  "stale.relogin.hermes":
    "откройте приложение Hermes один раз, чтобы оно записало локальный журнал",
  "stale.relogin.kimi": "запустите `kimi login` в терминале",

  "unpriced.tip":
    "{n} запросов шли на модели без публичных цен ({models}). Токены учтены, но в доллары их не перевести — реальная стоимость чуть выше, чем на экране.",

  "spend.title": "Всего потрачено",
  "spend.scanning": "Сканирование журналов сессий…",
  "spend.emptyFirst":
    "Пока нет данных о тратах — появятся, когда Claude Code, Codex или другой CLI запишет использование на этом ПК.",
  "spend.emptyPeriod": "За этот период трат нет.",
  "spend.emptyPeriodTip": "За этот период траты не записаны.",
  "spend.info":
    "Источники: {names}. Все цифры — локальные оценки по журналам самих инструментов.",
  "spend.clickTip":
    "{exact} — посчитано локально по журналам сессий. Нажмите, чтобы показать {next}.",
  "spend.today": "Сегодня",
  "spend.yesterday": "Вчера",
  "spend.days30": "30 дней",
  "spend.last30": "Последние 30 дней",
  "spend.trend": "Динамика",
  "spend.trendTip":
    "Последние 30 дней ({from} – {to}) · пик {tokens} tokens {peak} · из локальных журналов",
  "spend.others": "Другие",
  "spend.underEach": "Каждый меньше ${limit}:",
  "spend.metric.cost": "доллары",
  "spend.metric.mtok": "стоимость за млн токенов",
  "spend.metric.tokens": "tokens",
  "spend.centerTokens": "tokens",
  "spend.noUsage": "Нет использования",
  "spend.of30": "{n}% за последние 30 дней",
  "spend.noModelData": "За этот период нет разбивки по моделям.",

  "metric.session": "Сессия",
  "metric.weekly": "За неделю",
  "metric.monthly": "За месяц",
  "metric.daily": "За день",
  "metric.usage": "Использование",
  "metric.credits": "Кредиты",
  "metric.creditsUsed": "Использовано кредитов",
  "metric.api": "API",
  "metric.balance": "Баланс",
  "metric.vouchers": "Ваучеры",
  "metric.cash": "Наличные",
  "metric.limit": "Лимит",
  "metric.used": "Использовано",
  "metric.onDemand": "По факту",
  "metric.cursorModels": "Модели Cursor",
  "metric.otherModels": "Другие модели",
  "metric.totalUsage": "Всего",
  "metric.bonus": "Бонус",
  "metric.extraUsage": "Дополнительно",
  "metric.extraCredits": "Доп. кредиты",
  "metric.resetCredit": "Сброс лимита",
  "metric.resetCreditNumbered": "Сброс лимита {n}",
  "metric.resetCredits": "Сброс лимита",
  "metric.extraBalance": "Доп. баланс",
  "metric.kiloPass": "Kilo Pass",
  "metric.reqToday": "Запросы сегодня",
  "metric.reqMonth": "Запросы в этом месяце",
  "metric.reqCycle": "Запросы за цикл",
  "metric.lastUsed": "Последнее использование",
  "metric.recentModels": "Недавние модели",
  "metric.via": "Через",
  "metric.sessions": "Сессии",
  "metric.modelWeekly": "{model} за неделю",

  "link.Status": "Статус",
  "link.Dashboard": "Панель",
  "link.Usage": "Использование",
  "link.Credits": "Кредиты",
  "link.Platform": "Платформа",
  "link.Activity": "Активность",
  "link.API Keys": "Ключи API",
  "link.Console": "Консоль",
  "link.Coding Plan": "Тариф для кода",
  "link.Library": "Библиотека",
  "link.Site": "Сайт",
  "link.Quota": "Квота",
  "link.API": "API",

  "welcome.title": "Добро пожаловать 👋",
  "welcome.body":
    "Настроены инструменты ИИ, найденные на этом ПК. Карточки, звёзды трея и скрытые строки — в «Настройка».",
  "welcome.open": "Открыть настройку",
  "welcome.dismiss": "Закрыть",

  "customize.done": "← Готово",
  "customize.starred": "Отмечено {n} · порядок меняется перетаскиванием карточек",
  "customize.resetAll": "↺ Сбросить всё",
  "customize.resetAllTip":
    "Вернуть раскладки всех карточек по умолчанию — лимиты использования не трогает",
  "customize.resetLayout": "Сбросить раскладку",
  "customize.resetLayoutTip":
    "Вернуть раскладку этой карточки по умолчанию — лимиты использования не трогает",
  "customize.enable": "Включить сервис",
  "customize.configure": "Настроить",
  "customize.search": "Поиск сервисов",
  "customize.cliLoginHint":
    "Этот сервис получает учётные данные через вход в свой CLI или настольное приложение — ключ API здесь не нужен.",
  "customize.expand": "Развернуть",
  "customize.collapse": "Свернуть",
  "customize.dragRows": "Перетащите, чтобы изменить порядок",
  "customize.star": "Звезда в трее (не больше 2)",
  "customize.onDemand": "Ещё — за стрелкой на карточке",
  "customize.noData": "Пока нет данных — сначала включите этот сервис и обновите.",
  "customize.resetTitle": "Сбросить все раскладки?",
  "customize.resetBody":
    "Порядок, звёзды и скрытые строки вернутся к значениям по умолчанию, установленные инструменты ИИ будут найдены заново. Лимиты использования не изменятся.",
  "customize.resetConfirm": "Сбросить всё",
  "customize.credInfo": "Данные для входа",
  "customize.credAutoLabel": "Читает автоматически:",
  "customize.credMethodsLabel": "Способы входа:",
  "customize.credStatusLabel": "Текущий статус:",
  "customize.credMethod.paste": "Вставить API-ключ",
  "customize.credMethod.oauth": "Вход через OAuth",
  "customize.credMethod.local": "Локальный CLI / приложение",
  "customize.credMethodNone": "Учётные данные не нужны",
  "customize.credStored": "API-ключ сохранён",
  "customize.credNotStored": "Ключ не сохранён",
  "customize.credEnv": "найдена переменная окружения",
  "customize.credStatusLoading": "Проверяем…",
  "customize.chipStoredKey": "API-ключ сохранён",
  "customize.chipEnvKey": "Найдена переменная окружения",
  "customize.chipLocal": "Локальный вход: {x}",
  "customize.chipNone": "Не настроено",
  "customize.chipOAuth": "OAuth: {x}",
  "customize.oauth.login": "Войти через браузер",
  "customize.oauth.loginAlt": "Вместо этого войти через браузер",
  "customize.oauth.code": "Введите этот код на открытой странице:",
  "customize.oauth.waiting": "Ожидание авторизации…",
  "customize.oauth.cancel": "Отмена",
  "customize.oauth.logout": "Выйти",
  "customize.oauth.failed": "Ошибка входа: {err}",
  "customize.credSourcePane": "настройки Pane",
  "customize.acctDefaultName": "Аккаунт {n}",
  "customize.acctNone": "Дополнительных аккаунтов нет",
  "customize.acctLabelPh": "Метка (необязательно)",
  "customize.acctAdd": "Добавить аккаунт",
  "customize.acctClose": "Свернуть",
  "customize.acctAddBtn": "Добавить",
  "customize.acctAdded": "Аккаунт добавлен: {name}",
  "customize.acctAddFailed": "Не удалось добавить аккаунт: {err}",
  "customize.acctRemoved": "Аккаунт удалён",
  "customize.acctRemoveFailed": "Не удалось удалить аккаунт: {err}",
  "customize.acctDelTitle": "Удалить этот аккаунт?",
  "customize.acctDelBody": "Карточка «{label}» исчезнет при следующем обновлении.",
  "customize.acctDelConfirm": "Удалить",
  "customize.acctDelete": "Удалить аккаунт",
  "customize.acctCardHint": "Карточка аккаунта — управляйте аккаунтами в строке {name}.",
  "customize.loginHint.claude": "Запустите `claude` в терминале и войдите.",
  "customize.loginHint.codex": "Запустите `codex login` в терминале.",
  "customize.loginHint.cursor": "Откройте Cursor и войдите.",
  "customize.loginHint.copilot": "Выполните `gh auth login` или войдите в Copilot в редакторе.",
  "customize.loginHint.grok": "Войдите через Grok CLI один раз (создаёт ~/.grok/auth.json).",
  "customize.loginHint.devin": "Войдите через Devin CLI (создаёт credentials.toml).",
  "customize.loginHint.antigravity": "Запустите Antigravity и войдите.",
  "customize.loginHint.ollama": "Запустите локальный сервис Ollama (приложение или `ollama serve`).",
  "customize.loginHint.hermes": "Используйте Hermes desktop — Pane читает его локальный журнал.",
  "customize.test": "Проверить",
  "customize.testing": "Проверяем…",
  "customize.testOk": "Готово — показателей: {n}",
  "customize.testFailed": "Ошибка",
  "customize.testEmpty": "Сначала вставьте ключ",
  "customize.saveAfterTest": "Сначала успешно проверьте ключ",
  "customize.cred.claude":
    "Вход в Claude Code (~\\.claude); дополнительные каталоги конфигурации становятся отдельными карточками",
  "customize.cred.codex": "Вход в Codex CLI (CODEX_HOME, обычно ~\\.codex) или вход через браузер в Pane",
  "customize.cred.cursor": "Локальный вход редактора Cursor",
  "customize.cred.opencode": "auth.json OpenCode (запись opencode-go); подойдёт и вставленный Go-ключ",
  "customize.cred.copilot": "Вход GitHub Copilot (gh CLI или приложение Copilot)",
  "customize.cred.grok": "Вход Grok CLI (~\\.grok\\auth.json) или вход через браузер в Pane",
  "customize.cred.devin": "Вход Devin CLI (%APPDATA%\\devin\\credentials.toml)",
  "customize.cred.minimax": "Сохранённый ключ, MINIMAX_API_KEY или конфиг MiniMax Agent CLI",
  "customize.cred.openrouter": "Сохранённый ключ или ключ OpenRouter из auth.json OpenCode",
  "customize.cred.zai": "Сохранённый ключ, ZAI_API_KEY / GLM_API_KEY или файл ключа Z.ai CLI",
  "customize.cred.antigravity":
    "Локальный language server Antigravity / диспетчер учётных данных Windows",
  "customize.cred.deepseek": "Сам ничего не читает — вставьте ключ (или задайте DEEPSEEK_API_KEY)",
  "customize.cred.moonshot":
    "Сам ничего не читает — вставьте ключ (или задайте MOONSHOT_API_KEY / KIMI_API_KEY)",
  "customize.cred.elevenlabs": "Сам ничего не читает — вставьте ключ (или задайте ELEVENLABS_API_KEY)",
  "customize.cred.ollama": "Обращается к локальной службе Ollama — учётные данные не нужны",
  "customize.cred.codebuff": "Данные входа codebuff или вставленный ключ",
  "customize.cred.kilo": "Вход Kilo CLI или вставленный ключ",
  "customize.cred.aihubmix": "Сохранённый ключ или запись AihubMix в auth.json OpenCode",
  "customize.cred.qwen": "Сохранённый ключ или BAILIAN_TOKEN_PLAN_API_KEY / DASHSCOPE_API_KEY",
  "customize.cred.hermes": "Читает локальный журнал Hermes desktop — учётные данные не нужны",
  "customize.cred.kimi":
    "Сначала OAuth-вход Kimi Code CLI; при его сбое подхватывается сохранённый Kimi Coding API-ключ",
  "customize.cred.stepfun": "Сам ничего не читает — вставьте ключ (или задайте STEPFUN_API_KEY)",
  "customize.cred.siliconflow": "Сам ничего не читает — вставьте ключ (или задайте SILICONFLOW_API_KEY)",
  "customize.cred.novita": "Сам ничего не читает — вставьте ключ (или задайте NOVITA_API_KEY)",
  "customize.cred.relaybalance": "Сам ничего не читает — вставьте base URL и ключ релея",

  "redeem.title": "Использовать сброс лимита?",
  "redeem.body":
    "Это сразу сбросит окна лимитов Codex и отменить нельзя. Обновлённые окна могут появиться через пару минут.",
  "redeem.confirm": "Использовать",

  "share.tagline": "Следите за подписками ИИ с Pane",
  "tray.left": "{label}: осталось {n}%",

  "detail.unlimited": "Без лимита",
  "detail.moneyOfLeft": "{a} / {b} осталось",
  "detail.moneyOfLeftCredits": "{a} / {b} осталось · {n} кредитов",
  "detail.moneyLeftOf": "{a} / {b} осталось",
  "detail.moneyOfUsed": "Использовано {a} / {b}",
  "detail.moneyOfLimit": "{a} / {b} лимит",
  "detail.moneyOf": "{a} / {b}",
  "detail.moneyCredits": "{a} · {n} кредитов",
  "detail.countCreditsUsed": "Использовано {a} / {b} кредитов",
  "detail.countOfLeft": "{a} / {b} осталось",
  "detail.countOfUsed": "Использовано {a} / {b}",
};

const METRIC_KEYS: Record<string, string> = {
  Session: "metric.session",
  Weekly: "metric.weekly",
  Monthly: "metric.monthly",
  Daily: "metric.daily",
  Usage: "metric.usage",
  Credits: "metric.credits",
  "Credits used": "metric.creditsUsed",
  API: "metric.api",
  Balance: "metric.balance",
  Vouchers: "metric.vouchers",
  Cash: "metric.cash",
  Limit: "metric.limit",
  Used: "metric.used",
  "On-demand": "metric.onDemand",
  "Cursor Models": "metric.cursorModels",
  "Other Models": "metric.otherModels",
  "Total usage": "metric.totalUsage",
  Bonus: "metric.bonus",
  "Extra usage": "metric.extraUsage",
  "Extra credits": "metric.extraCredits",
  "Reset credits": "metric.resetCredits",
  "Extra balance": "metric.extraBalance",
  "Kilo Pass": "metric.kiloPass",
  "Requests today": "metric.reqToday",
  "Requests this month": "metric.reqMonth",
  "Requests this cycle": "metric.reqCycle",
  "Last used": "metric.lastUsed",
  "Recent models": "metric.recentModels",
  Via: "metric.via",
  Sessions: "metric.sessions",
  Expiry: "metric.expiry",
  "Usage Trend": "spend.trend",
  Today: "spend.today",
  Yesterday: "spend.yesterday",
  "Last 30 Days": "spend.last30",
  Others: "spend.others",
};

let active: Locale = "en";
/// Filled from Rust `system_ui_locale` so Auto matches tray/toasts.
let systemLocale: Locale | null = null;

export function detectSystemLocale(): Locale {
  const lang = (navigator.language || "").toLowerCase();
  if (lang.startsWith("zh")) return "zh";
  if (lang.startsWith("ru")) return "ru";
  return "en";
}

export function setSystemLocale(locale: Locale): void {
  systemLocale = locale;
}

export function resolveLocale(pref: string | undefined): Locale {
  if (pref === "zh" || pref === "en" || pref === "ru") return pref;
  return systemLocale ?? detectSystemLocale();
}

export function normalizeLocalePref(raw: unknown): LocalePref {
  return raw === "en" || raw === "zh" || raw === "ru" || raw === "auto" ? raw : "auto";
}

export function setActiveLocale(locale: Locale): void {
  active = locale;
}

export function getLocale(): Locale {
  return active;
}

export function localeTag(): string {
  if (active === "zh") return "zh-CN";
  if (active === "ru") return "ru-RU";
  return "en-US";
}

export function t(key: string, vars?: Record<string, string | number>): string {
  const dict = active === "zh" ? zh : active === "ru" ? ru : en;
  let s = dict[key] ?? en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      s = s.split(`{${k}}`).join(String(v));
    }
  }
  return s;
}

export function displayMetricLabel(label: string): string {
  const key = METRIC_KEYS[label];
  if (key) return t(key);
  const resetCredit = label.match(/^Reset credit(?: (\d+))?$/);
  if (resetCredit) {
    return resetCredit[1]
      ? t("metric.resetCreditNumbered", { n: resetCredit[1] })
      : t("metric.resetCredit");
  }
  if (label.endsWith(" weekly")) {
    return t("metric.modelWeekly", { model: label.slice(0, -7) });
  }
  return label;
}

export function displayLinkLabel(label: string): string {
  const key = `link.${label}`;
  const translated = t(key);
  return translated === key ? label : translated;
}

/// Rust still emits English captions ("$21.80 of $79.56 left · 545 credits").
/// Translate the known shapes at paint time so layout keys stay English.
export function displayMetricDetail(text: string): string {
  if (getLocale() === "en" || !text) return text;
  const money = "\\$[\\d,]+(?:\\.\\d+)?K?";
  const num = "[\\d,]+(?:\\.\\d+)?";
  let m = text.match(new RegExp(`^(${money}) of (${money}) left(?: · (\\d+) credits)?$`, "i"));
  if (m) {
    return m[3]
      ? t("detail.moneyOfLeftCredits", { a: m[1], b: m[2], n: m[3] })
      : t("detail.moneyOfLeft", { a: m[1], b: m[2] });
  }
  m = text.match(new RegExp(`^(${money}) left of (${money})$`, "i"));
  if (m) return t("detail.moneyLeftOf", { a: m[1], b: m[2] });
  m = text.match(new RegExp(`^(${money}) of (${money}) used$`, "i"));
  if (m) return t("detail.moneyOfUsed", { a: m[1], b: m[2] });
  m = text.match(new RegExp(`^(${money}) of (${money}) limit$`, "i"));
  if (m) return t("detail.moneyOfLimit", { a: m[1], b: m[2] });
  m = text.match(new RegExp(`^(${money}) of (${money})$`, "i"));
  if (m) return t("detail.moneyOf", { a: m[1], b: m[2] });
  m = text.match(new RegExp(`^(${money}) · (\\d+) credits$`, "i"));
  if (m) return t("detail.moneyCredits", { a: m[1], n: m[2] });
  m = text.match(new RegExp(`^(${num}) of (${num}) credits used$`, "i"));
  if (m) return t("detail.countCreditsUsed", { a: m[1], b: m[2] });
  m = text.match(new RegExp(`^(${num}) of (${num}) used$`, "i"));
  if (m) return t("detail.countOfUsed", { a: m[1], b: m[2] });
  m = text.match(new RegExp(`^(${num}) of (${num}) left$`, "i"));
  if (m) return t("detail.countOfLeft", { a: m[1], b: m[2] });
  if (/^available$/i.test(text.trim())) return t("card.available");
  if (/^unlimited$/i.test(text.trim())) return t("detail.unlimited");
  return text;
}

export function applyStaticI18n(): void {
  document.documentElement.lang = localeTag();
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const key = el.dataset.i18n;
    if (key) el.textContent = t(key);
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-html]").forEach((el) => {
    const key = el.dataset.i18nHtml;
    if (key) el.innerHTML = t(key);
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-title]").forEach((el) => {
    const key = el.dataset.i18nTitle;
    if (key) {
      el.title = t(key);
      delete el.dataset.tip;
    }
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-placeholder]").forEach((el) => {
    const key = el.dataset.i18nPlaceholder;
    if (key && "placeholder" in el) {
      (el as HTMLInputElement).placeholder = t(key);
    }
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-aria]").forEach((el) => {
    const key = el.dataset.i18nAria;
    if (key) el.setAttribute("aria-label", t(key));
  });
}
