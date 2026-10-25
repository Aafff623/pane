Status: implemented (2026-10-25) - automated green; owner acceptance pending
Type: task
Blocked by: —

# 01 · 未知≠0%：三处 null 分支修复

## 做什么

1. `src/main.ts` 5675-5676（多账号 meter）与 5687-5691（多账号 ring）：`window===null` 时 pct 为 null，渲染 — 与灰色态（对齐环本体 5815-5845 / 条 5970-5973 现有行为），title 去掉 "· 0%"。
2. `src/main.ts` 3162 `formatHoverMetric`：`used_percent ?? 0` 改 null → —（或该行不显示百分比）。
3. 顺带全量 grep `?? 0` / `: 0` 在卡片渲染路径的其它漏网点，一并核对。

## 验收

- `pnpm build` 全绿；`pnpm test`（如覆盖）全绿。
- UI 验收（用户本人）：构造无数据账号（或 mock），确认显示 — 而非 0%，hover 无 0%。

## Comments
- 2026-10-25：已实现。多账号 meter 与 ring 的 window===null 渲染 —（track 宽度保持 0）、title 同步 —；formatHoverMetric 的 used_percent==null 显示 —。pnpm build + Node 180/180 绿。
