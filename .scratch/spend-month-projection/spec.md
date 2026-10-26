# Spec: 花费面月度外推

**Status:** ready · **Priority:** P2 · **Area:** Spend
**Issue:** https://github.com/Aafff623/pane/issues/72
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-6（纯函数设计+规则表+9 条测试）

## 问题

「这个月要花多少钱」今天没有答案——Pane 全部外推只有 pace.rs 的 used%/elapsed% 窗口几何（服务配额百分比），无任何花费面投影。数据地基已就绪：spend_history.rs 永久按日 rollup（无 prune）、daily_spend() 暴露 day/id/cost、merge 幂等。

## 范围（1 票，~0.5-1 天）

1. spend_history.rs 纯函数 `project_month_from_series(days:&[(NaiveDate,f64)], today)->Option<SpendProjection{projected_total,month_to_date,median_daily,remaining_days,sample_days,sample_span_days}>`（**不碰 pace.rs**——它只认百分比）。常量 SAMPLE_DAYS=7/MIN_SAMPLE_DAYS=3/MAX_LOOKBACK_DAYS=31/EFFECTIVE_COST=0.004（对齐 :232 活跃日门槛）。
2. 规则：样本=从 today 回溯 ≤31 天取最近 7 个**有效日**（当日 alias fold 后 cost>0.004；全零日与仅有 token 未定价日不算）；中位数（偶数样本取中间两均值）；剩余天数=当月日历天数−today.day()（不含今天）；None=有效日<3 或 remaining≤0；跨月纯函数无状态 today 驱动；置信 sample_days≥7 高/5-6 中/3-4 低（UI「样本少」注脚）。
3. 出口：lib.rs 新命令或经 fetch_spend_history 送出；main.ts rangeCol()（:5520-5539）colTotalHtml 后插一行「本月预计 ≈$X」；仅 !rtkMode && cost 指标 && 有投影时渲染；取数扩 Promise.all 三元（:13642-13649）；构成写 title tooltip（已花+中位数×剩余），不做二级面板；i18n 3 key（monthProjected/monthProjectedTip/monthProjectedFew）；样式复用 .spend-est（styles.css:3328）。

## 验收

- parse-tests 9 条（中位数抗突发 [1×6,50]→median=1/<3 日 None/月初/月末 None/跨月重置/全零与未定价日排除/偶数样本/31 天帽/别名 fold）。
- `cargo check`+`pnpm build` 绿；本机 spend 卡片出现「本月预计 ≈$X」且 tooltip 构成可手算复核；有效日<3 与月末当天不显示；三语无 key 原文泄漏；RTK/tokens 指标与折叠态不显示。
- owner UI 验收（WebView2 缓存清理后）。

## 协调

#56（输入是已固化当时价日成本→外推不随调价跳变——写进票的稳定性说明）；#59-01 不同层（会话级 what-if vs 账户级时序统计）；#64-04/05 统计页 KPI 是 token 口径，**本期不进统计页**（混入美元投影破坏口径一致性）。
