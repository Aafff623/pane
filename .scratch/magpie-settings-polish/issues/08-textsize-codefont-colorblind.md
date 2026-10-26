Status: done (2026-10-26) — 100/110/125/150 pill, code font+weight+preview, colorblind patterns; cross-window event.
Type: task
Blocked by: 06

# 08 · 文字大小四档 pill + 代码字体 + 色盲模式

## 做什么

1. 文字大小 100/110/125/150% 四档 pill：根字号 CSS 变量 + 从 textScaleFactor（main.ts:8933）拆出用户偏好因子；config.json 持久化；与 popover 自适应缩放相乘不冲突（需单测边界）。
2. 代码字体：双下拉（字体+字重）+ 预览行「Aa 0123 ·中文」；复用 #ui-font 字体枚举与 datalist 模式（main.ts:18022-18028）；新增等宽字体变量供卡片数字/耗时区。
3. 色盲模式：pill 关/开；曲线色盲态换 dash 图例不换色相（magpie app.js:16478 最小实现）；与 meter-display-honesty#04 燃烧曲线预留位联动。
4. 三语 i18n（settings.* key/Desc 成对约定）。

## 验收

- 四档即时生效、重启保持；popover 缩放边界单测绿。
- 色盲开→曲线/徽章可辨；关→原样。
- `pnpm build` 绿 + i18n parity；owner 视觉验收（对照 magpie 常规页截图）。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
