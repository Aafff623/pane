# 02 — 中性新鲜度角标 + 轮询抖动

Type: task
Status: ready-for-agent
Blocked by: None

## What to build

- 底部状态行空闲时显示「数据更新于 X 分钟前」（前端 lastFetch；可见定时器每分钟刷新文案；刷新中被"正在刷新"覆盖是既有行为）。
- 主循环睡眠加 ±10% 抖动（rand_core）；15 秒断网重试保持固定。
- 不加配置键（中性信息，偏离原提案已在 spec 记录理由）。

## Acceptance

- [ ] 空闲状态底部持续显示分钟级新鲜度，随时间走动；手动刷新后归零重计。
- [ ] 连续两轮的实际间隔相差在 ±10% 内（日志或推算可证）。
- [ ] tsc / cargo check 全绿。
