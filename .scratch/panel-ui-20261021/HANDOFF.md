# Pane 交接稿（2026-10-21，交给 Codex）

冷启动用。所有事实都经复核：`git log/status`、`node --test`、`tsc --noEmit`、`site/*.test.mjs`、`sha256sum` 比对、`/v1/*` 现场查询。

## 0. 一句话状态

`main` 本地领先 3 个提交未 push；工作区有 2 个未提交文件（本轮 UI 改动，已编译通过但**未视觉验证**，并打红了 site 的 demo 哈希门禁）；6 项需求里 2 项改了代码、1 项只定位、3 项只有方案。

## 1. 仓库与运行实例

```
分支  main...origin/main [ahead 3]      未 push / 未打 tag / 未发布
提交  001d027 feat(auth): 授权中心显示重登原因与最近查询     7 files  +345/−137
      1aae6ed fix(accounts): 按 card id 删除/归档 + 压缩账号右键菜单  12 files +707/−64
      8a6a0d0 docs(readme): 刷新 0.8.8 截图、hero 与结构     30 files  +508/−424
未提交  src/main.ts (+20/−19)  src/styles.css (+112/−127)
调试实例  pane.exe PID 37308（19:12:37 启动），Vite 127.0.0.1:1420，API 127.0.0.1:6736
          build stamp 1021-1109 —— 跑的是改动前的构建，未含上面 2 个文件
```

注意：三个提交都**直接落在 main 上**，没有走 `codex/<feature>` 分支；评审已把这点记为流程问题。

## 2. 验证结果（实测，不是转述）

| 检查 | 结果 |
| --- | --- |
| `node --test tests/*.test.mjs` | 64 pass / 0 fail |
| `corepack pnpm exec tsc --noEmit` | 通过（无输出） |
| `node --test site/*.test.mjs` | **1 fail**：`demo: bundled frontend matches the current app sources` |
| 失败原因 | `source.json` 记 `src/main.ts = ef8a7c2b…`（= HEAD），工作区 = `5cbc9869…` |
| 结论 | **本轮未提交改动新造成的红**，不是历史遗留；改完要 `node site/scripts/build-demo.mjs` |
| parse-tests（会话内累计） | 915 → **917 passed / 14 ignored**，含 2 条 `delete_by_card_id` 新测试 |
| `cargo check` | 通过；`src-tauri` 完整 `cargo test` 本地链不起，由 CI 跑（ci.yml 标称 836 条） |
| `corepack pnpm build` | 通过（`pnpm` 不在 PATH，必须 `corepack pnpm`） |
| site + i18n 提交前 | 38/38 通过 |

## 3. 六项需求完成度

| # | 需求 | 代码 | 验证 | 备注 |
| --- | --- | --- | --- | --- |
| 1 | 删除=真删 / 归档=可恢复（设置、授权中心、Codex） | ✗ | ✗ | 只有方案，见 §5.3 |
| 2 | GPT/Codex 用量超时 | ✗ | 部分 | 根因定位，见 §4.2 |
| 3 | 暗色 `3/4` 胶囊（oil-ui-pro） | ✓ | ✗ | 未编译未截图 |
| 4 | 更新日志搬到 设置→关于 | ✓ | ✗ | 同上，且打红 site 测试 |
| 5 | Magpie OAuth 是否抄清 | — | ✓ | 结论见 §4.3 |
| 6 | 设置页垂直居中留白 | ✗ | ✓ 定位 | 见 §4.1 |

已改代码位置（按当前文件行号）：

- 胶囊：`src/styles.css:4388` `.provider-account-badge`、`src/styles.css:7289` `.overview-account-count`、`src/main.ts:4740` `overviewCountBadge` 新增 `is-ready/is-partial/is-empty`
- 更新日志：`src/main.ts:10367` `renderDashboard()` 去掉时间线、`src/main.ts:10385` `renderChangelogTimeline(variant)`、`src/main.ts:16243` About 插入 `.about-changelog`、`src/styles.css:3474` 样式、「查看全部」复用 `src/main.ts:17518` 的 `[data-open-changelog]` 委托

设计意图：状态只由 5px 圆点表达（ready 绿 / partial 橙 / empty 红），数字用 `--foreground`、分母用 `--muted-foreground`，去掉饱和渐变底与发光字。

## 4. 三个已定位的问题

### 4.1 设置短页面被垂直居中（用户当前最在意）

`src/styles.css:8309-8319`

```css
#panel-view-settings { display:flex; flex-direction:column; min-height: calc(100vh - 132px); }
#panel-view-settings::before, #panel-view-settings::after { content:""; flex:1 1 0; min-height:0; }
```

