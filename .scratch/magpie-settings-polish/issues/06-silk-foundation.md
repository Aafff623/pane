Status: done (2026-10-26) — page-in, --ease, press scale, pre-select, grid-rows fold; build/tests green.
Type: task
Blocked by: —

# 06 · 全局丝滑基座（page-in / --ease / grid-rows 折叠 / pointerdown 先行）

## 做什么

按 spec-round2.md §06 逐条：切页入场动画（.24s page-in，与 activatePanelPage 兼容）、全局 --ease 令牌 + hover .15s + :active scale(.96)、折叠改 grid-template-rows 0fr→1fr + chevron [aria-expanded] 旋转、prefers-reduced-motion 全局降级、@container 窄容器查询、pointerdown 先行选中（侧栏/分段按钮）、渲染路径盘点后 replaceChildren 替代 innerHTML 拼接。

## 验收

- `pnpm build` + `pnpm test` 绿；动画只出现在 transform/opacity（DevTools 性能面板无 layout 突刺）；reduced-motion 下动画消失。
- owner 视觉验收：切页/折叠/悬停手感与 magpie 同级。

## 证据

temp/research/20261025-magpie-ui-mechanics.md §二/§三（magpie app.css:520-521/3552/691-696、app.js:23712/23747-23750）。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
