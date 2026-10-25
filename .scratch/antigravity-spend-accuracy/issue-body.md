Status: ready
Issue: https://github.com/Aafff623/pane/issues/53
Priority: P1
Area: Correctness

# Antigravity 用量归日错配与解码口径修复

## 问题

Antigravity（AG）是本机 SQLite conversations 库 + protobuf 的数据源。本机全量实测（103 个 DB / 8913 行 gen_metadata，2026-10-25 只读扫描）：

1. **归日关联错误（主因，影响面最大）**：`spend.rs` 假设 `steps.idx == gen_metadata.idx`，实测每轮生成有 3 个 step（type 90/15/132 共享同一时间戳），steps:gen ≈ 3:1，同 idx 时间戳 100% 不一致（中位差 ~195s、最大 >23min），**978/2354 模型行（≈11%）归错 UTC 日**；会话越长错得越远，长会话跨午夜时整段尾巴归到前一天。
2. **field 21 完全未读**：两个独立参考实现（cc-switch v4.0.20 的 `native_models.rs`、openusage 的 `AntigravityProtoDecoder.swift`）都支持 field 21，且对 21 的语义解释互相矛盾（分档 ID vs 展示 label）。本机实测 **field 21 出现 0 次**，field 19 出现率 99.46%——所以 21 只是前向兼容，不是当前的少算来源。
3. **账本行处理**：48 行（0.54%）19/21 皆无，全部只带 system-prompt token（hex 可见 "request failed"/"RESOURCE_EXHAUSTED"），属账本/错误行。
4. **gen blob 无尺寸上限**：steps.metadata 有 1MB SQL 预过滤，`gen_metadata.data` 没有（实测最大 2.6MB），直接 decode 有浪费。

## 证据

- Pane 现状：`src-tauri/src/spend.rs` L6208-6260（`decode_antigravity_call`：field 19 缺失即丢行、field 9→4→1 时间戳）、L6277-6286（`antigravity_ts_from_step`）、L6400-6530（扫描器与归日链：内嵌 ts → `step_ts.get(&idx)` L6505 → summaries → db mtime → now）、L6490（`SELECT idx, data` 无上限）。
- 参考 A：`temp/cc-switch/src-tauri/src/services/session_usage_agy/native_models.rs`（10-24 提交：steps.metadata field 20 → gen idx 跳转，覆盖 98.9%；field 21 分档 ID 优先）。
- 参考 B：`temp/openusage/Sources/OpenUsage/Providers/Antigravity/AntigravityProtoDecoder.swift` + `AntigravityDbUsageScanner.swift`（field 19 主/21 label 兜底、-default 占位、-tiered 剥离、双无时间戳丢行、1MB SQL 预过滤）。
- 完整实测数据与行级方案：`temp/research/20261025-deepdive-5-topics.md` §1（本地文件，不随 issue 公开）。

## 修复范围（详见 spec.md 与 issues/）

1. model 解析改「19 优先、21 兜底」，`AntigravityCall.model` 改 `Option<String>`；`-tiered` 计价前剥离；`-n` 不剥离（既定决策 pricing.rs:907）。
2. **归日修掉同 idx 假设**：遍历 steps 解析 metadata field 20→field 3=gen idx 建 `gen_idx→secs` map；无引用的 94 行用邻居插值；仍无才走 summaries/mtime 粗锚。不对齐 openusage 的「双无丢行」（Pane 31 天 cutoff + summaries 锚是不同场景，丢整行代价高于错日）。
3. gen blob 加 1MB SQL 预过滤；DB 打开 `busy_timeout(Duration::ZERO)`；目录发现后 canonicalize 去重。
4. parse-tests 全覆盖（fixture 助手已存在于 spend.rs L5564-5645）。

## 验收

- 重跑本机实测脚本：UTC 日错配率从 ~11% 降到接近 0；field-20 路径覆盖率 ≥98%。
- parse-tests 全绿；改动只影响 antigravity 一个 provider 的聚合。
- 回归安全：归日 map 建空时退化到插值/粗锚，最坏日粒度变粗，不崩；21 兜底若实为 label 只多一个 unknown 分组，不算错钱。

## 不做

- 不对齐 openusage「双无时间戳丢行」（决策见上）。
- 不引入 cc-switch 的 `proxy_request_logs` 回查（Pane 无该表）。
- 不动 accounts / auth center / DB 只读打开方式之外的部分。
