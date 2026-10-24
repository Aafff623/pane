# 02 — RTK 节省卡

Type: task
Status: implemented (2026-10-24) — native dark verified live; light + skins and owner acceptance pending
Blocked by: 01
Issue: #50

## What to build

- `src/main.ts` 新增 `lastRtkSavings` 与 `renderRtkSavings()`；在 `renderDashboard()` 中紧跟 `renderTotalSpend()`。
- 头部：`RTK 节省`、估算徽标、`?` 帮助、折叠；复用 `popoverChromeHtml()`/fold 逻辑。
- 四窗口 tabs：今天/7d/30d/全部；saved tokens（`fmtTokens`）、命令数、压缩率（1 位小数）。
- 30 天 sparkline：纯 SVG，按日补 0；不显示坐标轴，hover 给日期+saved。
- 缺失/坏数据：紧凑提示卡，文案区分未安装/不可读/正在读取。
- i18n en/zh/ru 齐平；styles.css 新增 `.rtk-savings` 局部样式，复用 provider/Total Spend 皮肤规则。
- 不进入 `lastSpend`、donut、quota、日报；不出现原始命令/路径。

## Acceptance

- [x] 卡片在 Total Spend 之后，独立折叠，不影响现有总花费。
- [x] 四窗口数字和 01 的返回一致；sparkline 30 天补 0。
- [x] RTK 缺失/坏数据是紧凑提示，不显示 0。
- [ ] 亮/暗色和 native/signal/ink 下不穿透；`pnpm build` + node 测试全绿。
      native 暗色已目检；亮色与第三方皮肤留给 owner 验收。

## Implementation notes (2026-10-24)

- 头部：名称 + 「估算」徽标 + 当前窗口总量 + 帮助；折叠状态存 `layout.rtkCollapsed`。
- 主体：四窗口 tabs、三格统计、30 天柱状图（今天那根高亮，零日只留基线）、Top 5 + 「其他项目」+「未归属」。
- 估算徽标与帮助都说明 `saved_tokens = 原输出字节 ÷ 4`，不进总花费/额度。
- 缺 RTK、坏数据、读取失败、读取中是四种提示；正常态与两态截图已存于会话记录。
