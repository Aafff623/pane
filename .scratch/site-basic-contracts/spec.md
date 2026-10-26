# Spec: 站点基础契约（安全头 / 404 / SEO / a11y）

**Status:** implemented (2026-10-26) — headers, 404, robots/sitemap, OG cover, skip-link and tests shipped.
**Issue:** https://github.com/Aafff623/pane/issues/78
**Evidence:** temp/research/20261025-website-reference-sweep.md §一（审计发现 6）+ §二 codexbar-site 节；temp/research/20261025-website-ui-deepdive.md §二/§四

## 问题

Pane 官网（site/，Worker+静态单体页）无任何安全响应头、无 404.html、无 robots.txt/sitemap.xml；OG 有框架但缺封面图尺寸（twitter:card=summary）；skip-link 0 处（codexbar-site/agentsview 均有）。magpie 同构站同样缺 robots/sitemap——属 Pane 自身缺口。

## 范围（2 票）

**票 01 边缘契约**：
1. worker.js fetch 统一加安全头：`X-Content-Type-Options: nosniff`、`Referrer-Policy: strict-origin-when-cross-origin`、`Permissions-Policy`（camera/microphone/geolocation 全禁）、`X-Frame-Options: DENY`（注意 demo iframe 是自托管同源，DENY 不影响；若未来嵌外链 iframe 改 SAMEORIGIN）；`/skins` CORS 响应补 nosniff（worker.js:137-143）。
2. CSP 定制化后以 **Report-Only** 起步（Pane 有内联 theme 脚本/vendor gsap/demo iframe/自托管字体，直接 enforce 风险大）——报告收集 1-2 周再 enforce。
3. 新增 site/public/404.html（noindex+根绝对路径+完整 OG）；worker fetch 对 404 响应替换为该资产（CF ASSETS 不自动兜底）。

**票 02 SEO/a11y 收尾**：
1. site/public/robots.txt（Allow all+Sitemap 行）+sitemap.xml（/ 与 /en/ 两条+hreflang，与 index.html 现有 alternate :19-21 对齐）。
2. og:image 新增 1200×630 封面（brand-logo 派生或新设计）+og:image:width/height+twitter:card 改 summary_large_image。
3. skip-link（跳主内容）+导航 aria-current 打鉴（agentsview/codexbar 同款）。
4. 断言并入 site.test.mjs（36 条批）：404/robots/sitemap 存在性、OG 尺寸字段、skip-link 存在。

## 验收

parse-tests 不涉及（纯站点）；`node --test site/*.test.mjs` 新增断言绿；`cargo check`/`pnpm build` 不受影响；手动 curl 验证四头+404 替换；owner 验收。

## 协调

与 site-content-integrity（平台 badge 同批改 index.html 下载区）错开区域；CSP Report-Only 结论落 DESIGN.md 前不 enforce。
