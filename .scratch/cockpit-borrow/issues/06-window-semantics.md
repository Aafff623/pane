Status: done
Type: task
Blocked by: —

# 06 · provider 窗口语义补强（按情报表逐家核对）

## 做什么

对照 temp/research/20261025-cockpit-deep-dive.md §一情报表，逐家核对 Pane 解析完整性并补缺：
1. codex：5h+周双窗字段显式化（used_percent/window_minutes/reset_time；wham/usage 端点）。
2. github-copilot：premium_models⇄premium_interactions 档位映射 + entitlement≥1500 高档判定。
3. cursor：hourly+weekly 双 reset + billingCycleEnd→allowanceResetAt。
4. antigravity：Gemini 4 Argon 周窗（v1.3.74 新；与 openusage 侧 Argon 定价情报互证）。
5. grok：unified billing 单池语义（X 订阅共享+实际 reset 期；已购 credits 不算独立额度）。
6. 教训记录：kiro 403→BANNED 状态机批量跳过（避免逐家超时）。

## 验收

- 每家：情报核对结论（已支持/补齐/不适用）+ 补齐项有 fixture 测试。
- Pane 无 family 缺口（本轮确认），本票只做语义级补强。

## 注意

中风险途径（仿客户端头）在对等实现前需 owner 拍板；高风险（解密他人存储）保持拒绝。
