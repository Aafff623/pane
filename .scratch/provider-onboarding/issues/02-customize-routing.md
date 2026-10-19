# 02 — Customize 面板按描述表渲染

**Blocked by:** 01
**Status:** done

`renderCustConfig` 首个分支改为 `onboardingFor(fam)` 驱动：动作按钮只来自描述表；登录型一律走 `panelIntent` 打开授权中心并定位 family，不再出现 `openAccountDialog` 的 paste-key 输入框。

**验收**
- [x] 12 个误路由 family 的「添加账号」不再出现 API key 输入框
- [x] key 型 provider 仍可正常粘贴/测试/保存
- [x] antigravity/cursor/onenewapi/workbuddy 的专用动作保留
- [x] `tsc` 通过

## Comments

- 2026-10-19 已实现：`renderCustConfig` 对登录/本地读取型 family 改走
  `renderOnboardingConfig`；`pasteKey` 只留给真正的 key 型。真实 WebView 验证
  见工单 06。
