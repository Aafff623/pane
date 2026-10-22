# Panel UI 与账号管理：本地实现及验收记录

Status: awaiting-owner-acceptance

## 范围与核实

- 以 `HANDOFF.md` 为线索，复查当前 `git status/log`、调用链及源文件。保留接手时的 `src/main.ts`、`src/styles.css` 改动，工作分支为 `codex/panel-ui-accounts`。
- 独立确认设置页垂直居中来自 flex 伪元素，代理客户端缓存固定首次代理，三处删除入口行为不一致。
- 本地 Magpie 参考仓的 `loginAgents` 确实仅有 Claude/Codex，Codex 路径导入和刷新 CLI 凭据；没有其他 provider 的通用 OAuth 实现可直接移植。

## 已实现

1. 基元律动支持设置中的「添加账号」、多账号列表、重登、重命名、归档、恢复和永久删除；授权中心也展示并管理这些账号。每次登录使用 incognito WebView，验证服务端用户 ID 后保存，重复登录更新原身份并保留备注。第一个账号为默认卡，其余账号加入同 provider 的账号切换。
2. 旧基元律动单会话凭据经过服务端验证后先保存再迁移；过期会话保留账号及重登提示。其他登录族的旧 OAuth 槽在账号事务锁内完成持久化迁移，保留轮换 token 的写回行为；刷新身份检查只读取 overlay，不在检查时写回旧列表。
3. 设置、授权中心、卡片菜单的删除统一按 stable card id 调用 `account_delete`，清理匹配凭据及归档项，可选择保留历史。CLI 自主管理的默认 Codex 账号不能在 Pane 删除。
4. 归档与置顶支持 stable card id，保留 index 作为旧调用兼容入口。设置新增归档列表。外部归档进入 vault，保留项目原有的 vault 不可用时文件回退；写失败会回滚，且不删除既有归档。Cursor 回收旧 vault slot 改为在文件写成功后执行。
5. 登录卡、Cursor/Antigravity 卡和基元律动默认位参与刷新身份检查；账号命令与最终快照/历史写入共用同步事务锁。登录完成后的基元律动保存也参与该锁，网络等待期间不持锁。
6. 设置页统一顶部留白；更新日志移至「设置 → 关于」；账号计数使用中性色胶囊和状态圆点。网站 demo 已重建，受影响的 README 截图已重新生成。
7. 普通及禁止重定向的 HTTP 客户端跟随有效代理 URL 重建连接池，直连 fallback 保持 `no_proxy()`。

## 验证证据

| 检查 | 本地结果 |
| --- | --- |
| `corepack pnpm build` | 通过；Vite 仍有现有大 chunk 提示 |
| `node --test tests/*.test.mjs` | 71 passed |
| `cargo +stable-x86_64-pc-windows-gnu test --manifest-path parse-tests/Cargo.toml` | 934 passed / 14 ignored |
| GNU debug `cargo check` / `cargo build` | 通过；存在现有编译警告 |
| `node --test site/*.test.mjs` | 36 passed，含源码哈希门禁 |
| README 检查脚本 | ALL CHECKS PASSED |
| 两轴 code-review 与冗余扫描 | 已执行，发现的身份过滤、授权中心行、恢复历史及锁问题已修复 |

新增回归覆盖多账号稳定身份、会话轮换与备注、取消删除/取消登录、历史保留选项、归档/恢复写失败、已有归档保护、最后一个账号删除、旧 OAuth 槽不复活、Cursor 写失败保留凭据、旧查询结果过滤、两个 redirect policy 下真实 loopback mock proxy 切换，以及身份预览不迁移凭据、不重写文件。

完整 Tauri IPC 测试和真实登录回调未在本机运行；parse-tests 测试生产持久化核心，不等于完整桌面端到端测试。演示截图使用模拟数据，不含真实账号。Ego 截图因宿主窗口最小化失败，最终截图由仓库现有独立 headless 脚本生成。

演示页 7 个设置 tab 在亮暗两主题的内容起点均为 CSS y=100；更新日志仅在 About 可见，基元律动的添加入口存在。另有滚动至该行的截图。

