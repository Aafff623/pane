# 03 — 设置页登录分发与凭据状态

**Blocked by:** 01
**Status:** done

1. `settingsKeyRow` 的登录按钮：`OAUTH_PROVIDERS` 走 `startOauthLogin`；其余登录型 family 打开授权中心并定位，不再调 `oauth_start`。
2. `get_credential_status`：登录 family 读 `login-accounts/<family>.json`。
3. `stepfun-plan` 专用面板移到多账号分支之前。

**验收**
- [x] 9 个浏览器/扫码 family 不再报 “OAuth sign-in is not available”
- [x] 已登录的 CodeBuddy/WorkBuddy 显示已保存
- [x] stepfun-plan 能看到账号密码/Oasis token 面板
- [x] `cargo check` + `tsc` 通过

## Comments

- 2026-10-19 已实现：设置页统一走 `openFamilyLogin`（授权中心）；
  `get_credential_status` 登录 family 读 `login-accounts/<family>.json`；
  stepfun-plan 分支移到多账号分支前。
