Status: implemented (2026-10-25) - automated green; owner acceptance pending
Type: task
Blocked by: 01, 02, 03

# 04 · parse-tests 全覆盖 + 本机实测回归脚本

## 做什么

1. 新增 `antigravity_step_md_gen(secs, gen_idx)` fixture 助手（field 1 Timestamp + field 20{3=gen_idx}），复用 spend.rs L5564-5645 现有助手。
2. 断言清单按 spec.md「测试方案」节逐条落（19/21 优先级、坏线型、账本行、unknown 占位、f20 归日、邻居插值、-tiered 剥离、尺寸上限）。
3. 复刻 2026-10-25 的本机只读实测脚本为可重跑形式（103 DB 扫描：field 19/21 出现率、f20 覆盖率、UTC 日错配率），放 `parse-tests` 或作为一次性验证记录。

## 验收

- `cd parse-tests; $env:PATH="D:\Tools\mingw64\bin;$env:PATH"; cargo +stable-x86_64-pc-windows-gnu test` 全绿。
- 回归脚本输出：错配率接近 0、f20 覆盖率 ≥98.9%。
- `cargo +stable-x86_64-pc-windows-gnu check`（src-tauri）通过。

## Comments
- 2026-10-25：新增三组测试（模型解析优先级/账本行与 unknown 占位/跳表与邻居插值）；antigravity 套件 11 通过（旧用例修正为 Option 模型）；全量串行回归见主对话记录。
