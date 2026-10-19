# 06 — 验证与手工清单

**Blocked by:** 01–04
**Status:** done（自动化 + 真实 UI 冒烟；owner 完整验收待办）

## 结果（2026-10-19）

- `tsc --noEmit`：0
- `vite build`：成功
- `node --test tests/*.test.mjs`：51/51
- `cargo check`：通过
- 真实 WebView（CDP 9333，debug 构建）：
  - `codebuddy`：面板为「扫码提示 + 3 步指引 + 添加账号/登录」，`input[type=password]` 数量 **0**
  - `workbuddy`：登录按钮 + 桌面版导入按钮，无 key 输入框
  - `stepfun-plan`：账号/密码/Oasis token 面板可达（此前被多账号分支遮蔽）
  - `bocha`：按钮跳到设置页密钥卡
  - `opencode`：key 型路径仍正常渲染
  - `qodercn`：被用户从 Customize 移除（`config.removedProviders`），因此不可打开；这不是路由问题
- 观察：点击某个 provider 的 ⚙ 会同步重绘 64 行抽屉，WebView 约 7 秒无响应（改动前后同路径）；未在本轮优化。

## 待 owner 手工

- 每个登录型 provider 点「添加账号/登录」应跳到授权中心对应分组。
- 真实扫码/OAuth 由 owner 用各账号验收。

