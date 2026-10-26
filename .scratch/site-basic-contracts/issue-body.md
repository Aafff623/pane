Status: ready
Issue: https://github.com/Aafff623/pane/issues/78
Priority: P1
Area: Site

# 站点基础契约：安全头 / 404 / robots+sitemap / OG / skip-link

## 问题

官网无安全响应头、无 404.html、无 robots/sitemap；OG 缺封面尺寸；skip-link 0 处。

## 方案（详见 spec.md，证据 temp/research/20261025-website-reference-sweep.md）

- 票 01：worker 统一加 nosniff/Referrer-Policy/Permissions-Policy/X-Frame-Options+CSP（**Report-Only 起步**）；/skins CORS 补 nosniff；新增 404.html+worker 404 替换。
- 票 02：robots.txt+sitemap.xml（/与/en+hreflang）；og:image 1200×630+尺寸+summary_large_image；skip-link+aria-current；site.test.mjs 补断言。

## 验收

四头 curl 验证；404/robots/sitemap/OG/skip-link 断言绿；owner 验收。
