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

## 验收

- 每个来源有独立模块与回归测试；`/v1/sources` 状态与实际读取一致。
- 归因默认关闭；开启后本地展示，关闭后字节与现在一致。
