# 06 — 去掉桌面卡与扫码账号卡的同 ID 重复

**Blocked by:** 04
**Status:** done（代码层；实机双账号待 owner）
**Issue:** #9（实施）

**What to build:** `providers::codebuddy::snapshot` 的桌面通道在产出卡之前检查账号库：若同 family 已存在同 `account_id` 的 Pane 登录，跳过桌面卡（由登录卡的 future 负责该 ID）。避免同一次刷新出现两条同 ID 快照。

**验收**
- [ ] 纯函数 `desktop_shadowed_by_login(family, uid)` 测试：命中 / 未命中 / 不同 family
- [ ] 真实桌面登录 + 扫码登录同账号时只出现一张账号卡（owner 验收）

## Comments

- 2026-10-19 出自 spec 去重规则；审计发现 `lib.rs` 未按 ID 去重。
- 2026-10-19 已实现：`desktop_shadowed_by_login` 在桌面通道产出卡之前检查
  同 family 的账号库，命中则跳过桌面 future。
