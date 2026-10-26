Status: ready
Issue: https://github.com/Aafff623/pane/issues/67
Priority: P1
Area: Spend

# spend 解析加固：copilot fallback resume 做差 + 泄漏值护栏

## 问题

1. copilot JSONL fallback 每 session 只收第一条 shutdown（`seen.insert(session)`），而 modelMetrics 是全 session 累计值——**resume 过的老 session 后续增量全部少算**（主源 DB 每调用一行天然正确，只 fallback 有病）。
2. epoch 级泄漏值（1.7e12 类）被 `clamp(0,100)` 钳成 100%「已用尽」——比 None 更有害（误触发耗尽告警）。

## 方案（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-8-B）

- 票 01：scan_events_at 两遍结构（收集排序→HashMap<(session,model),四元组> 只加 delta=max(0,cur−last)；cur<last 视为上游重置取全量；day 用该次 shutdown ts）；现有 :264 用例补 resume 场景。
- 票 02：共享层 `Metric::progress` 加 `!finite || >101 → text "Usage unavailable"`；前置审计 102 调用点区分超支 meter（调用处 clamp 先例 claude.rs:351）。

## 验收

parse-tests 新断言+零回归；`cargo check` 绿；resume 场景总量正确；泄漏值不再误报告警。与 claude-utilization 票边界：那条管 claude 源头，本票兜底其余 provider。
