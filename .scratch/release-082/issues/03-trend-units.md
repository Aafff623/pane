# 趋势用量图改用 token / credits 数量

**What to build:** 趋势图不再画百分比：credits 行画每日 credits 消耗（已有 fetch_credit_history），token/cost 行画每日 token/花费量（fetch_spend_daily 或既有 token 趋势）。

**Blocked by:** None

**Status:** in-review（owner UI 终验；credits 行真机走查受环境限制未做，见 Comments）

- [x] quota 行的趋势不再以 used_percent 为数据源
- [x] credits 行按 credits 计量；token 行按 token 计量；cost 视图按金额
- [x] 轴/悬浮提示/标签文案同步（三语），不再出现「%」
- [x] 无数据的日期保持「无数据」态，不画成 0
- [x] 真机走查：三类行各看一遍（credits / token / cost）※ credits 行见 Comments

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
- 2026-10-14（实现）：趋势数据源收敛为两条真数量线——本地 CLI 日志（token/花费，随 spendMetric 切换）+ credits 台账（fetch_credit_history）；采样百分比管线整条退场：
  - 前端 `main.ts`：`trendSourceFor` 删 quota fallback；`showTrendTip` 删 percent 分支与「占 30 天 %」行；`fetch_usage_history` invoke 清除。
  - `i18n.ts`：三语删 `spend.quotaTrend` / `spend.quotaTrendTip` / `spend.quotaBarTip` / `spend.of30`。
  - Rust `usage_history.rs` 收敛为 credits-only 台账（percent 存储/采样/`trend_map`/`worst_used_percent` 全删，`migrate_card_id`/`remove_card_history` 只管 credit_history.json）；`lib.rs` 去采样与 `fetch_usage_history` 命令。`remove_usage_history` 命令保留（删卡时清 credits 台账）。既有 usage_history.json 冻结为无读者孤儿数据，不迁移。
  - 无数据日（credits 首日无基线等）保持 `trend-nodata` 态，不画 0——分类逻辑未动。
- 验证证据：tsc 0 错；vite build 过；parse-tests **735:0:13**（737 基线 −3 个 percent 测试 +1 个新 credit 采样测试，数目对账一致）；site 13/13；check-readme ALL PASSED；demo 已重建。
- 真机走查（CDP，temp/scripts/trend-units-walkthrough.mjs）：token 行「Token 趋势 · 峰值 144.7M/217.2M」、tooltip「10月8日周四 42.7M tokens」无 %；切 cost 后「花费趋势 · 峰值 $62.26」；已切回 tokens。**credits 行未走查**：qodercn/traecn 均在用户禁用清单，当前无 credits 卡可看；该分支渲染代码本轮零改动 + Rust credits 台账 9 测试全绿。owner 启用任一 credits 卡后即可补看。
- 顺带发现（未动）：`qodercn.rs:363` `all_claimed` 死变量 warning，属签到遗留，与本票无关。
