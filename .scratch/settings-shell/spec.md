# 设置面板骨架重构（以 QuickClipboard 为参照）

**Status:** ready-for-agent
**GitHub:** #49
**Feature dir:** `.scratch/settings-shell/`
**Date:** 2026-10-24
**分析依据:** `temp/research/20261024-quickclipboard-settings-shell.md`（源码 + 实拍 + 行号取证）
**参照物:** `mosheng1/QuickClipboard` v0.5.0（Apache-2.0），本地只读副本 `temp/repos/QuickClipboard`

---

## Problem Statement

Pane 的设置面板现在能跑，但**没有骨架**：

- 没有通用设置行组件。`panelRow()` / `panelSwitchRow()` / `panelBlock()` 三个 DOM 助手只覆盖了少数场景，多数行是 `index.html` 里手写、再被 `moveRow()` 搬进面板的。每加一个设置项都要重新抄一遍结构。
- 同一套设置存在两份物理形态：`index.html` 里的旧 sheet 已无入口，但仍参与 `initSettings()` 绑定。改一处容易只改到一份。
- 样式散在全局 `styles.css`，同一元素靠后置规则覆盖（区块标题 15px 被 11px 覆盖、卡片圆角 10 被 12 覆盖）。
- 设置项**不可搜索**。407 个 `settings.*` key 里找一项只能逐个分区翻。
- 「关于」页只有更新行 + changelog + 重置 + 版号，没有品牌区、没有外链入口、没有把"更新策略"收进来。

用户原话（2026-10-24）：

> 我当前在系统托盘右键，点击设置弹出的那个设置界面，我觉得做得非常 nice，非常好。反观我们看板项目的设置界面就非常草率。所以我打算以它的为骨架……包括尺寸基本 1:1 复刻，然后把咱们看板项目里的东西移进去就行了。

参照物已经逆向清楚：它的成熟感来自**一层很薄的骨架**——两个原子组件（`SettingsSection` 页签式卡片 + `SettingItem` 三档文字行）、数据驱动的侧栏、i18n 驱动的跨页搜索、四段式的关于页。

## Solution

在 Pane 里建立同形态的设置骨架，分三层：

**第一层：原子组件（与业务无关，可 1:1 照搬）**

- `SettingsSection`：页签式标题条（标题 18/600 + 描述 12 + `-mb-px` 折角）+ 内容卡（`px-5 py-5`、圆角 12）。
- `SettingItem`：左标签（14/600）+ 描述（12/400，`--muted-foreground`）+ 右控件；`py-3.5`、`border-b`、`last:border-0`；`stacked` 变体；带 `data-setting-anchor`。
- 三档文字层级固定为 18 / 14 / 12。

**第二层：外壳**

- 设置窗口改为无边框透明：外层 5px 留白 + 8px 圆角 + 淡阴影；Header 左侧标题、中间搜索、右侧 40×40 窗控；整条 Header 可拖动。
- 侧栏由 72px 纯图标改为 **200px 文字栏**（图标 18 + 标签 14/500，激活态为强调色底 + 白字 + 阴影）。
- 窗口默认 980×700、最小 860×620，可缩放、不可最大化。

**第三层：能力**

- 跨页搜索：显式设置索引（不依赖 i18n key 命名约定），命中后切分区 + 滚动到 `data-setting-anchor` + 高亮 1.6s。
- 关于页四段式：品牌（logo/名称/可点复制的版本/一句话定位）→ 动作（检查更新 + 3 个外链）→ 更新配置卡 → 生态（Star/反馈/分享/赞赏）。

**明确的约束**：不引入第二套颜色令牌。参照物的 `--qc-*` 只取语义分层，值映射到 Pane 现有 shadcn 令牌；4 套皮肤（native/signal/ink/comic-pixel）× 明暗必须继续作用于新骨架。

## User Stories

1. 作为开发者，我新增一个设置项时只需要写一个 `SettingItem`，不必再动 `index.html`。
2. 作为开发者，我新增一个分区时只改三处：导航数组、页面渲染分支、一个新 section 文件。
3. 作为用户，我能在设置搜索框里输入关键词，跨分区找到那一项，回车直接滚到它并高亮。
4. 作为用户，我能一眼看出当前在哪个分区（侧栏文字 + 激活态），而不是靠悬停猜图标。
5. 作为用户，打开设置时看到的是一个有留白和圆角的悬浮面板，而不是贴边的系统窗口。
6. 作为用户，我在「关于」页能看到版本、检查更新、更新频率策略、项目入口与支持方式，且这些都在同一页。
7. 作为使用 ink / comic-pixel 皮肤的用户，我打开设置时仍然是那套皮肤，不会突然变成原生蓝色。
8. 作为开发者，我能确认旧 sheet 的 DOM 不再参与事件绑定，只有一份设置 DOM 在跑。
9. 作为使用键盘的用户，我能用 Tab 在设置行之间移动，控件可聚焦、有可见 focus 环。
10. 作为低性能机器用户，关闭动画后侧栏与卡片没有入场动画和粒子。

## Implementation Decisions