原始日志保留在本机 `temp/logs/panel-*.log`；截图在 `temp/panel-readme-final/` 和 `temp/panel-ui-demo-checks/`。

## Owner 验收

- 在设置 → 密钥与额度 → 基元律动添加账号 A，再添加 B；两者都应保留，并能分别显示余额与切换账号。
- 重登 A，确认仍是同一账号且备注不变；关闭未完成的登录窗口，原账号列表应保持不变。
- 归档一个账号，在设置的归档列表恢复；再分别测试保留/不保留历史的永久删除，刷新后不应复活。最后一个账号删掉后应显示待配置。
- 检查设置 7 个 tab 的顶部留白、亮暗主题胶囊、「关于」更新日志及「查看全部」。
- Codex 浏览器登录及 1455 端口占用时的设备码 fallback；切换系统代理端口后无需重启再刷新。

调试服务使用仓库 debug exe 和 Vite；启动器已完成 API 探活，但报告窗口 hidden，桌面可见性/托盘/快捷键仍需 owner 本人确认。

用户已在对话中明确授权本地 commit；实现已提交为 `f549357`（`fix(accounts): complete multi-account lifecycle and Codex OAuth`）。尚未 push、tag、合并或发布。分发侧旧 winget PR、token 轮换和客户端安装不在本地 UI 实现范围内，本轮未操作。


## 2026-10-22 Codex / UI 收口

- Codex loopback 的无关请求导致监听提前退出已实际复现（red log），修复后同一回归通过；加入错误 state、重复 code、错误路径、取消/替代流程不保存的验证。
- device-code 成功直接写多账号存储，不再通过可被后台迁移消费的单槽二次取值；Codex/Grok/Copilot 的取消均在最终保存前校验 pending。前端防止重叠 poll 和旧响应覆盖新 flow。
- Codex 重新授权保留备注；刷新按账号串行，写回前比较凭据版本，删除期间返回的旧刷新不能复活账号。JSON 写失败保留原 vault 凭据；同邮箱本地来源与托管账号分别展示。本地配置的 BOM 解析一致。
- 卡片菜单捕获真实 stable account id；bare 默认位仅映射 API-key 池与基元律动，本地 Codex 等来源不会误操作首个托管账号。回归使用真实 providerCatalog。
- 基元律动授权中心重试走专用网页登录；界面提示关闭实际登录窗口取消，异步响应有 flow 身份检查。
- 右键菜单字号、换行、标题、边界定位已调整；合并相邻重复分隔线。中文字体和字体包控件复用现有输入框外观。最终亮暗主题、450×640 与320×480的76次实际菜单展开几何检查无越界；独立读图复核设置页通过。截图为演示数据，不代替 owner 验收。
- 双语 README 的多账号能力、只读本地登录、归档/恢复位置与历史选项已按实际实现校正；demo 和受影响素材已刷新。
- 最终日志：`panel-final-build.log`、`panel-final-frontend-tests.log`（71通过）、`panel-final-rust-tests.log`（934通过/14忽略）、`codex-cancel-final.log`（补强的取消保存回归通过）、`panel-final-rust-build.log`（GNU debug构建通过）、`panel-site-tests.log`（36通过）、`panel-readme-gate.log`。均位于本机 `temp/logs/`。
- 最新完整设置截图位于 `temp/panel-readme-final/settings-*.png`；菜单截图和76项报告位于 `temp/panel-menu-final/`。已完成 Standards / Spec 两轴代码复核与冗余扫描，并修复发现项。

追加 owner 验收：保留现有 Codex CLI 登录，依次 OAuth 添加两个不同订阅账号；检查两者均保留且额度对应，重登一个不影响另一个，自定义备注不变。分别取消浏览器/设备码流程，刷新后不应出现未完成授权的账号。完成基元律动两账号和 Codex 两账号验收后，再在对话明确授权目标版本的 tag / release。


## 2026-10-22 Owner 现场反馈后的逻辑修复

