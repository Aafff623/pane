Status: partial (2026-10-26) — changed-lines gatekeeper + CI + tests shipped; full baseline/triage not needed for v1.
Type: task
Blocked by: —

# 01 · family gatekeeper v1：扫描器 + 规则 + Rust 5 文件

## 做什么

1. 新建 `parse-tests/src/family_gatekeeper.rs`：family 真源=运行时读 `provider_catalog::provider_definitions()`；扫描目标 v1=lib.rs / spend.rs / auth_center.rs / tray_projection.rs / provider_dispatch.rs。
2. 命中规则：行含精确 family 字面量（词边界）且**非** `providers::<该family>::` 纯转发臂（derived 机械判，参照 CodexBar derived 语义）。
3. 豁免两件套：行内 `// provider-specific: <理由>`（理由必填，一个标记只免其覆盖行）；静态白名单 {file, 精确 trim 锚点, 可选 occurrence, expected_families, reason}——逐 token 消耗（锚点行多出新 family 仍 flag）；锚点失配/文件缺失/重复(行,集)即失败。
4. providers/ 整目录豁免；顶层 per-family 文件（codex_accounts.rs/*_login.rs/cursor_accounts.rs）按文件名映射只免自己。
5. 报告一次列全：`file:line unjustified family construct (families; count); delegate to providers::<family>:: or add '// provider-specific: <reason>'`。

## 验收

- v1 跑 5 文件：derived 豁免后基数为两位数（若仍数百 → 先补 derived 判定，不要堆豁免槽）。
- parse-tests 全绿即守门生效；故意在 lib.rs 加一个裸 family 字面量 → 红，加理由注释 → 绿。

## 证据

temp/research/20261025-codexbar-deep-dive.md §一（GKT 扫描语法/豁免语义/维护成本实测 7.8 槽每 provider）。
