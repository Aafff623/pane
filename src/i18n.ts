// Frontend locale: Settings stores auto / en / zh. Metric row labels from
// Rust stay English in config.layout (stars, pins, Customize keys); only
// the painted text switches.

export type LocalePref = "auto" | "en" | "zh";
export type Locale = "en" | "zh";

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

  "settings.apiKeys": "API keys",
  "settings.apiKeysNote":
    "Stored only on this PC (%APPDATA%\\Pane). Leave empty and save to remove.",
  "settings.save": "Save",
  "settings.keyPlaceholder": "API key",
  "settings.keyPhMinimax": "API key (auto-detected from CLI)",
  "settings.keyPhMoonshot": "sk-… (platform.kimi.ai key)",
  "settings.keyPhCodebuff": "API key (auto-detected from CLI)",
  "settings.keyPhKilo": "API key (auto-detected from CLI)",
  "settings.keyPhAihubmix": "sk-… (auto-detected from OpenCode)",
  "settings.keyPhQwen": "sk-sp-… (auto-detected from env)",

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
  "metric.resetCredits": "Reset credits",
  "metric.extraBalance": "Extra balance",
  "metric.kiloPass": "Kilo Pass",
  "metric.reqToday": "Requests today",
  "metric.reqMonth": "Requests this month",
  "metric.reqCycle": "Requests this cycle",
  "metric.lastUsed": "Last used",
  "metric.via": "Via",
  "metric.sessions": "Sessions",
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
  "customize.starred": "{n} starred · drag ⠿ to reorder",
  "customize.resetAll": "↺ Reset all",
  "customize.resetAllTip":
    "Restore all cards' default layouts — does not touch your usage limits",
  "customize.resetLayout": "Reset layout",
  "customize.resetLayoutTip":
    "Restore this card's default layout — does not touch your usage limits",
  "customize.enable": "Enable provider",
  "customize.expand": "Expand",
  "customize.collapse": "Collapse",
  "customize.dragRows": "Drag to reorder",
  "customize.dragProviders": "Drag to reorder providers",
  "customize.star": "Star for tray strip (max 2)",
  "customize.onDemand": "On Demand — behind the card's caret",
  "customize.noData": "No data yet — refresh with this provider enabled first.",
  "customize.resetTitle": "Reset all layouts?",
  "customize.resetBody":
    "Order, stars, and hidden rows go back to defaults, and installed AI tools are re-detected. Your usage limits are not affected.",
  "customize.resetConfirm": "Reset all",

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

  "settings.apiKeys": "API 密钥",
  "settings.apiKeysNote":
    "只存在这台电脑上（%APPDATA%\\Pane）。留空再保存即可删除。",
  "settings.save": "保存",
  "settings.keyPlaceholder": "API 密钥",
  "settings.keyPhMinimax": "API 密钥（可从 CLI 自动读取）",
  "settings.keyPhMoonshot": "sk-…（platform.kimi.ai 密钥）",
  "settings.keyPhCodebuff": "API 密钥（可从 CLI 自动读取）",
  "settings.keyPhKilo": "API 密钥（可从 CLI 自动读取）",
  "settings.keyPhAihubmix": "sk-…（可从 OpenCode 自动读取）",
  "settings.keyPhQwen": "sk-sp-…（可从环境变量自动读取）",

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
  "metric.resetCredits": "重置额度",
  "metric.extraBalance": "额外余额",
  "metric.kiloPass": "Kilo Pass",
  "metric.reqToday": "今日请求",
  "metric.reqMonth": "本月请求",
  "metric.reqCycle": "本周期请求",
  "metric.lastUsed": "上次使用",
  "metric.via": "经由",
  "metric.sessions": "会话数",
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
  "customize.starred": "已加星 {n} 项 · 拖动 ⠿ 排序",
  "customize.resetAll": "↺ 全部重置",
  "customize.resetAllTip": "恢复所有卡片的默认布局 — 不影响你的用量上限",
  "customize.resetLayout": "重置布局",
  "customize.resetLayoutTip": "恢复这张卡片的默认布局 — 不影响你的用量上限",
  "customize.enable": "启用此服务",
  "customize.expand": "展开",
  "customize.collapse": "收起",
  "customize.dragRows": "拖动以排序",
  "customize.dragProviders": "拖动以调整服务顺序",
  "customize.star": "加星到托盘（最多 2 项）",
  "customize.onDemand": "更多 — 藏在卡片的下拉箭头后面",
  "customize.noData": "还没有数据 — 先启用此服务并刷新。",
  "customize.resetTitle": "重置全部布局？",
  "customize.resetBody":
    "顺序、加星和隐藏的行会回到默认，并重新检测已安装的 AI 工具。你的用量上限不受影响。",
  "customize.resetConfirm": "全部重置",

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
  Via: "metric.via",
  Sessions: "metric.sessions",
  "Usage Trend": "spend.trend",
  Today: "spend.today",
  Yesterday: "spend.yesterday",
  "Last 30 Days": "spend.last30",
  Others: "spend.others",
};

let active: Locale = "en";

export function detectSystemLocale(): Locale {
  return (navigator.language || "").toLowerCase().startsWith("zh") ? "zh" : "en";
}

export function resolveLocale(pref: string | undefined): Locale {
  if (pref === "zh" || pref === "en") return pref;
  return detectSystemLocale();
}

export function normalizeLocalePref(raw: unknown): LocalePref {
  return raw === "en" || raw === "zh" || raw === "auto" ? raw : "auto";
}

export function setActiveLocale(locale: Locale): void {
  active = locale;
}

export function getLocale(): Locale {
  return active;
}

export function localeTag(): string {
  return active === "zh" ? "zh-CN" : "en-US";
}

export function t(key: string, vars?: Record<string, string | number>): string {
  const dict = active === "zh" ? zh : en;
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
  if (getLocale() !== "zh" || !text) return text;
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
  if (/^unlimited$/i.test(text.trim())) return t("detail.unlimited");
  return text;
}

export function applyStaticI18n(): void {
  document.documentElement.lang = active === "zh" ? "zh-CN" : "en";
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
