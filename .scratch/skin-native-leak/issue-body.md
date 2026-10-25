# 全新安装原生界面左上角显示皮肤品牌「云端邮局」

> 官方仓库 issue 稿（未提交）。正文可直接粘贴到 GitHub New issue。权威规格：`.scratch/skin-native-leak/spec.md`。

## 现象

全新安装 v0.9.1（build 1024-1214）后，未选择任何皮肤，默认原生界面（亮/暗）看板左上角出现「云端邮局」字样。选择任意皮肤或安装过皮肤包的机器上不出现。

## 复现

1. 全新安装 v0.9.1（或清空 `%LOCALAPPDATA%\com.jazii.pane\` 下的皮肤包数据 + 清 WebView2 缓存）。
2. 不选任何皮肤，直接打开看板。
3. 左上角即显示「云端邮局」。

## 根因

- 「云端邮局」是 signal（信号邮局）皮肤的显示名（`src/i18n.ts` 的 `skin.signal`）。
- `src/main.ts` 渲染看板时**无条件**注入皮肤专属的 `post-banner` div（文字即 `t("skin.signal")`），引入于 2026-10-11「云端邮局皮肤转正」提交。
- 隐藏该元素的 CSS 只写在皮肤包 `skins/signal/skin.css` 内；应用自身 `src/styles.css` 没有对应规则。皮肤包 CSS 仅在机器上**安装过**皮肤包时才会注入，因此全新安装（零皮肤包）机器上没有任何规则隐藏它，裸文本直接露出。
- 同类机制下 `.skin-period`（周期时长文本）也可能在新装机重复显示。

## 修复方向

在应用 `src/styles.css` 补一行基线隐藏 `.post-banner, .post-icon, .skin-period, .post-mascot { display: none; }`，由 signal 皮肤的 `[data-skin="signal"]` 规则覆盖回来。不动 `main.ts`、不重打包皮肤。

## 验收

- 零皮肤包原生亮/暗：无「云端邮局」、无重复时长文本。
- signal 启用后 banner 渲染无回归。
