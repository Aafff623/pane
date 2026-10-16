# 02: 前端镜像 + 图标 + 凭据提示

**What to build:** 前端认识 giphy 家族：目录镜像一致、MCP 分组里卡片有图标、密钥页给出正确的粘贴提示（面板 access_token，不是 api_key）。

**Blocked by:** 01

**Status:** resolved（2026-10-16，owner 已亲自验收并授权发布）

- [ ] `src/providerCatalog.ts` 加镜像条目（category: "mcp"）
- [ ] `src/assets/providers/giphy.svg` 自绘中性图标 + `providerVisuals.ts` 注册
- [ ] `PROVIDER_CRED_INFO` 加 giphy 提示文案（三语键）
- [ ] catalog_consistency 测试过（Rust ↔ TS 双向 + 图标注册）
- [ ] tsc + vite 过；i18n 门禁过


## 2026-10-16 Review

- Backend app matching now uses identity, ignores current/future/invalid dates, derives ceilings only for explicit beta apps, and binds cached successes to the current credential.
- Frontend keeps derived historical ceilings out of live exhaustion, handles collapsed metrics without empty card bodies, and provides credential repair/help actions.
- Verification: frontend build and 30 frontend tests passed; GNU Rust build passed; serial parse suite 819 passed / 0 failed / 13 ignored. Earlier parallel suite transport failure is recorded in credential-liveness rather than concealed.
- Status remains implemented; owner UI acceptance is pending.


## Owner acceptance — 2026-10-16

Owner confirmed personal UI acceptance and authorized v0.8.5 publication. Release CI outcome is recorded separately from feature acceptance.
