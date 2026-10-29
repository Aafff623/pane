export type Lang = "zh" | "en";

export type Stage = { title: string; body: string };
export type QA = { q: string; a: string };

type Dict = {
  meta: { title: string; description: string; ogAlt: string };
  nav: {
    features: string;
    demo: string;
    faq: string;
    github: string;
    download: string;
    language: string;
    theme: string;
    menu: string;
    closeMenu: string;
    primary: string;
    skip: string;
  };
  hero: {
    eyebrow: string;
    titleLead: string;
    titleFocus: string;
    description: string;
    download: string;
    demo: string;
    stats: { providers: string; accounts: string; license: string };
    caption: string;
    imageAlt: string;
  };
  workflow: {
    eyebrow: string;
    title: string;
    description: string;
    leftTitle: string;
    leftWindow: string;
    leftHeading: string;
    leftBody: string;
    leftFoot: string;
    bridge: string;
    bridgeNote: string;
    rightTitle: string;
    rightWindow: string;
    rightHeading: string;
    rightBody: string;
    rightFoot: string;
  };
  panel: {
    eyebrow: string;
    title: string;
    description: string;
    stages: Stage[];
    hint: string;
    imageAlt: string;
  };
  faq: { title: string; description: string; items: QA[]; more: string; moreLink: string };
  demo: { eyebrow: string; title: string; description: string; open: string; reset: string; note: string; iframeTitle: string };
  download: {
    eyebrow: string;
    title: string;
    description: string;
    windows: string;
    macos: string;
    macosCta: string;
    linux: string;
    facts: { title: string; body: string }[];
    winget: string;
    mirror: string;
    source: string;
    license: string;
    versionLabel: string;
    platform: string;
  };
  closing: { eyebrow: string; title: string; description: string; download: string; demo: string };
  footer: {
    product: string;
    productLinks: { label: string; href: string }[];
    resources: string;
    resourceLinks: { label: string; href: string }[];
    project: string;
    projectLinks: { label: string; href: string }[];
    disclaimer: string;
    copyright: string;
  };
};

