# 首启引导与来源探测交接（#109）

## 2026-10-28 状态

**状态**：五张票实现完毕，构建/测试与部分真机走查通过；**owner 视觉验收待做**（见文末清单）。未 push、未发布。

**分支**：`codex/onboarding-source-detection`，基于 `main` `041a3d0`（v0.11.1 之后）。

**Issue**：https://github.com/Aafff623/pane/issues/109

## 已改

**后端（只读探测，不执行 CLI、不联网）**
- 新增 `src-tauri/src/source_probe.rs`：PATH/PATHEXT + 已知安装目录 + 进程环境 + HKCU/HKLM 环境（另一终端 `setx` 后重扫可见）；按 family 返回 `installState`（installed/missing/unknown）、安装证据、凭据有无与来源（`env:NAME` / `keyvault` / `local` / `none`）、环境变量名。只暴露变量名与存在性，绝不读取或返回变量值。
- `provider_env_vars` 从 `lib.rs` 移到 `provider_catalog.rs`，让 `parse-tests` 能以 `#[path]` 编译真实探测代码；`parse-tests` 增加 `winreg`（仅 Windows）。
- 新增 `source_probe` 命令；`get_credential_status` 增加 `envVars`/`installState`/`installEvidence`/`credentialSource`。
- `/v1/providers` 增加 `installState`/`installEvidence`/`credentialState`/`credentialSource`/`credentialEnvs`（纯增量）；`/v1/doctor` 增加 `installed_but_not_signed_in`，有内容时归为 `info` 提示而非故障。
- 凭据判断与 `get_credential_status` 对齐：登录账号、账号列表、family 密钥文件依次检查；只存在账号列表里的来源（Linkso、TokenRhythm 等）也会进入检测清单。

**前端**
- 首启合并为一条四步全屏流程：欢迎 → 检测到的来源 → 连接/验证 → 完成；旧的「先选服务商再静态三页」双重弹窗已删。
- 检测步骤展示证据（可执行文件/目录/环境变量名）、安装与凭据状态、逐项操作和重新扫描；常见但 Pane 暂无额度读取器的 key（`ANTHROPIC_API_KEY` 等）显示为「已发现但暂不支持」。
- 完成条件改为真实 `status == ok` 的来源，而不是点「下一步」；空面板保留「继续设置」入口直到有可用来源。
- 无凭据卡片区分「已安装未登录」与「未安装/未知」；凭据助推行展示来源与环境变量名。
- 设置 → 通用沿用原有检测行，改为两个按钮：「重新扫描来源」（环境探测）与「重新检测工具」（原有 provider picker）。
- 设置里重放引导只读：`onboardingApplyPicks=false`，不会改写用户的服务商配置；首启与空态续接才应用选择。

**文档/资产**
- `docs/providers.md`、`docs/provider-credentials.md`、`docs/local-http-api.md`、`README.md`、`CONTEXT.md`、`CHANGELOG.md`（Unreleased）、`docs/project-facts.md` 已同步。
- 顺带修复：`pricing` 两个用例共用全局 overrides 表导致的并行偶发失败（新增测试静态锁），属于验证中暴露的既有竞态。

## 已验证

- `corepack pnpm build` 通过；`corepack pnpm test` 239/239（新增 `tests/onboarding-source.test.mjs` 2 项）。
- `cargo +stable-x86_64-pc-windows-gnu check` 通过。
- `parse-tests`：串行并清空代理变量后 **1195 passed / 0 failed / 18 ignored**（含新增的 2 个探测用例）；并行跑会出现既有 loopback/全局状态类 flake，单独重跑均通过。
- 真机（debug 构建）：四步引导完整走通，检测行显示真实路径与凭据来源；临时 `setx AZURE_OPENAI_API_KEY` 后点重新扫描，环境键立即出现（随后已删除该变量）；设置页扫描行与重放无副作用已确认。
- `/v1/providers`、`/v1/doctor` 在本机 6736 端口实测返回新增字段与提示。

## 未验证（留给 owner）

- 全新安装形态下「已安装未登录」卡片文案与操作；空面板的「继续设置」续接（本机数据不是空面板）。
- 英文/俄文视觉、最小窗口宽度、reduced-motion 下的引导布局。
- 任何发布动作：按项目规则需 owner 明确授权。

## 验收清单（owner）

```powershell
cd D:\code\pane && corepack pnpm build
Stop-Process -Name pane -Force -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\com.aafff623.pane\EBWebView" -ErrorAction SilentlyContinue
& .\temp\scripts\restart-pane-dev.ps1
```

1. 设置 → 通用：看到「重新扫描来源」与「重新检测工具」两个按钮；点「重新扫描来源」不报错、不重启。
2. 设置 → 关于/引导入口重放引导：四步可走完，检测页有真实证据与来源；退出后服务商配置未被改写。
3. 在另一个终端 `setx AZURE_OPENAI_API_KEY test-109`，回到设置点「重新扫描来源」：检测页出现该环境变量名；验完在该终端 `reg delete HKCU\Environment /v AZURE_OPENAI_API_KEY /f`（或系统属性里删掉）。
4. 无凭据的已安装工具卡片应显示「已安装、未登录」而不是纯「未找到登录」。
5. 把面板清空到没有可用来源（可临时隐藏全部卡片）时，空面板出现「继续设置」并回到检测步骤。
6. 切换中文/English/Русский，四步引导与检测文案无遗漏、无截断。

## 提交与授权

- 本分支的提交只做本地记录；push / PR / 发布需 owner 明确授权。
- winget 新包 PR microsoft/winget-pkgs#463652 与本 ticket 无关，仍在外部校验队列；落地后需在 #105 补最终回复。

