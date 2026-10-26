Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · Provider 图标调大与嵌入自然化

## 做什么

1. `.provider-icon` 改造（styles.css:362）：24px 图标 + 30px 圆角容器（border-radius:8px、border:1px solid var(--border)、底 var(--card)）；透明 glyph 居中；tile 型（自带实心底）铺满裁圆角——先全走透明型，后续按 SVG 是否自带 rect 分档。
2. main.ts:3683 图标 DOM 移到 name 左侧（行首化）；auth 分组头（styles.css:9453）复用同参数对齐。
3. 首字母 fallback 字号 `max(size*0.5,12)`。

## 验收

- 明暗两模式 + 三皮肤下 60+ 图标无糊边/顶边；折叠卡头部不超高；与 cc-switch 应用列表观感对照（截图级验收 by owner）。
