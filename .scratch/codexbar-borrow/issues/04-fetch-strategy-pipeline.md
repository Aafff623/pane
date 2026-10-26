Status: ready
Type: task
Blocked by: —
Priority: P2

# 04 · FetchStrategy 流水线骨架 + 错误 8 分类（大票，分批迁移）

## 做什么

1. Rust 侧定义 `FetchStrategy` trait 四段 {is_available/fetch/should_fallback/diagnostic}（学 CodexBar ProviderFetchPlan.swift:335 语义，原创实现）；每 family 定义 strategy_chain()。
2. 错误统一 8 分类枚举（authExpired/missingCredential/permissionDenied/rateLimited/providerUnavailable/parseFailure/networkFailure/apiFailure）+ retryAfterSeconds ≤10s 单次重试（ProviderFetchDelayedRetry 语义）；与 #63 票 01 的 Retry-After 解析衔接。
3. 降级成功用 diagnostic(forPriorFailure:) 保留前序失败解释（不丢「为什么走到备用通道」）。
4. pipeline 先行，77 家分批迁移（每批 5-10 家，parse-tests 逐批断言）。

## 验收

- pipeline 单测：isAvailable 跳过/shouldFallback 继续/diagnostic 保留三语义。
- 分批迁移每批全绿；`cargo check` 绿。
- 与 #54 刷新闸门、#56 定价探针无耦合冲突。

## 注意

大票，预估 3-5 天；先与 owner 对齐迁移批次表。
