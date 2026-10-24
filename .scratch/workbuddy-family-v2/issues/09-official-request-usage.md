# 09 — 官方 Credits 请求流水：先验证，后决定

**Blocked by:** 03（token 生命周期可用）
**Status:** blocked（缺可验证账号）
**Issue:** #12（有条件保留）

**What to verify before any implementation**

1. CN（`www.workbuddy.cn`）与 AI（`www.workbuddy.ai`）的 `POST /billing/meter/get-user-request-usage` 是否都可用。
2. 已授权账号（个人 / 企业）是否都能查询；`X-Enterprise-Id`/`X-Tenant-Id`/`X-Domain` 哪个必需。
3. 分页语义：`startTime` 推进 + `pageNum=1` 是否稳定；是否出现同一时间戳卡死。
4. 返回字段是否含 `requestId`/`requestTime`/`model`/`credit`；30 天窗口是否完整。
5. 与 `credit_history` 快照差额的关系（不得相加）。

**判定标准**

- 全部可验证 → 另立实施票，接「用量面板的账号筛选 + 每日积分趋势」。
- 任一项不可验证或数据不完整 → 记录限制，**不实现、不落库、不展示**。

**产出**：`temp/scripts/probe-workbuddy-request-usage.mjs`（只打印状态码/字段名/条数，绝不打印 token 与正文）+ 一份结论记录。

**验收**
- [ ] 探针脚本存在且不泄密
- [ ] 结论写入 `.scratch/workbuddy-family-v2/` 的 comments
- [ ] 未验证前 UI/台账零改动

## Comments

- 2026-10-19 出自 spec D10；上游 `official_usage.rs:24-49,393-527`。

- 2026-10-24：重新用本机 CN 登录态跑 `probe-workbuddy-request-usage.mjs`，S0 五项全部可验证（200/code 0；26 条 30 天；字段齐全；pageSize=3000 全量、pageNum=2 边界不前进；total 与行为 26）。按 spec 判定可以进入实施。已复用 `workbuddy_official_usage` 的纯解析/分页/聚合模块，在 WorkBuddy CN 卡片追加只读 `Official requests (30d)` 文本行（请求数 + credits 合计），不写 meter、不与 credit_history 相加。AI 档未探测，保持不启用；账号筛选与每日趋势另票。
