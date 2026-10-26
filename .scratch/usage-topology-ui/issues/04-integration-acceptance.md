Status: done (2026-10-26) — entry button beside Quota Overview title opens a read-only page; build/tests green; owner visual acceptance pending.
Type: task
Blocked by: 01, 02, 03

# 04 · 集成验收与性能门

## 做什么

1. 拓扑页挂入看板导航（owner 定入口位置：总览页新 tab 或独立页）。
2. 性能门全绿：busy-only rAF / 不可见全停 / 测量缓存 / 单 rAF 合并 / setText 变化才写（magpie 四招照单验收）。
3. `pnpm build`+`pnpm test`+`cargo check` 绿；WebView2 缓存按 AGENTS.md 清理后验收。
4. owner 本人视觉与动效验收（release gate：验收通过前不打包不发布）。

## 验收

- 对照 magpie 路由页截图逐项（连线形态/脉冲/流动/折叠）；CPU 空闲时拓扑页零动画开销。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
