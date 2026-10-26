Status: ready
Type: task
Blocked by: site-basic-contracts, site-i18n-hardening

# 01 · PRIVACY.md + privacy/terms 子页 + 页脚法务链接

## 做什么

1. 仓库根 PRIVACY.md：凭据本机 keyvault、本地 HTTP API loopback-only、无账号体系、skins CDN 行为、telemetry 现状（**先核实 docs/privacy.md 与 telemetry.json 实际行为再写**）。
2. site/public/privacy/index.html+terms/index.html：prose 版式（46rem 栏宽）+source-note（声明同步自 PRIVACY.md+日期）；三语 i18n。
3. 页脚：Privacy/Terms/LICENSE 直链+「不隶属于任何 AI 服务商」免责句。
4. site.test.mjs 补两页存在性+页脚链接断言。

## 验收

三语页可访问；owner 核对 telemetry 表述与事实一致；断言绿；support 页不做（本期）。
