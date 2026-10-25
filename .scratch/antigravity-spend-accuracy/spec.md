# Spec: Antigravity 用量归日错配与解码口径修复

## 背景与问题定义

Antigravity 数据源 = 本机 `~/.gemini/antigravity*/conversations/*.db`（SQLite），用量在 `gen_metadata.data`（protobuf blob）里，每轮生成一行；`steps` 表有该轮的多个 step。Pane 要把每行用量归到「消费发生的那一天」并按模型计价。

2026-10-25 本机全量实测（103 DB / 8913 行 gen_metadata）确立的三个事实：

| 事实 | 数据 |
|---|---|
| field 19（内部模型 ID）出现率 | 99.46%（8865/8913），取值仅 gemini-3.8-flash / -flash-n / 3.7-flash / 3.6-flash 四种 |
| field 21（分档 ID 或展示 label，两参考实现语义矛盾） | **全库 0 次** |
| steps:gen 比例 | ≈3:1（每轮生成 3 个 step 共享同一 ts） |
| 同 idx 时间戳一致性 | 0%（中位差 ~195s，最大 >23min） |
| 归错 UTC 日的行 | 978/2354 ≈ **11%** |
| field-20 反查覆盖率（cc-switch 方案） | 98.9%（8771/8865） |
| 账本/错误行（19/21 皆无） | 48 行（0.54%），仅 system-prompt token |
| 内嵌时间戳（field 9→4→1） | 0% 存在 |
| gen blob 大小 | 最大 2.6MB（15 文件样本 3 个 >1MB） |

## 方案

### 1. model 解析：19 优先、21 兜底（spend.rs L6216-6222）

```
解析顺序：
1. field 19：trim 后非空、非 "unknown"、不以 "-default" 结尾 → 用之
2. 否则 field 21：仅接受 wire type 2（length-delimited）；varint 等坏线型忽略（不遮挡 19）
3. 两者皆无 → model = None（见规则 2）
```
- `AntigravityCall.model: String` → `Option<String>`。
- `-tiered` 后缀在 `probe_lookup` 前剥离（openusage 口径）；`-n` 不剥离（pricing.rs:907 既定决策）。
- 21 兜底若实为展示 label：值进 unpriced 分组，token 仍计、$0+⚠，不算错钱。

### 2. 丢行规则（spend.rs L6498 附近，对齐 openusage）

- 无 model 且 usage 的 field 2/3/5 全为 0（即仅 system-prompt）→ 账本行，丢（本机 48 行全落这里，行为不变）。
- 无 model 但有生成 token → 保留，model 记 unknown 占位，走现有 `note_unpriced`（本机 0 行触发，前向兼容）。

### 3. 归日：field-20 跳转（核心修复）

现状链：内嵌 ts → `step_ts.get(&idx)`（**错：同 idx 假设**）→ summaries last_modified → db mtime → now。

新链：
1. 建 map：遍历 steps，解析 metadata 的 field 20 → field 3 = gen idx，配该 step 的 ts，建 `gen_idx → secs`（改 L6472-6489）。
2. 归日查新 map（改 L6505）。
3. map 未覆盖的 gen（94 行，首轮 gen 0 之类）：**邻居插值**——取前一个已引用 gen 的 ts，无则后一个；仍无才走 summaries → mtime → now 粗锚。
4. **不对齐 openusage「双无时间戳丢行」**：Pane 的粗锚策略保留。理由：Pane 是 31 天 cutoff + summaries 锚的场景，丢整库/整行的代价高于错日；openusage 的丢行配合其滑动窗口扫描。

### 4. blob 上限与加固

- L6490 SQL 加 `WHERE length(data) <= 1048576`，与 steps 上限对齐；超限跳过（至少不再 decode 2.6MB blob）。
- DB 打开 `busy_timeout(Duration::ZERO)`（cc-switch L83：防独占写锁拖慢刷新）。
- 目录发现后 canonicalize 路径去重（防符号链接双扫；实测 antigravity-ide 无 conversations 目录，风险低但便宜）。

## 测试方案（parse-tests）

fixture 助手已存在于 spend.rs L5564-5645（`pb_msg`/`pb_varint_field`/`antigravity_blob`/`antigravity_step_md`）。新增 `antigravity_step_md_gen(secs, gen_idx)`（field 1 Timestamp + field 20{3=gen_idx}）。

断言清单：
- 仅 21 无 19 → 解码成功且 model 取 21；19+21 并存 → 取 19。
- 19 = "unknown" 或以 `-default` 结尾 → 取 21；21 为 varint 线型 → 不隐藏 19。
- 无 model 且 usage 仅 field 1 → None；无 model 但有 output → Some 且 unknown 占位。
- **归日**：3 个 step（ts=T、f20=1）+ gen idx=1，同时令 `steps[1]` ts 明显不同 → 断言归日取 T（f20 路径）。
- gen 无 f20 引用 → 邻居插值 ts；`-tiered` 计价前剥离；>1MB blob 被拒（尺寸 guard 抽 helper 后直接测）。

## 验收（机械 + 本机实测）

1. `cd parse-tests; $env:PATH="D:\Tools\mingw64\bin;$env:PATH"; cargo +stable-x86_64-pc-windows-gnu test` 全绿。
2. 重跑实测脚本（复刻 2026-10-25 的只读扫描）：field-20 路径覆盖率 ≥98%，UTC 日错配率 ~11% → 接近 0。
3. `cargo check`（src-tauri，mingw PATH）通过；UI 验收由用户本人确认。

## 风险与回滚

| 风险 | 缓解 |
|---|---|
| 归日改动影响最大：某版 AG 改掉 steps 结构（无 field 20）→ map 建空 | 退化到插值/粗锚，最坏日粒度变粗，不崩；回滚 = 还原 L6472-6489 与 L6505 两处 |
| 21 实为展示 label | 只多 unknown 分组，不算错钱 |
| 丢行放宽 | 本机 0 行触发；未来触发时 AG 总 token 小幅上升，属修正 |
| panic 行为变化 | 从「卡片消失」变「错误快照」进 fail_state bench，需在 release note 提 |

改动收敛在 antigravity 一个 provider 的聚合路径，不碰 accounts / auth center / DB 只读语义。
