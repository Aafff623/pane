# 03 — 更新弹窗重做 + changelog 解析修复

**Blocked by:** None
**Status:** done (2026-10-20) — 三种形态已实现并实际渲染；`tests/changelog-parse.test.mjs` 覆盖分组折叠、Unreleased 跳过、真实 CHANGELOG 无假版本（61/61 通过）。

- `parseChangelog` 只把 `^## ` 中匹配 `版本 — 日期` 的块当版本；`### 新增/变更/修复` 解析成条目类型；正文首段作为版本主题行。修掉 Unreleased 分组被当成「v新增」的问题。
- `showChangelogDialog` 扩展三种形态：`update`（星标 + 已更新到 vX + 自 vX 以来 + 3 版折叠 + 完整日志入口）、`browse`（全部版本）、`plain`（字体引导的原文 + 自定义按钮，行为不变）。
- 条目 chip 用语义色（新增绿 / 变更蓝 / 修复黄），主色仍是黑白。
- 新增 i18n：`dialog.whatsNew` 改「已更新到 v{version}」、`whatsNew.since/showEarlier/viewFull`、`changelog.added/changed/fixed`、panel 页说明；en/zh/ru 三语。
- 「查看完整更新内容」在当前窗口打开 browse 形态（Pane 的更新日志在本地 CHANGELOG.md，没有外链）。

**验收**
- [ ] 更新后弹窗显示最近 3 版、chip、折叠、完整日志
- [ ] 从 0.8.5 跳到 0.8.6 不再出现分组假版本
- [ ] 字体引导弹窗的 accept/decline/标题与原来一致
- [ ] node 解析测试覆盖分组与假版本
