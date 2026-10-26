# 设置面板细节微调：参照 Magpie 的精髓点

**Status:** implemented 01-04 (2026-10-25) - 05 screenshots + owner acceptance pending; **第二轮扩展（06-09）见 `spec-round2.md`（2026-10-25 用户追加）**
**Type:** polish
**GitHub:** [#58](https://github.com/Aafff623/pane/issues/58)（正文 `issue-body.md` + 第二轮扩展评论区）
**Feature dir:** `.scratch/magpie-settings-polish/`
**Date:** 2026-10-25（第二轮追加同日晚）
**Owner decisions:** 抄精髓不大改；范围锁定 A+B+C+D；pill 复用 `.tabs/.tab`；布尔保留现有 switch；皮肤包改动是否重新打包由 owner 在第 4 票前单独授权。**第二轮追加**：侧栏二级菜单、文字大小/代码字体/色盲、pill 推广、全局丝滑基座（见 spec-round2.md）。

## 1. 用户需求

Pane 设置面板骨架重构已完成（QuickClipboard 形态，8 票已实现未验收），owner 认为骨架抄到了精髓，但「有些地方显示的不是很好」。以 Magpie（yetone/magpie）设置界面为参照，做细节微调：抄其精炼的精髓点，不做大改。

本文件是实施范围与验收标准的权威规格。调研底稿 `temp/research/20261025-magpie-settings-ui-analysis.md`、深度 diff 看板 `temp/ui-diff-20261025/index.html`（含 10 张双栏原型图与源码速查表）均在维护者本地；毒舌评审 8 条判决与 11 项不背锅区已并入本规格第 4 节。

## 2. 现场与证据

- 参照物：`yetone/magpie` @ `defbb1a`（`temp/magpie-yetone/`），设置 UI 在 `internal/gui/assets/index.html` 的 `#view-settings`（135-404 行）+ `app.css` + `app.js`。
- Magpie 原型图：headless Edge 按真实 `app.css` 静态渲染（`temp/magpie-shots/`，控件由 shim 示意填充；usage 页暖机卡片无后端为空；crumb 误显 Work/Agents 为渲染副产物）+ 官方截图（`site/public/img/picker-light.png`、`docs/screenshots/font-settings.png`）。
- Pane 原型图：`temp/panel-cdp.png`（10-24，新骨架，Signal 皮肤暗，隐私页）+ `temp/panel-readme-final/`（10-21，骨架重构前旧版，仅对照）。**native 默认主题新截图缺失**，A/B/C 在 native 亮/暗下的真实渲染目前只能代码补证。
- Pane 代码现场：骨架组件 `src/settings/{section,item,search,controls}.ts`；样式 `src/styles.css` 10192-10694 段（骨架+外壳）；既有分段控件 `.tabs/.tab` 在 2651-2678 行；装配逻辑 `src/main.ts` 的 `openSettingsPanel`（约 20025 行）附近。
- 皮肤现场：三个 pack 源 `skins/{signal,ink,comic-pixel}/skin.css`；运行时 CSS 从远端 zip 注入（`src/skins.ts` 78-101）；`site/public/skins/index.json` generatedAt 2026-10-23，与仓库源版本号核对一致（均 1.0.0）。
- 工作分支 `codex/rtk-orcaterm-handoff`，主对话同工作区工作；本票涉及 `src/main.ts` 与 `src/styles.css`，实施前重新核对共享 WIP，不切分支或覆盖别人的改动。

## 3. 参照物精髓（Magpie 的做法，源码取证）

1. **分段 pill 控件**：灰轨道（`--pill` 底、2px 内衬、7px 圆角）+ 白色 thumb（`0 1px 2px` 阴影 + `.5px` 环）滑到选中项；选项 22px 高 / 12px 字；未选 muted、选中 `--fg`。设置页零 toggle switch。thumb 位置按控件在页面中的位置记忆，重绘不乱滑（`app.js:6648-6707`、`app.css:2957-2978`）。
2. **行结构**：名称 13px/500 + 副标题 11.5px muted 默认单行省略、个别行特例换行；行高 window 48px / panel 40px；名称与控件 gap 9-12px（`app.css:562-576`）。
3. **分隔线两档**：卡片边框 `--line`，行间 `--line-2`（浅一档），卡看起来是一张卡（`app.css:16-17,562-566`）。
4. **小节标签**：11px/600 大写 + `.05em` 字距，muted；卡片间距 8px（`.label`、`app.css:1971`）。
5. **危险按钮 hover 才红**（`.text.danger:hover`，`app.css:1986`）；侧栏 hover 只有底色无位移（`app.css:386`）。
6. **行内搜索**（`app.js:19282-19339`）：本范围不做（见第 7 节范围 E）。

## 4. Pane 现状与判决（毒舌评审，file:line 取证）

1. **【高】候选名单纠错**：`#pinned`（托盘图标显示）选项数 = 1+N×M 运行时生成且 `applyLocale` 重建（`main.ts:14628-14647,17002`），换 pill 会得到 20 项的分段控件。**A 锁定 6 个静态 select**：`#locale`（4 项，controls.ts:10-15）、`#appearance`（3 项，38-42）、`#timeformat`（3 项，30-34）、`#jump-animation`（2 项，73-76）、`#spend-tier-mode`（2 项，83-86）、`#update-frequency`（4 项，main.ts:18295-18308）。
2. **【高】保存语义不对称**：`#jump-animation` 是唯一写 `pendingJumpAnimation` + `setSettingsDirty(true)` 的设置（`main.ts:17345-17351`，`applySettingsAndReload` 才落盘 894-902）；其余 5 个 change 即 `patchConfig`（timeformat 17094-17096、locale 17100-17105、appearance 17241-17243、spend-tier-mode 17161-17164、update-frequency 18310-18315）。「Pane 是显式保存模式」的旧判断不准确，实际保存按钮 = 强制 reload + 应用延迟项。
3. **【高】pill 必须复用 `.tabs/.tab`**：Pane 已有同形态分段控件（`styles.css:2651-2678`，5 处在用）。三个包只样式化既有类名：signal 点名 `.tab`（skin.css:145,147）；ink 处理 `.tab`（39-43，active 背景被抹掉、只剩笔触下划线）；comic-pixel 含 `.tabs` 容器（123-132，全套像素风）。新造 `.set-segs` 类 = 皮肤失明。thumb 用 `var(--card)` 不能用 `--background`（亮色下与轨道只差约 7 个色阶，thumb 几乎消失）。signal 的 `.tabs` 容器不在 pack 列表里，轨道实色会露馅。
4. **【高】C 特异性陷阱**：`.set-section__card > .setting-row`（0,2,0）压过 `.set-row`（0,1,0）；快捷键行（`setting-row shortcut-row`，main.ts:6857）和 kv-add-row（controls.ts:116）是双类行；`.backup-title`（10305）同档。三个包已各自实现两档线 → C 本质是 native 补课。
5. **【中】B 已落地但双写**：`panelSwitchRow` 同时写 `toggle.title` 与行内 desc（`main.ts:17616-17624`）；`adoptControl` 没删输入框自带 title（`main.ts:17888-17905`，controls.ts:46/50）。旧 `.setting-row` 被 remove 时其 title 已随之消失，panel 形态活下来的只有输入框两处。
6. **【中】B 之后行高暴涨**：desc 12px 全换行（`overflow-wrap:anywhere`），实拍单行约 70px vs Magpie 约 40px。
7. **【中】D「hover 才红」三个包失效**：pack 里恒红规则特异性压顶（signal skin.css:116 (1,2,1)、ink:46 (1,2,1)、comic-pixel:175 (1,3,0)）；ink 不覆盖 `.mini-btn.danger` → D 在 ink 的 mini-btn 上有效。附带既有 bug：comic-pixel 重置按钮圆角分裂（170 输给 ID 规则、175 行又赢）。
8. **【低】侧栏去位移唯一全皮肤有效**（无人重发 transform）；native 无截图，A/B/C 在 native 上是盲飞。

**不背锅区（查过不动）**：保存按钮不对称属既定行为（接线时知道即可）；侧栏激活态 primary 实底是 spec 既定用户故事；28px 分区间距（820px 单列 vs Magpie 多列，密度不可比）；label 14/600 骨架级；搜索交互（范围 E）；布尔保留 switch（owner 决定）；keys 页「?」按钮是帮助入口非 title；快捷键图标按钮 title 是正当用法；comic-pixel 圆角 bug 既有；`.st-radio-group`（窗口形态二选）留 owner 明确；测试未运行（静态阅读）。

## 5. 实施范围定义

### A — 6 个静态 select 换 pill 控件（票 01）

- **形态**：复用 `.tabs/.tab` 视觉体系。新样式写在 `styles.css` 骨架段（10192 之后），选择器为覆盖形式 `.set-row__control .tabs, .set-row__control .tab { ... }`，让三个包的 `.tab` 规则自动继承。
- **值**：容器 `display:inline-flex; padding:2px; border-radius:7px; background:var(--muted); gap:2px; flex:none; margin:0`（覆盖 `.tabs` 的 `margin-bottom:4px`）；选项 `height:22px; padding:0 10px; border-radius:5px; font-size:12px; white-space:nowrap; flex:0 0 auto`（覆盖 `.tab` 的 `flex:1`，避免长选项把 pill 撑宽）；active `color:var(--foreground); background:var(--card); box-shadow:0 1px 2px rgb(0 0 0/.12), 0 0 0 .5px rgb(0 0 0/.06)`；未选 `color:var(--muted-foreground)`；hover 未选 `color:var(--foreground)`。
- **接线**：隐藏 `<select>` 作唯一真值源（`hidden` 但保留 DOM 与 id），pill 点击设 `select.value` 并派发 `change`。6 条现有监听器、`setSelect()` 回填（`main.ts:17563-17578`）、`#spend-tier-mode` 的 `paintTierInputs()` 联动全部不动。`#jump-animation` 的延迟保存语义原样保留（不 patchConfig）。
- **aria**：`adoptControl` 的 `aria-label` 从 select 移到 pill 容器（`role="group"`），每个选项 `aria-pressed`（对齐 Magpie app.js:6661）。
- **滑动 thumb 第一版不做**：`activatePanelPage` 用 `block.hidden` 切页（`main.ts:17728-17738`），非当前页 `offsetWidth=0`，真滑动需在切页时重新 measure（Magpie 解法 ResizeObserver，app.js:8954）；第一版用 `.tab` 现成的背景切换，滑动 thumb 列第二阶段。
- **排除**：`#pinned`（动态选项，代码注释写清原因防补全）；`.st-radio-group` 待 owner 定。

### B — 说明去重 + 副标题两行封顶（票 02）

- 删双写：`panelSwitchRow` 去掉 `toggle.title`/`dataset.i18nTitle`（保留 desc）；`adoptControl` 对 `#ui-font`/`#ui-font-cjk` `removeAttribute("title")` + `delete dataset.i18nTitle`；controls.ts 源同步（popover 形态同样受益）。
- 保留 title 的正当情形：图标按钮（快捷键重置/清除，main.ts:6860/6862）。
- 行高：`.set-row__desc` 的 `overflow-wrap:anywhere` 改 `break-word` + 两行 clamp（`-webkit-line-clamp:2`）；label 侧（10261）同理。中文副标题 25-40 字在 820px 列宽两行内可容。

### C — 两档分隔线（票 03）

- `:root`（60-92）与 `:root[data-theme="light"]`（96-117）各加 `--row-line`：dark `color-mix(in srgb, var(--border) 55%, var(--card))`；light `color-mix(in srgb, var(--border) 60%, var(--card))`。语义同 Magpie：行线对比度 < 卡边框。
- 四处规则统一引用：`.set-row`（10244）、`.set-section__card > .setting-row`（10319）、`.backup-title`（10305）；两条 `:last-child` 规则保持 0。

### D — 微调（票 03，native 部分）

- `#reset-all-settings`：静态回落中性（`border-color:var(--border); color:var(--secondary-foreground); background:transparent`），hover 才 `color:var(--destructive)` + 12% 红底（值搬自 7657-7658/7661-7662）；`.mini-btn.danger`（4384-4387）同理。
- 侧栏 hover 删 `transform: translateX(2px)`（10402），transition 去掉 transform（10384）。

### 皮肤包（票 04，需 owner 授权打包）

- signal：settings-skeleton 段（279-293 有先例）给 `.set-row__control .tabs` 补轨道皮肤（`var(--signal-inset)` + post-stitch 虚线边）。
- 三个包的恒红 reset 按钮（signal:116、ink:46、comic-pixel:175）：owner 二选一——接受恒红并写入验收标准，或同步改 pack 为 hover 版本。comic-pixel 若改，顺手补 `#reset-all-settings` 的 2px 圆角分裂。
- 改 pack = 重新打包 zip + 更新 `site/public/skins/index.json`；打包与上传/发布严格区分，发布授权另走门禁。

## 6. 皮肤与兼容

四个皮肤（native/signal/ink/comic-pixel）× 明暗必须继续工作。pill 复用 `.tab` 后：signal 拿到纸质边框+不等圆角+硬阴影、active 米色底；ink active 背景被有意抹掉只剩笔触下划线（不算露馅，但别宣称「四皮肤一致」）；comic-pixel 全套像素风零成本。thumb 在 comic-pixel 暗色下若用新类+`--card` 会消失，只有复用 `.tab` 才能被 132 行紫底救回。C 的三个包已两档 → native 补课。D 的 hover 红三个包失效（见 4-7）。验收前补 4 张 native 截图（常规/隐私 × 亮/暗）。

## 7. 非目标与边界

不重做外壳/侧栏激活态/分区 tab 头；不改保存语义与任何设置的持久化行为；不做搜索交互重构（范围 E：Magpie 式行内过滤——Pane 现有跨页索引+双向覆盖校验更防呆，不动）；布尔保留 switch（不换 Off/On）；不引入 UI 框架；不动与设置面板无关的代码。`.st-radio-group`（窗口形态二选）是否顺手换 pill 留 owner 明确，默认不动。

## 8. 验收标准

- [ ] 6 个 select 换 pill 后：观感对齐 Magpie 原型图；点击、联动（spend-tier）、语言切换、jump-animation 延迟保存、setSelect 回填零回归；aria 达标。
- [ ] 说明无双写；副标题两行封顶行高回落；无关键信息被截断。
- [ ] `--row-line` 两档线在 4 皮肤 × 亮暗可见不突兀；native 亮/暗新截图核对。
- [ ] 危险按钮/侧栏 hover 按预期；键盘可达。
- [ ] 皮肤包改动（若授权）：zip + index.json 同步；未授权则 signal 轨道露馅与三个包恒红写进遗留说明。
- [ ] `pnpm build`、相关 Node 测试（`tests/settings-primitives.test.mjs`、`settings-search.test.mjs` 等）、代码审查与冗余扫描完成并如实记录。
- [ ] 用户亲自 UI 验收；未验收前不宣称完成，不提交、推送、打 tag、上传或发布产物。
