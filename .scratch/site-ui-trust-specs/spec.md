# Spec: 官网信任与规格区（边界卡 / stat-row / hero-meta / 免责）

**Status:** implemented (2026-10-26) — trust cards, stats and disclaimer are localized on /en; data-dir path corrected.
**Issue:** https://github.com/Aafff623/pane/issues/84
**Evidence:** temp/research/20261025-website-ui-deepdive.md §二（agentsview 边界卡=本批次最高价值单项）

## 问题

Pane 核心卖点「本地优先、数据不出本机」在官网上没有结构化表达；hero 缺规格行；footer 缺免责句。agentsview 的边界卡（Default / Your choice / Visible boundaries 三卡）是现成的信任表达组件，可近乎照搬。

## 范围（1 票，纯静态当天可交付）

1. **新增「本地优先·边界可见」区**（agentsview boundary-grid 结构）：三卡——默认本地存储（凭据/快照/账本全在本机，列路径级事实）/ 同步由你选（无云账号，导出格式开放）/ 边界可见列明（每条出网行为：provider API、skins CDN、更新检查——链接 privacy 页）。发丝线网格样式（gap:1px+background:rule）。
2. **stat-row 三格**：78 providers / 0 accounts required / MIT license（数字走现有 data-number 滚动或静态，tabular-nums）。
3. **hero-meta 行**：Local-first · MIT · Tauri · Windows（magpie M:434 同款 meta 行）。
4. **footer 免责句**：「Pane is an independent tool and is not affiliated with any AI provider.」（codexbar-site 句式，三语）。

## 验收

新区三语渲染正确（受 site-i18n-hardening key 锁保护）；stat-row 数字与 provider_catalog 实际一致（引用 site-content-integrity 的计数治理结论）；owner 文案验收（边界三卡的措辞是转化关键，需本人过目）；site.test.mjs 补新区存在性断言。

## 协调

依赖 site-design-governance（令牌/规范）；与 site-content-integrity 的免费表达卡相邻不重叠（一个讲边界一个讲免费）。
