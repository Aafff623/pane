# Pane 官网（site/）

基于 [Design Mode 官网](https://github.com/SandeepBaskaran/design-mode)（commit `0c7b573`）的 `website/` 源码迁移而来：Next.js 16 App Router + React 19 + Tailwind 4，静态导出后由 Cloudflare Worker 提供服务和业务路由。

- 线上：https://pane.threetwoa.live
- 上游许可：MIT + Commons Clause（`site/LICENSE`，原样保留；不是纯 MIT）
- 设计基线：上游的 Navbar / Hero / 工作流 / 面板解剖 / FAQ / 下载 / Closing / Footer 结构，品牌与内容替换为 Pane。

## 本地开发

```powershell
cd site
corepack pnpm install
corepack pnpm dev        # http://localhost:3000
corepack pnpm build      # 静态导出到 site/out/
node --test tests/*.test.mjs
```

在线演示来自主项目真实前端：主项目 `src/` 变更后先跑 `node scripts/build-demo.mjs`，再 `pnpm build`。

本地跑完整 Worker（业务路由 + 静态资源）：

```powershell
npx wrangler dev --port 8791
```

## 结构

| 路径 | 作用 |
|---|---|
| `src/app/` | 路由：`/`、`/en/`、`/privacy/`、`/terms/`、`/en/privacy/`、`/en/terms/`、404 |
| `src/components/blocks/` | 页面区块（hero、workflow、panel-anatomy、demo、faq、download、closing、footer、navbar） |
| `src/components/ui/` | 仅保留用到的 Radix primitives（button、accordion） |
| `src/i18n/index.ts` | 中英词典 + 类型约束；两语言键必须一致 |
| `src/content/facts.ts` | 版本、服务商数量、仓库/下载/镜像/许可地址 |
| `src/styles/globals.css` | Tailwind 4 入口、亮/暗 token、字体、法务排版 |
| `public/demo/` | 由主项目构建的真实面板（勿手改） |
| `public/skins/` | 皮肤包与 `index.json`（原样迁移） |
| `public/img/` | 面板截图（hero / 解剖用）与服务商 logo |
| `worker.js` | Cloudflare Worker：业务路由 + 静态资源 + 安全头 |
| `wrangler.jsonc` | assets 指向 `./out`，自定义域 `pane.threetwoa.live` |
| `scripts/build-demo.mjs` | 从主项目 `src/` 构建 demo |
| `scripts/postbuild.mjs` | 把 `out/en/**` 的 `<html lang>` 改为 `en` |
| `scripts/build-skin-packs.mjs` / `check-skin-packs.mjs` | 皮肤包构建与校验 |
| `tests/site.test.mjs` | 静态导出、Worker 路由、iframe 头、许可与版本一致性 |

## 不能丢的契约

| 路由 | 行为 |
|---|---|
| `/download/windows` | 302 到最新 Windows 安装包；失败回 Releases 页 |
| `/api/latest` | 最新版本 + 资产列表（平台分流用） |
| `/update/latest.json` | 已发布桌面客户端的更新清单；CN 优先 Gitee 镜像，镜像不可用自动回退 |
| `/skins/*` | 皮肤包资源，带 CORS（GET/HEAD/OPTIONS） |
| `/demo/` | 真实 Pane 前端；`x-frame-options: SAMEORIGIN`，其余路由 `DENY` |
| `/en/` | 英文首页；`/en/privacy/`、`/en/terms/` 带 `content-language: en` |

## 主题、图标与语言

- 主题：`<html class="dark">` 控制，`localStorage.pane-site-theme` 记忆，首次跟随系统。**不要**依赖 `prefers-color-scheme` 的 `dark:` 变体（已在 `globals.css` 用 `@custom-variant` 改为 class 驱动）。
- 图标：统一用 `@phosphor-icons/react`；服务端组件从 `@phosphor-icons/react/dist/ssr` 引入。服务商 logo 用 CSS mask（`ProviderMark`），保证亮/暗两套主题都可见。
- 语言：`/` 中文、`/en/` 英文；文案只写在 `src/i18n/index.ts`，不要在组件里硬编码。

## 部署

push `main` 且仓库变量 `SITE_DEPLOY=true` 时，`.github/workflows/site-deploy.yml` 会：安装依赖 → `pnpm build` → `wrangler deploy` → 线上冒烟（含 demo iframe 头检查）。Secrets：`CLOUDFLARE_API_TOKEN` / `CLOUDFLARE_ACCOUNT_ID`。

## 素材与许可

- `public/img/pane-panel-*.png`：从 demo 构建产物截取的真实面板截图；面板 UI 更新后重新截取。
- 服务商 logo：`public/img/providers/`，来自主项目品牌素材。
- 上游 `website/LICENSE`（MIT + Commons Clause）必须随源码保留；上游版权声明在 `LICENSE` 中，不得删除。
