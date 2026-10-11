# 02 — MCP 卡片多 key：可用/总数徽章 + 折叠展开

**What to build:** MCP 搜索类 provider 下面有多把 api-key 时，卡片要像 Coding agent 的多账号卡片那样，有一个 **可用/总数** 徽章和**折叠/展开**控制（现在是直接平铺 N 行，没有徽章、不能折叠）。

**Blocked by:** None — can start immediately.

**Status:** needs-triage

- [ ] 卡片头部加 `可用/总数` 徽章（key 级：能取到值的算可用）
- [ ] key 行支持折叠/展开（默认展开或折叠待定，与 Coding agent 保持一致）
- [ ] 折叠态下仍能看出有几把 key、几把可用
- [ ] 单 key 时不显示徽章、不显示折叠控件（避免噪音）
- [ ] 与既有的"每 key 一条进度条"共存（进度条不变，只是外层可折叠）

## Comments

- 2026-10-11 用户要求："MCP 搜索，如果一个 provider 下面有多个 api-key，那你应该也像 CodingAgent 一样，有对应的一个 x/x 机制，还有对应的折叠展开栏"。
- 现状：MCP 卡的 key 行是**同一张快照上的多个 metric**（不是多张快照），所以不能直接套用多账号的 `card-account-tabs`。需要在 `renderCard()` 里为 MCP 家族单独加徽章 + 折叠。
- 参考锚点（Coding agent 的多账号实现）：`src/main.ts:2422` `accountAvailabilitySummary()`、`≈2448-2473` `provider-account-badge`（`acct-count-avail` / `acct-count-sep` / `acct-count-total`）、`≈2475` `card-account-tabs`（≥4 走 `compact`）；样式 `src/styles.css:3929-3962`。
- 卡片指标渲染入口：`renderCard()` `≈2442`、`renderItem()` `≈1871`、布局 `ensureLayout()` `≈1025`（`isMcpCard` 分支 ≈1288）。
