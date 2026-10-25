Status: implemented (2026-10-25) - automated green; owner acceptance pending
Type: task
Blocked by: —

# 03 · gen blob 尺寸上限 + 打开加固

## 改哪里

`src-tauri/src/spend.rs` L6490（`SELECT idx, data`）、DB 打开处、目录发现处。

## 做什么

1. L6490 SQL 加 `WHERE length(data) <= 1048576`，与 steps.metadata 的 1MB 上限对齐；超限跳过并计数（日志可见）。
2. DB 打开加 `busy_timeout(Duration::ZERO)`（防 AG IDE 独占写锁拖慢刷新周期；cc-switch L83 同款）。
3. `antigravity*` 目录发现后 canonicalize 去重（防符号链接双扫）。

## 验收

- parse-tests：>1MB blob 被拒（尺寸 guard 抽成 helper 后直接测）。
- 本机实测：不再 decode 2.6MB blob（重跑脚本确认跳过计数 > 0 且结果不变）。

## Comments
- 2026-10-25：已实现。gen_metadata SQL 加 1MB 上限（与 steps 对齐）、busy_timeout(0)、canonicalize 去重符号链接目录；账本行（无模型且仅 system token）丢弃规则落地 antigravity_is_ledger_row。
