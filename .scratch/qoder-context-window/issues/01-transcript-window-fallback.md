# 01: 转录内同模型窗口回退

**What to build:** 同一会话里，`runtime-config` 的 `contextWindow` 为 `null` 时，后续同模型请求继续使用该模型最近一次非空窗口；子代理继承父会话解析出的窗口；credits、请求次数和去重口径不变。用户在面板 Qoder 部分看到更新后请求的 token 恢复，且不同模型之间不串窗口。

**Blocked by:** None (can start immediately)

**Status:** done (owner UI acceptance pending)

- [x] fixture：`config 1M → 请求 → config null → 请求`，两条请求都按 1M 折算，credits 不变。
- [x] fixture：`dfmodel 1M → qfmodel 首次 config 为 null`，qfmodel 记为不可用，dfmodel 不受影响。
- [x] fixture：子代理没有 `runtime-config`，继承父会话解析出的窗口；同一 `request_id` 跨主/子转录只计一次。
- [x] 现有 Qoder 转录测试全部通过，31 天扫描窗口、会话发现和请求去重不变。
- [ ] 面板对“更新前已经出现过非空窗口的会话”显示恢复后的 token；未知窗口仍显示不可用，不显示伪造的 0。

## Comments

- 2026-10-23：`null` runtime-config 现在保留同模型最近非空窗口；子代理只继承父会话同模型窗口，不再借用 `last_window` 别的模型窗口。新增回归覆盖同模型 null、跨模型 null、子代理跨模型不借用；Qoder 定向 13 passed / 1 ignored。
