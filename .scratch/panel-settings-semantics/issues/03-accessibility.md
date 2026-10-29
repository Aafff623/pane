# 03 · 对比度、字号与键盘可达

Status: done 2026-10-29 | Priority: P1 | Depends: 01, 02

## Scope

- 对比度：搜索占位符、`#splash-replay`、`.ovbar-pct.is-maxed`、`.overview-subgroup-count` 等被检测器点名的元素达到 4.5:1（暗/亮两套）。
- 字号：悬浮板里 10px / 10.5px 的功能文字（图表图例、列头、徽章、账号页签）提到 11px；检查窄宽下不溢出。
- 标题镜像：`title` 元素自动补 `aria-label`（只在缺失时补），状态点与图标按钮可被读屏读到。
- 概览项 / 到期行：`[data-jump-provider]` 增加 `role="button"`、`tabindex="0"` 与 Enter/Space 激活；补 `:focus-visible` 样式。

## Done when

- `impeccable detect --cdp` 复测：settings 与 panel 的低对比、undersized 警告清零或只剩已解释的误报。
- 键盘 Tab 能聚焦额度总览项并打开。
- 构建通过。