export const zh: Dict = {
  meta: {
    title: "Pane — 本地优先的 AI 用量工作台",
    description:
      "Pane 把 AI 工具的额度、余额、重置时间、多账号与本地 CLI Token 花费汇集到一个随时可唤出的桌面面板。凭据保存在本机。",
    ogAlt: "Pane — 本地优先的 AI 用量工作台",
  },
  nav: {
    features: "功能",
    demo: "演示",
    faq: "常见问题",
    github: "GitHub",
    download: "下载",
    language: "切换语言",
    theme: "切换主题",
    menu: "打开菜单",
    closeMenu: "关闭菜单",
    primary: "主导航",
    skip: "跳到主要内容",
  },
  hero: {
    eyebrow: "本地优先 · 无需注册",
    titleLead: "所有 AI 用量，",
    titleFocus: "一屏看清。",
    description:
      "Pane 把额度、余额、重置时间、多账号和本地 CLI 的 Token 花费，收进一个随时唤出的悬浮窗。凭据留在你的电脑上。",
    download: "下载 Windows 版",
    demo: "打开在线演示",
    stats: { providers: "服务商", accounts: "Pane 账号", license: "开源许可" },
    caption: "真实 Pane 面板 · 示例数据",
    imageAlt: "Pane 桌面面板，展示额度环、服务商卡片与花费统计",
  },
  workflow: {
    eyebrow: "从本地日志到一块面板",
    title: "用量散在各处，Pane 只读汇总。",
    description:
      "Pane 不接管你的工作流。它读取本机已有的登录态与 CLI 日志，把每个服务的额度和花费整理成同一份快照。",
    leftTitle: "你的电脑",
    leftWindow: "本机",
    leftHeading: "工具与日志",
    leftBody: "Claude Code、Codex、Cursor、Qoder 等工具的登录态和本地日志留在原处，Pane 只读。",
    leftFoot: "本机读取 · 不改写",
    bridge: "本机",
    bridgeNote: "只读快照",
    rightTitle: "Pane 面板",
    rightWindow: "悬浮窗",
    rightHeading: "一块面板",
    rightBody: "刷新结果写入快照，悬浮窗、托盘与本地 API 共用同一份数据。",
    rightFoot: "不上传凭据 · 无需账号",
  },
  panel: {
    eyebrow: "面板解剖",
    title: "一块悬浮窗，装下所有周期。",
    description: "从总览到设置，每个区域都对应一个真实操作。点击左侧步骤，查看它在面板中的位置。",
    stages: [
      { title: "总览", body: "额度环与剩余比例按服务商平铺，异常状态直接标在卡片上。" },
      { title: "周期", body: "5 小时、7 天和每月额度在同一张总览里切换，重置时间跟着走。" },
      { title: "多账号", body: "同一服务商可以挂多个账号，徽章显示池中数量与可用状态。" },
      { title: "花费", body: "本地 CLI 日志按模型拆分 Token 与费用，回看每天的使用。" },
      { title: "设置", body: "快捷键、保险箱、皮肤和通知策略集中在设置面板。" },
      { title: "快捷键", body: "Alt + 2 唤出，Esc 收起，Shift 轮换周期。" },
    ],
    hint: "示意图基于真实面板截图，标记点对应当前步骤。",
    imageAlt: "Pane 面板截图，标注了总览、周期、多账号、花费、设置与快捷键的位置",
  },
  demo: {
    eyebrow: "在线演示",
    title: "真实前端，示例数据。",
    description: "下面运行的是 Pane 的真实前端构建，使用固定的示例数据。可以切换标签、周期、设置与主题。",
    open: "新窗口打开",
    reset: "重置",
    note: "示例数据，不会连接你的真实账户。",
    iframeTitle: "Pane 在线演示",
  },
  faq: {
    title: "安装前想知道的",
    description: "关于 Pane、数据与本机凭据的常见问题。",
    items: [
      {
        q: "Pane 是做什么的？",
        a: "Pane 是一个本地桌面面板，汇总 AI 编程工具和模型厂商的额度、余额、重置时间，以及本地 CLI 的 Token 与花费。",
      },
      {
        q: "支持哪些工具？",
        a: "目前收录 78 个服务商，覆盖 Claude、Codex、Cursor、Copilot、Kimi、Z.ai、OpenRouter、Qoder 等；各服务支持的查询项目有所不同。",
      },
      {
        q: "需要注册 Pane 账号吗？",
        a: "不需要。Pane 没有账号体系，安装后直接使用；你现有的服务商登录态就是数据来源。",
      },
      {
        q: "数据会上传吗？",
        a: "不会。凭据、快照和花费台账都保存在本机，应用不含遥测；出网请求只用于服务商 API、公开价格表和更新检查。",
      },
      {
        q: "可以管理多个账号吗？",
        a: "可以。同一服务商下能挂多个账号，账号徽章显示池中数量和可用状态。",
      },
      {
        q: "支持哪些操作系统？",
        a: "主要在 Windows 上开发和验证；GitHub Releases 也提供 macOS 与 Linux 构建，macOS 标记为 Beta。",
      },
    ],
    more: "没找到答案？",
    moreLink: "提交一个 issue",
  },
  download: {
    eyebrow: "获取 PANE",
    title: "装好，按 Alt + 2。",
    description: "Windows 已提供，macOS Beta，Linux 计划中。下载的是官方 Release 安装包。",
    windows: "下载 Windows 版",
    macos: "macOS Beta",
    macosCta: "下载 macOS 版（Beta）",
    linux: "Linux 计划中",
    facts: [
      { title: "全功能免费", body: "没有付费墙，也没有功能裁剪" },
      { title: "无 Pro / 无订阅", body: "不存在高级版本或续费" },
      { title: "无需注册账号", body: "没有 Pane 账号体系，数据留在本机" },
    ],
    winget: "winget",
    mirror: "国内镜像",
    source: "源代码",
    license: "MIT 许可",
    versionLabel: "最新发布",
    platform: "Windows · x64",
  },
  closing: {
    eyebrow: "少切几个后台",
    title: "用量在手边，\n专注留在当前窗口。",
    description: "下载安装 Pane，把额度、花费和重置时间放进一个随时可唤出的面板。",
    download: "下载 Windows 版",
    demo: "先看看演示",
  },
  footer: {
    product: "产品",
    productLinks: [
      { label: "功能", href: "/#features" },
      { label: "在线演示", href: "/#demo" },
      { label: "常见问题", href: "/#faq" },
      { label: "下载", href: "/#download" },
    ],
    resources: "资源",
    resourceLinks: [
      { label: "文档", href: "https://github.com/Aafff623/pane#readme" },
      { label: "反馈", href: "https://github.com/Aafff623/pane/issues" },
      { label: "更新日志", href: "https://github.com/Aafff623/pane/releases" },
    ],
    project: "项目",
    projectLinks: [
      { label: "源代码", href: "https://github.com/Aafff623/pane" },
      { label: "隐私", href: "/privacy/" },
      { label: "条款", href: "/terms/" },
      { label: "MIT 许可", href: "https://github.com/Aafff623/pane/blob/main/LICENSE" },
    ],
    disclaimer:
      "Pane 是独立工具，不隶属于任何 AI 服务商。Product names and logos belong to their respective owners.",
    copyright: "© 2026 Pane",
  },
};

