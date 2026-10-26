# Spec: spend 解析加固（copilot fallback resume 做差 + 泄漏值护栏）

**Status:** ready · **Priority:** P1 · **Area:** Spend
**Issue:** https://github.com/Aafff623/pane/issues/67
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-8-B（小件 1/2 实现级设计）

## 问题

1. **copilot fallback resume 少算**：主源 session-store.db 每调用一行（sources/copilot.rs:109-120，天然正确），但 JSONL fallback 的 `scan_events_at` 里 `!seen.insert(session)`（:133/:156）使每 session 只收**第一条** shutdown，而 modelMetrics（:159-172）是**全 session 累计值**——resume 后每次进程 shutdown 都带累计 breakdown，老 session 的后续增量全部少算。
2. **泄漏值被钳成 100%**：providers/mod.rs:107-126 已拒 NaN/Inf 并 clamp(0,100)，但 epoch 级泄漏值（如 1.7e12，上游 statusline bug 同类）被钳成 100%「已用尽」——比 None 更有害（误触发 alerts 耗尽告警）。

## 范围（2 票）

- **票 01 copilot fallback 做差**（2-3h）：scan_events_at 改两遍结构——先收集 `(session, ts, 各 model 累计四元组)` 排序，第二遍维护 `HashMap<(session,model), 四元组>` 只加 `delta=max(0,cur−last)`（cur<last 视为上游重置取全量并注释；day 归属用该次 shutdown ts；新 model 键基线 0）。现有 :264 用例补 resume 场景（累计 1100→2200 断言 2200）。
- **票 02 泄漏值护栏**（1.5-2h，审计占大头）：共享层 `Metric::progress` 加守卫 `!finite || used_percent > 101.0 → Self::text(label, "Usage unavailable")`（101 留四舍五入余量，100.0/100.9 合法）。前置审计 102 个 `Metric::progress` 调用点找「合法 >100」的超支 meter（这类在调用处 clamp 如 claude.rs:351，共享层看不到）。

## 边界（与 claude-utilization 票的关系）

claude-utilization 票管 claude.rs 源头（0-1↔0-100/epoch→None）；本票兜底其余 30+ provider——两条都落则 claude 双保险，只落共享层 claude 也被兜住。

## 验收

parse-tests 新断言全绿+既有零回归；`cargo check` 绿；本机或 fixture 验证 resume 场景总量=2200、泄漏值显示 text 不误报告警。
