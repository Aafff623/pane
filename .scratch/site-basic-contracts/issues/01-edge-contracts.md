Status: ready
Type: task
Blocked by: —

# 01 · 边缘契约：安全头 + CSP Report-Only + 404.html

## 做什么

1. worker.js fetch 统一加 `X-Content-Type-Options: nosniff`/`Referrer-Policy: strict-origin-when-cross-origin`/`Permissions-Policy`（禁 camera·microphone·geolocation）/`X-Frame-Options: DENY`；/skins CORS 响应补 nosniff（:137-143）。
2. CSP 定制（script/style 允许内联 theme+gsap、iframe 同源 demo、font 自托管）以 `Content-Security-Policy-Report-Only` 起步，报告端点落地后收集 1-2 周再 enforce。
3. site/public/404.html（noindex+根绝对路径+OG 完整）；worker fetch 对 404 响应替换该资产。

## 验收

curl 四头验证；CSP Report-Only 无实际阻断（控制台收集报告）；404 路由返回自定义页；site.test.mjs 补 3 断言。
