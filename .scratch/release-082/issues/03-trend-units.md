# 趋势用量图改用 token / credits 数量

**What to build:** 趋势图不再画百分比：credits 行画每日 credits 消耗（已有 fetch_credit_history），token/cost 行画每日 token/花费量（fetch_spend_daily 或既有 token 趋势）。

**Blocked by:** None

**Status:** ready-for-agent

- [ ] quota 行的趋势不再以 used_percent 为数据源
- [ ] credits 行按 credits 计量；token 行按 token 计量；cost 视图按金额
- [ ] 轴/悬浮提示/标签文案同步（三语），不再出现「%」
- [ ] 无数据的日期保持「无数据」态，不画成 0
- [ ] 真机走查：三类行各看一遍（credits / token / cost）

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
