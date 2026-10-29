# Spec: 官网重建 —— 基于 Design Mode 源码迁移

**Status:** implemented 2026-10-29 — local build + tests green; **local-only draft (not pushed, not deployed)**; owner visual acceptance pending · **Started:** 2026-10-29 · **Owner:** Aafff623
**Upstream:** https://github.com/SandeepBaskaran/design-mode @ `0c7b5739ef68f779092a3acf4b7cf056772003b9`
**Upstream path:** `website/`（Next.js 16 / React 19 / Tailwind 4）
**Backup:** `temp/backup/pane-site-20261029-092537/`（旧站源码 + 部署 workflow + 截图）

## 目标

用 Design Mode 官网的真实源码作为新官网工程，保留它的页面骨架、组件结构、样式系统、字体、布局、动效与响应式行为，把品牌与内容替换成 Pane 的真实产品信息。旧站的设计与实现整体作废，只迁移仍被线上依赖的业务能力。

## 已确认的架构决定

1. **新工程根目录 = `site/`**，独立 `package.json`，不影响根目录 Tauri/Vite。
2. **Next.js 静态导出**（`output: "export"`）→ 构建产物 `site/out/`；现有 Cloudflare Worker（`site/worker.js`）继续作为唯一入口，静态资源目录指向 `out/`。
3. **保留业务路由契约**，不得丢失：
   - `/download/windows`（302 → 最新 Windows 安装包，失败回 Releases）
   - `/api/latest`（版本 + 资产列表，供平台分流）
   - `/update/latest.json`（已发布桌面客户端的更新清单，CN → Gitee 镜像）
   - `/skins/`（皮肤包 + CORS）
   - `/pane-demo/`（真实 Pane 前端构建，`/demo/` 页面用 iframe 嵌入）
   - `/zh/` 中文入口、隐私政策、服务条款；旧 `/en/*` 301 到根路径
4. **语言**：`/` 英文，`/zh/` 中文；两套路由共享组件，词典集中在 `src/components/i18n.ts`（构建期静态，不靠客户端选择器映射）。
5. **主题**：默认浅色（继承 Design Mode），新增暗色 token 与切换；两套主题都要验收。
6. **图标**：沿用上游 `lucide-react`（统一尺寸/描边规则），品牌 logo 用 `public/logos/icons/` 的 lobehub 图标。
7. **内容**：只写 Pane 真实能力（额度/余额/重置/多账号/本地 CLI Token/托盘悬浮/本地优先/在线演示/下载/开源），不虚构用户评价、兼容性或安全承诺。
8. **Demo**：继续用 `site/scripts/build-demo.mjs` 从主项目 `src/` 构建，嵌入新设计的演示区；不另写假 UI。
9. **许可**：上游 `website/LICENSE` 是 MIT + Commons Clause（shadcnblocks.com - Rob Austin），新站保留该声明，不按纯 MIT 处理。
10. **不推送、不部署**：本地完成构建与验收，push/deploy 需 owner 单独授权。

## 页面范围

- 首页（单页叙事）：Navbar / Hero / 工作流 / 面板解剖 / FAQ / 下载 / Closing CTA / Footer / Background
- `/privacy`、`/terms`（英文）+ `/zh/privacy`、`/zh/terms`
- `/zh/`（中文首页）
- `/demo/`、`/zh/demo/`（内嵌 `/pane-demo/` 真实前端构建产物）
- 其余上游页面（blog、docs、compare、use-cases、mcp、about、contact、changelog）不迁移。

## 迁移映射（旧 → 新）

| 旧位置 | 处置 |
|---|---|
| `site/public/index.html`、`site.js`、`site.css`、17 个前端模块 | **Delete**（视觉与交互作废；文案事实按需取用） |
| `site/worker.js` | **Adapt**：静态目录改 `out/`，业务路由原样保留 |
| `site/wrangler.jsonc` | **Adapt**：assets 指向 `./out` |
| `site/scripts/build-demo.mjs` + `site/demo/bridge.ts` | **Migrate**：demo 构建链路保留 |
| `site/public/demo/` | **Replace**：用新构建产物重新生成 |
| `site/public/skins/` + `index.json` | **Migrate**：静态资源原样保留 |
| `site/public/img/`、品牌资产 | **Migrate**：只取 Pane 真实素材，按新设计重新裁切 |
| `site/i18n.js` | **Replace**：改 TS 词典 + 路由级语言 |
| `site/public/privacy.html`、`terms.html`、`en/*` | **Adapt**：内容迁到 Next 路由，事实不变 |
| `.github/workflows/site-deploy.yml` | **Adapt**：先 `pnpm build`（静态导出）再 `wrangler deploy`，冒烟检查保留 |
| `site/*.test.mjs`（50 条） | **Adapt**：保留路由/合同测试，换成对 `out/` 与 worker 的断言；补浏览器冒烟 |

## 验收

- `pnpm build`（Next 静态导出）通过；`out/` 可直接由 Worker 提供服务。
- 1440 / 768 / 390 三个宽度截图对照上游骨架；亮/暗两套主题。
- 中英文页面可切换，SEO/OG/canonical 正确。
- `/download/windows`、`/api/latest`、`/update/latest.json`、`/skins/*`、`/demo/` 全部回归通过。
- 旧站 50 条测试中仍适用的合同迁移完成，新增浏览器冒烟（demo 可渲染、暗色图标可见）。
- 根目录 `src/`、`src-tauri/`、Vite 配置不动。

## 明确不做

- 不迁上游的 blog / docs / compare / use-cases / MCP / 社区页面与内容台账。
- 不保留旧站的像素主题切换、WebGL 光标、轨道/跑马灯等装饰系统。
- 不改桌面应用、不发布、不部署。
