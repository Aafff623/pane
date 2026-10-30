# Spec: Provider 实测（生态地图）

**Status:** in progress · **Started:** 2026-10-29 · **Owner:** Aafff623

## 目标

把 Pane 的 provider 从「目录有条目」推进到三层可审计状态：**已收录 → 已实测 → 有用户反馈**。结论只回写 `docs/provider-credentials.md`、`docs/provider-api-verification.md`、`docs/provider-intel.md`，不另建平行系统。

## 三轮

1. **现有生态摸底**（进行中）：39 张在卡中的报错 / stale / needs_setup，以及归档账号的恢复验证。
2. **跨赛道开卡**：46 个已收录未配置，优先覆盖非 token 计量（字符、分钟、请求数、团队池、云账单）。
3. **外部反馈**：小批测试者按同一清单回答，补齐开发者账号覆盖不到的盲区。

## 每家一家的验收（7 项）

真实认证 · 数据一致性（对照官方控制台）· 计量语义 · 刷新与重置 · 异常与恢复 · 多账号隔离（不适用记 N/A）· 真实使用价值（连续几天）。

## 取证方式

只读 `127.0.0.1:6736/v1/providers`（status / error / stale / lines，不回显密钥）+ 官方控制台数字 + 本机凭据元数据（存在性、mtime、exp，不读值）。原始快照存 `temp/provider-bench/<date>/`（本地，不提交）。

## 已知取证缺口（2026-10-30 已修）

`/v1/providers` 的 `credentialState` 曾少报：`source_probe` 只接 21 家提示，gear 面板接 63 家。7 张在卡与状态矛盾；ticket 02 已把提示表收进 `source_probe::local_credential_hint`，HTTP API、首启扫描和 gear 共用，zcode 补 presence-only 判断。`/v1/providers` 39 张卡中已无该矛盾，剩余 `absent` 的 grok/qwen/qoder 都是真实 `no_credentials`。

## 边界

- 第一轮只验证、不改代码；缺陷记证据，修复另开 ticket。
- 不为了凑数量付费开通；没有真实账号的标「阻塞：待账号」。

## Tickets

| # | 文件 | 覆盖 | 状态 |
|---|---|---|---|
| 01 | `issues/01-round-1-failure-samples.md` | 6 报错 + 5 stale + zcode 的取证与归因 | done（7 项待 owner 重登后复测） |
| 02 | `issues/02-credential-state-shared-table.md` | credentialState 共享凭据表 | done |
