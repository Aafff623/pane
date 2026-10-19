# 02 — 档位隔离：domain 校验 + 桌面文件分档

**Blocked by:** 01
**Status:** done（代码层，单测覆盖）
**Issue:** #8（实施）

**What to build:** 让 CN / AI 两条档位在凭据层面严格隔离。

- 扫码落库前校验 token 的 `domain`：AI 必须以 `.workbuddy.ai` 结尾；CN 不得是 `.workbuddy.ai`。不匹配 → `Err` 可读文案，**不保存**。
- `workbuddy_local` 按档位读文件：CN `workbuddy-desktop.info`、AI `workbuddy-desktop-ai.info`；空域名视为相符（上游口径）。
- 网页 Cookie 兜底只对 CN 生效，AI 不跨域 fallback。
- 桌面读取兼容根级 `uid`/`email`、`account.uid`/`account.id`，用于导入身份。

**验收**
- [ ] `matches_variant_domain` 纯函数测试：AI 拒 CN 域、CN 拒 AI 域、相似后缀不误判
- [ ] `auth_path_for(family)` 测试：两档位同目录不同文件
- [ ] 解析测试：根级 uid/email 可读；`$wbEncrypted` 仍拒绝
- [ ] 网页通道对 `workbuddyai` 直接返回 None

## Comments

- 2026-10-19 出自 spec D2；上游 `variant.rs:84-154,216-235`、`auth_file.rs:288-353`。
- 2026-10-19 已实现：`Family::matches_domain`（只认后缀）、扫码落库前校验、
  `workbuddy_local::read_for(family)` + `auth_path_for`、根级 uid/email/
  enterpriseId 读取、网页通道仅 CN。单测覆盖域名/路径/身份解析。
