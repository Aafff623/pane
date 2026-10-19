# 05 — Qoder CN 多账号（device flow）

**Blocked by:** 真实 CN 账号验收
**Status:** blocked

参照 Magpie `internal/qoder/`：
- 站点 `qoder.cn` / `openapi.qoder.com.cn`
- PKCE device flow：`/device/selectAccounts` → `/api/v1/deviceToken/poll` → `/api/v1/me/jobToken`
- 账号身份 `uid`，多账号存 Pane `login-accounts/qodercn.json`
- 刷新由 Pane 负责；保留现有 `auth.v1.dat` 只读作为第一档

**验收**
- [ ] 真实账号扫码成功并出现独立卡
- [ ] 同 uid 重登录不重复
- [ ] 401 后刷新或给出可读重登提示
- [ ] 不改写官方客户端文件
