# 08 · 业务接口、部署与测试

Status: done 2026-10-29 | Priority: P0 | Depends: 01

## Scope

- `worker.js`：静态目录改 `out/`；保留 `/download/windows`、`/api/latest`、`/update/latest.json`、`/skins/*` CORS、`/en/*` 301、法务路由与 404；安全头保留，仅 `/pane-demo/*` 放行同源 iframe（`x-frame-options: SAMEORIGIN` + `frame-ancestors 'self'`），其余保持 `DENY`。
- `wrangler.jsonc`：assets.directory → `./out`，`run_worker_first` 保持开启。
- `site-deploy.yml`：Node 22 + npm（`npm ci` → `npm run build` → `wrangler deploy`）；冒烟覆盖 `/`、`/zh/`、法务、`/download/windows` 302、`/skins/index.json`、`/pane-demo/` 200 + SAMEORIGIN 与根路径 DENY。
- Demo：`scripts/build-demo.mjs` 产物写入 `site/public/pane-demo/`（Next 原样复制到 `out/pane-demo/`），Vite `base` 同步为 `/pane-demo/`。
- 测试：`site/tests/site.test.mjs` 7 条 Node 合同测试（导出产物与双语、Worker 框架策略与 `/en` 301、下载回退、CN 更新镜像、皮肤 CORS 与 404、版本/计数一致、demo 溯源哈希）。无头浏览器冒烟没有进 CI，作为本地 CDP 脚本放在 `temp/scripts/`（截图证据 `temp/logs/new-site/`）。
- 文档：更新 `site/README.md`、`docs/project-facts.md`，写明构建、预览、目录、接口与素材替换方式。

> 2026-10-29 收尾：以上均已落地；站点仍为**本地草稿**，未推送、未部署，线上继续跑旧站。

## Done when

- `npm run build` + `npm test` 全绿；`wrangler dev` 下所有路由人工可验证。
- CI 只做本地等价检查；push/deploy 需 owner 明确授权。
