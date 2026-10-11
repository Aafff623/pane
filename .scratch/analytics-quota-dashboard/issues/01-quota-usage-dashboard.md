# 01 — 把「总花费」面板改造成多计量单位的「用量」面板

**What to build:** 把现有「总花费」面板**本身**改造成「用量」面板：顶层按**计量单位**分节（token / 积分 / 分钟），每节一个图 + 明细。token 节沿用现有圆环 + 时段 + 热力图（跨厂商可拼）；积分节按「家」分组、每家自己的逐日消耗；分钟节（闪电说）单环/数字、不做饼图。**不再另起一个并列看板**——原描述里的「与现有总花费面板互补」已作废（见下方 2026-10-11 定稿）。

**Blocked by:** None — can start immediately.

**Status:** needs-triage

- [ ] 先出 `temp/preview/` 静态原型给用户拍板（布局、图表类型、口径），再动正式代码
- [ ] Rust 聚合命令（按日/周聚合额度与积分，复用现有台账，不新建数据源）
- [ ] 自绘 SVG 图表接线（不引重型图表库，对齐现有花费图风格）

## Comments

- 2026-10-11 从退役的 TODO.md 迁入（原任务号 **T-0005**，板由用户拍板退役、账本改为 `.scratch/`）。参考物：`coding-plan-dashboard`（国内 Coding Plan 额度看板，克隆已清理、上游 margrop/coding-plan-dashboard 可重拉）；credits 数据源已随 0.7.0 上线（Qoder CN / Trae CN 按日罗列，`usage_history::credit_extras`）。
- 2026-10-11 **grill-with-docs 定稿设计**（决策记录见 `docs/adr/0003-usage-panel-grouped-by-unit.md`）。本项实际收敛为**改造「总花费」面板本身**，而非另起一个并列看板——需确认这是否取代本条原描述里的「与现有总花费面板互补」。
  - 面板改名「**用量**」；顶层按**计量单位**分节（token / 积分 / 分钟），**无跨单位总数**。
  - 只有 **token 允许跨厂商**拼图；积分 / 分钟按「家」分开；一卡多单位（闪电说 = 分钟 + 积分）**各归各节**、不折算。
  - 成员 = 能给出该单位**绝对量**的 provider（**本地台账**或**服务端快照**）；只报百分比的归「额度总览」。
  - 来源两类并明确标注：本地台账（有逐日趋势 / 热力图）+ 服务端快照（仅当前值，无历史）。
  - **美元 = token 的一个视图**，不是独立计量单位。
  - 词汇已入 `CONTEXT.md`：额度 / 用量 / 计量单位。
  - **待定事实**：MCP 搜索的 credits 是否有绝对量 —— 决定它进不进积分节（后台调查中）。
  - 下一步：`temp/preview/` 静态原型给用户拍板 → `/to-spec` → `/to-tickets`。
- 2026-10-11 **MCP 搜索 credits 实况核查**（读运行中 Pane 的 `/v1/providers` + `searchquota.rs`）。四个 provider 的"每 key credits"能力**不一样**：
  - **Tavily** ✅ 每 key：`GET api.tavily.com/usage` → `key.usage`/`key.limit`（key 级已用）+ `account.plan_usage`/`plan_limit`。实测 3 把 key：424 / 0 / 5 已用。
  - **Brave** ✅ 每 key：无额度端点，靠真实搜索响应的 `X-RateLimit-*` 头；实测 1 把：229 / 2000。
  - **Firecrawl** ❌ 只有**团队**剩余 credits（`/v1/team/credit-usage`），给不出 key 级消耗。
  - **Bocha** ❌ 是**人民币余额**（`/v1/fund/remaining`），不是 credits。
  - **逐日 credits 历史的可行接法**（已核实）：`lib.rs:2971` 对**每个 ok snapshot** 调 `usage_history::credits_used_from_metrics`，命中即写入 `credit_history.json`；判据是「label == "Credits" 且 detail 以 `credits used` 结尾、首个 token 是绝对已用量」。所以 provider 只要发一条该形状的 metric 就自动进台账。
  - **但诚实性有坑，需先拍板**：**多 key 的 Tavily 给不出一个诚实的单一累计值**（代码自己标"可能共享"：key 级已用之和 429 ≠ 账户 424），台账按 card id 存一个数 → 要么改成**按 key 记账**（id 用 `tavily@<keyhash>`），要么只收单 key 情形。**Firecrawl（无绝对量）与 Bocha（¥）不进积分节**（符合 ADR 0003 的成员规则）。
  - 附：`bocha` / `firecrawl` 现在都是**真的耗尽**（¥0.00 / 剩余 0），显示 0 不等于坏了。
- 2026-10-11 **已修的显示缺陷**：单 key 的 Firecrawl 卡会把同一行打印两遍（`firecrawl_snapshot_from_results` 的 `count == 1` 分支 clone 了 headline 又 extend rows）——已改为单 key 时只出行本身，测试同步更新（`parse-tests` 3/3 通过）。
- 2026-10-11 **另一个观察（非代码缺陷）**：本机 `keyvault.json` 是 **v3 且 0 条 entries**，所以 bocha/tavily/firecrawl/brave 的 key 全从 legacy 兜底读（环境变量 + `tavily-keys.json`），卡面标签是 key 尾掩码而非用户标签。要恢复"自己的名字"需把 key 加进保险箱（Settings 里操作），不是代码 bug。
