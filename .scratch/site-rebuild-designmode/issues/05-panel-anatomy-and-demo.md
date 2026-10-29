# 05 · 面板解剖 + 在线演示

Status: done 2026-10-29 | Priority: P1 | Depends: 04

## Scope

- 沿用上游 `PanelAnatomy` 的 sticky 走查结构（左侧序号 + 折叠项 + 右侧舞台 + 注释点），把内容换成 Pane 悬浮面板的 6 个部位：总览环 / 周期切换 / 多账号徽章 / 花费与 Token / 设置与保险箱 / 快捷键。
- 舞台使用真实 Pane 截图（亮/暗各一套），注释点用绝对定位 + 连接线，必须随断点重算；截图加载失败时降级为纯说明文本，不留空白框。
- `#demo`：嵌入 `site/public/demo/` 的真实前端 iframe，懒加载；容器沿用上游 8/5 视觉；提供「新窗口打开演示」链接与重置按钮。
- iframe 必须能加载：Worker 对 `/demo/*` 不得再输出 `X-Frame-Options: DENY`（改为 `frame-ancestors 'self'` 或对 `/demo/` 跳过）。

## Done when

- 桌面 1440 / 平板 768 / 手机 390 下，舞台与注释不重叠、不溢出。
- `/demo/` 在线上与本地都能渲染出真实 Pane 面板。
