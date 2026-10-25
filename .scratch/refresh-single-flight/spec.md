# Spec: provider 刷新单飞去重闸门（refresh_gate）

## 问题定义

刷新有三个并发入口，彼此零协调：
1. `refresh_provider`（lib.rs:2648，卡片 ⟳ / reset 刷新 / backoff 重试）——无锁无去重，直穿出网；qoder 签到是网络副作用，重复调用 = 重复签到。
2. 全量周期（后台 `spawn_auto_refresh` 3624 → `fetch_and_broadcast_usage` 3598，及 `fetch_usage` 2624）——全局 `usage_fetch_lock`(2876) 只互斥不去重，第二周期等锁后整轮重拉。
3. `test_provider`(2664)——无锁，但 `allow_disabled=true` 语义不同，P0 不接。

前端兜底全是视图级：`refreshState.ts` busy 闸只挡前端自己的 refresh()（12483-12495），busy 时非 force 丢弃、force 合并不超 1 个待办（23-35）；三个单卡调用点各自为政。

## 设计（原创重写；思路来源 cockpit-tools `codex_quota_refresh_scheduler.rs`，CC BY-NC-SA 4.0，仅学语义）

### 模块 `src-tauri/src/refresh_gate.rs`

```rust
IN_FLIGHT: LazyLock<Mutex<HashMap<String /*card id*/, Entry>>>
Entry { waiters: Vec<oneshot::Sender<T>>, kind: Manual|Background, started: bool, generation: u64 }

pub async fn run<T, F>(id: &str, kind: Kind, op: F) -> T
where F: Future<Output = T>
```

- 同 id 在飞 → 注册 waiter 共享结果（follower）；否则当 leader spawn 执行。
- leader 所有出口（Ok/Err/超时/panic）：先按 generation 校验摘除自己，再 fan-out；poison 一律 `into_inner()`。
- `REFRESH_TIMEOUT = 120s`（`tokio::time::timeout` + `catch_unwind` + `AssertUnwindSafe`，panic → 错误结果而非卡片消失）；waiter patience 130s（必须 > leader 超时），超时得 Err（前端 catch 只停 spinner 保留旧值，main.ts:20540 注释即此契约）。
- 优先级：Manual 到达时同 id Background 未 started → promote；已 started → 共享在飞结果。**不加 Semaphore**（Pane 每卡一 task 全并发，lib.rs:2923-2926 注释即爆栈改 spawn 的由来；串行信号量只会拖慢整轮）。`MAX_WAITERS=32`。
- 无独立清理线程；entry 生命周期与 leader 任务绑定。

### 接入点

1. `refresh_provider`(2648)：fetch + `mirror_week_wall` + `apply_qoder_checkin`(2652-2653) 整体包进 `run(id, Manual, …)`，gate 泛型保 `Result`。
2. 周期 spawn 点(3284-3287)：改 `spawn(run(id, Background, fut))`——一处覆盖全部 ~8 个 push 站点（2935-3264），且包在 `guarded` 之外，bench 更新/checkin 也共享。
3. `test_provider` P0 不接。

### 与 usage_fetch_lock 的关系

保留不动（它保护 snapshot cache/history/telemetry 整轮互斥）；gate 嵌套其外、粒度是卡。无死锁：周期持全局锁等某卡 Manual leader，而 Manual leader 不持全局锁。

### 前端（src/refreshState.ts + src/main.ts）

- 新增 `cardInFlight: Set` + `requestCard/finishCard`；`accepted` 复用卡片键 `card:<id>`，`acceptCardResult` 用与 53-57 行同款代际比较（单卡在飞时全局刷新以更新代完成 → 卡片结果按旧代丢弃）。
- 12305/12444 的 busy 早退改放行 + 代守卫；`usage-updated`(20980) 不再整包丢弃，按卡合并（无在飞单卡的直接采用）。
- `request()` force 合并不超 1 待办的逻辑不动。

## 验收

见 issue-body.md；核心机械验收：并发 10 次 run 同 id → 出网计数 ==1；panic → waiter 得 Err 不挂起、map 空。

## 风险

- 周期持锁等 Manual leader 被 120s 超时兜底，不永久挂。
- panic 行为从「卡片消失」变「错误快照」进 fail_state bench（2035-2043），release note 需提。
- waiter patience 必须 > leader 超时，否则误报。
- 代际比较写错会丢更新，须同款单测。

## P2（延后）

epoch 取消排队后台任务：`BACKGROUND_EPOCH` AtomicU64，force 全量刷新 bump，未 started 旧 epoch Background entry 收 deferred。Pane 有全局锁串行化周期，价值有限。
