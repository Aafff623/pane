Status: done (2026-10-26) — Claude/MiniMax already prefer log cost; local Grok unified.jsonl audit found no cost field, so that source is skipped per spec.
Type: task
Blocked by: —

# 01 · CLI 自报成本优先

## 做什么

1. 先验证：grep 真实日志样本（Grok `~/.grok/...updates.jsonl` 找 costUsdTicks/costUsd；其他源逐个排查），产出「哪些源自带成本」清单；无字段的源跳过。
2. spend.rs 行解析：自带成本 → 入账用它；无 → catalog 价；两者并存时以自报为准并记漂移告警（偏差>阈值一行日志）；partial 标记视为下界。
3. 与 #56 定价时间线协调：自报成本不受价格快照影响（它是事实，非推算），在文档中写明口径优先级。

## 验收

- parse-tests：自报/目录两路径断言 + 漂移告警触发；本机 Grok 实测金额对照。
