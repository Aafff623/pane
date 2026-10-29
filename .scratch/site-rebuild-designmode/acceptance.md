# Acceptance — Design Mode 迁移（2026-10-29）

## 溯源

- 上游：`SandeepBaskaran/design-mode` @ `0c7b5739ef68f779092a3acf4b7cf056772003b9`，克隆在 `temp/repo/design-mode/`（local-only）。
- 旧站备份：`temp/backup/pane-site-20261029-092537/`（`site/` 全量 + `site-deploy.yml` + 旧站截图）。
- 旧站工作副本：`temp/site-legacy-20261029/`（迁移期间对照用，可删）。
- 许可：上游 `website/LICENSE`（MIT + Commons Clause，shadcnblocks.com - Rob Austin）保留在 `site/LICENSE`。

## 迁移结果

- 新工程：`site/` = Next.js 16 App Router + React 19 + Tailwind 4，静态导出 `site/out/`。
- 保留并适配：`site/worker.js`（业务路由 + 安全头）、`site/wrangler.jsonc`（assets → `./out`）、`scripts/build-demo.mjs`、`demo/bridge.ts`、`public/skins/`、`public/img/providers/`、法务事实、皮肤脚本。
- 删除：旧 `index.html`、`site.js`/`site.css`、17 个旧前端模块、旧测试、旧设计页、旧品牌资产（7.7MB 未引用图）。
- 上游专属页面（blog/docs/compare/use-cases/MCP/about/contact/changelog）不迁移，官网收敛为「首页 + 法务页」。

## 验证记录

构建与测试（本地）：

```
cd site && corepack pnpm build      # ✓ static export, 13 routes
node --test tests/*.test.mjs        # ✓ 9/9
node site/scripts/sync-site-version.mjs --check   # ✓ v0.12.0, 78 providers
```

Worker 路由（`wrangler dev`，http://127.0.0.1:8791）：

| 路由 | 结果 |
|---|---|
| `/` | 200，`x-frame-options: DENY` |
| `/en/` | 200，`<html lang="en">` |
| `/privacy/`、`/terms/` | 200 |
| `/en/privacy/`、`/en/terms/` | 200，`content-language: en` |
| `/demo/index.html` | 200（307→`/demo/`），`x-frame-options: SAMEORIGIN` |
| `/api/latest` | 200，返回 v0.12.0 与资产 |
| `/download/windows` | 302 → Release 安装包 |
| `/update/latest.json` | 200，CN 走 Gitee 镜像字段 |
| `/skins/index.json` | 200，CORS `*` |
| `/does-not-exist` | 404 |

视觉检查（无头 Chrome，截图在 `temp/logs/new-site/`）：

- 1440 桌面 亮/暗：`desk-light-*.png`、`desk-dark-*.png`
- 768 平板 暗：`tab-dark-*.png`
- 390 手机 亮：`mob-light-*.png`
- 英文：`en-light-*.png`
- 交互：`int-demo.png`（demo iframe 真实渲染）、`int-mobile-menu.png`（移动菜单）
- 主题切换：light → dark 成功，`localStorage.pane-site-theme=dark`
- 语言：`/en/` 的 `lang`、canonical、英文法务链接全部正确

## 已知差异与风险

1. **面板解剖**用真实面板截图 + 编号标记，不是上游那种 CSS 复刻面板；6 个步骤对应总览/周期/多账号/花费/设置/快捷键。
2. **下载区平台行**文案固定为 `Windows · x64`；按钮本身按 UA/架构分流（macOS 走 Release 资产匹配，Linux 显示计划中）。
3. **demo 控制台**有一条 `restore popover size failed` 的 bridge 拒绝（演示环境没有桌面账户），不影响渲染；属于主应用在嵌入环境的既有行为。
4. 上游的分析/营销组件（GA、Product Hunt、testimonials、blog）未迁移；站点无遥测。
5. 面板截图是静态素材：主应用 UI 变更后需要重新截取 `site/public/img/pane-panel-{dark,light}.png`。
6. **未 push、未部署**；线上仍是旧站，直到 owner 授权后走 `site-deploy.yml`。

> 第二轮深度 review 的发现与修复见 [`review.md`](review.md)。

## Owner 验收入口

```powershell
cd site
corepack pnpm build
npx wrangler dev --port 8791
# 打开 http://127.0.0.1:8791/ 与 /en/
```

检查清单：首屏主视觉、工作流区块、面板解剖点选、demo 交互、FAQ、下载入口、亮暗切换、中英切换、390/768 布局。