- 已按 owner 指令关闭安装版，运行仓库 debug exe 与 Vite。最终探活：两服务均 HTTP 200，进程为 `src-tauri/target/debug/pane.exe`。owner 明确认证、选择账号及回调验收由其本人操作；Agent 不再操作登录窗口或更改真实账号记录。
- 实机复现授权中心标题已切换但设置列表仍显示。独立 headless 检查 red 为 `settingsHidden=true/settingsDisplay=flex`；精确限定两个 panel view 的 `[hidden]` 后 green 为 `settingsDisplay=none/authDisplay=block`。
- MiniMax 默认账号备注由 bare 槽位转为 stable credential id；保留默认账号的 stable layout note，账号升为默认后不会继承已删除账号的备注。原生 CLI 仍使用 synthetic note key，与 provider 标题隔离。
- 默认账号标签中文“置顶”改为“默认”，俄文同步校正。删除默认账号后同时清除该身份及旧 bare 槽位的 pin，保留其他账号的 pin；失效 stable pin 由 layout reconciliation 清理。
- 修正 reconciliation 的 folding 边界：仅 API-key 池和基元律动折叠默认位，不误清首个托管 Codex 的快照。
- 新增回归覆盖备注随身份迁移、默认账号 note 保留、CLI 两级备注隔离、删除后 pin 清理及 sibling pin 保留。前端最终75通过，网站36通过，`corepack pnpm build`通过；该轮无Rust改动，未重复Rust全量测试。两轴复核发现项已修复。
- 日志：`temp/logs/owner-fixes-*.log`、`panel-auth-{red,green}.log`、`minimax-note-{red,green}.log`、`minimax-pin-red.log`。真实OAuth同意与双账号回调仍待owner本人验收。


## 2026-10-22 MiniMax 归档后复活的根因修复

- 现场只读核对发现同一旧身份既在archive又在active。根因是`remove_stored_key_file`只移除文件，`provider:<family>`凭据仍存活，每轮`accounts_with_imported_main_key`将它重新插到index 0。已用真实secretstore/harness复现red。
- 迁移与CRUD共用mutation锁；归档身份优先修复旧active副本；先持久化active，再退休provider vault/file，再回收旧active副本。永久删除/归档同步退休匹配legacy源。最终snapshot验收不再执行迁移，避免递归锁。
- vault退休失败时保留relay baseUrl文件，迁移写失败保留来源；均有当前源码定向回归。三条main_key回归和两条retirement回归通过（有一条交集）；完整Rust回归937通过/14忽略是在追加relay失败用例前运行，未虚报为全量938。
- 总览与卡片tabs共用真实账号过滤，默认bare与首个stable副本只计一次；单账号隐藏badge/tabs；空saved列表的有效env/CLI bare来源保留。前端77通过、site36通过、frontend/GNU debug构建通过。
- 新增API-key账号从卡片菜单进入对应Customize看板；headless结果`drawer=true/provider=true/directModal=false`。
- 重启后的实际验证：active MiniMax仅`11-4`一行，查询接口仅`providerId=minimax`一个MiniMax来源；旧归档身份不再查询。仅核对非秘密身份和数量，未操作认证窗口；认证与回调仍由owner本人完成。
- 修复前相关本地数据备份位于Git忽略的`temp/secrets/minimax-20261022-before-legacy-retirement/`。本轮只恢复归档与活动状态的一致性，保留归档记录与使用历史，不把归档当作永久删除。
- 日志：`minimax-resurrection-{red,green}.log`、`minimax-latest-{main-key-tests,retirement-tests,rust-build}.log`、`minimax-final-{rust-tests,frontend-tests,site-tests,build}.log`、`minimax-customize-check.log`，均位于本机`temp/logs/`。


## 2026-10-22 深度核实后的残余收口

- 独立复核 `85557c1`：根因修复有效；active MiniMax 仅 `11-4`，归档保留旧身份，live API 仅一个 MiniMax 来源。
- 补齐残余：`account_add`/`account_update` 拒绝已归档身份；`account_update` 与旧 `account_remove` 先退休匹配 legacy provider 源；`relaybalance`/`sub2api` 在 `baseUrl` 元数据已丢失时从 active 行恢复，并按 API Key 退休旧源；迁移失败写运行日志。
- 验证：parse-tests 940 passed / 14 ignored；frontend 77 passed；site 36 passed（在 `85557c1` 干净状态）；`pnpm build`、`cargo check` 通过。
- 当前 `site/` 存在另一会话的未提交改动，本轮未触碰、未纳入账号提交。真实 Codex / 基元律动双账号认证与 UI 验收仍由 owner 完成；release/tag 仍待 owner 明确授权。
- 2026-10-22：账号改动已合并到 `main` 并推送 `origin/main`（`b57864e`），本地与远端分支均只保留 `main`；`site/` 的并发改动仍在工作区，未提交。


