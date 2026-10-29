export type Lang = "en" | "zh";

export const langFromPath = (pathname: string): Lang => (pathname === "/zh" || pathname.startsWith("/zh/") ? "zh" : "en");

export const messages = {
  en: {
    nav: {
      items: [
        { label: "Features", href: "/features" },
        { label: "Demo", href: "/demo" },
        { label: "MCP", href: "/mcp" },
        { label: "Use cases", href: "/use-cases" },
        { label: "Compare", href: "/compare" },
        { label: "Docs", href: "/docs" },
        { label: "Blog", href: "/blog" },
      ],
      download: "Download",
      github: "Pane source code on GitHub",
    },
    hero: {
      lead: "All your AI usage,",
      before: "in one",
      after: "panel.",
      description:
        "Pane brings quotas, balances, reset times, multiple accounts and local CLI token spend into one floating panel. Credentials stay on your machine.",
      primary: "Download for Windows",
      secondary: "Try it yourself",
      placeholder: "Product video coming soon",
      caption: "A short walkthrough of Pane's floating panel.",
    },
    workflow: {
      eyebrow: "Local first. Read only.",
      title: "Usage scattered across tools? Pane gathers it.",
      description:
        "Pane reads the sign-ins and CLI logs already on your machine, then assembles every service into one snapshot.",
      leftTitle: "Pane",
      leftBody: "Reads local sign-ins and CLI logs, then writes one snapshot for every provider.",
      leftFoot: "Local reads · never rewritten",
      bridge: "LOCAL",
      bridgeNote: "read-only snapshot",
      rightTitle: "Your harness tools",
      rightBody: "Claude Code, Codex, Cursor, Qoder and the rest, in one panel.",
      rightFoot: "78 providers · local logs",
    },
    panel: {
      title: "Every quota window, in one floating panel",
      items: [
        { title: "Overview", lines: ["Quota rings and remaining percentages for every provider.", "Problems are marked on the card."] },
        { title: "Header", lines: ["Refresh, settings and theme in one row.", "The snapshot time stays visible."] },
        { title: "Action bar", lines: ["Refresh a single service or switch categories.", "Tabs follow your own workflow."] },
        { title: "Accounts", lines: ["Attach several accounts per provider.", "The badge shows the pool size and health."] },
        { title: "Spend", lines: ["Local CLI logs break tokens and cost down by model.", "Look back day by day."] },
        { title: "Settings", lines: ["Shortcuts, the vault, skins and notifications.", "One drawer for all of it."] },
        { title: "Shortcuts", lines: ["Alt + 2 to summon, Esc to dismiss.", "Shift cycles the quota period."] },
      ],
    },
  },
  zh: {
    nav: {
      items: [
        { label: "功能", href: "/features" },
        { label: "演示", href: "/demo" },
        { label: "MCP", href: "/mcp" },
        { label: "场景", href: "/use-cases" },
        { label: "对比", href: "/compare" },
        { label: "文档", href: "/docs" },
        { label: "博客", href: "/blog" },
      ],
      download: "下载",
      github: "Pane 源代码（GitHub）",
    },
    hero: {
      lead: "所有 AI 工具的用量，",
      before: "一屏",
      after: "看清。",
      description:
        "Pane 把额度、余额、重置时间、多账号和本地 CLI 的 Token 花费，收进一个随时唤出的悬浮窗。凭据留在你的电脑上。",
      primary: "下载 Windows 版",
      secondary: "打开在线演示",
      placeholder: "产品视频待补",
      caption: "Pane 悬浮窗的简短演示。",
    },
    workflow: {
      eyebrow: "本地优先 · 只读",
      title: "用量散在各处，Pane 只读汇总。",
      description: "Pane 读取本机已有的登录态与 CLI 日志，把每个服务整理成同一份快照。",
      leftTitle: "Pane",
      leftBody: "读取本机登录态与 CLI 日志，为每个服务启用的通道写出一份快照。",
      leftFoot: "本机读取 · 不改写原始数据",
      bridge: "本机",
      bridgeNote: "只读快照",
      rightTitle: "你的 Harness 工具",
      rightBody: "Claude Code、Codex、Cursor、Qoder 等工具，集中在一个面板。",
      rightFoot: "78 个服务商 · 本地日志",
    },
    panel: {
      title: "一块悬浮窗，装下所有周期",
      items: [
        { title: "总览", lines: ["额度环与剩余比例按服务商平铺，异常状态直接标在卡片上。", "会话、周度与月度额度一眼可见。"] },
        { title: "顶栏", lines: ["刷新、设置与主题集中在同一行。", "上次快照时间常驻显示。"] },
        { title: "操作栏", lines: ["可以刷新单个服务，也能切换分类。", "标签顺序跟着你的工作流。"] },
        { title: "多账号", lines: ["同一服务商可以挂多个账号。", "徽章显示池中数量和可用状态。"] },
        { title: "花费", lines: ["本地 CLI 日志按模型拆分 Token 与费用。", "支持按天回看。"] },
        { title: "设置", lines: ["快捷键、保险箱、皮肤和通知策略。", "都在同一个抽屉里。"] },
        { title: "快捷键", lines: ["Alt + 2 唤出，Esc 收起。", "Shift 轮换额度周期。"] },
      ],
    },
  },
} as const;

export type Messages = (typeof messages)["en"];
