# Pane 官网（site/）

Magpie 式零框架单文件官网：无构建、无依赖，`public/index.html` 为中文源页面，Worker 按 `i18n.js` 字典重写出 `/en/`。

- 本地预览：`cd site && npx wrangler dev`（127.0.0.1:8787，`/` 中文、`/en/` 英文）
- 测试锁：`node --test site/*.test.mjs`（i18n parity 双向锁 + T() 参数数 + 资产存在性 + 链接格式；node 22+，零依赖）
- 部署：push main 且仓库变量 `SITE_DEPLOY=true` 时由 `.github/workflows/site-deploy.yml` 跑 `wrangler deploy`；secrets 为 `CLOUDFLARE_API_TOKEN` / `CLOUDFLARE_ACCOUNT_ID`
- 换截图：同名替换 `public/img/` 下 `overview / spend / heatmap` 的 `-dark.png` 与 `-light.png`（尺寸按 `<img>` 的 width/height），零代码改动
- 加语言 checklist：`i18n.js` 顶部注释（字典 + LANGS + 页面语言菜单 + T() 参数）

已上线：**https://pane.threetwoa.live**（`wrangler.jsonc` 的 `routes` + `custom_domain`；workers.dev 试玩地址已随自定义域自动关闭）。Roadmap：`/api/latest`（worker.js 里有 TODO，release.yml 已产 latest.json + sha256）、交互 Demo（额度刷新倒计时，静态 90 分之后再做）。
