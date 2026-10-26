Status: done (2026-10-26) — pane logo hub, CSS/SVG only (owner chose zero dependency), active-only breathing.
Type: task
Blocked by: 02

# 03 · 吉祥物枢纽与主题动效（pane-mark + 皮肤绑定 + oil-ui-pro 打磨）

## 做什么

1. hub 节点：pane-mark.svg（唯一品牌主形，勿手改派生 PNG）+ 副标「本地只读 · 不代理」；底色/描边走 CSS 变量，三皮肤+明暗自动生效。
2. 动效（替换喜鹊飞行）：首选零依赖 CSS/SVG——hub 呼吸/漂浮（transform 微动）+ 活跃时能量脉冲与流动线衔接；**Lottie/lottie-web 为可选，引依赖前必须 owner 拍板**（Pane 零运行时依赖哲学）。
3. 用 oil-ui-pro 技能做动效打磨（用户点名）；打磨产出（缓动/时长/层次）回写本票备注。
4. 可选后置：沿 path 运动的光点（circle.pkt + pose 按路径百分比，需配合 measure 缓存）。

## 验收

- 三皮肤+明暗下 hub 观感正确（对比截图）；动效克制不喧宾（reduced-motion 降静态）。
- owner 视觉验收：动效形式最终拍板（CSS/Lottie 二选一）。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
