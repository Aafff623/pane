# 01: 通用 web-session provider 机制（prefactor）

**What to build:** 把当前 `tokenrhythm` 专用的网页登录、会话状态、设置行、定制入口、认证中心添加账号和登录/清除命令，收敛成 catalog 驱动的通用机制。完成后，任何 provider family 只要声明 web-session 能力，就能复用登录窗口、会话状态、重新登录/清除会话和设置行交互；`tokenrhythm` 的用户可见行为保持不变。

**Blocked by:** None (can start immediately)

**Status:** done (owner UI acceptance pending)

## 要点

- 在 provider catalog 的 Rust 与前端定义里增加明确的 web-session 能力标记，由 family 自己声明；前端不再维护只包含 `tokenrhythm` 的硬编码集合。
- 把设置行、Customize 跳转、认证中心“添加账号”、登录中状态、登录/重新登录/清除会话、onboarding 文案等分支改成能力驱动；`tokenrhythm` 的现有文案和行为不变，新增通用 fallback 文案。
- 把 `tokenrhythm` 专用的登录/状态/清除命令收敛为按 family 分发的通用命令面；各 provider 只实现自己的登录窗口、会话读取和验证逻辑。
- 远端 WebView 不能直接发 app command（`tokenrhythm` 已踩过），登录完成检测继续由 Rust 侧轮询网站接口，不依赖远端 IPC。
- 不新增第二个真实 provider；用合成的 web-session family/catalog 测试证明通用性，避免把“通用”做成 `tokenrhythm` 的另一个名字。
- 不读浏览器 profile 的 SQLite/Cookies 文件；不改变现有 vault/明文回退约定。

## 验收

- [ ] `tokenrhythm` 登录、重新登录、清除会话、设置行、Customize 跳转、认证中心添加账号全部无回归（代码/测试已覆盖，UI 待 owner）。
- [x] 新 family 只声明 web-session 能力即可出现在对应 UI，不再需要在多个前端分支加 family 特判。
- [x] 通用命令按 family 分发；远端 webview 不依赖 IPC。
- [x] 公开状态接口不泄露 cookie/token；vault 失败回退行为与现有约定一致。
- [ ] parse-tests、tsc、vite、i18n、node 测试全绿；owner 可复验 `tokenrhythm`（定向测试与 build 已通过，全量门禁待跑）。

## Comments

- 2026-10-23：01 已实现 catalog 的 `webSession` 能力、通用 `web_session_login*` 命令面和前端能力判断；`tokenrhythm` 的会话读写、账号折叠、归档/恢复和文案保持原路径。`pnpm build`、`cargo check`、`tokenrhythm` 8 个定向测试、web-session catalog 测试、前端 77 个 node 测试通过；全量 parse-tests/UI 验收待完成。
