# Spec: httpapi 契约卫生（schemaVersion/statusCode + MCP readOnlyHint + 端口提示）

**Status:** ready · **Priority:** P3 · **Area:** API
**Issue:** https://github.com/Aafff623/pane/issues/76
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-8-C（三项小件实现级设计）

## 范围（1 票，~2.5-3.5h，三个不依赖子项一个下午梭哈）

1. **MCP readOnlyHint**（0.5-1h）：mcp/pane-mcp/lib.mjs:5-46 六工具加 annotations——`READONLY={readOnlyHint:true,destructiveHint:false,idempotentHint:true,openWorldHint:false}`（server.mjs:267 tools/list 原样返回无需改；协议 2025-06-18 支持）；加 5 行断言（每工具 readOnlyHint===true）。
2. **httpapi schema 契约**（1.5-2h）：status（:280-322）/doctor（:480-488）已有 version/generatedAt，providers（:250-276）/sources（:490-498）缺——四对象 payload 统一加 `"schemaVersion":1`，providers/sources 补 `"generatedAt"`；provider_full_json（:225-238）每行加 `"statusCode":u16`（ok→200/no_credentials→401/needs_setup→424/error→503，**词表需 owner 确认**），原 status 字符串保留，pane-mcp 等消费方零影响；`/v1/usage` 裸数组不动（破兼容单独议）；httpapi.rs 测试直接驱动 payload_from（:250/:810）同步更新。
3. **端口占用 UI 可见**（0.5h）：httpapi.rs:570-584 bind 失败只 eprintln（:575）→`start(app)` 收 AppHandle，失败时 eprintln 保留+一条系统通知「本地 API 端口 6736 被占用，本会话 API 未启动」（通知通道现成，lib.rs:3648-3656 alerts 在用）。

## 验收

- MCP：tools/list 六工具带 annotations；断言 5 行绿。
- schema：四个 payload 有 schemaVersion；provider 行有 statusCode；/v1/usage 字节不变；既有测试零回归。
- 端口：占用时系统通知出现（手测：起一个占 6736 的进程再启动 pane）。
- `cargo check`+`pnpm build` 绿；owner 确认 statusCode 词表。

## 不做

不改 /v1/usage 裸数组形状；不做设置页红点（低频一次性事件，通知足够）；不给 MCP 加 write/refresh 工具（read-only 是 pane-mcp 设计前提）。
