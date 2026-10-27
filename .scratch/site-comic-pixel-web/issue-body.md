Status: ready
Issue: https://github.com/Aafff623/pane/issues/89
Priority: P2
Area: Site

# comic-pixel 皮肤官网化：漫画七件套最小子集

## 定位

aigcnews comic-pages.css 的漫画手法（前 6 件纯 CSS 零素材）抽 ~200 行最小子集注入官网，comic 皮肤激活时呈现漫画风。

## 范围（详见 spec.md，证据 temp/research/20261025-website-ui-deepdive.md §二）

七件套：手绘不等边边框/硬阴影 4px4x0/Ben-Day 网点/荧光笔标题/爆炸角标/贴纸 chip/描边字。不做（二期）：webp 贴纸探边、对话泡、漫画字体子集。

## 验收

comic 皮肤激活时卡片/标题/徽章漫画风且无布局错位；切其他皮肤零影响；owner 视觉验收。实施纪律：200 行内、令牌挂 site.css、只动官网样式层不动皮肤包本体。
