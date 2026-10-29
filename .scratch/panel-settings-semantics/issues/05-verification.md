# 05 · 验证、复测与交付证据

Status: done 2026-10-29 | Priority: P0 | Depends: 01, 02, 03, 04

## Scope

- `pnpm build` 通过；根目录现有测试不回归。
- 重建 demo（`cd site && npm run build:demo && npm run build`），`npm test` 7/7。
- CDP 截图：悬浮板暗/亮 440、设置 10 页暗、常规/快捷键/授权中心亮、搜索；保存到 `temp/logs/panel-semantics-20261029/`。
- `impeccable detect --cdp` 复测两套界面，记录剩余发现与误报。
- 真实窗口冒烟：Alt+2、打开设置、改语言/字号、Ctrl+S 重载、删除确认（不真正删除）。
- 代码 review 一遍（冗余、可访问性、回归面），落在 `review.md`。

## Done when

- 所有证据落盘并在交付说明里列出；未修的项明确标注原因。
- 不 push、不部署站点；线上仍是旧站。
