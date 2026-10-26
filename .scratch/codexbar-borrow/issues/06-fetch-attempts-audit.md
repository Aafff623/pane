Status: ready
Type: task
Blocked by: 04

# 06 · fetch attempts 审计 + 诊断导出

## 做什么

1. 每 family 每次 fetch 记录 attempts [{strategy, kind, outcome(skipped/failed/succeeded), error_category}]；失败时一行日志：`provider=X errorCategory=Y sources="A (api): skipped: unavailable -> B (oauth): failed: rateLimited -> C (cli): succeeded"`（CodexBar ProviderFetchPlan.swift:465-492 形态）。
2. 诊断导出：schema 照 ProviderDiagnosticExport.swift:36-49 模板（schemaVersion/timestamp/provider/sourceMode/auth/usage/fetchAttempts/error）——接 Pane 现有诊断面板/doctor 出口。

## 验收

- 单测：多策略链的 attempts 序列与一行日志格式。
- 诊断导出 JSON 可被 owner 人工核对（脱敏后）。
