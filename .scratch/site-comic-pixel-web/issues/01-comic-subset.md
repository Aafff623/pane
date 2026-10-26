Status: ready
Type: task
Blocked by: site-design-governance

# 01 · 漫画七件套最小子集注入官网样式层

## 做什么

1. 从 aigcnews comic-pages.css 抽 200 行内最小子集：手绘不等边边框（2px/2.5px/3px/2px+四角不等圆角）、硬阴影 4px4px0、Ben-Day 网点（局部+暗色降 opacity）、荧光笔标题（rotate+skew 盖下部 68%）、爆炸角标（clip-path）、贴纸 chip（skew+italic 800）。
2. 作用域：comic 皮肤激活时生效（skin.css 体系 or body[data-skin]——实施时定）；令牌（--comic-shadow 等）挂 site.css 派生并进 DESIGN.md。
3. 应用位：卡片边框/标题/服务商徽章。

## 验收

comic 皮肤下漫画风且无布局错位；其他皮肤零影响；owner 视觉验收（对照 aigcnews 截图）；不碰皮肤包本体。
