# 07 — 卡片上加"去授权中心扫码登录"的跳转按钮

**What to build:** provider 详细卡片在**读不到数据**（`no_credentials` / 降级提示）时，给一个按钮，直接跳到**面板窗的授权中心页**，省掉"自己开设置、自己找页签"这一步。

**Blocked by:** 无（但建议与票 06 的降级路由 UI 一起做，见下）

**Status:** needs-triage

**来源:** 用户 2026-10-12 原话："在咱们主页看板的 provider 详细信息里面，应该加一个类似'跳转到授权中心扫码'的按钮，直接跳转到设置界面的扫码页，这样体验更顺畅一些"。

## 现状锚点（已读码核实）

- 面板窗有两个视图：`#panel-view-settings`（7 个页签）与 `#panel-view-auth`（授权中心，左栏独立一项）。**授权中心不在 `PANEL_PAGES` 里**（`main.ts:13132-13140` 只有 general/usage/shortcuts/notifications/privacy/network/about）。
- 切换视图的函数现成：`setPanelView("settings" | "auth")`（`main.ts:13464`），内部会 `loadAuthCenter()`。
- 授权中心的行：`<div class="auth-row" data-auth-row="<account id>">`，家族在 `data-auth-menu="<family>|<index>"` 上（`main.ts:13574-13584`）；每家族已有"添加账号"按钮 `data-auth-add="<family>"`（`main.ts:13636`）——即**扫码流程本身已存在**，本票只是把入口挪到卡片上。
- **关键约束：popover 与面板是两个窗口、两套 JS 上下文**，卡片点按钮无法直接操作面板 DOM，必须走 IPC。
- 现有事件：`update-available` / `update-progress` / `tray-strip-restore` / `config-updated` / `usage-updated` / `popover-shown`（`main.ts:14845-14896`），**没有**"面板意图"通道。
- 卡片侧：`no_credentials` 的卡片目前在首启/升级时被**自动禁用**（`main.ts:8690-8720`），并没有渲染"去登录"的入口——所以本票同时要解决"卡片在什么状态下显示这个按钮"。

## 设计（推荐方案：配置意图 + 既有事件，不新增 IPC 命令）

1. 卡片按钮（仅 `no_credentials` 或降级提示状态显示）点击时：
   - `patchConfig({ panelIntent: { view: "auth", family: <family>, at: Date.now() } })`（`patchConfig` 已会 emit `config-updated`）；
   - 再 `invoke("open_panel_window")`（已有命令，`main.ts:14230` 等处就是这么开的）。
2. 面板侧在**初始化**与 **`config-updated`** 两处读 `config.panelIntent`：
   - 比上次已应用的 `at` 新才执行（避免重复应用）；
   - `setPanelView("auth")`，把该 family 的分组滚进视野并高亮一下；
   - 可选：直接对该 family 触发一次 `data-auth-add` 等价动作（即**直接进扫码**），而不是只落到页面上——用户要的是"直接跳转到扫码页"，建议做到这一步。
3. i18n：按钮文案三语（en / zh / ru，见 `src/i18n.ts` 的三段结构）。

**为什么不用新 Rust 命令**：`patchConfig` + `open_panel_window` 都已存在且语义正确；新增命令要动 `lib.rs` 的 invoke_handler 与权限面，收益不匹配。若日后意图种类变多（比如跳设置的具体页签），再抽一个 `open_panel_at` 命令更划算。

## 待确认

- **按钮显示范围**：只给"登录型家族"（codebuddy / codebuddycn / workbuddy / copilot / grok / kiro / qoder / trae / windsurf / zed）显示，还是所有 `no_credentials` 卡片都显示（API-key 型卡片该引导去密钥页而不是授权中心）？**建议按家族区分**，密钥型卡片引导到"密钥与额度"页。
- 是否直接进扫码（第 2 步的可选项）——**建议做**，否则只算"跳页面"。

## 建议合并的邻居

票 06 的**降级路由 UI**（总览 error → 详情卡片给 扫码 / cookie / 详情 三选项）与本票是同一块界面：都是"读不到数据时把用户导到正确的补救入口"。**建议一起实现**，一次把卡片的"读不到数据"状态做完，避免两次改同一处渲染。
