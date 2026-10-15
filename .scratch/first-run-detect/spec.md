# 首启全量检测 + 勾选上板（first-run-detect）

**Status:** implemented（2/2 票，2026-10-15；tsc/vite/parse-tests 794:0:13/i18n 门禁全绿；首启 UI 验收待 owner）
**来源**：owner 2026-10-15 拍板——「首次启动应该全量检测本机有哪些 coding harness（Claude Code / Codex / Copilot / Trae CN / Qoder CN / WorkBuddy …），列出来让用户勾选，勾中的直接进额度总览；必须要全量检测」。
**分支**：`codex/first-run-detect`（从 main 起）。

## Problem Statement

现状：首启只探测「默认启用」的约 9 家主流——静态 `DEFAULT_DISABLED_PROVIDERS`（57 家）里的 provider **连探测都不跑**（`run_usage_fetch_inner` 在 spawn 前就跳过禁用项），所以本机装了 Trae CN / Qoder CN / Copilot / Kiro 的用户首启看不到它们，必须自己进定制面板一家家找。检测能力本身早就存在（各 provider 读本机凭据文件 + 环境变量兜底），缺的是「首启全量探测 + 让用户确认要盯哪些」。

## Solution

首启（真·首次，非升级）流程改为：

1. **全量探测**：首启那次 fetch 不带默认禁用名单（只排除用户显式移除的），所有 family 都跑一次检测——无凭据者只做本地文件/env 读、不发网络；有凭据者真查询。
2. **勾选弹窗**：把「检测到凭据」的 provider 列成清单（图标 + 名字 + 状态提示），默认全勾；另外把 Claude / Codex 作为「常用但未检测到」的建议项列出（未勾）。用户确认后：
   - 勾中的 → 启用，直接进额度总览与看板；
   - 未勾的 → 进定制面板（禁用）。
3. **关闭/跳过的兜底**：直接关掉弹窗 = 采用默认（检测到的全选），同样写入结果，不二次打扰。
4. **一次性**：新增 `setupPicked` 配置键标记已完成，此后不再弹。设置里提供「重新检测」入口，可随时重跑同一流程。

## User Stories

1. 作为新用户，我首次打开 Pane 时看到「本机检测到这些工具」的清单（Trae CN、Qoder CN、Claude Code…），勾选我要盯的，确认后额度总览直接就有这些卡。
2. 作为新用户，我本机什么都没装时，弹窗告诉我没检测到，并建议先接入 Claude / Codex（两张引导卡仍在）。
3. 作为升级用户（已有布局与配置），我完全不被打扰——弹窗只在真·首启出现。
4. 作为用户，我误关弹窗后不会被反复骚扰（按默认结果落配置）。
5. 作为用户，我后来装了新工具，可以从设置里的「重新检测」重跑同一流程。
6. 作为俄语/英语用户，弹窗文案三语齐备。

## Implementation Decisions

- **新 config 键 `setupPicked`**（bool，默认 false）：后端 config_with_defaults or_insert(false) + CONFIG_KEYS；前端 Config 接口 + FRONTEND_CONFIG_KEYS + 两处默认对象 + resetAllSettings 同步。
- **探测调用**：复用现有 `fetch_usage`，首启时 `disabled` 只带 `removedProviders`（全量探测）；`clearBenches: false`。
- **判定**：`detected = snapshot.status !== "no_credentials"`（有凭据即算检测到——查询失败/过期也算，卡片会显示「需重新登录」，正是要告诉用户的信息）。
- **弹窗**：新函数 `appProviderPicker(...)`，沿用 `#confirm-overlay` 既有遮罩与样式基底（加宽变体），列 detected（默认全勾）+ starters（未检测到时列为建议、未勾）；状态提示复用现有 i18n（customize.connOk / connError 等）；确认 / Esc / 点遮罩关闭都走同一落配置路径。
- **落配置**：`disabled = 全部 family − 勾选项`（并入 removedProviders 语义不变）+ `setupPicked: true` + 初始化 `layout`（沿用现有首启分支的标记方式，使旧的首启自动禁用分支不再触发）。
- **时序**：boot 里 `initSettings` 之后、`maybeStartTour` 之前；弹窗完成后再起聚光灯引导（引导聚焦的是已选好的看板）。面板窗（IS_PANEL_FORM）不参与。
- **兜底**：若弹窗流程异常（探测失败/无快照），保留现有 refresh() 内 `layout === null` 的自动禁用分支作为退路。
- **重新检测**：设置 → 常规加入口，重跑探测 + 弹窗（复用同一函数，跳过 setupPicked 判断）。

## Testing Decisions

- 前端：tsc --noEmit + vite build。
- i18n 三字典键集由 tests/i18n-pairing.test.mjs 门禁守护。
- 后端键默认值与白名单：代码走查 + MSVC CI job（lib.rs 测试已复活）。
- 探测/勾选/落配置的核心逻辑（detected 判定 + disabled 计算）抽成纯函数，便于未来单测；本轮以 tsc 与 UI 验收为主。
- **UI 验收由 owner 亲自做**：真·首启弹窗、勾选结果上板、升级用户不被打扰、重新检测入口。

## Out of Scope

- 不做「首次之后装的新工具自动提示启用」（二期）。
- 不改 `DEFAULT_DISABLED_PROVIDERS` 名单本身——它退化为「未参与首启勾选时的默认值/性能名单」，不再决定首启可见性。
- 不动已有的 5 步聚光灯引导内容，只调整其启动时序（在勾选之后）。
- 不动 unconfigured-fold 的折叠逻辑（两者互补：折叠管「未配置仍启用」的卡，本票管「首启决定启用哪些」）。

## Further Notes

- 检测是本地为主的：无凭据 provider 不发网络；有凭据者一次真查询（首启一次性成本）。
- 与 0.8.3 已发的折叠机制关系：折叠解决「已经满屏怎么办」，本票解决「首启就不该满屏、且要按用户意愿上板」。
