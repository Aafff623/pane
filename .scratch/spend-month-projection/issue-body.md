Status: ready
Issue: https://github.com/Aafff623/pane/issues/72
Priority: P2
Area: Spend

# 花费面月度外推（近 7 天日中位数法）

## 问题

「这个月要花多少钱」没有答案——Pane 只有配额百分比的 pace 外推，无花费面投影。数据地基已就绪（spend_history.rs 永久按日 rollup）。

## 方案（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-6）

1. spend_history.rs 纯函数 `project_month_from_series`（**不碰 pace.rs**）：最近 7 个有效日（cost>0.004）取**中位数**（抗单日大任务突发）× 剩余天数 + 本月已花；有效日<3 或月末当天不出投影。
2. UI：spend 卡片 d7/d30/all 旁一行「本月预计 ≈$X」（仅 cost 指标+非 RTK）；构成写 tooltip；i18n 3 key 三语。
3. 9 条 parse-tests（含中位数抗突发 [1×6,50]→1、跨月重置、别名 fold）。

## 验收

9 测全绿+零回归；本机真实数据 tooltip 构成可手算复核；三语无泄漏；owner UI 验收。#56 协调：用已固化成本，调价不追溯不外推跳变。