export const en: Dict = {
  meta: {
    title: "Pane — Local-first AI usage dashboard",
    description:
      "Pane brings quotas, balances, reset times, multiple accounts and local CLI token spend for your AI tools into one floating desktop panel. Credentials stay on your machine.",
    ogAlt: "Pane — Local-first AI usage dashboard",
  },
  nav: {
    features: "Features",
    demo: "Demo",
    faq: "FAQ",
    github: "GitHub",
    download: "Download",
    language: "Switch language",
    theme: "Toggle theme",
    menu: "Open menu",
    closeMenu: "Close menu",
    primary: "Main navigation",
    skip: "Skip to content",
  },
  hero: {
    eyebrow: "Local first · No account",
    titleLead: "Every AI quota,",
    titleFocus: "one panel.",
    description:
      "Pane collects quotas, balances, reset times, multiple accounts and local CLI token spend into a floating panel you can summon anywhere. Credentials stay on your machine.",
    download: "Download for Windows",
    demo: "Open the live demo",
    stats: { providers: "providers", accounts: "Pane accounts", license: "open source" },
    caption: "The real Pane panel · sample data",
    imageAlt: "Pane desktop panel showing quota rings, provider cards and spend metrics",
  },
  workflow: {
    eyebrow: "From local logs to one panel",
    title: "Usage is scattered. Pane only reads.",
    description:
      "Pane does not take over your workflow. It reads the sign-ins and CLI logs already on your machine, then assembles every service into one snapshot.",
    leftTitle: "Your machine",
    leftWindow: "Local",
    leftHeading: "Tools and logs",
    leftBody: "Sign-ins and logs for Claude Code, Codex, Cursor, Qoder and more stay where they are. Pane reads them.",
    leftFoot: "Read locally · never rewritten",
    bridge: "Local",
    bridgeNote: "read-only snapshot",
    rightTitle: "Pane panel",
    rightWindow: "Floating",
    rightHeading: "One panel",
    rightBody: "Each refresh writes a snapshot that the panel, the tray and the local API share.",
    rightFoot: "No uploads · no account",
  },
  panel: {
    eyebrow: "Panel anatomy",
    title: "One floating panel for every window.",
    description: "Every area maps to a real control. Pick a step on the left to see where it lives.",
    stages: [
      { title: "Overview", body: "Quota rings and remaining percentages sit side by side; problems are marked on the card." },
      { title: "Periods", body: "5-hour, 7-day and monthly limits switch on the same overview, reset times included." },
      { title: "Accounts", body: "Attach several accounts per provider; the badge shows the pool count and health." },
      { title: "Spend", body: "Local CLI logs break tokens and cost down by model, day by day." },
      { title: "Settings", body: "Shortcuts, the vault, skins and notification rules live in one drawer." },
      { title: "Shortcuts", body: "Alt + 2 to summon, Esc to dismiss, Shift to cycle periods." },
    ],
    hint: "Based on a real panel screenshot; each marker matches the selected step.",
    imageAlt: "Pane panel screenshot with markers for overview, periods, accounts, spend, settings and shortcuts",
  },
  demo: {
    eyebrow: "Live demo",
    title: "The real front end, sample data.",
    description: "What runs below is Pane's real front-end build with fixed sample data. Switch tabs, periods, settings and themes.",
    open: "Open in a new tab",
    reset: "Reset",
    note: "Sample data. Your real accounts are never connected.",
    iframeTitle: "Pane live demo",
  },
  faq: {
    title: "Before you install",
    description: "Common questions about Pane, your data and local credentials.",
    items: [
      {
        q: "What is Pane?",
        a: "Pane is a local desktop panel that collects quotas, balances and reset times for AI coding tools, plus token and cost data from local CLI logs.",
      },
      {
        q: "Which tools are supported?",
        a: "78 providers today, including Claude, Codex, Cursor, Copilot, Kimi, Z.ai, OpenRouter and Qoder. Each provider exposes a different set of metrics.",
      },
      {
        q: "Do I need a Pane account?",
        a: "No. There is no Pane account system. Your existing provider sign-ins are the data source.",
      },
      {
        q: "Is my data uploaded?",
        a: "No. Credentials, snapshots and spend ledgers stay on your machine and the app ships without telemetry. Network requests only go to provider APIs, public price lists and update checks.",
      },
      {
        q: "Can I manage multiple accounts?",
        a: "Yes. A provider can hold several accounts, and the badge shows the pool size and availability.",
      },
      {
        q: "Which systems are supported?",
        a: "Windows is the primary development and verification target. GitHub Releases also ships macOS and Linux builds; macOS is marked Beta.",
      },
    ],
    more: "Still unsure?",
    moreLink: "Open an issue",
  },
  download: {
    eyebrow: "GET PANE",
    title: "Install it. Press Alt + 2.",
    description: "Windows is available, macOS is Beta and Linux is planned. Every download comes from the official release.",
    windows: "Download for Windows",
    macos: "macOS Beta",
    macosCta: "Download for macOS (Beta)",
    linux: "Linux planned",
    facts: [
      { title: "Free, fully featured", body: "No paywall and no feature gating" },
      { title: "No Pro, no subscription", body: "There is no paid tier or renewal" },
      { title: "No account", body: "No Pane account system; data stays local" },
    ],
    winget: "winget",
    mirror: "CN mirror",
    source: "Source",
    license: "MIT licence",
    versionLabel: "Latest release",
    platform: "Windows · x64",
  },
  closing: {
    eyebrow: "FEWER TABS",
    title: "Keep usage close.\nKeep focus where you work.",
    description: "Install Pane and put quotas, spend and reset times into a panel you can summon anywhere.",
    download: "Download for Windows",
    demo: "See the demo first",
  },
  footer: {
    product: "Product",
    productLinks: [
      { label: "Features", href: "/en/#features" },
      { label: "Demo", href: "/en/#demo" },
      { label: "FAQ", href: "/en/#faq" },
      { label: "Download", href: "/en/#download" },
    ],
    resources: "Resources",
    resourceLinks: [
      { label: "Documentation", href: "https://github.com/Aafff623/pane#readme" },
      { label: "Issues", href: "https://github.com/Aafff623/pane/issues" },
      { label: "Changelog", href: "https://github.com/Aafff623/pane/releases" },
    ],
    project: "Project",
    projectLinks: [
      { label: "Source", href: "https://github.com/Aafff623/pane" },
      { label: "Privacy", href: "/en/privacy/" },
      { label: "Terms", href: "/en/terms/" },
      { label: "MIT licence", href: "https://github.com/Aafff623/pane/blob/main/LICENSE" },
    ],
    disclaimer:
      "Pane is an independent tool and is not affiliated with any AI provider. Product names and logos belong to their respective owners.",
    copyright: "© 2026 Pane",
  },
};

export const dictionaries = { zh, en } as const;
export type Dictionary = typeof zh;
