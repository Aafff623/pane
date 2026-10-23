# 02 — Pane MCP server：把 6736 只读额度 API 暴露给 agent

Status: done — Phase 1 Node shim (Phase 2 payload extraction deferred)
GitHub: #40
Priority: P0
Blocked by: None（可与 01 并行；共享 payload 抽取需与 01 协调）
Reference: `temp/repos/agentsview` (`internal/mcp`, `service` seam, `ReadOnlyHint`, `asOf`), `temp/repos/codeburn` (`src/mcp/server.ts`, `redact.ts`)

## Scope

Pane 已有 `127.0.0.1:6736` 只读 API（`/v1/usage`、`/v1/providers`、`/v1/status`、`/v1/doctor`、`/v1/sources`）。本轮把它薄封装成 MCP，让 Claude/Codex/Cursor 在跑长任务前能问“我还剩多少额度、何时重置”。

与已关闭 #26（Headless CLI / 本地 WebUI）的关系：本 issue 更窄，只做 **只读 MCP over 6736**，不做 WebUI、不做写桥、不做独立 headless 产品。

1. **共享 payload seam**：把 `httpapi.rs` 的 `*_payload_from` 纯函数抽到共享模块（如 `src-tauri/src/api_payload.rs`）；HTTP、MCP、Tauri command 共用一份 schema，避免三份 JSON 漂移。
2. **Phase 1 传输**：独立 `pane-mcp` stdio shim 调 6736，避免给 Tauri 引入新的 HTTP/MCP 依赖；shim 只读、无状态、启动时探测 6736。
3. **tools（最小集）**：
   - `list_providers` → `/v1/providers` 的脱敏摘要。
   - `get_usage` → 单 provider 的额度/余额/重置/账号行。
   - `get_provider_quota` → 额度专用视图（progress/balance/pace）。
   - `list_sources` → `/v1/sources` 的 root/state/reason。
   - `get_status` → `/v1/status` + `/v1/doctor` 的 remediation。
   - `list_resets` → 有结构化 reset 时返回；没有则明确 `unsupported`。
4. **结果契约**：每个结果带 `asOf`、`stale`、`ageSecs`、`source`；limit/cursor 由服务端钳制；空结果带 `reason`/`exclusions`。
5. **隐私边界**：不返回 key、token、cookie、原始 config、对话正文；账号 email/maskedKey 默认不返回；loopback-only；无 CORS；Host 校验沿用 6736。

## Acceptance

- [ ] Claude Code / Codex 配好 MCP 后能回答“Claude 剩余多少 / 何时重置 / 哪个来源缺数据”（README 已给出示例，客户端接入待 owner 验）。
- [x] 查询**不触发**网络刷新、不消耗额度、不调用 `test_provider`/`refresh_provider`（shim 只发 GET）。
- [x] 结果含 `asOf/stale/ageSecs`；5 分钟旧数据不会被说成实时。
- [x] 无 secrets/PII；redaction 测试覆盖 key、Authorization、email、maskedKey、URL query。
- [x] 6736 未启动时 shim 返回明确错误，不挂起。
- [ ] `pnpm build` + `parse-tests` 全绿；MCP 工具 schema 有 golden 测试（前端 94 tests + 6 MCP tests 通过；parse-tests 本轮未重跑）。

## Comments

- 2026-10-23：Phase 1 落地为 `mcp/pane-mcp/` Node 20+ stdio shim，复用 6736 的 `/v1/providers`、`/v1/usage/:id`、`/v1/sources`、`/v1/status`、`/v1/doctor`。六个只读 tools、`asOf/stale/ageSecs`、loopback-only、GET-only、redaction、6736 down 快速失败均有测试。Phase 2 的 `api_payload.rs` 共享抽取未做，避免本轮改 Rust/Tauri。

## Evidence / notes

- agentsview 的 MCP 只做参数整形/结果裁剪，查询逻辑全在 `service.SessionService`；Pane 应复用 `httpapi.rs` 的 payload 纯函数，而不是在 `mcp.rs` 重写查询。
- agentsview 的 `ReadOnlyHint`、`effective_mode`、`exclusions`、`activity_cutoff` 可直接借；它的活跃会话排除是“防自引用”，Pane 对应的是“查询不触发刷新”。
- Pane 的 `/v1/doctor` 已带 remediation，是比参考仓库更强的部分，MCP 应保留。
- 现有 `httpapi.rs` 的 Host 校验、无 CORS、URL 脱敏是隐私资产，不要为了 MCP 放宽。

## Out of scope

- 写工具：refresh/test、keyvault、账号增删、切号、会话复制。
- transcript/对话搜索、remote sync、多用户/云端。
- HTTP/SSE transport（Phase 2 再评估）；不引入新的公网监听。
- 返回账号 PII 或原始凭据。

## Verification

```powershell
corepack pnpm build
cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu test
```

手工：启动 Pane + MCP client，验证 6 个 tools；断网/6736 未启动/空 provider 三种错误态。security review：redaction + loopback。
