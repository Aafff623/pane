# Pane 官网（site/）

基于 Design Mode 官网源码二次构建的 Pane 官网：Next.js 16 App Router 静态导出（`out/`），由 Cloudflare Worker 提供静态资源与业务路由。

- 线上：https://pane.threetwoa.live
- 中文：`/zh/`；英文在根路径 `/`
- 上游许可：MIT + Commons Clause（`site/LICENSE`，必须保留）

## 本地开发与预览

```powershell
cd site
npm ci
npm run dev                 # http://localhost:3000（开发模式）
npm run build               # 静态导出到 site/out/
npm run preview             # wrangler dev，带 /download 与 /api 路由
npm test                    # 构建后运行路由/契约测试
```

在线演示来自主项目真实前端：主项目 `src/` 变更后先 `npm run build:demo`，再 `npm run build`。

## 路由

| 路由 | 说明 |
|---|---|
| `/` | 英文首页 |
| `/zh/` | 中文首页 |
| `/demo/`、`/zh/demo/` | 真实 Pane 前端（示例数据） |
| `/privacy/`、`/terms/`（及 `/zh/*`） | 法务 |
| `/pane-demo/` | demo 运行资源（iframe 用，`x-frame-options: SAMEORIGIN`） |
| `/download/windows` | 302 到最新 Windows 安装包，失败回 Releases |
| `/api/latest` | 最新版本与资产列表 |
| `/update/latest.json` | 桌面客户端更新清单；CN 优先 Gitee 镜像 |
| `/skins/*` | 皮肤包（CORS） |
| `/en/*` | 301 到根路径对应页面（旧链接兼容） |

## 结构

| 路径 | 作用 |
|---|---|
| `src/app/` | 路由：首页、`/zh`、`/demo`、法务、404 |
| `src/components/blocks/` | 页面区块（hero、workflow、panel-anatomy、trust、faq、download、closing、footer、navbar） |
| `src/components/pane-demo-frame.tsx` | 嵌入真实 Pane 前端的 iframe（主题跟随） |
| `src/components/smart-download.tsx` | 按 UA 分流的下载按钮 |
| `src/i18n/index.ts` | 中英词典 |
| `src/content/facts.ts` | 版本、服务商数量、仓库/下载/许可地址 |
| `src/styles/globals.css` | Tailwind 4、亮/暗 token |
| `public/pane-demo/` | demo 构建产物（勿手改） |
| `public/skins/` | 皮肤包 |
| `worker.js` / `wrangler.jsonc` | Worker 业务路由与 Cloudflare 部署 |
| `scripts/{build-demo,postbuild,build-skin-packs,check-skin-packs}.mjs` | 构建辅助 |
| `tests/site.test.mjs` | 导出产物、Worker 路由、iframe 头、事实与 demo 哈希 |

## 约定

- 主题：`html.dark` + `localStorage.pane-theme`；Tailwind `dark:` 已改为 class 变体（见 `globals.css` 的 `@custom-variant`）。
- 语言：根路径英文、`/zh` 中文；`scripts/postbuild.mjs` 会把中文导出的 `<html lang>` 改成 `zh-CN`。
- 文案只写在 `src/i18n/index.ts`；版本与 78 个服务商由 `src/content/facts.ts` 提供，测试会与 `package.json` 和 `src/providerCatalog.ts` 对账。
- 服务商 logo 只在确认已支持时放进首页轮播（目前不含 Gemini）。
- 部署：push `main` 且 `SITE_DEPLOY=true` 时由 `.github/workflows/site-deploy.yml` 执行 `npm ci` → `npm run build` → `wrangler deploy`，并做线上冒烟。
