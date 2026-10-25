Status: done (2026-10-25)
Type: task
Blocked by: —

# 01 · refresh_gate 模块 + Rust 侧两处接入 + 单测

## 做什么

1. 新建 `src-tauri/src/refresh_gate.rs`（设计见 spec.md：IN_FLIGHT map + waiter fan-out + Manual/Background + generation + 120s 超时 + panic 兜底）。
2. 接入 `refresh_provider`(lib.rs:2648)：fetch+mirror+checkin 整体包进 `run(id, Manual, …)`。
3. 接入周期 spawn 点(lib.rs:3284-3287)：`spawn(run(id, Background, fut))`。
4. `test_provider`(2664) 不接。
5. parse-tests 以 `#[path]` 收 `refresh_gate.rs` 加 `#[cfg(test)]`：并发 10 次同 id → 计数==1 且结果相同；Manual+Background 混合==1；超时→waiter 全 Err 且 map 空；panic→waiter 得 Err 不挂起、map 空。

## 验收

- parse-tests / `cargo check` 全绿；`usage_fetch_lock` 语义不变（整轮互斥保留）。
- 无死锁：周期持全局锁等 Manual leader 场景有超时兜底。

## 注意

原创重写，不搬运 cockpit-tools 代码（CC BY-NC-SA）；可在注释标注「思路来源：cockpit-tools scheduler 语义」。
