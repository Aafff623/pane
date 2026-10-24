# 04: 把 web-session 的六处命令臂收成一张注册表

**What to build:** `lib.rs` 里每加一个 web-session family 都要在六处 match 里各加一行：
`migrate_web_session`、`web_session_login_status`、`web_session_login`、`web_session_login_clear`、
单卡 `snapshot_with_login`、多账号取名 + future。第三家（`xfyun-speech`）落地时已经确认这是
**同一判断写六遍**，不是能力驱动。

**Blocked by:** 无（`web-session × NativeSnapshot` 已在 `xfyun-speech` 上验证过，形态稳定）

**Status:** open（本需求里明确不做，单独成票）

## 要点

- 目标形态：一张 `WebSessionAdapter { migrate, status, login, clear, snapshot_with_login, card_name }`
  表，六个命令面只查表，缺家时统一报 `"{family} is not a web-session provider"`。
  与 `provider_dispatch.rs` 的 `native_snapshots!` 用同一种写法。
- `WEB_SESSION_FAMILIES` 常量保留还是改成 `ProviderDefinition.web_session` 字段，本票一并定：
  实测它已经被 `is_web_session()` 单一入口消费，**不构成重复判断**；真正的重复是上面六处命令臂。
  若改字段，需要给全部 family 定义补字段（~90 处结构体字面量），权衡后再定。
- 异步签名不同家可能不一致（`login(&app)`、`snapshot_with_login(login)`），表里要用
  `Box<dyn Future>` 或宏展开，注意 `Send` 约束与 `tauri::AppHandle` 的生命周期。
- 不改行为：任何 family 的登录/清除/取数结果必须与现在逐字一致，用现有 auth-center 与
  catalog 回归兜底。

## 验收

- [ ] 新增第四家 web-session family 只需改**一处**注册表 + 它自己的模块
- [ ] `parse-tests` 全量绿；auth_center 的 family 覆盖断言不放松
- [ ] 删除六处 match 后没有 `family == "..."` 的残留特判

## Comments

- 2026-10-23：`xfyun-speech`（#47）实现时确认成本：六处全硬编码、默认臂直接报错、没有注册表。
  当时为避免在一个未验收的新 family 上同时做重构，按「先落地、再收敛」拆出本票。
