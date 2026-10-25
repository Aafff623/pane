# 修复：皮肤专属 DOM 泄露到原生（无皮肤）界面

**Status:** implemented (2026-10-25) - owner UI acceptance pending
**Type:** bug
**GitHub:** 待提（正文见 `issue-body.md`）
**Feature dir:** `.scratch/skin-native-leak/`
**Date:** 2026-10-25
**Owner decisions:** 首选方案 B（应用 CSS 补基线隐藏）；JS 条件渲染仅作可选加固且默认不做（会破坏跨窗口切皮肤的免重渲染路径）；demo 重同步单独一票；UI 验收由 owner 亲自执行。

## 1. 用户需求

Owner 的同学全新安装 v0.9.1（build 1024-1214）后，看板左上角出现「云端邮局」字样。同学未选任何皮肤，默认应呈现原生亮/暗主题。诊断结论：这不是皮肤被误启用，而是皮肤专属 DOM 在原生模式下泄露。修复目标：全新安装（零皮肤包）的原生界面不得出现任何皮肤品牌元素。

## 2. 现场与证据

- 泄露元素：`src/main.ts:11727`（`renderBoard`，原 `renderAll`）每次渲染**无条件**注入
  `<div class="post-banner"><strong>${escapeHtml(t("skin.signal"))}</strong><span class="post-mascot" aria-hidden="true"></span></div>`。
- 「云端邮局」= `src/i18n.ts:1693` 的 `"skin.signal"`，即 signal（信号邮局）皮肤显示名。
- 引入提交：`ee82bd9`（2026-10-11，`feat(skins): launch Cloud Post Office outside experimental features`）把该 div 放进无条件渲染路径。
- 隐藏规则只存在于皮肤包：`skins/signal/skin.css:180`（`.post-icon, .post-banner, .skin-period { display:none }`）与 `:root[data-skin="signal"] .post-banner { display:flex }`（skin.css:166/191）。应用自身 `src/styles.css` 对 `.post-banner` **零命中**（grep 实证）。
- 包 CSS 注入机制：`src/skins.ts` 的 `refreshInstalledPacks()` 只注入**已安装**包的 CSS（skins.ts:88-101）。全新安装零包 → 没有任何 CSS 隐藏该 div → 裸文本露出。
- 开发机被掩盖的原因：机器上安装过 signal 包后，其 CSS（含 skin.css:180 这条**不带 `data-skin` 门控**的基线隐藏）即使皮肤未启用也会注入，把 banner 藏住；owner 与测试机因此看不到。
- 同类泄露：`src/main.ts:2628` 的 `.skin-period`（承载 `fmtDuration(m.period_ms)` 可见文本），隐藏机制同上，新装机可能出现重复时长文字（不如标题显眼，随本票一并修）。
- 皮肤确实未启用：`activeSkin()` 对空 config/localStorage 返回 undefined（skins.ts:110-120、main.ts:1419-1426），`applySkin()` 删除 `data-skin` 属性。截图中的手写体是应用默认品牌字体，非皮肤效果。
- 复现环境：v0.9.1 build 1024-1214 全新安装（同学机器，截图由 owner 提供）。该 bug 自 10-11 起随各版本发布，影响所有全新安装用户。

## 3. 修复方案

### 首选（B）：应用 CSS 补基线隐藏

`src/styles.css` 基线区新增一行：

```css
.post-banner, .post-icon, .skin-period, .post-mascot { display: none; }
```

- 与 skin.css:180 对齐，把基线隐藏从「包内」搬到「应用内」，零包机器也有兜底。
- 特异性已核实：signal 的 `:root[data-skin="signal"] .post-banner`（skin.css:166/191）压过类选择器，皮肤启用后显示不变。
- 保留皮肤系统「双表示 + CSS 切换」设计（skin.css:178-179 注释即此意图）：跨窗口 storage 事件切皮肤免重渲染（main.ts:1488-1494 的监听器不调 `requestRender()`）。

### 不推荐（A）：JS 条件渲染 banner

在 main.ts:11727 仅在 signal 启用时插入。会破坏跨窗口切换的即时性，且只修 banner 不修 `.skin-period` 家族。如后续想消灭死 DOM，可在 B 之上叠加，本规格不要求。

## 4. 实施范围

- **票 01**：`src/styles.css` 一行基线隐藏 + `pnpm build` 验证。
- **票 02**：`node site/scripts/build-demo.mjs` 重同步 bundled demo（修复后 demo 原生模式同款泄露随之消失）。

## 5. 验收标准（owner 亲自 UI 验收）

- [ ] 全新安装等效环境（清 `%LOCALAPPDATA%\com.jazii.pane\EBWebView` + 零皮肤包）原生亮/暗：左上角无「云端邮局」、无重复时长文本。
- [ ] signal 已安装但未启用：banner 隐藏。
- [ ] signal 启用：banner 与修复前渲染一致（邮戳条 + 吉祥物），无回归。
- [ ] `pnpm build` 通过；`tests/` 无 `.post-banner` 引用（grep 实证），相关断言不受影响。
- [ ] 票 02 后 demo 站点原生模式无泄露。

## 6. 约束

- 主对话在同一工作区并行，未提交改动集中在 `src-tauri/src/providers/`、`docs/`、`.github/`；本票只碰 `src/styles.css`，零冲突。实施前仍按惯例 `git status` 复核 WIP。
- 不 commit / 不 push（Git 副作用按 AGENTS.md 等 owner 授权）；分支随当前 `codex/rtk-orcaterm-handoff` 或新开 `codex/*` 由主对话定，验证后由 owner 拍板合入 main。
- 不改任何 `skins/*/skin.css`：泄露修复不依赖重打包皮肤。
- 工作区 13 个参考克隆已于 2026-10-25 全部同步（cockpit-tools 到 v1.3.74），与本任务无关。
