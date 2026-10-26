Status: ready
Type: task
Blocked by: —

# 05 · capability typed 化（PaceRule / MenuBarMetrics）

## 做什么

1. provider_catalog.rs 加 `PaceRule` 枚举（Unsupported / ResetDatePresent / WindowDuration(u32) / Custom(fn)）与 `MenuBarMetrics` struct（supported 集合 + tertiary_requires_window + 短标签）——语义照 CodexBar ProviderPaceCapability（ProviderDescriptor.swift:244-333）。
2. 77 家现有定义把零散布尔挪进两个 struct（编译器漏出全部 match 点，逐个收敛）。
3. 消费方（前端渲染/诊断导出）改查 descriptor 字段；`Custom(fn)` 用 fn 指针+window_minutes: Option<u32>，无需 trait object。

## 验收

- grep 确认共享代码 per-family match 数量下降（记录前后对比）；新增 provider 漏配窗口 → 编译错误而非 UI 静默错乱（构造一个故意漏配的测试 family 验证）。
- `cargo check` 绿；UI 行为无回归。
