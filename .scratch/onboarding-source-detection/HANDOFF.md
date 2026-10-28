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