## 2026-10-22 备注跨窗口同步与授权入口收口

- 真实现场：设置页改账号备注后，弹窗 Customize 仍显示旧备注。根因有两个：两个 WebView 各自缓存 `account_list`，设置窗口的改名没有通知弹窗；Customize 的“备注名”写 backend label，而卡片显示优先读 `config.layout` 的 note。
- 修复：新增 `accounts-updated` 跨窗口事件，账号增删改归档恢复都会让两个窗口丢掉并重取各自缓存；Customize 账号备注统一写 `config.layout`；设置页账号行也显示同一份显示备注，改名会清理旧 override 并广播；`config-updated` 检测 layout 变化后重绘 Customize；Codex/Grok/Copilot/Kiro/Windsurf/Trae/Zed 在 Customize 只显示“去设置添加或认证”；账号行 `?`/⚙ 图标按钮缩到 24px。
- 验证：frontend 79 passed；`pnpm build` 通过；demo 重建成功且 provenance 2 passed。官网 agent 的 `site/` 静态改动未触碰、未纳入本轮提交。真实双账号认证仍待 owner。


## 2026-10-22 备注解析、截断与全表面同步

- 审计发现三个结构性问题：provider note 与 account note 两套 key；账号缓存未就绪时写入 `@__default__` 后读不到；渲染回退到 fingerprint / masked key，长备注又没有统一截断。
- 修复：账号显示名统一为“稳定 id note → 旧 `@__default__` 别名 → backend label → email → 本地化账号 N”；`setAccountNote` 归一到稳定 id 并清理旧别名；设置改名、Customize 备注、卡片右键备注、授权中心共用同一解析器；provider note 继续只负责 Provider/卡片标题。
- 详情/概览：概览 tile、条形视图、到期提醒、spend 明细改为按实际展示账号解析备注；账号胶囊最低字号提高到 10px，账号子行、到期名称、设置 Provider 名统一 ellipsis；备注单行化并限制 48 个码点。
- 保留：`reconcileAccountLayout` 不再删除带 note 的账号条目，归档/删除/刷新不会丢备注。
- 验证：frontend 82 passed；`pnpm build`、demo provenance 通过。真实 UI 验收仍由 owner 完成。


## 2026-10-22 自定义看板筛选、弹层定位与 One/New API 删除残留

- Customize 顶部移除皮肤工坊入口（侧边栏 palette 入口保留），改为“全部 / 已配置”分段 Tab；已配置筛选按 A-Z 排列，账号型 Provider 会先补拉账号列表再筛选。
- One/New API 删除 key/site 后立即从 `lastSnapshots`、`lastSpend`、`accountsCache` 移除对应卡片，并 `refreshAccounts + notifyAccountsChanged`，不再等下一轮刷新；修复删除 api2 后仍显示旧胶囊。
- 弹层漂移：根因是 `body.ui-scaled` 的 CSS zoom 下，`getBoundingClientRect()` 是视觉坐标，而 fixed 菜单/提示用布局坐标。新增 `scaledUiRect / uiViewport / placeMenuAt`，统一改造分组菜单、卡片右键、分享范围、字体菜单、授权中心菜单、Peak 帮助、Tour、拖拽 ghost 与通用 title tooltip；`placeFixedTip` 继续用于消费趋势 tooltip。
- 账号状态条：`.card-account-tabs` 居中，账号 tab 用 `gap` 替代 dot 的右边距，竖向主题下圆点不再偏左。
- 验证：frontend 85 passed（新增 ONA 删除与 CSS zoom 定位回归），`pnpm build`、demo provenance 通过；官网 agent 的 `site/` 静态改动未触碰。真实 UI 验收仍由 owner 完成。
