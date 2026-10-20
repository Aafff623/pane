# 02 — panel 外壳：72px 设置目录导轨

**Blocked by:** 01
**Status:** done (2026-10-20) — 7 页 + 授权中心全部程序化切换验证通过，页头标题/说明、aria-current、保存按钮显隐、绿点同步均正确；截图待 owner 验收。

把 `buildPanelShell` 的 56px 双入口导轨 + 横向 `.panel-tabs` 换成 cc-switch 式设置目录：

- `#panel-nav` 72px：logo → 7 个页面图标（tooltip / active / focus）→ 底部授权中心（分隔线 + 账号图标）。
- `PANEL_PAGES` 提升为模块级，带页面说明 i18n key；`setPanelView` 同步页头标题/说明和保存按钮显隐。
- 页头 56px，标题 18/600 + 右侧说明文字；保存按钮改为白底黑字主按钮，授权中心隐藏。
- 内容列 940px；`#panel-view-auth` 同样限宽。
- About 图标在 `updateVersion` 存在时显示绿点。
- `panelIntent` 的 `.panel-tabs ... .click()` 选择器改到导轨按钮。

**验收**
- [ ] 8 个入口都能切到正确视图；tooltip 可见；键盘可聚焦
- [ ] 悬浮设置页的控件在 panel 内仍可用（密钥、快捷键、字体）
- [ ] panelIntent 两条路径不回退
- [ ] popover 外观无变化（同一 bundle）
