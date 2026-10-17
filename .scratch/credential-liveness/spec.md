# 凭据可用即上板：查询不到用量时显示状态 + 操作指引（credential-liveness）

**Status:** resolved — owner UI accepted 2026-10-16
**来源**：owner 2026-10-15 明确要求（原话要点）：
> 「只要用户接入了，你都得在咱们的看板里显示，只不过用量先别显示，因为查不到……你必须在我们这个详细看板里面去展示这个 provider。不管他是在设置里面去添加账号，还是在详情卡片的设置里面去添加账号，反正只要连通性测试通过了，你就要把它显示出来。只不过用量查询的那个进度条你可以先不显示……但是你要去提示他、告诉他需要提供什么，并给出具体的操作流程。」

**已确诊的两个缺陷（同一次报告里，根因不同）**

1. **卡片的 body 全空**（截图里 StepFun 卡只剩标题 + 账号页签 + 「平台」链接）：
   `layout.providers["stepfun"].onDemand = ["Balance","Cash","Vouchers"]` —— 该家族当前的全部指标都落在「按需展开」里，而卡片是收起的 → `always` 为空 → body 为空。用户在 stepfun 还是 Credits 口径时能看到（那时有 `Credits used` 常显），指标换成 Balance/Cash/Vouchers 后全部落进 onDemand。
2. **凭据可用但查不到用量 → 整张卡被当成不存在**（更本质，owner 要修的就是这条）：
   `stepfun-plan` 遇到 API key（非会话令牌）→ 控制台接口 401 → 错误阶梯跑完 → `Snapshot::error(...)` → 前端按失败卡处理；owner 期望的是「连通性通过就上板 + 显示状态 + 告诉我还缺什么」。

## Solution

**规则（owner 定的）**：一个 provider 只要**凭据存在且能用**，就必须出现在看板上并给出可读状态；用量拿不到时不是消失，而是**明确告知缺什么、怎么补**。

分三层实现：

1. **后端：provider 增加「凭据已就位但用量需授权」这一态**（`Snapshot` 加一个语义明确的标记，例如 `needs_setup`/`credential_ok` + `setup_hint`），由各 provider 在「凭据存在且形式正确、但读用量被拒」时返回，携带面向用户的补齐步骤文案。
   - `stepfun-plan`：token 不是会话令牌对（缺 refresh JWT / `oasis-webid`）或账号密码为空时命中此态，提示「需要从浏览器导入会话令牌，或填写 StepFun 账号密码」。
   - 此态**不进错误退避**（不是故障，是缺授权），刷新节奏与正常卡一致。
2. **前端：该态渲染成正常卡的外壳**——图标、名称、账号页签、`n/m` 徽章齐全，body 显示状态行 + **可操作指引**（文案 + 直达设置/该账号配置面板的按钮），不显示百分比进度条。
3. **修 body 全空**：`metricOrder` 里的键若在当前快照里不存在，且该家族**全部**指标都落在 onDemand → 不能渲染成空卡；改为至少显示一行状态（或在指标集合变化时自动把新键加入常显，见 Testing）。

## Implementation Decisions

- 不新增「第 4 种 status」以外的语义：沿用现有 `status` 字符串集合（ok / no_credentials / error），新增 `needs_setup`，前端 `muted`/body 分支按它分流；后端所有 `match status` 处同步（含 `isBareHintCard`、`accountHealthDot`、概述分组、托盘投影——**逐处核对**）。
- 文案走 i18n 三语（新增 `card.needsSetup*` 键），指引里必须写明**具体操作路径**（浏览器 DevTools → Application → Cookies → 取哪个键）。
- 账号卡与家族卡同一套逻辑：账号级 `needs_setup` 也要显示该账号页签与状态。
- `stepfun-plan` 的凭据形态判定要能区分三种输入：会话令牌对（可用）/ 裸 access JWT（提示缺 refresh 半）/ 开放平台 API key（提示这不是 Token Plan 凭据）。
- 不改 `stepfun`（按量）的既有行为：它本来就查得到余额，属正常卡。

