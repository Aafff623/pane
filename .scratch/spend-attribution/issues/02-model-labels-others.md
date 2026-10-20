# 02 — 模型标签与 Others 展开

**Blocked by:** 01
**Status:** done（实现/测试完成，UI 验收待 owner）(2026-10-20)

- 模型标签：目标卡用裸 `model_id`；ZCode 卡用 `providerName · model_id`。
- `Others` 悬浮列出每个被折叠 provider 的模型明细。
- 新增 `spendShowAll` 设置（默认 false），成本模式可关闭折叠。

**验收**
- [x] `Others` 里能看到每个 provider 的模型与金额
- [x] 开启「显示全部」后不再折叠
- [x] token 视图行为不变（`foldDonutEntries` 只在成本模式折叠）

**实现** `src/main.ts`（`spendShowAll` 配置/开关、`showSpendPop` 模型行、`foldDonutEntries` 放行）、`src/i18n.ts` 三语、`src-tauri/src/lib.rs` 白名单与默认值、`src/styles.css`。测试 `tests/spend-others-folding.test.mjs`、`tests/spend-fold-aliases.test.mjs`（顺带修复 minimax→mcode 求和别名被大 token 行覆盖丢金额的缺陷，`normalizeDaily` 同一缺陷一并修复）。真机 CDP 验证 Others 悬浮与开关均生效。

**偏差** D3 的「价格目录 display name 作为 hover 补充」未实现：前端拿不到 pricing 的 display name 字段；当前 hover 展示 model id 原文。需先在后端补字段，另开票。
