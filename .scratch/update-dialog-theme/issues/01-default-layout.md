# 01 — 默认主题布局还原与样式隔离

Type: task
Status: implemented (2026-10-25) - automated green; owner visual acceptance pending
Blocked by: None
Spec: ../spec.md
Issue: #51 — https://github.com/Aafff623/pane/issues/51

## Scope

- 对照 CC Switch v4.0.19，校准默认面板、宽度、留白、裸线条图标、标题/副标题、标签与底部按钮区。
- 保留 Pane 蓝色主按钮与当前主题变量，默认去除玻璃渐变、徽章星标、副标题缩进和底栏分隔线。
- 修复摘要列表被旧 `#whatsnew-body ul` 规则额外缩进 16px 的问题。
- 区分 update/browse/plain，避免破坏完整日志和字体引导。保留现有关闭行为。

## Acceptance

- [ ] 相近内容、相同缩放下，默认主题的比例、对齐线和留白接近参考。
- [ ] 头部、正文、底部共用横向对齐；正文滚动时按钮可访问。
- [ ] 摘要列表不再额外缩进，旧 Markdown 列表仍正常。
- [ ] 默认亮暗均检查；运行构建及直接受影响检查。

## Comments

- 2026-10-24：已规格化，尚未实施或验收。执行前领取，不覆盖主对话已有 WIP。
- 2026-10-25：已实施。不透明 --card 表面、448px/14px、24px 横内边距、16/24/600 标题、20px 裸星光、13/20 正文、中性标签、32px/6px 按钮、页脚连续；ul 特异性已修（ul:not(.wn-items) + ID 级 ul.wn-items 复位）。pnpm build 与 176 项 Node 测试通过。