## Testing Decisions

- 新增 status 的**穷尽匹配**检查：`parse-tests` 里对 status 字符串的 match 加测试，保证新增分支不会漏（本仓已因这类漏项踩过坑）。
- `stepfun-plan` 的凭据形态判定做成纯函数（token 字符串 → 形态枚举），配 3 个单测（会话对 / 裸 JWT / API key）。
- 前端：tsc + vite；i18n 键集门禁。
- 「body 不为空」用 renderCard 的纯渲染路径验证（若能抽出纯函数）。
- **UI 验收由 owner 亲自**：StepFun 卡显示 2/2 账号 + 状态 + 指引；点指引能跳到补凭据的地方。

## Out of Scope

- 不做「本小时/本日用量推断」——用量就是拿不到，不编数字。
- 不改 owner 已有的 stepfun 布局（onDemand 归位由「指标集合变化」规则自然处理，见 Testing）。
- 不去实现 StepFun 会话令牌的自动续期之外的其它厂商同类改造（本票只做通用「凭据可用即上板」机制 + StepFun 落地；其它 provider 后续按同机制各自接入）。

## 2026-10-16 Review and acceptance record

- Owner expanded the requirement: even failed or unsubscribed credentials may be saved. Once enabled, their cards remain visible with a query-error state and repair guidance; a successful probe is not a save prerequisite.
- Implemented shared post-save reconciliation for settings/customize, immediate account counts and placeholder/probe snapshots, stable-id credential replacement, default-account retry selection, in-flight identity checks, and ordered cross-window usage events.
- Added source/permission help for all 69 families. Public API audit and the three credential tables live in `docs/provider-api-verification.md` and `docs/provider-credentials.md`.
- Verification: frontend build passed; 30 frontend tests passed, including connected/failed eighth-account counts and guide dismissal persistence. GNU Rust build passed. Parse harness single-thread run: 819 passed / 0 failed / 13 ignored; earlier two-thread run had one One/New API transport failure (819 passed / 1 failed / 13 ignored). No assertion or check was weakened.
- Remaining: owner personally checks account creation, failed-key repair, X/X synchronization, and one-time font education. Real online testing was not performed for every provider or subscription. Publication remains gated on UI acceptance and explicit release authorization.


## 2026-10-16 Owner acceptance

Owner personally accepted account synchronization, failed-account repair/visibility, X/X updates and font-guide dismissal, and explicitly authorized v0.8.5 publication in this conversation. Implementation acceptance is complete. Release CI and distribution results will be recorded after execution; the known parallel local mock-transport failure remains disclosed above.

## 2026-10-16 v0.8.5 release outcome

- Clean release history pushed to `origin/main`: `74b6a37` (chore: release v0.8.5) -> `4e85475` -> `36fdcac`. Each was built with `commit-tree` on parent `ab0fbf4`, so the local branch's pre-scrub history (which still contains the real shared StepFun key fixture) never reached the remote.
- `v0.8.5` tag pushed on `36fdcac`. Release workflow run `38919004888` completed **success**: all three platform builds, the `release` job, and `gitee-mirror`.
- Published as GitHub **Latest** (not draft, not prerelease), `targetCommitish: main`. Assets: `latest.json`, `Pane_0.8.5_x64-setup.exe` (21.3 MB) + `.sig`, `Pane_0.8.5_aarch64.dmg` (24.6 MB), `Pane_0.8.5_amd64.deb` (27.3 MB) + `.sig`, `Pane_0.8.5_amd64.AppImage` (100.9 MB) + `.sig`. Body carries the theme headline, real-size download matrix and 新增/变更/修复 sections per `docs/release-format.md`.
- `latest.json` reports 0.8.5 with signed `windows-x86_64` / `linux-x86_64` entries. Winget publish run `38920615519`: success.
- Still open and disclosed: the One/New API mock transport failure under parallel test threads (serial run 819 passed / 0 failed / 13 ignored). macOS/Linux runtime UX and the real in-app update path remain unverified.
