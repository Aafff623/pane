Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · MCP 注解 + httpapi schema/statusCode + 端口通知

## 做什么

1. mcp/pane-mcp/lib.mjs：`READONLY` 常量展开进六 TOOLS；加 5 行断言（tests 或内联）。
2. httpapi.rs：providers_payload_from/status/sources/doctor 统一 schemaVersion:1；providers/sources 补 generatedAt；provider_full_json 每行加 statusCode（词表见 spec，owner 确认后落）；测试同步更新。
3. httpapi.rs start(app)：收 AppHandle，bind 失败→eprintln+系统通知。

## 验收

三项子项验收（spec 验收节）；/v1/usage 字节不变；既有测试零回归；`cargo check`+`pnpm build` 绿；owner 确认 statusCode 词表。
