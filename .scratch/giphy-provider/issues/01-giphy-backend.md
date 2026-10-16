# 01: GIPHY provider（后端模块 + 目录注册 + 指标）

**What to build:** 看板的 MCP 分组里出现 GIPHY 卡片，显示账号下每个应用的近 7 天调用次数（数据来自 GIPHY 开发者面板的隐藏统计接口，凭据是面板 access_token）。

**Blocked by:** None (can start immediately)

**Status:** resolved（2026-10-16，owner 已亲自验收并授权发布）

- [ ] `providers/giphy.rs`：`giphy_snapshot()` + 纯解析函数（apps × 日序列 → Metric 行）+ 30 分钟 TTL 缓存
- [ ] `provider_catalog.rs` 加 giphy 定义（NativeBalance / supports_api_key / category Mcp）+ `providers/mod.rs` 注册
- [ ] `lib.rs` 两处分派 + `DEFAULT_DISABLED_PROVIDERS` 追加
- [ ] 纯解析函数单测进 parse-tests（空 stats / 多应用 / 无应用三种）
- [ ] cargo check 过；parse-tests 全绿


## 2026-10-16 Review

- Backend app matching now uses identity, ignores current/future/invalid dates, derives ceilings only for explicit beta apps, and binds cached successes to the current credential.
- Frontend keeps derived historical ceilings out of live exhaustion, handles collapsed metrics without empty card bodies, and provides credential repair/help actions.
- Verification: frontend build and 30 frontend tests passed; GNU Rust build passed; serial parse suite 819 passed / 0 failed / 13 ignored. Earlier parallel suite transport failure is recorded in credential-liveness rather than concealed.
- Status remains implemented; owner UI acceptance is pending.


## Owner acceptance — 2026-10-16

Owner confirmed personal UI acceptance and authorized v0.8.5 publication. Release CI outcome is recorded separately from feature acceptance.
