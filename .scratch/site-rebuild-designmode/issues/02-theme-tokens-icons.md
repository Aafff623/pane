# 02 · 设计令牌与主题（亮/暗）

Status: done 2026-10-29 | Priority: P0 | Depends: 01

## Scope

- 保留上游 `globals.css` 的 shadcn/Tailwind token 结构；补一套 `.dark` 变量（背景/卡片/文本/边框/强调），对比度按旧站标准（正文 ≥ 4.5:1）。
- 主题切换：`html.dark` + `localStorage('pane-site-theme')`，首屏内联脚本防闪烁；切换按钮在 Navbar。
- 字体：Inter 变量（next/font）用于拉丁，CJK 走系统栈（PingFang / Microsoft YaHei / Noto Sans SC）；中文标题不额外加载字体包。
- 图标基线：安装 `@phosphor-icons/react`，统一 `weight="regular"` 与 16/20/24 三档；写一条使用规则到站点 README。
- 移除上游 lucide-react 依赖；所有引用替换为 Phosphor 对应图标。

## Done when

- 亮/暗两套主题在 1440/768/390 下无不可读文本。
- `grep -r lucide-react site/src` 为空。
- `prefers-reduced-motion` 下无动画依赖。
