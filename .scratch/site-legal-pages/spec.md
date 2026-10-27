# Spec: 官网法务页（privacy / terms / PRIVACY.md 单一源）

**Status:** ready · **Priority:** P2 · **Area:** Site
**Issue:** https://github.com/Aafff623/pane/issues/82
**Evidence:** temp/research/20261025-website-reference-sweep.md §二 codexbar-site 节（privacy/terms/support 版式+source-note+SYNC 标记机制）

## 问题

Pane 官网无法务页：页脚仅源代码/文档/反馈链接。codexbar-site 的正解：privacy/terms 独立子页+`article.prose` 版式+source-note（声明同步自仓库某 commit）+页脚 License 直链；内容以仓库事实文档为单一源。

## 范围（1 票）

1. **PRIVACY.md 单一事实源**：仓库根新建 PRIVACY.md（内容以 docs/project-facts.md 与 docs/privacy.md 为权威：凭据本机 keyvault、telemetry.json 是否上报——**实施前必须核实 telemetry 现状**、本地 HTTP API 仅 loopback、无账号体系、skins CDN 拉取行为）。
2. **privacy 页**：site/public/privacy/index.html（prose 版式+source-note 声明同步自 PRIVACY.md）；三语 i18n（受 site-i18n-hardening 的 key 锁保护）。
3. **terms/license 段**：site/public/terms/index.html——MIT license 分段（Pane MIT，Copyright Jazii）+「不隶属任何 provider」免责（codexbar footer 句式）+winget/GitHub Releases 分发说明。
4. **页脚**：加 Privacy/Terms/LICENSE 直链+免责句。
5. **同步机制**：PRIVACY.md 改后手工同步进 privacy 页（或最小脚本只重写 SYNC 标记间内容，仿 sync_ios_docs.py——Pane 无 iOS 上游，降级为手工+source-note 记日期）。

## 验收

privacy/terms 三语可访问；页脚链接通；内容与 PRIVACY.md/project-facts 一致（owner 核对 telemetry 表述）；site.test.mjs 补页面存在性断言；owner 内容验收。

## 协调

依赖 site-basic-contracts（子页走 worker 路由+404 兜底）；i18n 锁先立（site-i18n-hardening）则文案天然受检；support 页本期不做（无支持工单体系）。
