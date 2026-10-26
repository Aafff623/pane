Status: ready
Issue: https://github.com/Aafff623/pane/issues/82
Priority: P2
Area: Site

# 官网法务页：privacy / terms / PRIVACY.md 单一源

## 问题

官网无法务页，页脚仅源代码/文档/反馈。codexbar-site 正解=独立子页+prose 版式+source-note+仓库 MD 单一源。

## 方案（详见 spec.md，证据 temp/research/20261025-website-reference-sweep.md §二）

1. 仓库根 PRIVACY.md（以 project-facts/privacy.md 为权威，**实施前核实 telemetry 现状**）。
2. privacy 页（prose+source-note+三语）+terms 页（MIT 分段+「不隶属任何 provider」免责+分发说明）。
3. 页脚 Privacy/Terms/LICENSE 直链+免责句。

## 验收

三语可访问；链接通；内容与仓库事实一致（owner 核对）；存在性断言进 site.test.mjs。
