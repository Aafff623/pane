# 04 · Hero 与工作流区块

Status: done 2026-10-29 | Priority: P0 | Depends: 03

## Scope

- Hero：沿用上游 `HeroHeadline` 的两行 macro 标题结构与字体层级，文案换成 Pane 主张（中文「所有 AI 用量，一屏看清」类，需与旧站事实一致）；CTA = 下载 Windows（primary）+ 打开在线演示（secondary）。
- Hero 主视觉必须是 Pane 真实界面：用 demo 构建产物在无头 Chrome 里截图（亮/暗各一套），按上游 `aspect-[8/5]` 容器裁切；不使用虚构 UI。
- 三条事实（服务商数量、无需注册、MIT）以真实数字渲染，数字来源与旧站测试一致（package.json / provider catalog）。
- `AgentWorkflow` 改为 Pane 的数据汇集叙事：本地 CLI 日志 / 各服务商 API → Pane 快照 → 悬浮面板与本地 API；保留窗口卡片 + 中间箭头结构。
- 服务商 logo 使用真实 SVG；暗色下必须是可见的单色遮罩（禁止再出现黑图标）。

## Done when

- Hero 首屏在 1440/768/390 下完整，主视觉是真实 Pane 截图。
- 工作流区块无 Design Mode 文案残留。
