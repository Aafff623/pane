# 05 — 企业成员兜底与不限量

**What to build:** 企业账号在**个人资源接口**上会返回**空资源列表**，导致用量被显示成 0。本票加兜底：企业成员改走**企业用量接口**（企业 ID 走请求头、body 为空），已用 = `credit`、剩余 = `limitNum − credit`；`limitNum == -1` 读作**不限量**（显示「不限量」，而不是一个假数字）。三个租户共用同一条兜底路径。

**Blocked by:** 04

**Status:** done（落地 main cb7c676；企业账号实测待用户）

- [ ] 企业账号：个人资源接口返回空 → 自动走企业用量接口，显示真实总量 / 已用 / 剩余
- [ ] `limitNum == -1` → 显示「不限量」
- [ ] 兜底失败时**保留上一次读数**，不把卡片打回 0
- [ ] 纯解析测试覆盖：空资源列表触发兜底、`limitNum − credit`、`-1` 不限量；`parse-tests` 全绿

## Comments

- 2026-10-11 出自 `.scratch/codebuddy-family/spec.md`（决策 D5）。上游修复记录：cockpit-tools PR #1911（企业账号用量查询为空）。
- 免费档用户不受影响，可排在 04 之后做。
- 2026-10-12 **已实现**：个人资源列表为空**且**账号带企业 ID 时，改调 `POST /v2/billing/meter/get-enterprise-user-usage`（body `{}`、企业 ID 走既有 tenant 头），解析 `{limitNum, credit, cycleEndTime/cycleResetTime}` → 已用 = `credit`、剩余 = `limitNum − credit`、`limitNum == -1` → `Unlimited`。
  - 比上游更直接：不做 cockpit 的「包装成资源形状」hack，直接产出同一个 `CreditsFace`，三个租户共用一条路径。
  - 周期时间戳三种形态都吃（`yyyy-MM-dd HH:mm:ss` 按 UTC+8、epoch 秒、epoch 毫秒）。
  - 「兜底失败保留上一次读数」由 lib.rs 既有的 stale-snapshot 机制承担（本函数失败即不产出 face，不会把卡片打成 0）。
  - 新增 3 条纯解析测试（2000/378 → 1622、`-1` 不限量、epoch 毫秒 + 下划线键 + 无 limit 返回 None）。
