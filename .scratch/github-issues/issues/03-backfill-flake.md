# 03: backfill 测试 flake 修复（GitHub #6）

**What to build:** `scheduled_backfill_does_not_block_key_cards` 不再依赖固定时间窗轮询，改用确定性完成信号；保留「后台任务不能阻塞 Key 卡片读取」核心断言；CI 高负载下稳定。

**Blocked by:** None

**Status:** implemented（2026-10-18，78c3d52；复现率 ~6%，32/32 稳定）

**来源**：<https://github.com/Aafff623/pane/issues/6>

## 验收标准（issue 原文照抄）

- [x] 查明超时发生在哪个异步阶段
- [x] 保留「后台任务不能阻塞 Key 卡片读取」的核心断言
- [x] 消除不必要的调度时序依赖
- [x] 连续运行相关测试及完整 CI，确认结果稳定
- [x] 不单纯依靠延长超时时间掩盖问题

## 已知事实（已对照 v0.8.6 源码核实）

`src-tauri/src/providers/onenewapi/snapshot.rs:1171-1240`：
- 500ms「不阻塞 key 卡片」断言（1218-1222）；
- 3 秒 deadline 轮询等 status 探针到达（1224-1228）；
- 3 秒 deadline 轮询等后台 backfill 持久化（1230-1238）；
- 服务端 5 秒 gate 等 release 信号（1185-1191）。

诊断路径：负载下重复运行建立复现回路 → 定位先爆的时间窗 → 以 channel/condvar/notify 替代轮询。
