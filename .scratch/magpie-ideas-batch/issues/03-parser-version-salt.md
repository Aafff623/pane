Status: done (2026-10-26) — per-source parser_rev in PersistEntry; stale source drops only its own files; parse test green.
Type: task
Blocked by: #56（pricing-effective-time 的 PERSIST_VERSION 迁移先行）

# 03 · 扫描缓存 parser_rev（代码版本哈希失效）

## 做什么

1. spend 光标（byte-offset+boundary_hash，spend.rs:295）与 PersistEntry 加 per-source `parser_rev` 常量。
2. load 时 parser_rev 不匹配 → 仅重扫该源（替代 PERSIST_VERSION 全局全弃；PERSIST_VERSION 保留给跨源结构变更）。
3. 与 #56 协调：#56 升 v6 一次性全重解析在其前完成；03 的 per-source rev 在 v6 基线上建立。

## 验收

- 改某源解析逻辑 + bump 其 parser_rev → 仅该源重扫、其余 cursor 原样（parse-tests 断言）。
- 本机实测：升级模拟后重扫范围仅受影响源。

## 证据

temp/research/20261025-magpie-deep-dive.md §一（internal/usage/request_disk.go:50-77 指纹=CallCacheVersion+源码 hash）。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
