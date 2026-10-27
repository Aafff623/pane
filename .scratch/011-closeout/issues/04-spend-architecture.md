Status: pending
Type: task
Blocked by: 02

# 04 · spend 架构收口（#42 / #43）

## #42 来源适配器

1. 把 `spend.rs` 里的 19 个来源解析拆成 `spend/sources/*` 模块；注册表统一
   `roots/collector/status`，删除重复的路径与状态逻辑。
2. 修正 gemini 注册表的 `needs_probe: true`（已有 fixture 与解析测试）。
3. 扫描耗时对比记录：在 `source_statuses`/诊断里暴露每源耗时或写入本地
   evidence，不新增遥测。

## #43 归因

1. 项目/分支归因：opt-in 配置，默认关闭；从本地日志提取 project/branch，
   账号与路径做假名化；不进入 HTTP API 默认输出。
2. `pricing_mode` 接到 UI 徽标（billing/stored/missing）。
3. web search/flex 分项计费：无可靠价目则保留关闭并文档化，不猜价。

## 核实（2026-10-27，逐行对代码）

本票不是零起点，之前说「未开始」不准确：

- **注册表已经在了。** `src-tauri/src/spend/sources/mod.rs` 有 `SourceSpec`
  （id / tool / form / collector / roots / feeds / runtime / needs_probe /
  parse_probe）和 `registry()` / `statuses()`；`spend.rs:292` 的 `source_statuses`
  就是 `sources::statuses()`，`spend.rs:4725` 也走 `sources::registry()`。
- **但物理拆分只有 1/19。** `spend/sources/` 里只有 `copilot.rs` + `mod.rs`；
  其余来源的 `collector` 仍是 spend.rs 内的函数名（如 `collector: "claude_line"`），
  代码还堆在 7801 行的 spend.rs 里。
- **#42-2 未修。** `sources/mod.rs:290` 的 `gemini-jsonl` 仍是
  `needs_probe: true`，而本票的理由（已有 fixture 与解析测试）依然成立。
- **#42-3 未做。** 注册表没有每源耗时字段，`SpendSourceStatus` 也不带时长。
- **#43 三项全部未做**：`pricing_mode` 徽标没接 UI、项目/分支归因没有、web search/
  flex 分项计费没有文档化。

所以本票的真实进度是「注册表抽象 ✅ / 迁移 1/19 / gemini 标记 ❌ / 耗时 ❌ / 归因 ❌」。

## 验收

- 每个来源有独立模块与回归测试；`/v1/sources` 状态与实际读取一致。
- 归因默认关闭；开启后本地展示，关闭后字节与现在一致。
