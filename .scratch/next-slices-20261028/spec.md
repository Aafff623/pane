# Spec: 2026-10-28 next user-visible slices

**Status:** implemented 2026-10-28 — owner acceptance pending · **Started:** 2026-10-28 · **Owner:** Aafff623

## 目标

0.12.0 之后只做有用户可见价值的三件事，其余低价值架构项归档：

1. **#106 分享卡最小版**：分享按钮打开预览，可见反馈，复制/保存 PNG；先做 2 套主题，不做 7 套。
2. **#31 WorkBuddy 卡片展开**：把已有后端数据接成卡片内「每日趋势 + 最近请求」，不做完整四层面板。
3. **#104-4 resetDay**：让月度外推按账单周期而不是自然月，配置 1–28 日。

## 明确不做

- #42 的 19 个来源全量拆分（归档；以后按单来源增量迁移）。
- #43 的项目/分支归因（归档；有具体成本报表需求再开）。
- #65 的 FetchStrategy/capability/attempts 等 7 票（归档；gatekeeper 已保留）。
- #106 的 7 套主题、分享历史/排行、第三方上传。
- 任何遥测、账号上传或把凭据写进导出图。

## 验收总则

- 每个切片独立可验收；UI 切片附 owner 可复现步骤和真实截图。
- `pnpm build`、`node --test tests/*.test.mjs`、`site` 测试和 `parse-tests` 全绿。
- 不 push、不发布、不打 tag，除非 owner 在发布门单独授权。
- 发布前 owner 亲自做 UI 验收；自动化通过不算视觉通过。

## Tickets

| # | 文件 | 覆盖 | 状态 |
|---|---|---|---|
| 01 | `issues/01-share-preview.md` | #106 最小分享卡 | done |
| 02 | `issues/02-workbuddy-details.md` | #31 卡片展开 | done (owner visual pending) |
| 03 | `issues/03-reset-day.md` | #104-4 resetDay | not planned (archived) |
| 04 | `issues/04-closeout.md` | 归档、文档、验证 | done |
