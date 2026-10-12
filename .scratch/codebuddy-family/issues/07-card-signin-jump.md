
## Comments

- **2026-10-12 已实现并端到端验证**（用户点题："主界面这块没有显示跳转授权的按钮，应该点一下就直接跑到设置里的授权中心 WorkBuddy 相关部分"）。
  - **机制**：popover 与面板是两个窗口、两套 JS 上下文，所以走 **config 当消息**——卡片按钮写 `config.panelIntent = { view: "auth", family, at }`（`patchConfig` 会广播 `config-updated`），再 `invoke("open_panel_window")`；面板在**启动后**与**收到 `config-updated`** 两处 `applyPanelIntent()`：按 `at` 去重、`setPanelView("auth")`、把 `[data-auth-group="<family>"]` 滚进视野并短暂高亮。**未新增 IPC 命令**。
  - **Rust 侧必须放行这个键**：`lib.rs` 的 `apply_config_patch` 只接受 `CONFIG_KEYS` 白名单，未知键会被丢弃（还会打 `ignoring unknown key` 日志）——`panelIntent` 已加进白名单，否则意图写不进 config。
  - **按钮显示范围**：卡片 `status === "no_credentials"` 且目录里该家族 `supportsOAuth === true` 时显示（登录型家族）。只填 API Key 的家族不显示，它们该去密钥页。
  - **踩到的坑**：`scrollIntoView` 用 `behavior: "smooth"` 在 WebView2 上是**静默空操作**（仓库里已有注释记录），必须用本仓的 `scrollBehavior()` 探测结果——否则视图切了、区块高亮了，但没滚过去。
  - **验证**（`temp/scripts/verify-auth-jump.mjs`，CDP 驱动真实两窗口）：点 `[data-auth-jump="workbuddy"]` → 面板窗出现且 `panel-form` → `#panel-view-auth` 可见、标题"授权中心" → `[data-auth-group="workbuddy"]` 存在且 `inView: true`（rectTop 309 / viewport 567，滚动容器 `panel-scroll` 到底）。**PASS**。已开面板的那条路径（`config-updated`）同样通过。
  - 新增：`card.signIn` 三语（en "Sign in" / zh "去登录" / ru "Войти"）、`.auth-jump` 与 `.auth-jump-target` 两条 CSS。