1. **组件边界**：新增 `src/settings/` 目录（`section.ts`、`item.ts`、`shell.ts`、`search.ts`、`index.ts`），用纯函数返回 HTML 字符串 + 一次事件委托，与 Pane 现有无框架风格一致；不引入 React/Vue。
2. **令牌映射**：`SettingsSection`/`SettingItem` 只消费 Pane 现有令牌（`--card`、`--border`、`--muted-foreground`、`--primary`、`--foreground`），硬编码颜色只在"图标语义色"（Star 黄、Bug 红等）出现一次。
3. **索引而非约定**：`settingsIndex` 显式数组 `{ section, labelKey, descKey?, anchors[] }`；搜索遍历它，而不是扫 i18n 的 `*Desc`。漏项是静默的，因此新增测试断言"每个分区渲染出的 `data-setting-anchor` 都能被索引命中"。
4. **侧栏宽度**：200px 固定，`≤1024px` 窗口宽度时收窄到 170px 并隐藏描述；不隐藏文字（这是本次复刻的核心观感）。
5. **窗口**：Rust 侧 `panel` 窗口改 `decorations(false)` + `transparent(true)`，前端画 Header 窗控；保留现有 `panel-window.json` 尺寸记忆，但把默认/最小尺寸改为 980×700 / 860×620。
6. **皮肤兼容**：三个 `skins/*/skin.css` 中针对 `#panel-nav`、`.st-card`、`.setting-row` 的选择器改为同时匹配新类名（`.set-shell__nav`、`.set-card`、`.set-row`），并在每个皮肤下加一条覆盖新强调色的规则。**不删旧类名**，直到 06 票完成。
7. **迁移顺序**：先在一个低风险页（常规）落地样板，其余页照抄；密钥页最后做，因为它有 A–Z 索引、内联编辑器和多账户状态，最容易被骨架改动碰坏。
8. **淘汰策略**：06 票完成后删除 `index.html` 中旧 sheet 的 DOM 与 `setSettings()` / `#settings` 相关 CSS；在此之前新旧并存，但新骨架只渲染新 DOM，旧 DOM 保持 `display:none` 且不再新增行。

## Testing Decisions

- **单元（node --test）**：`SettingsSection` / `SettingItem` 的 HTML 结构与 `data-setting-anchor` 生成；`settingsIndex` 与渲染出的 anchor 双向覆盖；搜索匹配（中文/英文/大小写/空查询）。
- **可观察行为**：搜索命中后 `activeSection` 切换、滚动调用一次、高亮 class 在 1.6s 后被移除；`uiAnimationEnabled=false` 时不产生动画 class。
- **视觉回归**：5 个视口（1440/1280/1024/768/390 里设置窗口实际可用的 980/860 + 皮肤 × 明暗组合），至少覆盖 native 与 ink 两套皮肤，确认没有颜色穿透。
- **手工验收（owner）**：打开设置 → 逐页翻一遍 → 搜"代理"与"字"各一次 → 切 ink 皮肤再看一遍。

## Out of Scope

- 不改任何设置项的业务语义与配置 key；本 spec 只动呈现层与页面组织。
- 不引入路由/懒加载（继续保持"全部常驻 + hidden"）。
- 不做 QuickClipboard 的粒子/回弹动画，也不做其背景图主题与液态玻璃。
- 不做设置项分组折叠、不做设置项级的"重置为默认"。
- 不改 i18n key 命名空间与 key 数量（只做再分配）。

## Further Notes

- 参照物窗口是 900×630；Pane 定 980×700 的唯一原因是「密钥与额度」页有几十个 provider + A–Z 索引，900 宽会把密钥行挤成两行。这是本 spec 中唯一偏离"1:1"的地方，其余盒模型与文字层级完全照抄。
- 最大风险是皮肤：`skins/{signal,ink,comic-pixel}/skin.css` 显式覆盖了设置面板的旧类名。03 票必须先落兼容层，再动外壳。
- 参考文献：`temp/research/20261024-quickclipboard-settings-shell.md` §1（规格）、§2（原子组件）、§6（搜索）、§9（复刻建议分层）。

## Final implementation notes (2026-10-24)

- **Search index**: implemented as a rendered-row index (`[data-setting-anchor]`) in `src/settings/search.ts`, not a hand-maintained array. The original "explicit array" decision was to avoid the i18n `*Desc` convention; collecting rendered rows meets that goal and removes the silent-omission risk. A runtime coverage check plus `tests/settings-search.test.mjs` enforce two-way coverage.
- **Files**: `src/settings/{section,item,search,controls}.ts`; there is no `shell.ts`/`index.ts` — the shell still lives in `main.ts` because it owns window/IPC wiring.
- **Legacy source**: the old sheet markup moved to `src/settings/controls.ts` as an inert hidden source, adopted into the skeleton. `#settings`, `settings-open`, `setSettings`, `moveRow`/`moveToCard` and the sheet CSS are gone.
- **Skins**: legacy class names stayed because the panel elements still carry them; new `.set-*` selectors were added and the active-nav rule was lowered to class specificity so skins can own its colour. Comic-pixel's pack is not installed on this machine (its worktree is deferred), so only native/signal/ink were verified live.
- **Window calibration**: the code uses 980×700 / 860×620 Tauri logical units. On this dev box Tauri's 150% scale and WebView2's 169.5% DPR render that as ~868×620 CSS; the rail therefore uses its ≤1024px 170px width at the default size. This is a machine DPI mismatch, not a CSS bug.
