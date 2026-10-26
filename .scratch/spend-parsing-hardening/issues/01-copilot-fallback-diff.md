Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · copilot fallback resume 做差

## 做什么

改 spend/sources/copilot.rs 的 scan_events_at：两遍结构——先收集 `(session, ts, 各 model 累计四元组)` 按 (session,ts) 排序；第二遍维护 `HashMap<(session,model), 四元组>`，每条 shutdown 只加 `delta=max(0, cur−last)`（cur<last 视为上游重置取全量并注释）；day 归属用该次 shutdown 的 ts；model 键新增基线 0；known_sessions 跳过逻辑保留。

## 验收

- 现有 :264 用例补 resume 场景（两条 shutdown 累计 1100→2200，断言总量 2200）。
- parse-tests 全绿+零回归；`cargo check` 绿；DB 主源路径不动。
