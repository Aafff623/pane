Status: done (2026-10-25)
Type: task
Blocked by: 01

# 02 · 前端卡片级在飞跟踪与按卡合并

## 做什么

1. `src/refreshState.ts`：新增 `cardInFlight: Set` + `requestCard/finishCard`；`accepted` 复用卡片键 `card:<id>`；`acceptCardResult` 用与 53-57 行同款代际比较。
2. `src/main.ts`：12305/12444（reset/backoff）busy 早退改放行+代守卫；20980 `usage-updated` 不再整包丢弃，按卡合并；20531 单卡调用点接 requestCard/finishCard。
3. 扩 `tests/refresh-state.test.mjs`：卡片代际用例（单卡在飞时全局刷新以更新代完成 → 卡片结果按旧代丢弃；按卡合并语义）。

## 验收

- `pnpm test` + `pnpm build` 全绿。
- UI 验收（用户本人）：对同一 provider 连点 ⟳ 撞上自动刷新 → provider 侧请求只一次、spinner 不停摆、旧值不被清空。
