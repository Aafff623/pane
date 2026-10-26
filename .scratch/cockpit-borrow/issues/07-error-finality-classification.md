Status: done
Type: task
Blocked by: —

# 07 · 刷新错误终态性分类（撤销类才判死）

## 做什么

1. 在 provider 刷新错误路径加「终态 vs 瞬态」纯函数分类：撤销类错误码（token_revoked / token_invalidated / invalid_grant 且排除 refresh 词）→ 才进 fail_state 终态（卡片标「需重新登录」）；泛化 401/403、代理错误、超时 → 只标「需关注」可恢复（沿用退避）。
2. auth_mode 为 api_key/本地文件的 family 无撤销概念，恒瞬态。
3. 与 alerts.rs 状态机衔接：终态触发「需重新登录」类告警，瞬态不触发重置类告警。

## 验收

- parse-tests：错误分类纯函数断言（撤销/泛化 401/代理/超时四分支）。
- 本机：把某账号 token 改错 → 卡片进「需重登」；临时 503 → 不进。

## 证据

temp/research/20261025-cockpit-multi-account.md §一状态机（cockpit codex_account_token_refresh.rs:151-172 终态性判定语义，原创重写）。
