Status: done (2026-10-26) — version/count generated + CI `--check` gate + chain assertion (Rust 77 + `mcode` whitelist = site 78); free-facts spec row; root `PRIVACY.md` pointer; `/en/privacy` + `/en/terms` routes with LICENSE links and sitemap alternates. Site tests 25/25.
Type: task
Blocked by: 01

# 03 · 官网内容/法务收口（#79 / #82）

## #79 内容口径

1. 版本注入：新增 `site/scripts/sync-site-version.mjs`，从 `package.json`
   生成 `site/public/site-meta.json`（version/providerCount），前端启动读取；
   HTML 里的占位值只作 fallback，测试锁定生成物与 package.json 一致。
2. provider 计数单一来源：从 Rust catalog 导出计数到 `site-meta.json`；
   站点、README、i18n 的 78/77 不一致全部改为读取同一值。
3. 免费表达：下载区增加一条明确“全功能免费 / 无 Pro / 无订阅 / 无账号”
   的规格行或卡片，避免只藏在长句里。

## #82 法务

1. `docs/privacy.md` 继续做唯一权威；根目录 `PRIVACY.md` 只做指针。
2. `privacy.html` / `terms.html` 增加 `/en/privacy`、`/en/terms` 路由。
3. footer 增加 `LICENSE` 链接；测试覆盖路由与链接。

## 验收

- 版本/计数只改一处；`site.test.mjs` 覆盖注入、计数、法务路由。
- 英文法务页可访问且与中文事实一致；owner 文案验收。
