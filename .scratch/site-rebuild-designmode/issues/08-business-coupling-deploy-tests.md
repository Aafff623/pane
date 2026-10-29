# 08 · 业务接口、部署与测试

Status: done 2026-10-29 | Priority: P0 | Depends: 01

## Scope

- `worker.js`：静态目录改 `out/`；保留 `/download/windows`、`/api/latest`、`/update/latest.json`、`/skins/*` CORS、`/demo/`、`/en` 法务路由与 404；安全头保留，但对 `/demo/*` 放行同源 iframe（`frame-ancestors 'self'`，移除 XFO 或按路径豁免）。
- `wrangler.jsonc`：assets.directory → `./out`，其余路由不变。
- `site-deploy.yml`：Node 22 + pnpm（corepack），`pnpm install --frozen-lockfile` → `pnpm build` → `wrangler deploy`；冒烟检查保留并新增 `/demo/index.html` 200 与 iframe 头检查。
- Demo：`scripts/build-demo.mjs` 指向新路径，构建产物写入 `site/public/demo/`（Next 复制到 `out/demo/`）。
- 测试：把旧站 50 条中仍适用的合同（路由、下载回退、更新镜像、皮肤 CORS、安全头、法务路由、版本/计数一致）迁到新的 Node 测试；新增无头浏览器冒烟（首页渲染、demo iframe 有卡片、暗色图标可见、语言切换）。
- 文档：更新 `site/README.md`，写明构建、部署、目录、接口与素材替换方式。

## Done when

- `pnpm build` + `node --test` 全绿；`wrangler dev` 下所有路由人工可验证。
- CI 只做本地等价检查，不推送。
