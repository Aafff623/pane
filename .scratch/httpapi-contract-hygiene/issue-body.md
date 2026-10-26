Status: ready
Issue: https://github.com/Aafff623/pane/issues/76
Priority: P3
Area: API

# httpapi 契约卫生：schemaVersion/statusCode + MCP readOnlyHint + 端口占用提示

## 范围（1 票 3 子项，详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-8-C）

1. MCP readOnlyHint：pane-mcp 六工具加 annotations（四 hints 一致）+5 行断言。
2. httpapi schema：四对象 payload 统一 schemaVersion:1，providers/sources 补 generatedAt；provider_full_json 每行加 statusCode（ok→200/no_credentials→401/needs_setup→424/error→503，词表 owner 确认）；/v1/usage 裸数组不动。
3. 端口占用：bind 失败时系统通知「本地 API 端口 6736 被占用，本会话 API 未启动」（不做设置页红点）。

## 验收

tools/list 带注解；四 payload 有 schemaVersion；provider 行有 statusCode 且消费方零影响；占用时通知出现；/v1/usage 字节不变；既有测试零回归。
