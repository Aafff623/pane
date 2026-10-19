# 04 — 账号来源标注与「导入当前客户端登录」

**Blocked by:** 02
**Status:** done（代码层 + 授权中心按钮；实机导入待 owner）
**Issue:** #9（实施，收缩）

**What to build**

- `LoginAccount.extra.sourceKind`：扫码写入 `qr`，桌面导入写入 `desktop`。
- 新命令 `workbuddy_import_desktop(family)`：只读桌面明文登录态 → `record_login`（按 uid upsert）→ Vault-first；返回 label 与是否已存在。**绝不回写官方文件**；`$wbEncrypted` 返回可读错误。
- `auth_center.rs` 行增加 `source`、`enterpriseName`（来自 extra；无则 null）。
- 桌面卡在 `Source` 行注明 `desktop app (read-only)`。

**不做**：不做批量选择、跨设备导出、改名同步。

**验收**
- [ ] 导入同一 uid 两次不产生第二张卡
- [ ] 加密信封导入返回错误且不落库
- [ ] 授权中心行含 source 字段
- [ ] 删除账号只删本家族该卡，官方会话不变

## Comments

- 2026-10-19 出自 spec D6；上游 `auth_file.rs:254-353`、`account.rs:510-516`。
- 2026-10-19 已实现：扫码写 `sourceKind=qr`；新命令
  `workbuddy_import_desktop`（只读、按 uid upsert、Vault-first、不写官方文件）；
  授权中心行新增 `source` / `enterpriseName` 字段与「导入桌面版登录」按钮
  （三语）。未做：批量选择、跨设备导出。
