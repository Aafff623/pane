Status: ready
Issue: https://github.com/Aafff623/pane/issues/87
Priority: P3
Area: Site

# 官网滚动叙事：scroll-scrub / reveal 门控统一 / 共享 ticker

## 定位

Relay 滚动驱动型动效移植（触发型已由 site-motion-pack 覆盖）。**ego lite 实测后交付**。

## 范围（详见 spec.md，证据 temp/research/20261025-website-ui-deepdive.md §四）

1. scroll-scrub 区块淡入淡出（IO 启停+缓存 document offset 禁每帧 rect）。
2. reveal 门控统一（site.js 两套 IO 收敛一套+seen 才播）。
3. 共享单 rAF ticker（引用计数订阅制，替 4+ 处独立 rAF）。
4. 暂停态联动+后台标签停 rAF。

## 验收

ego 实测无抖帧/暂停联动/后台停；reduced-motion 全静态；owner 手感不过不交付。不做 Lenis/content-visibility/three.js。
