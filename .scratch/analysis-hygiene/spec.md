# 工程卫生批次：分析包核实后的落地（analysis-hygiene）

**Status:** implemented（10/10 票落地 codex/analysis-hygiene 分支，2026-10-15；UI 无关项以 parse-tests 794:0:13 + tsc/vite 证据交付，CI 两项待 push 后验证）
**来源**：`D:\code\pane-analysis-20261014\` 三轮分析包 + 08-agent-verification 独立核实（13/14 证实）+ owner 2026-10-14 深夜三拍板（P1-5=keyring 扩展、P1-2=删 package-lock 切 pnpm、批次①立即做）。
**分支**：`codex/analysis-hygiene`（主工作树直接改，不用 worktree——owner 指定）。

## Problem Statement

外部三轮深度分析 + 工作区 agent 独立核实确认了一批工程缺陷，分四类持续伤害项目：

1. **测试静默死亡**：src-tauri 全树 836 个 `#[test]` 中 82 个（lib.rs 43 + tray_projection.rs 26 + telemetry.rs 13）因不在 parse-tests harness 的 `#[path]` 清单且 CI 从不跑 src-tauri 自身测试而从不执行——其中包含「写失败回滚/purge 作用域」等数据损失防线断言。
2. **安全面**：cursor_oauth.rs 与 telemetry.rs 两处手搓 RNG（splitmix64/xorshift，~64 bit 熵），而 `rand_core`（getrandom 特性）就在依赖树、全库 11 文件在用 OsRng；OAuth refresh token（寿命最长、权限最高的凭证）明文落盘 `%APPDATA%\Pane\oauth\*.json`，保护等级低于已进系统钥匙串的 API key。
3. **文档/配置漂移**：AGENTS.md 写 main.ts「~5k lines」实际 15,945 行（3 倍失真，误导每个读它的 agent）；privacy.md 称遥测「one auditable file」实际 payload 构造跨 lib.rs+telemetry.rs 两文件；package-lock.json（CI 用）与 pnpm-lock.yaml（开发用）双锁并存，CI 验证的依赖解析 ≠ 实际运行的。
4. **小缺陷**：俄语字典缺 43 键（One/New API 设置区整片英文回退）且 en/zh 反缺 1 个 ru 独有键；spend 热力图 daily_rows_from 不做 kimi→moonshot 别名折叠（同天双行双计）；49 处 `lock().unwrap()` 只有 1 处防中毒写法（任一 mutex 中毒级联整个刷新循环）。

## Solution

按 owner 已拍板的方向逐项修复：复活 82 个测试（harness 两行 + 一条 MSVC CI job）；两处 RNG 换 OsRng；AGENTS.md/privacy.md/telemetry 注释对齐事实；删 package-lock.json 并把 CI 前端 job 切 pnpm；i18n 三字典键集 CI 门禁 + 补齐 44 键（ru 43 + en/zh 1）；spend_alias 折叠提到共用归一化步骤；49 处 mutex 统一为容忍中毒写法；OAuth 令牌照 API key 已验证的三步迁移模式进系统钥匙串。

## User Stories

1. 作为 pane 用户，我要 lib.rs 里守护快照缓存/卡片设置不被误删的数据损失防线测试真的在 CI 里跑，这样以后改动引入回归时 CI 会红而不是静默放过。
2. 作为 pane 用户，我要 tray_projection 与 telemetry 模块自带的 39 个测试被 harness 执行，这样 UTF-16 裁剪与遥测 payload 的既有断言不再是摆设。
3. 作为 pane 用户，我要 PKCE code_verifier 由操作系统级随机源生成（256 bit 熵），这样登录流的安全属性符合 RFC 7636 量级而不是 64 bit 近似。
4. 作为 pane 用户，我要遥测匿名 UUID 不再依赖 time^pid 推导，这样注释宣称的「OS entropy」与实现一致。
5. 作为 pane 用户，我要我的 OAuth refresh token（Copilot/Codex/xAI 等）存进 Windows 凭据管理器而不是明文 JSON，这样 %APPDATA% 泄露（备份/同步/整机镜像）不再等于全部账号失守。
6. 作为 pane 用户，我要已进钥匙串的令牌在应用启动时无缝读回、明文文件被安全删除，这样迁移对我无感。
7. 作为俄语用户，我要 One/New API 设置区、皮肤预览、footer 提示显示俄语而不是英文回退，这样界面语言一致。
8. 作为俄语用户，我要 en/zh 字典补上 `customize.cred.stepfunPlan`，这样三字典完全对齐、未来缺键被 CI 拦住。
9. 作为 pane 用户，我要 i18n 三字典键集比对进 CI，这样下次加键漏翻任何语言 CI 直接红。
10. 作为 pane 用户，我要花费热力图上 kimi 与 moonshot 同一天只出一行，这样日消耗不被视觉双计。
11. 作为 pane 用户，我要单个 mutex 意外中毒不再级联毒化整个刷新循环，这样一次局部 panic 后应用其余功能还能用。
12. 作为任何读 AGENTS.md 的 coding agent，我要项目地图对 main.ts 的描述不再差 3 倍，这样我对改动范围的规划基于真实体量。
13. 作为审计 pane 隐私声明的用户，我要文档说的「全部遥测实现在哪」与代码一致，这样「能离开机器的内容」有可信的单一审计入口。
14. 作为 CI 维护者，我要 CI 前端 job 用与开发一致的 pnpm 锁文件，这样 CI 验证的依赖解析就是实际发布构建用的解析。
15. 作为 CI 维护者，我要 src-tauri 自身的 cargo test 在 windows MSVC job 里跑起来，这样 82 个测试的复活有持续执行的载体。

