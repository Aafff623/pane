Status: done (2026-10-25)
Type: task
Blocked by: —

# 01 · 观察时间线 store：schedules.json + resolve_pin_at

## 做什么

1. 新增 `src-tauri/src/pricing_timeline.rs`（或 pricing.rs 内新模块）：`schedules.json` 的 schema（spec.md §①）、只追加写入（原子写 + 临时文件 rename）、篡改检测（条目数减少/哈希链断 → 隔离并要求显式重建，不自动修）。
2. `resolve_pin_at(scope, seg_end_ms, model) -> ResolvedPin { pin: Option<PricePin>, estimated: bool }`：取 observed_at ≤ seg_end 的最后一条；无则取第一条 + estimated=true。
3. 每 (scope, model) 的 observed_at 单调性保护（拒绝早于末条的时刻，或钳制并告警）。
4. `PricePin` 结构：input/output/cache_read/cache_write/各 200k 变体/cache_write_1h/fast_mult/priority_mult/long_context/basis。

## 验收

- parse-tests：区间边界查价（[obs_i, obs_{i+1})）；首条覆盖 (-inf, obs_1) 且 estimated；pin:null 区间走 unpriced；篡改检测隔离；时钟回拨被拒/钳制。
