# 03 — 认证可靠性：家族级扫码状态、刷新单飞、错误分类

**Blocked by:** 01
**Status:** done（代码层，单测覆盖）
**Issue:** #10（实施）

**What to build**

1. `PENDING` 改为 `HashMap<family_id, Pending>`；同家族覆盖、跨家族互不影响。
2. 按 `family:account_id` 的刷新单飞锁：并发刷新只发一次请求，`write_back` 在锁内完成。
3. `billing_post` 保留 HTTP status 与业务 code，映射可读错误（401/403、WAF 10085、429、业务错误、网络）。
4. refresh 失败 → 在账号 `extra` 写 `needsRelogin` / `lastErrorKind`（布尔与枚举，不含 token）；成功认证后清除。

**不做**：不新增后台保活调用；不改刷新频率。

**验收**
- [ ] 两个家族连续 start，两个 login_id 都能 poll
- [ ] 单飞锁的单元测试（用计数器模拟刷新函数）
- [ ] 错误映射纯函数测试覆盖 401/403/10085/429/业务/网络
- [ ] `needsRelogin` 写入/清除测试
- [ ] 真实 401 由 owner 验收

## Comments

- 2026-10-19 出自 spec D3/D4/D5；上游 `refresh.rs:13-56,148-179`、`credits.rs:343-399`。
- 2026-10-19 已实现：`PENDING` 改为按 family 的 map；`refresh_lock`
  按 `family:account_id` 单飞并在锁内复查 store；`Billing` 带 status/code，
  错误映射覆盖 401/403/10085/404/429/5xx/业务/网络；refresh 失败写
  `needsRelogin` + `lastErrorKind`，成功认证清除。单测覆盖分类与成功判据。
