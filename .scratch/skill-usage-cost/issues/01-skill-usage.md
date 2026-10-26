Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · skill_usage 旁路模块 + 游标 + 设置卡片

## 做什么

1. src-tauri/src/skill_usage.rs：SkillRow 聚合（calls/tokens/cost/last_used_ms）；口径与去重照 spec；lib.rs 注册 fetch_skill_usage(force)。
2. 游标 %APPDATA%\Pane\skill_usage_cache.json（files+totals；offset+4KiB 边界哈希；半行留下次；shrink/不匹配→扣旧账重读；rev+1 重建）；与 fetch_spend 并行 spawn_blocking。
3. main.ts 设置页 skillUsage 卡片（表格+重新扫描）；i18n 8 key（en/zh/ru）。
4. parse-tests 7 组（spec 验收节）；隐私断言（两 JSON 不含 args 子串）。

## 验收

7 组全绿+spend 既有测试零回归（diff 为证 spend.rs 未改）；`pnpm build` 绿；本机扫描产出非空卡片；owner UI 验收。