这对弹性伪元素就是「不满一屏时垂直居中」的实现。常规/密钥内容长撑满，看不出；快捷键、通知、隐私、网络、关于内容短，被顶到中间。

改法：删 `::before/::after` 与 `min-height`，留白交给 `#panel-scroll` 的上内边距（28px → 40–48px）；**保留** `#panel-view-settings > * { max-width:940px; margin-inline:auto }` 的水平居中。7 个 tab 逐个看。

### 4.2 Codex 用量超时

- 现场：`GET /v1/usage/codex` 返回 `status: ok`（Session 0%、Weekly 16%、plan team）→ 登录与端点没问题，用户截图是瞬时/环境性失败。
- 文案出自 `src-tauri/src/providers/codex.rs:435`：`send_with_direct_fallback` 代理×2 + 直连×1 全败后统一报「无法访问 chatgpt.com」，`operation timed out` 是 reqwest 20s 总超时。
- **真缺陷**：`src-tauri/src/providers/mod.rs:327` 用 `OnceLock` 缓存 `reqwest::Client`，代理在客户端创建那一刻定死；而 `outbound_proxy_url()` 每次重读系统代理。Pane 先启动、Clash（`127.0.0.1:7892`）后开或换端口 → 只能直连 chatgpt.com → 超时，必须重启应用才好。
- 修法：`http()` / `http_no_redirect()` 改成以当前 `outbound_proxy_url()` 为 key 缓存，变了就重建；`http_direct()` 保持 `no_proxy()`。
- 本机事实：`netsh winhttp show proxy = http=127.0.0.1:7892`；`config.json.proxy = {enabled:true, url:""}`（走系统代理分支）。

### 4.3 Magpie OAuth 复核结论

参照仓 `D:/code/pane/temp/magpie-yetone`（`yetone/magpie`，Go 网关）。

- `internal/provider/logins.go` 的 `loginAgents` 只有 `claude`、`codex`，机制是**导入/切换/保活厂商 CLI 的已登录凭据**（Codex 读 `~/.codex/auth.json` + refresh，见 `internal/provider/account.go:936-1121`），授权动作交给厂商 CLI。
- 所以**没有「每个 provider 的 OAuth 登录代码」可抄**，用户的前提不成立；Pane 这条线自研且更全。
- Pane 侧：`src-tauri/src/codex_login.rs` 浏览器 PKCE（`127.0.0.1:1455`、S256、client id `app_EMoamEEZ73f0CkXaXp7hrann`、`codex_cli_simplified_flow`/`id_token_add_organizations`/`originator=Codex Desktop`），端口被 CLI 占用时返回 `CODEX_OAUTH_PORT_IN_USE` 并静默降级设备码 `oauth_start/oauth_poll`；登录完成经 `codex_accounts::record_login` 成为独立卡片。另有 kiro/qoder/trae/windsurf/zed/codebuddy 家族浏览器登录、cursor PKCE、antigravity 捕获、tokenrhythm 网页会话。
- 已有单测：RFC 7636 S256 向量、callback 解析、authorize URL 参数。
- **未做**：真实点一次「添加 Codex 账号」走通浏览器回调（用户说他自己登录验收）。风险：1455 被 Codex CLI 占用会降级成设备码，属预期。
- 明确没抄的（`docs/project-facts.md:532`）：插件运行时、`migrations.json`、巨型 `Provider` 结构、按名字的 stable id、风险确认弹窗。

## 5. 待办（按依赖排序）

### 5.1 设置页取消垂直居中
纯 CSS，最低风险，先做。

### 5.2 定稿胶囊 + 更新日志两项
1. 决定方案是否接受；
2. `node site/scripts/build-demo.mjs` 修 site 哈希门禁；
3. 亮/暗两主题各截 `.provider-account-badge`（三态）与 `.overview-account-count`；
4. **README 贴图会过期**：`attachments/overview-dark.png` 里 ClinePass `5/6` 还是旧蓝胶囊 → 定稿后重跑 `temp/scripts/readme-shots.mjs` 并重提交 `attachments/*`；
5. 提交这 2 个未提交文件。

### 5.3 删除/归档语义收口
- `src-tauri/src/oauth.rs:226` `logout()` 拆出 `logout_from(dir, provider)`；
- `src-tauri/src/lib.rs:3602` `account_delete` 取消对 codex / login 族的拒绝：按 card id 删 + 清 legacy OAuth 槽（**不清就会下次读取复活，正是 MiniMax 那类根因**）+ 回收 vault/快照/layout/置顶；`card_id == "codex"`（CLI 默认登录）拒绝并说明；
- 前端三处入口统一走 `account_delete`：设置账号行 `src/main.ts:14661`、授权中心 `src/main.ts:17103`、卡片右键 `src/main.ts:9104`；
- **坑 1**：`appConfirm` 带 `checkboxLabel` 时返回 `{ok, checked}` 而非 boolean（`src/main.ts:5310-5361`），设置里现有 `.then(async (ok) => { if (!ok) return })` 加复选框后会永远判定为「已确认」；
- **坑 2**：`forgetAccountFromBoard` 用 `removedIndex === 0` 判定默认位并连带清 bare family 快照（`src/main.ts:9087`），登录族 index 0 不是默认位，传参前按 `providerDefinition(family)?.supportsApiKey` 区分。

