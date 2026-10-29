# 深度 Review — Design Mode 迁移（2026-10-29）

对 `edcdea7` 的第二次专家审查：先重放构建与路由，再按可复现证据分类。下列问题已在本轮直接修复并复测；未改项单独列出等你判断。

## 已修复

| 级别 | 问题 | 证据 | 修复 |
|---|---|---|---|
| P1 | 英文页的「跳到主要内容」仍是中文 | `/en/` 的 skip link 文本为中文 | skip link 移入 `SiteShell`，按 `dict.nav.skip` 渲染 |
| P1 | 导航 landmark 的 `aria-label` 错用「功能/Features」 | `nav[aria-label]` 实测 = `功能` | 新增 `nav.primary`（主导航 / Main navigation） |
| P1 | Hero 与面板解剖同时请求亮/暗两张 PNG | Network 同时出现 `pane-panel-dark.png` 与 `pane-panel-light.png` | 改 CSS 背景 + `dark:` 变体，只下载当前主题一张；复测仅 `pane-panel-light.png` |
| P1 | macOS 用户的主 CTA 变成「macOS Beta」，丢了「下载」 | `?os=macos&arch=arm64` 下按钮文案 | 新增 `download.macosCta`（下载 macOS 版（Beta）），CTA 统一用它 |
| P2 | Demo iframe 指向 `/demo/index.html`，每次先吃一个 307 | `Location: /demo/?theme=…` | 直接请求 `/demo/`，查询串保留 |
| P2 | 缺 `theme-color` | `<head>` 无 theme-color | `viewport` 导出亮/暗两档 |
| P2 | `?arch=x64` 验收钩子被忽略 | 代码只认 `arm64`，其余走 UA 探测 | 显式 `arm64`/`x64` 都尊重 |
| P2 | favicon 同一 URL 被请求两次（30KB×2） | Network 两条 `/favicon.png` | 只保留 `app/icon.png` 一个来源，删除 metadata icons 与 public favicon |
| P2 | 404 只有中文 | `/en/` 下 404 仍是中文 | 增加英文说明段 |
| P3 | 未使用依赖与过期 `components.json` | `@radix-ui/react-collapsible`、`react-navigation-menu` 无引用；`components.json` 的 css 路径不存在且写 lucide | 删除依赖与文件，重装锁文件 |
| 测试 | 缺少版本漂移与 `/demo/` 合约覆盖 | — | 新增 site package 版本对比；demo 测试改走 `/demo/` |

## 复测结果

- `pnpm build` 通过；`node --test tests/*.test.mjs` 9/9。
- HEAD/GET：`/`、`/en/` 200 + `x-frame-options: DENY`；`/demo/` 200 + `SAMEORIGIN`；`/en/privacy/` 200 + `content-language: en`。
- 旧 URL：`/privacy.html`、`/terms.html`、`/en/privacy.html` 307 到新路径；`/en` 301 到 `/en/`。
- 站内链接：144 条，0 断链；每页 1 个 `h1`。
- 语言切换：`/privacy/` ↔ `/en/privacy/`，不再回首页。
- 对比度：亮色主按钮 5.04:1、暗色主按钮 11.0:1、正文 6.12 / 7.78:1。
- 平台分流：`?os=macos&arch=arm64` 得到 `…aarch64.dmg`，标签「下载 macOS 版（Beta）」。
- 首屏：31 个请求；本地静态服务无压缩约 600KB，线上 Cloudflare 会压缩。

## 未改，留给你判断

1. **CSP 仍是 report-only**：继承旧站决定，未强制。
2. **旧法务 URL 靠 assets 的 307**：现在可用；若想锁定行为，可在 worker 显式 301（`/privacy.html` → `/privacy/`）。
3. **macOS 只有 aarch64 dmg**：x64 Mac 会回退拿到 arm64 包，这是发布产物问题，不是站点路由。
4. **面板解剖标记 1 的标签压在环图上**：纯视觉 nit，可把标签固定到圆点右侧或下方。
5. **demo 切主题会重载 iframe**：演示状态丢失；如需保留状态，可改为 postMessage。
6. **面板截图是静态素材**：主应用 UI 变更后要重新截 `pane-panel-{dark,light}.png`。
7. **上游 blog/docs/compare 等页面未迁移**：交接范围明确收敛为首页 + 法务页。
