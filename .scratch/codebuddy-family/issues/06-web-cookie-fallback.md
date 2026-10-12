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