### 5.4 生命周期残留（票 `.scratch/account-lifecycle/issues/05-lifecycle-residuals.md`）
- `account_archive` / `account_set_default` 仍按 index，行号过期会操作错账号；
- command 层测试与写失败注入未补；
- 外部族（cursor/antigravity）归档项删除按钮隐藏、`account_delete` 可能留 live slot；
- 归档回滚可能误删既有墓碑，且被 `let _` 吞掉；
- 删最后一个账号的 no-account / needs-setup 未验证；
- 删除确认文案未说明密钥去向；归档删除硬编码 `keepUsage:false`。

票 03 已核实：add-account 代码本来就是 save-first，无需改动。

### 5.5 代理客户端热更新
见 §4.2。

### 5.6 交用户验收
重启调试实例后由 owner 走：MiniMax 删除/归档/置顶接力、Codex 登录、暗色胶囊、About 更新日志、设置各页顶部留白。

## 6. 分发侧遗留

- winget：`WINGET_TOKEN` 已轮换（PAT 存 `temp/secrets/github-winget-pat.txt`，**值在聊天里出现过，建议合并后由 owner 再轮换一次**）；workflow run `39559876109` 成功；`microsoft/winget-pkgs#457718` 已 MERGED（`mergedAt = 2026-10-21T08:17:33Z`，merge commit `f834fcd8bb62153138ec46f9b8331a442332cf80`，161 项检查全过，CLA 由 owner 本人回帖通过）；0.8.8 清单已核。
- 本机无 `winget` 命令 → **客户端安装未验证**。
- 旧的 0.8.0–0.8.6 **7 个 winget PR 仍 open 未关**。
- GitHub：#34 已关闭（附代码/单测证据）；#33 / #27 的漂移已写入票面。

## 7. 约束

- **不 push、不打 tag、不发布、不合并 PR**，除非 owner 明确点头。
- 不动 `D:\code\pane` 无关区域；owner 说过「你别影响到我这个项目」。
- 不读取/回显 `temp/secrets` 内容；账号凭据只取标签与 id，密钥一律脱敏。
- 不在 owner 机器上代签任何 CLA 或同意条款。
- Git Bash 里 `reg query` 反斜杠路径会被吞，改用 `powershell -NoProfile -Command '...'`（外层单引号，否则 `$_` 被 bash 展开）；PowerShell 脚本必须用 pwsh 7：`C:/Users/Lenovo/AppData/Local/Microsoft/WindowsApps/pwsh.exe`。

## 8. 常用命令

```bash
cd /d/code/pane
corepack pnpm test                       # 64 条 JS 测试
corepack pnpm exec tsc --noEmit          # 类型检查
corepack pnpm build                      # tsc + vite build
cd site && node --test *.test.mjs        # 含 demo 源码哈希门禁
node site/scripts/build-demo.mjs         # 改主项目 src 后必须重跑
node temp/scripts/check-readme.mjs       # README 结构/资产门禁
node temp/scripts/readme-shots.mjs       # 重拍 README 贴图
cd src-tauri && cargo check
C:/Users/Lenovo/AppData/Local/Microsoft/WindowsApps/pwsh.exe -NoProfile -File \
  D:/code/pane/temp/scripts/restart-pane-dev.ps1        # 重启调试实例
C:/Users/Lenovo/AppData/Local/Microsoft/WindowsApps/pwsh.exe -NoProfile -File \
  D:/code/pane/temp/scripts/pane-shot.ps1 -Out D:/code/pane/temp/shots/after.png
curl -s http://127.0.0.1:6736/v1/providers | head -c 2000   # 只有 /v1/*，没有 /api/*
```

唤出窗口：`Alt+2`、托盘，或 `pwsh -File temp/scripts/pane-summon.ps1`。

## 9. 并行会话提醒

本项目**同时有另一个会话在写 auto-memory**（`~/.qoder-cn/memory/MEMORY.md` 与项目 memory 索引在本次会话期间被外部更新，新增了「账号生命周期规则」「Magpie 上游范围」「winget 发布路径」「UI 批评裁定」「上下文上限交接」等条目）。代码工作区没有冲突（`git status` 只有本文 §1 列的 2 个文件），但动记忆层前先重读，别覆盖对方写入。
