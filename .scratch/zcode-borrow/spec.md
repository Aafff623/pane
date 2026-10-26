# Spec: ZCode 借鉴批次（引导流程 / 快捷键体系 / 使用统计页）

**Issue:** https://github.com/Aafff623/pane/issues/64

> 用户 2026-10-25 点名三项，**明确不抄其他任何模块**。参照物 temp/repos/ZCode（v3.16.2，**Apache-2.0 可正式抄码**，需在 LICENSE 加 ZCode 署名——照 LICENSE 现有 OpenUsage 段落先例；整函数级照抄须保留版权头）。证据：本 spec 即移植级细节，补充研究底稿 temp/research/20261025-zcode-ui-borrow.md。

## A. 首启引导流程（票 01）

ZCode 实为两层：全屏三步首启向导（OccupationOnboarding.tsx:35-419，职业/模式/偏好，判定靠独立 onboarding-record.json + 3s 超时保守不弹）+ 迁移向导（Pane 无此场景，不抄）。ZCode 的 record 文件/多用户锚点**不需要**——Pane 单用户，config.json 双布尔（onboardingSeen/dismissed，沿用 tourSeen 模式 onboardingState.ts/main.ts:8205 已有 spotlight tour 雏形）。

**Pane 三步设计**：①欢迎+权限/托盘/Alt+2 呼出说明 ②快捷键设置（内嵌现有 recorder，完成即写 config）③provider 扫描等待/完成页。交互规则照抄：每步可跳过（跳过=显式答案）、可返回、ESC 关闭且不再弹、**判定失败保守不弹**（防误扰老用户）。实现：renderOnboarding() 模板 + 单个 document keydown capture（Esc），复用 tour 的 overlay 定位；settings 里加「重看引导」入口。全屏覆盖（含标题栏），三段进度条+返回+关闭。

## B. 快捷键体系（票 02 内核 + 03 设置 UI）

ZCode 精华=「shared 命令表 + 单点 capture 分发 + 纯函数内核」；全仓零 globalShortcut（Pane 的 Alt+2/Shift+1 系统热键体系独立，不动）。

**Pane 缺口**（现状）：应用内 6 条 LOCAL_SHORTCUTS（main.ts:6845-6853），分发散在约 15 处独立 keydown 监听；冲突检测只是排序字符串比较（:6977）；无保留键黑名单、无物理等价归一、无抢绑确认、无多绑定。

**票 02 内核**（Apache-2.0，bindings.ts/conflicts.ts 纯函数可直接抄，带版权头）：
1. `shortcutCommands` 命令表：Pane 版命令清单（导航/搜索/翻页/发送/设置…按实际盘点，参考 ZCode 26 条结构：id/channel/scope/defaultBindings[]）。
2. 纯函数内核：IME/长按噪声过滤（keyCode 229）、修饰键精确匹配+平台隔离、event.code 兜底防布局改写、录制器（code 反查物理基键、裸键防呆）、生效表=默认+覆盖整组替换（**显式空数组=用户清除不回退**）。
3. 冲突策略：保留键黑名单（编辑原生/刷新/F 键/方向键/Enter）、物理等价归一（Ctrl+m ≡ CmdOrCtrl+m）、同作用域占用检测、**抢绑二次确认**（被抢命令保留剩余绑定）、多绑定行级增删换。
4. 分发收口：现有 ~15 处 keydown 监听收拢为单 capture 监听 + 命令表路由（handlers 走 ref，录制态短路，命令无 handler 不 preventDefault）。

**票 03 设置 UI**：按键搜索（文字+按组合键）、行内录制（替换/追加/删除行级）、抢绑确认、恢复默认（冲突 toast + 全部恢复破坏性确认）；持久化沿用 config 现有快捷键结构升级为 Record<id, bindings[]>。

## C. 使用统计页（票 04 壳+迁移 / 05 图表 / 06 口径补齐）

