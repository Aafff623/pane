Status: ready
Type: task
Blocked by: site-motion-pack

# 01 · scroll-scrub + 门控统一 + 共享 ticker + 暂停联动

## 做什么

1. scroll-scrub（IO 启停+缓存 document offset+CSS var 写 opacity/y）：why-pane↔demo↔ecosystem 交界。
2. site.js 两套 IO 收敛为一套 reveal 门控（threshold/rootMargin 约定+seen 才播）。
3. 共享 ticker.js（引用计数订阅+帧率节流），usage-flow/demo 标注/grid/frag 接入。
4. 暂停态联动（paneMotion 暂停时 scrub 停）+document.hidden 停 rAF。

## 验收

ego lite 实测四项硬条件（无抖帧/暂停联动/后台停/reduced-motion 静态）；owner 手感验收；不交付不算完。
