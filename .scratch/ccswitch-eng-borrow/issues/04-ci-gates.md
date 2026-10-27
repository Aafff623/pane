Status: done (2026-10-26) — release-notes gate already runs before packaging; CI path filtering deliberately declined so required checks never skip on docs-only PRs.
Type: task
Blocked by: —

# 04 · CI 路径过滤 + release 文档门禁

## 做什么

1. 新建/增强 ci.yml：paths-filter（固定 SHA）区分前端/Rust/docs 触发面；concurrency.group 带 github.event_name；测试 job 按触发面裁剪（pnpm test / cargo check / parse-tests）。
2. release.yml 加文档门禁 job：tag 发布前校验 CHANGELOG 含该版本小节、what's-new/发布说明素材存在（release-format 规则的可失败化），缺失直接挂 release。
3. 不动 codeql/dependabot。

## 验收

- PR 只改 docs → 不跑 Rust job；release 缺 CHANGELOG 小节 → 门禁红；齐全 → 绿。
