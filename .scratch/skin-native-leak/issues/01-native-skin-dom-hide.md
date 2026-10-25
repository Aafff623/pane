# 01 — styles.css 补基线隐藏，修复皮肤 DOM 原生泄露

Status: implemented (2026-10-25) - automated green; owner UI acceptance pending
Type: bug
Blocked by: —

## Scope

在 `src/styles.css` 基线区（与其他元素基线规则同段）新增一行：

```css
.post-banner, .post-icon, .skin-period, .post-mascot { display: none; }
```

- 只此一行；不改 `src/main.ts`，不动任何 `skins/*/skin.css`。
- 特异性已核实：signal 的 `:root[data-skin="signal"] .post-banner { display:flex }`（skin.css:166/191）压过类选择器，皮肤启用后显示不受影响。
- 根因与完整证据链见 spec 第 2 节：`src/main.ts:11727` 无条件注入 `post-banner`（文字 = `t("skin.signal")` =「云端邮局」，引入于 `ee82bd9`）；隐藏规则只存在于皮肤包 `skins/signal/skin.css:180/191`；`src/skins.ts` 的 `refreshInstalledPacks()` 仅注入已安装包的 CSS，零包机器无任何规则隐藏它。
- 同族泄露 `.skin-period`（`src/main.ts:2628`，可见时长文本）由同一行一并修复，无需单独改动。
- **不要**改成 JS 条件渲染：跨窗口 storage 事件切皮肤的免重渲染路径（main.ts:1488-1494）依赖 DOM 常驻 + CSS 切换。

## Acceptance

- [ ] `pnpm build` 通过。
- [ ] owner 验收（spec 第 5 节）：零皮肤包原生亮/暗无「云端邮局」、无重复时长；signal 安装未启用时隐藏；signal 启用时 banner 渲染与修复前一致。
- [ ] 不 commit、不 push；改动范围如实记录。
- 2026-10-25：已实现。基线隐藏四类皮肤元素（.post-banner/.post-icon/.skin-period/.post-mascot）落在主题变量块后；main.ts 零改动（保留 storage 事件免重渲染路径）。pnpm build 与 Node 180/180 通过；tests/ 无 .post-banner 引用（grep 实证）。
