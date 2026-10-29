# 01 · 脚手架：把上游 website 搬进 site/

Status: done 2026-10-29 | Priority: P0 | Depends: —

## Scope

- 清空旧 `site/` 的前端源码（`index.html`、`site.js`、`site.css`、旧模块与旧测试），保留 `worker.js`、`wrangler.jsonc`、`scripts/build-demo.mjs`、`demo/bridge.ts`、`public/skins/`、`public/demo/`、`public/img/`、法务 HTML（供内容迁移）。
- 从 `temp/repo/design-mode/website/` 复制：`src/`、`package.json`、`tsconfig.json`、`next.config.ts`、`postcss.config.mjs`、`eslint.config.mjs`、`components.json`、`next-env.d.ts`、`fonts/`、`public/` 中通用资源、`LICENSE`。
- 删除上游页面：about / blog / changelog / compare / contact / demo（上游版）/ docs / features / mcp / use-cases / privacy（上游版）及其 content/、lib 中仅服务这些页面的模块。
- `package.json` 改名 `pane-site`，scripts 固定为 `dev/build/start`，`packageManager: pnpm@9.15.0`。
- `next.config.ts`：`output: "export"`、`images.unoptimized: true`、`trailingSlash: true`、保留 MDX 能力（法务页可先用 TSX）。
- 删除上游 `.git`、缓存、密钥、analytics、indexnow、link-tracker 等站点专属集成。

## Done when

- `cd site && pnpm install && pnpm build` 生成 `out/`。
- `out/` 中没有 Design Mode 的品牌、插件安装 CTA、演示视频或无关页面。
- 上游 LICENSE 的 MIT + Commons Clause 文本保留在 `site/LICENSE`。
