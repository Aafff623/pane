# 06 — 网页会话 cookie 兜底（显式启用）

**What to build:** 当自管登录不可用 / 用户未登录时，用 `www.workbuddy.cn` 的**网页会话**兜底读 credits。这必须是**用户的显式动作**：在设置里选择启用并导入（Chrome 导入或手贴 Cookie），**不静默扫描浏览器**。

硬约束：该站点把会话**绑定浏览器 User-Agent**——同一份 cookie 换任何其他 UA（含旧版 Chrome 大版本）直接 401。需读本机 Chrome 的大版本号拼 UA，被拒时用**上一个大版本**重试一次，仍失败则报「会话过期」。金额口径与 02 一致；网页摘要接口只给资源摘要，重置时间需另调列表接口（须带 package codes，否则被拒）。

**Blocked by:** 04

**Status:** ready-for-agent

- [ ] 设置里可显式启用并导入会话（Chrome 导入 / 手贴 Cookie 两种方式）
- [ ] 启用后卡片显示 credits；**未启用时该通道完全不发请求**
- [ ] UA 绑定：用本机 Chrome 大版本号；被拒时用上一个大版本重试一次
- [ ] 重置时间缺失（列表接口失败）时**保留余额**，不清空
- [ ] 所有 OS 相关解密经 platform seam（ADR 0002），无内联平台分支
- [ ] 解析测试覆盖摘要字段与「重置时间缺失」的降级；`parse-tests` 全绿

## Comments

- 2026-10-11 出自 `.scratch/codebuddy-family/spec.md`（决策 D4）。
- 网页接口在**根路径**、不在 `/v2` 下：`POST /billing/meter/get-user-resource-summary`（body 为空）。
- 复用 `doubao` 已有的 cookie 提取 seam（Chromium Cookies + `os_crypt` + `platform::dpapi_unprotect`）。

- **2026-10-12 用户拍板：降级路由方案 + UI 形态**（优先级：① 读本地文件 → ② 降级路由给用户选）
  - **① 首选**（已实现，见 `codebuddy.rs` + 新模块 `workbuddy_local.rs`）：完全照 magpie 的路线读桌面 App 的 auth 文件——明文 token 直接用；**到期前 60 秒内才主动刷新**（`now >= expiresAt - 60_000`，magpie 的窗口）；刷新结果**只留在 Pane 内存**，绝不写 WorkBuddy 的文件、也不落进 Pane 的登录库；刷新令牌自身过期则视为"已登出、需重新登录"；刷新失败仍拿现有 token 试一次。凭据加密（`$wbEncrypted`）时不猜、直接降级。
  - **② 降级路由 UI**（用户原话整理，待实现）：
    - 打开悬浮面板时，**额度总览那块**若读不到该 provider 的数据 → 显示一条 **error** 信息。
    - 点这条 error → **跳转到该 provider 的详细卡片**，卡片里给三个选项：
      1. **扫码登录**（已有的 auth center 流程）
      2. **网页端授权 cookie**（本票的主体：Chrome 导入 / 手贴 Cookie）
      3. **详细信息**：说明**读不到本地的哪个文件、哪个凭据**（例如 `…/auth/workbuddy-desktop.info` 的 token 是加密的）
  - 现状：① 已实现（parse-tests 699 通过）；② 未实现。
  - 待确认：本机 `%LOCALAPPDATA%\CodeBuddyExtension\Data\Public\auth\workbuddy-desktop.info` 的 token 是加密信封 → **① 在本机读不到 token**，需要用户确认 WorkBuddy 是否有"凭据保护"开关可关（在 `~/.workbuddy/settings.json` 里没找到）。