## Implementation Decisions

- **死测试复活两步走**：harness 清单补 tray_projection.rs 与 telemetry.rs 两行 `#[path]`（复活 39 个）；src-tauri 自身测试不搬动（lib.rs 43 个依赖 Tauri runtime），以 ci.yml 新增 windows-latest + MSVC toolchain 的 `cargo test` job 承载。GNU 本地链接死因（STATUS_ENTRYPOINT_NOT_FOUND）不适用于 CI 的 MSVC 环境。
- **OsRng 换法**：cursor_oauth.rs 的 `random_bytes()` 与 telemetry.rs 的 `getrandom_fill()` 改用 `rand_core::OsRng`（依赖已在 Cargo.toml），删除 splitmix64/xorshift 实现与失实注释（"No new dependency"、"rand isn't in the tree"）。
- **OAuth 令牌进钥匙串（owner 拍板 A 案）**：沿用 API key 已验证的 secretstore 模式与三步迁移（先写 keyring → 读回确认 → 删明文文件）；keyring 条目命名沿用既有 `account:<card_id>` 风格语义，按 provider+账号作用域命名；读取路径 keyring 优先、明文文件兜底（迁移前旧装机态）；失败不 panic、降级并保留原文件。覆盖 oauth.rs（device-code 流令牌）、login_accounts.rs、cursor_accounts.rs、codex_accounts.rs、search_keys.rs 五处明文存储点。keyvault 不用于机器自动使用的凭证（需人工解锁，与静默刷新冲突）。
- **双锁收一（owner 拍板）**：删除 package-lock.json；ci.yml 前端 job 从 `npm ci`/`npm run build` 改为 pnpm 安装（`--frozen-lockfile`）与构建。
- **i18n 门禁**：新增零依赖 node 测试脚本比对 en/zh/ru 三字典键集（照抄 site 的零依赖不变量测试模式），接进 ci.yml site job 同层；补 ru 缺失 43 键（skin.* 5、footer.onenewapi* 9、settings.onenewapi* 28、metric.expiry 1，翻译按 en 语义翻成俄语）与 en/zh 各补 `customize.cred.stepfunPlan`（按 ru 现有语义回填）。
- **spend_alias 折叠**：别名归一化从 range_from 内联提到 daily_rows_from 与 range_from 共用的归一化步骤，热力图行与汇总口径一致。
- **mutex 容忍写法**：49 处非测试 `lock().unwrap()` 统一为 `lock().unwrap_or_else(|e| e.into_inner())`（lib.rs:425 既有先例）；已验证不可达的防御性 unwrap 不动；不引入新辅助函数（最小改动）。
- **文档修正**：AGENTS.md 项目地图 main.ts 行去掉具体行数；privacy.md「one auditable file」改为指向 telemetry.rs + lib.rs 两处（或合并说明）；隐私文档补 provider_refresh_daily 有损性一句。

## Testing Decisions

- **主 Rust 验证 seam = parse-tests harness**（既有最高 seam）：新增两行 `#[path]` 后 39 个既有测试自然复活；spend_alias 折叠与 OAuth keyring 读写/迁移的新测试写进对应真实源文件的 `#[cfg(test)]` 模块、经 harness 执行。测试只断言外部行为（折叠后行数、迁移后明文是否删除、keyring 优先级），不断言实现细节。
- **前端验证**：`tsc --noEmit` + `vite build`（agent 环境直调 node 路径）。
- **i18n 门禁测试**：键集比对脚本本身即测试（node --test 雎依赖模式，site/*.test.mjs 先例）。
- **CI 配置**（ci.yml 两处改动）无法本地验证：按快速发版协议用 ci-probe 分支预跑（push 前须 owner 授权）；在本地以 yaml 语法与逻辑走查交付。
- **cargo check**：本地 mingw64 GNU 快速检查每个 Rust 改动。
- 先例参考：parse-tests 现有 673 测试、catalog_consistency 的 include_str! 前端源比对、reset_wake 的先落盘后计费测试风格。

## Out of Scope

- **P0-3 拖拽监听累积**——已核实为误报（监听 setup 层注册一次 + pending 门控），不修。
- **P2 架构深化**（main.ts 外提 keyvault/ONA、登录流共享 seam、provider registry、platform seam 收口）——每项独立走完整主流程（grill → spec → tickets），另行排期，本批次不碰。
- **P3 全组**（契约校验、keyvault 拆文件、端点注册表、config schema 版本、updater manifest 测试等）——值得探索层，不立项。
- **P1-3 前端 lint/格式化引入**——单独排期（依赖 P1-2 先定锁文件策略后落地体验更好）。
- renderAll 归拢建议以 amendment 备注并入既有 refresh-ux spec，不在本批次实施。
- 不动 `.scratch/` 既有其它专题、不重写任何测试断言、不降任何检查规则。

## Further Notes

- 核实报告与证据行号：`D:\code\pane-analysis-20261014\08-agent-verification.md`。
- 仓库活跃增长中，实施前各 ticket 自行重测锚点行号（08 报告基于 a2c8ebc 时点）。
- 本地 commit 由 agent 自行划分粒度；**push 一律等 owner 明确授权**（ci-probe 预跑同样属于 push，先问）。
- MSVC CI job 若首跑因环境差异挂掉，单独修，不与本批次其它 commit 混合。
