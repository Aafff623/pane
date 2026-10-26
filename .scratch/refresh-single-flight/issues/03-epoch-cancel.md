Status: done (2026-10-26)
Type: task
Blocked by: 01, 02

# 03 ·（P2）epoch 取消排队后台任务 + 去重命中日志

## 做什么

1. `BACKGROUND_EPOCH` AtomicU64：force 全量刷新 bump epoch，未 started 的旧 epoch Background entry 收 deferred（不取消运行中）。
2. 去重命中记一条 debug 日志（id + kind + waiter 数），便于验证与排障。

## 验收

- parse-tests：epoch bump 后新 Manual 到达 → 旧 Background 未 started 的不再执行；运行中的照常完成。

## 注意

Pane 有全局锁串行化周期，本项价值有限，排最后。
