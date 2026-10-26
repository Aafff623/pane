Status: ready
Type: task
Blocked by: —

# 04 · helpTip 问号悬浮组件（与票 01 合并验收）

## 做什么

1. 新组件 `helpTip({title, body})`：13px 问号 icon（--muted-foreground、hover --accent）放 18px 圆形命中区（-my-0.5 不占布局）；返回 span。
2. 弹层：复用 placeMenuAt 定位（gap 8、prefer below、防溢出已有）；`.help-card`（max-width 300px、标题 font-semibold + 正文 caption、全 CSS 变量 → 三皮肤自动适配）。
3. 交互：hover 300ms 开 / 移开 200ms 关（移进卡片不关）；focus-visible 立即开（键盘可达）；点击固定 pin；Escape/点外关。
4. 替换现有 5 处 title 式 ？（main.ts:5258/6054/11206/11552/16272）+ overview-peak-help（styles.css:8048）；#hover-tip 加 data-rich 变体承载标题+正文。

## 验收

- 行内不挤换行；hover 不闪、移进卡片不消失；Tab 聚焦即弹、Escape 收；缩放 ≠1 定位不漂（复用 placeFixedTip 的 zoom 修正）。
- 与票 01 合并做行布局回归（图标行首化后 ？ 位置协调）。