## 2026-10-28 补充：已配置来源被首启选择关掉

- 现场：数据目录里 apigoto、tokenrhythm、workbuddy、minimax、stepfun-plan、sensenova、xfyun、qodercn 等 13 个 family 位于 `disabled`，账号卡被 `isCardDisabled` 一并隐藏，看起来像“凭据/服务商丢了”。
- 原因：首启选择器只把 family 卡当作检测证据；这些来源的密钥在账号列表里，family 卡是 `no_credentials`，确认/跳过时被整族写进 `disabled`。
- 代码修复：`detectedFamilies` 把账号卡计入 family 证据；引导默认勾选 ok 快照对应 family；跳过/关闭引导不再应用勾选，只有最后一步「完成」才写入选择。
- 本机恢复：从 `config.json` 的 `disabled` 移除上述 13 项（备份 `config.json.bak-before-disabled-restore-20261028`），重启后 active 28 → 39，apigoto/tokenrhythm/workbuddy/minimax/xfyun/qodercn 等卡回到面板。
- 恢复后仍有外部会话问题（非 Pane 数据丢失）：Claude token 为空、SenseNova 会话过期、Trae CN token 被拒、Grok/Qoder/Qwen 未登录；需要对应客户端重新登录。StepFun Step Plan 返回无配额数据，ZCode 无 Start Plan。

## 2026-10-28 独立复审与修复

- **Blocker 已修**：空看板「继续设置」走的是非首装路径，应用勾选时现在会并入当前已启用的 family，扫描暂时漏掉的已配置来源不会被写进 `disabled`；旧检测器在已有配置的首跑路径也保留已启用项。设置里的重看仍只读。
- **探测改为真只读**：不再调用会迁移并改写文件的 `stored_key_file`，新增只读存在性检查；账号/登录/Codex 登录只数文件行，不读 vault。
- **Windows PATH 修正**：变量名大小写不敏感，注册表 `%VAR%` 会展开，机器级 PATH 不再丢；npm 全局安装的 CLI 现在能作为安装证据（本机已出现 `.local\bin\claude.exe`）。
- **性能**：`probe_families` 每次请求只读一次环境和一份 PATH 目录索引；`/v1/providers` 从 8–9 秒降到约 0.1 秒，`/v1/doctor` 从 13 秒降到约 0.15 秒。
- **连接步骤**按账号卡判断「已连接」，不再被 family 卡的 `no_credentials` 误导。
- 门禁：Node 244/244；`parse-tests` 串行去代理 1200 通过 / 0 失败 / 18 忽略；`cargo check` 通过。

## 2026-10-28 补充：存在 ≠ 有效（OpenCode 过期 key）

- 现场：OpenCode 卡显示 “Go — this PC only” 和 $0/$12，实际是 OpenCode auth.json 里一把 2026-10-05 的 key 已过期（用量接口返回 `api_key_expired`）；旧逻辑把任何查询失败都回退到本地 opencode.db，并仍标注 Go 计划。
- 机制修复：`providers::credential_rejected` 统一识别 401/403/过期/撤销；OpenCode 被拒时直接返回错误、不再本地回退（离线/5xx 仍可本地回退且标注清  楚）；`/v1/doctor` 新增 `credentials_rejected`；首启检测状态结合最新查询结果，过期凭据显示“凭据已过期或被拒绝”，不再叫“已找到凭据”。
- 影响：过期 key 不再触发每次刷新复制 400MB 的 opencode.db。
- 追加：`snapshot_store` 对“凭据被拒绝”的错误不再恢复上次成功快照，并清除持久缓存；否则旧 Go 卡会以 last-good 形式复活。实测 `/v1/providers` 的 opencode 已变为 error、plan 为空，`/v1/doctor` 的 credentials_rejected 同时列出 claude 和 opencode。

## 2026-10-28 异常状态卡统一（UI follow-up）

- 旧实现：`error` / `no_credentials` 用 placeholder + 虚线框 + “测试连接 / 怎么补 / ?” 三个同权重按钮；`needs_setup` 只有一行文字。问题：中文套英文、按钮语义含糊、没有主次、把刷新说成测试。
- 新实现：`repairKind` → `repairView` → `repairPanelHtml` 一条映射。每张失败卡显示：额度状态（点+文案）、标题、人话解释、一个主操作、可展开诊断（三步、凭据来源、安装状态、服务商原始返回、接入说明/重新查询）。
- 动作按凭据方式分流：key/oauth/web-session 由 Pane 代劳（设置或授权中心）；local/session/cookie/cloud 显示“查看登录方法”，不假装 Pane 能替用户登录。
- 新增 `empty` 失败类型：StepFun 的 “no quota data” 是查询完成但没有可展示数据，显示“额度未获取 / 暂未获取到额度数据”，与 `unsupported`、凭据错误分开；overview 的悬浮提示也使用同一分类。
- 缓存规则：被上游拒绝的凭据驱逐 last-good（不会复活旧计划）；暂态错误仍保留旧数据并标 stale。
- 真机截图验证：SenseNova / Trae CN（红色“需重新登录”，主操作“查看登录方法”，详情含三步与原始返回）、StepFun（灰色“额度未获取”，主操作“重新查询”）、ZCode（绿色“已连接 / 需要补充授权信息”，主操作“完善授权”）。
- 门禁：Node 247/247（新增 empty 分类、repairKind/repairView 映射与本地来源动作测试）；前端构建通过；本轮未改 Rust，parse-tests 维持 1200/0/18。
