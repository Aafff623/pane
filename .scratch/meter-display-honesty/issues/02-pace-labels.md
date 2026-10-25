Status: done (2026-10-25)
Type: task
Blocked by: 01

# 02 · pace 文字标签（slow down / on track / speed up）+ 三段 tooltip

## 做什么

1. `src-tauri/src/pace.rs`：`PaceState` 增加「快慢」维度（used_pct − elapsed_pct，±10pp 容差判 on track，超出判 slow down / speed up）；不动现有 Ahead/Close/Over 判定。
2. `src/main.ts` renderMetric 2617-2619 徽章分支：over→火焰+"runs out in X"（run_out_at 已有）；close→琥珀 "~N% left at reset"；ahead→默认隐藏（偏好开才暗显）；nil→不显示。
3. tooltip 三段可验算："已过 X% · 已用 Y% · 预计重置时 Z%"（QuotaBar projectionHelp 式）。
4. `src/i18n.ts` 2718-2723 三语键表补齐（含 tooltip 三段）。

## 验收

- parse-tests：pace 快慢维度单测（边界 ±10pp、unknown/not_started 不判）。
- `pnpm build` 全绿；UI 验收（用户本人）三语正确、ahead 默认不扰民。