用户已定改造方案：**每日详情移出悬浮窗、使用统计成为设置左侧目录栏独立项、页面对齐 ZCode**。

**ZCode 页面**（AppUsagePanel.tsx:34-131）：终身 KPI 摘要条 5 格（累计 Token/单日峰值/最长会话/连续活跃/最长连续）→ 52 周 Token 热力图（纯 CSS grid 52 列、日/周/累计三 pill、月标签锚定每月 1 日列、四色阶+hover 缩放）→ 7d/30d tabs → 每日按模型折线（Recharts top6 非堆叠；Y 轴上限取可见序列单点峰值、tooltip 过滤 0 值、X 轴稀疏刻度三规则）→ 模型分布 donut（innerRadius 56%、环心总量、>6 合并其他）→ 右下角刷新。

**Pane 现状问题**（renderSpendDetailOverlay main.ts:4537-4623）：挤在窄悬浮窗塞 26 周格、无 KPI、无范围切换、无趋势/分布图、hover 只有原生 title。**Pane 数据优势**：fetch_spend_daily 365 天逐日×provider×模型持久台账（spend_history.rs:273-280）vs ZCode 仅 30 天滚动——呈现层改造即可，Rust 几乎不动。

**票 04（壳+迁移）**：PANEL_PAGES 新增 `["stats","st-stats",...]` 项（main.ts:17868-17876；**注意 nav 已占用 `usage` id=密钥页，必须用新 id**）+ panelBlock 建 stats 块 + i18n 三语 2 键；装 KPI 条（lastSpendDaily JS 聚合：总量/峰值日/streak 照抄 builder :117-135 逻辑）+ 52 周热力（扩展现有 .spend-heat-* CSS styles.css:2981-3067 到 52 周+周/累计列 hover）+ 选日明细（spendDetailDayRows :4427 + Qoder 块原样迁入，热力格点击即内页选日）；**删除悬浮窗 overlay 与 data-spend-details 按钮**（:4775/4973/20429-20462）。

**票 05（图表）**：范围 tabs + 每日按模型折线 + 模型 donut——**自绘 SVG 不引 Recharts**（复用 renderTrend :2704 模式；照抄 Y 轴单点峰值/0 值过滤/稀疏刻度/紧凑单位四规则）；donut innerRadius 56% 环心总量+右侧明细列表。

**票 06（口径补齐，可选）**：spend.rs 增读 ZCode turn_usage/tool_usage（会话数/轮数/工具调用/错误数，schema.baseline.sql:253/283）补 KPI 三格；热力 tooltip 升级三行式。

## 不抄清单（用户明确只抄三项；以下仅列名备查）

迁移向导整层（Claude 会话/技能/MCP 导入）、OnboardingMeshBackground WebGL 动画、useOnboardingTelemetry、多用户 onboarding-record.json、CommandCenter、Lexical composer、CodingPlanUsagePanel 及 coding-plan 额度链路、usage-stats RPC 服务层（Pane 直读本地库）、resource-manager、desktop E2E、设置侧栏其余分区、session 库写入侧。

## 验收

每票独立验收（见 issues/）；引导/快捷键/统计页 UI 全部 owner 本人验收；三皮肤+明暗全覆盖；Apache-2.0 署名落 LICENSE。

## 实施结果（2026-10-26）

全部 6 票代码完成：stats 页（KPI/52 周热力/选日明细/自绘折线+donut）、删除旧 overlay、三步引导、快捷键内核+单 capture 分发、多绑定设置 UI、ZCode turn/tool 指标。新增测试：`tests/stats-math.test.mjs`、`tests/shortcut-kernel.test.mjs`、`tests/account-discovery.test.mjs`、parse-tests 的 `zcode_turn_tool_metrics_match_zcode_usage_semantics`。`pnpm build`、`pnpm test`（205 通过）、`cargo check`、parse-tests（1142 通过，1 个断言已修正后单测通过）有实际记录。UI 手感与三皮肤/明暗仍由 owner 验收。
