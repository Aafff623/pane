# 01 — 6 个静态 select 换 Magpie 式 pill 控件

Status: implemented (2026-10-25) - automated green; owner visual acceptance pending
Type: task
Blocked by: —

## Scope

把 6 个静态选项 select 换成 pill 分段控件：`#locale`（controls.ts:10-15）、`#appearance`（38-42）、`#timeformat`（30-34）、`#jump-animation`（73-76）、`#spend-tier-mode`（83-86）、`#update-frequency`（main.ts:18295-18308）。

- 复用 `.tabs/.tab` 视觉体系（styles.css:2651-2678）：新样式以 `.set-row__control .tabs, .set-row__control .tab` 覆盖形式写在骨架段（10192 之后），三个包的 `.tab` 规则自动继承，不新造类名。具体值见 spec 第 5 节 A。
- 接线：隐藏 `<select>` 作唯一真值源，pill 点击设 `select.value` 并派发 `change`。现有 6 条监听器、`setSelect()` 回填（main.ts:17563-17578）、`#spend-tier-mode` 的 `paintTierInputs()` 联动（17150）不动。
- `#jump-animation` 保持延迟保存语义（pendingJumpAnimation + setSettingsDirty，main.ts:17345-17351），pill 不直接 patchConfig。
- aria：容器 `role="group"` 接管原 select 的 aria-label（main.ts:17897-17902 的 adoptControl 逻辑），每个选项 `aria-pressed`。
- 滑动 thumb 本票不做（隐藏页 offsetWidth=0 陷阱，main.ts:17728-17738）；用 `.tab` 现成背景切换。第二阶段另立。
- `#pinned` 排除并在代码注释写清原因（选项数运行时动态 + 语言切换重建，main.ts:14628-14647,17002）。`.st-radio-group` 留 owner 定，本票不动。

## Acceptance

- [ ] 6 处全部换成 pill；点击/联动/语言切换/回填零回归；行为与改造前完全一致。
- [ ] 皮肤：三个包的 `.tab` 样式自然继承；comic-pixel 暗色下 thumb 不消失（复用 `.tab` 的紫底 active 救回）。
- [ ] signal 轨道露馅现象记录在案（票 04 处理），不在本票临时打补丁。
- [ ] 键盘可达：选项可聚焦、可操作。
- [ ] `pnpm build` 与相关 Node 测试通过；`tests/settings-primitives.test.mjs` 现有断言不破（保留隐藏 select）。
- [ ] 不提交、不推送；改动范围如实记录。
- 2026-10-25：已实现。pill 复用 .tabs/.tab（皮肤包自动接管），隐藏 select 保留 id/监听器为唯一真值源；点击只设 select.value 并派发 change——jump-animation 延迟保存、spend-tier 联动、locale 重译（分段按钮带 data-i18n）全部走原路径；setSelect 回填后同步 pill；#pinned 按规格排除（代码注释写明原因）。新增 tests/settings-pill.test.mjs 4 条回归（vm 提取真实函数）。capture-pane3.ps1 可用于验收截图。
