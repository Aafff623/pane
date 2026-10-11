# 01 — Analytics 用量/额度看板面板

**What to build:** 基于 Pane 已积累的本地数据（`spend_history` / `usage_history` / 0.7.0 新落的 credits 台账 `fetch_credit_extras`）做一个额度/用量分析看板：额度趋势、积分（credits）逐日消耗、模型分布等图表视图，与现有总花费面板互补。

**Blocked by:** None — can start immediately.

**Status:** needs-triage

- [ ] 先出 `temp/preview/` 静态原型给用户拍板（布局、图表类型、口径），再动正式代码
- [ ] Rust 聚合命令（按日/周聚合额度与积分，复用现有台账，不新建数据源）
- [ ] 自绘 SVG 图表接线（不引重型图表库，对齐现有花费图风格）

## Comments

- 2026-10-11 从退役的 TODO.md 迁入（原任务号 **T-0005**，板由用户拍板退役、账本改为 `.scratch/`）。参考物：`coding-plan-dashboard`（国内 Coding Plan 额度看板，克隆已清理、上游 margrop/coding-plan-dashboard 可重拉）；credits 数据源已随 0.7.0 上线（Qoder CN / Trae CN 按日罗列，`usage_history::credit_extras`）。
