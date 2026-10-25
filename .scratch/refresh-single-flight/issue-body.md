Status: ready
Issue: https://github.com/Aafff623/pane/issues/54
Priority: P2
Area: Architecture

# provider 刷新单飞去重闸门（refresh_gate）

## 问题

同一 provider 的刷新请求没有去重，并发入口之间零协调：

- `refresh_provider`（lib.rs:2648-2655，卡片 ⟳ / reset 刷新 / backoff 重试）：**无锁无去重**，直穿 `fetch_provider_snapshot`(2681) 出网；qoder 签到是网络副作用，重复调用 = 重复签到。
- 全量周期（`fetch_usage` 2624 / 后台 `spawn_auto_refresh` 3624 → `fetch_and_broadcast_usage` 3598）：全局 `usage_fetch_lock`(2876-2879) **只互斥不去重**——第二个周期等锁后整轮重拉。
- 前端 busy 闸（refreshState.ts:15-17）只挡前端自己的 `refresh()`；三个单卡调用点（main.ts:12307/12450/20531）各自为政，reset/backoff 在 busy 时直接 return（12305/12444），`usage-updated` 在 busy 时整包丢弃（20980）。

## 方案（详见 spec.md）

新增 `src-tauri/src/refresh_gate.rs`：`IN_FLIGHT: HashMap<card_id, Entry{waiters, kind, started, generation}>`。同 id 在飞 → 挂 waiter 共享 leader 结果（fan-out）；否则当 leader 执行。leader 所有出口（Ok/Err/120s 超时/panic）按 generation 摘除自己再 fan-out。Manual（单卡刷新）优先，可 promote 未 started 的 Background。**不加 Semaphore**（Pane 每卡一 task 全并发，串行只会拖慢整轮）。与 `usage_fetch_lock` 嵌套其外、粒度是卡，无死锁。

接入 3 处：① `refresh_provider`(2648) 整体包进 `run(id, Manual, …)`；② 周期 spawn 点(3284-3287) 改 `spawn(run(id, Background, fut))`——一处覆盖 ~8 个 push 站点；③ `test_provider` P0 不接（语义不同，要真实请求）。前端 refreshState.ts 加 `cardInFlight` + 代际比较，busy 早退改放行+代守卫。

**许可证说明**：思路来源 `temp/cockpit-tools`（jlcodes99/cockpit-tools）是 CC BY-NC-SA 4.0——只学数据结构与调度语义，代码必须原创重写，不搬运。

## 验收

- parse-tests 收 `refresh_gate.rs` 加 `#[cfg(test)]`：并发 10 次 `run("k", …)` → op 内计数 ==1 且 10 结果相同；1 Manual+5 Background 混合 ==1；op sleep 超时 → 全部 waiter 得 Err 且 map 空；op `panic!()` → waiter 得 Err 不挂起、map 空。
- 前端扩 `tests/refresh-state.test.mjs` 卡片代际用例；`pnpm test`/`pnpm build`/`cargo check`/parse-tests 全绿。
- UI 验收：对同一 provider 连点 ⟳ 撞上自动刷新，provider 侧请求只一次。

## 不做

- 不抄 cockpit-tools 的双 Semaphore、队列深度封顶、后端 i18n message()、runtime_snapshot 批处理参数、generation 作批取消令牌（均为其串行 worker 架构产物，与 Pane 每卡一 task 不匹配）。
- epoch 取消排队后台任务（P2，见 issues/03）——Pane 有全局锁串行化周期，价值有限。
