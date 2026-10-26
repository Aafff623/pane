Status: done (2026-10-26) — snapshot rows + ModelSpend endpoints, usage-updated activity, 12s linger.
Type: task
Blocked by: —

# 01 · 数据层：快照底座 + usage-updated 活跃事件 + LINGER 语义

## 做什么

1. 拓扑数据装配：provider 分组行（Snapshot）→ 模型端点行（ModelSpend 每 provider 过滤）；空 provider/模型行隐藏。
2. 活跃检测：`listen("usage-updated")` 里 diff 该 provider metrics（has_change 判定）→ 发拓扑事件 {provider, ts}；前端维护 active map，LINGER 8-12s 过期降级（.now→.lit→灭）。
3. 事件节流：usage-updated 高频时 250ms gather 合并（magpie trace.go:875-892 同思路）。

## 验收

- parse-tests/单测：变化检测与 LINGER 过期；无事件不产生动画状态。
- 手动验证：触发一次刷新 → 对应 provider 行进入活跃态 → 12s 后熄灭。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
