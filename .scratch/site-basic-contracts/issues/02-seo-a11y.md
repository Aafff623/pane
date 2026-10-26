Status: ready
Type: task
Blocked by: 01

# 02 · SEO/a11y 收尾：robots+sitemap+OG 封面+skip-link

## 做什么

1. site/public/robots.txt（Allow all+Sitemap）+sitemap.xml（/与/en/ 两条+hreflang，对齐 index.html:19-21 alternate）。
2. og:image 1200×630 封面（brand-logo 派生或新设计）+og:image:width/height+twitter:card=summary_large_image。
3. skip-link（跳 #main）+导航 aria-current。
4. site.test.mjs 补断言（404/robots/sitemap 存在性、OG 尺寸、skip-link）。

## 验收

断言绿；三语页面 OG 正确；键盘 Tab 首站到 skip-link；owner 验收。
