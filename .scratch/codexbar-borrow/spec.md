# Spec: CodexBar 借鉴批次（family gatekeeper 最小版 / FetchStrategy 流水线 / capability typed 化 / 端点情报扩展）

**Issue:** https://github.com/Aafff623/pane/issues/65

> 2026-10-25 深挖产出。CodexBar（steipete/CodexBar）为 **MIT——可抄码带署名**；gatekeeper 做最小版（完整版维护成本超线性，实测 7.8 豁免槽/provider）。证据：temp/research/20261025-codexbar-deep-dive.md（扫描语法/豁免机制/109 家全清点/对照情报）。

## A. family gatekeeper 最小版（票 01-03）

**目标**：防 provider 特判扩散——共享代码（lib.rs/spend.rs/auth_center.rs 等）里出现 family 字面量必须走 `providers::<family>::` 转发，或带 `// provider-specific: <理由>` 行内标记，或进静态白名单；新 provider 不再往共享文件加 match 分支。

**设计要点（原创实现，学 CodexBar 语义）**：
- 落点 `parse-tests/src/family_gatekeeper.rs` 新 mod，与 catalog_consistency.rs 并列；family 真源=运行时读 `provider_catalog::provider_definitions()`。
- v1 先开 5 文件：lib.rs/spend.rs/auth_center.rs/tray_projection.rs/provider_dispatch.rs；命中=行含精确 family 字面量且非 `providers::<该family>::` 纯转发臂（derived 机械判）。
- 豁免两件套：行内 `// provider-specific: <理由>`（理由必填，一个标记只免其覆盖行）+ 静态白名单 {file, 精确 trim 锚点, 可选 occurrence, expected_families, reason}；**逐 token 消耗语义**（锚点行多出新 family 仍 flag）；锚点失配/文件删除/重复(行,集)即失败。
- providers/ 整目录豁免；顶层 per-family 文件（codex_accounts.rs/*_login.rs/cursor_accounts.rs）按文件名→family 映射只免自己。
- 报告一次列全：`file:line unjustified family construct (families; count); delegate to providers::<family>:: or add '// provider-specific: <reason>'`。
- **规模警告**：不做 derived 豁免则需 700+ 槽——v1 成败在先压基数到两位数。

**分票**：01 扫描器+规则+Rust 5 文件 → 02 残量标记/白名单填量 → 03 TS 侧（main.ts/providerMechanisms.ts）+ 数据表文件豁免决策。
不做：引用指纹/聚类/AllowedConstruct 三件套（完整版 61% 是数据、维护超线性）；调色板指纹等非扫描门禁（Pane 无对应物）。

## B. FetchStrategy 流水线（票 04-06）

- **票 04 流水线骨架**（大票，分批迁移）：`FetchStrategy` trait 四段 {is_available/fetch/should_fallback/diagnostic} + 每 family strategy_chain()；错误统一 8 分类枚举（authExpired/missingCredential/permissionDenied/rateLimited/providerUnavailable/parseFailure/networkFailure/apiFailure）+ retryAfter≤10s 单次重试；降级成功用 diagnostic(forPriorFailure:) 保留前序失败解释。先落枚举与 pipeline，再分批迁移 family（与 #54 刷新闸门、#63 票 01 Retry-After 协调）。
- **票 05 capability typed 化**：provider_catalog.rs 加 PaceRule 枚举（Unsupported/ResetDatePresent/WindowDuration(u32)/Custom(fn)）+ MenuBarMetrics struct；77 家定义挪字段（编译器漏出全部 match 点）；消费方查 descriptor 不再 per-family match。
- **票 06 fetch attempts 审计**：每 family fetch 记录 [{strategy,kind,outcome,error_category}]，失败一行日志 sources="A: skipped -> B: failed: <cat>"；诊断导出 schema（schemaVersion/provider/sourceMode/auth/usage/fetchAttempts/error）照 CodexBar ProviderDiagnosticExport 模板。

## C. 扩展与体验（票 07-08）

- **票 07 候选 family 扩展**（依赖 04）：109 家全清点中 **B 组 typed 16 家 + D 组插件 55 家**为 Pane 缺口；按用户量挑 5-10 家（Alibaba/QwenCloud/Gemini/HuggingFace 等 typed 实现可 MIT 署名直译；D 组需先过票 09 评估）。全表见 research 文件 §二③。
- **票 08 多窗口前缀+离线兜底**（小票）：claude/codex 双窗口同卡时数值加 `5h/周` 短前缀（仅多窗口且唯一归因时；同值不加）；策略链全失败时离线兜底返回旧快照+降级解释。

## 备查（不立项，记录在案）

- **插件式 web provider**（CodexBar T-arch-4）：55 家 D 组的前置条件是引入嵌入式 JS（rquickjs/boa）或 TOML 声明式 endpoints——大架构决策，等票 07 实际需求出现再评估。
- 非扫描门禁 11 个、调色板指纹：Pane 无对应物。
- macOS 专属手段（/proc 端口探测、Keychain）：Windows 需换进程枚举+凭据管理器。

## 验收

见各票；01-03 机械可验（parse-tests 红/绿）；04-06 有单测+分批迁移验收；07/08 owner 场景验收。
