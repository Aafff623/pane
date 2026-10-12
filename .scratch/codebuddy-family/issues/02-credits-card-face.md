# 02 — 卡面统一：credits 计量条口径

**What to build:** 把 CodeBuddy 家族的卡面从现有的「文本行」升级为 spec D6 的口径，**先在现有 `codebuddy` 上落地**，用户可直接在现有卡片上验收。

- **Credits 计量条**：总量 / 剩余，副行形如 `450 / 500 credits left`；已用 = 总量 − 剩余。
- **套餐名**：取订阅包名（免费档为「体验版」）。
- **重置时间**：周期结束时间**无时区**，按**中国标准时间 UTC+8** 解释，取最早的未来周期。
- **Reserved 行**：冻结额度大于 0 时显示。
- **退化**：总量为 0 时显示 **Left / Total 文本行**，而不是一条 0/0 的条。
- 只累加单位为 `credits` 的包；**结转高于发放时读作 0%**，不出现负百分比。
- 同时发出**台账形状**的 metric（`label == "Credits"`、detail 以 `credits used` 结尾、首个 token 为绝对已用量），让 credits **自动**进「用量」面板的积分节（ADR 0003）。**不新建台账**。

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] 卡面出现 credits 计量条 + 套餐名 + 重置时间；冻结额度 > 0 时出现 Reserved 行
- [ ] 总量为 0 → 退化为 Left / Total 文本行
- [ ] 多包只累加 credits；结转高于发放 → 0%
- [ ] 重置时间按 UTC+8 解释，取最早未来周期
- [ ] credits 自动出现在「用量」面板积分节（复用既有台账，无新数据源）
- [ ] 纯解析测试覆盖以上各分支；`parse-tests` 全绿

## Comments

- 2026-10-11 出自 `.scratch/codebuddy-family/spec.md`（决策 D6 / D7）。展示口径参照 CodexBar 0.71.2 的已发布实现。
- 2026-10-12 **已实现**：`credits_face_from_items` 聚合 `CapacityUnit == credits` 的包（精确字段为十进制字符串，兼容旧键），产出 `Metric::progress("Credits", …)` + detail `"{used:.0} of {total:.0} credits used"` + `with_reset(最早的未来 CycleEndTime)`；冻结额度 → `Reserved` 行；总量 0 → `"{remain:.0} credits left"` 文本退化；全部不限量 → `Unlimited`。
  - **台账接线零新增**：detail 形状正好命中 `usage_history::credits_used_from_metrics` 的既有判据（label=Credits / 以 `credits used` 结尾 / 首 token 为绝对已用量），credits 自动进「用量」面板积分节（ADR 0003）。
  - 计量条已聚合的 credits 包不再重复出行文本；非 credits 包仍保留文本行。
  - 新增 9 条纯解析测试（十进制字符串、只累加 credits、结转→0%、不限量、冻结、0 总量退化、UTC+8 最早未来周期、无单位旧形状兜底）。实测 686 通过 / 0 失败 / 13 忽略。
