# 03: 首次加载期间空态与「去添加 Provider」并存

**What to build:** 首屏还在取数时，不要把「还没有数据」渲染成「你一个 provider 都没加」；
加载中与空态必须是两个互斥的呈现。

**Blocked by:** 无（但与 02 同区，建议同批做）

**Status:** done (code + fixture) — 与 02 同批落地

## 复现（不依赖瞬时状态）

从恢复件里带出来的既存问题：以 `?comic=1&state=loading` 打开本地前端，卡片处于 pending，
界面却同时渲染「添加服务」空态。原文见 `git show dc520c2:.scratch/comic-desk/issues/02-loading-empty-state.md`。

## 验收

- [ ] pending 期间只显示加载态（本轮已上线的仪表动画），不出现空态文案与添加按钮
- [ ] 全部来源都失败时才是真·空态，且能区分「没添加」与「都读不到」
- [ ] 皮肤侧不需要为这个状态再造一套样式（空态类名保持稳定）
- [ ] 与 `spend sources` 的 `unreadable` / `needs_probe` 口径一致：读不到不等于 0，也不等于没配

## Comments

- 2026-10-23：属现存渲染状态逻辑，不是 `comic-pixel` 皮肤引入的问题；皮肤票里没顺手改。

## Comments

- 2026-10-23 修复：`renderGroupedCards` 只要快照为空就渲染「去添加」是根因——它没有 pending/failed 概念。
  现在 `boardLoadPhase` 三态驱动：pending → skeleton（`aria-busy`），failed 或「有启用 provider 但零快照」
  → 「读不到」+ 重试，只有真正全部禁用/未配置才给添加 CTA；重试会把状态翻回 pending，
  不让失败文案压在飞行中的请求上。
- 顺带清掉三处 `typeof boardLoadPhase !== 'undefined'` 防御：改为让 `tests/refresh-state.test.mjs`
  的沙箱显式声明该状态（与 window-appearance 夹具同一做法），生产代码不留测试兼容分支。
