Status: done (2026-10-26) — owner chose 4 groups (常规/数据/网络隐私/关于); tree, aria-expanded, localStorage, arrow keys.
Type: task
Blocked by: 06

# 07 · 侧栏二级菜单（设置 → 子页丝滑展开）

## 做什么

1. 信息架构先行：盘点 Pane 现有设置分区，提出分组方案（常规/外观/花费/更新/关于…）交 owner 拍板。
2. 设置侧栏（#49 骨架的 200px 文字栏）加树形子菜单：父项 chevron + 子项缩进；展开动画 grid-template-rows 0fr→1fr + 子项 --i stagger（06-3 的机制复用）。
3. aria-expanded + localStorage 记忆展开态；键盘可达（Arrow 在子项间移动）。
4. 看板侧栏若分区过多，同样收纳（owner 决定是否本轮做）。

## 验收

- 展开收起无跳变（无 height 测量）；刷新后记忆；键盘可走完。
- owner 视觉验收（对照 magpie 截图的丝滑感）。

## 证据

temp/research/20261025-magpie-ui-mechanics.md §二（app.css:691-696、app.js:24312-24314 子行导航）。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
