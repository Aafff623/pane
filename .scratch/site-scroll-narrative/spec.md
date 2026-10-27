# Spec: 官网滚动叙事（scrub / 门控统一 / 共享 ticker）

**Status:** ready · **Priority:** P3 · **Area:** Site
**Issue:** https://github.com/Aafff623/pane/issues/87
**Evidence:** temp/research/20261025-website-ui-deepdive.md §四（Relay 模式 3/8/9+ADR-0027 缓存 offset）

## 定位

Relay 模板移植的「滚动驱动型」动效（与 site-motion-pack 的「触发型」分开）。**必须 ego lite 实测后交付**——WebView2 滚动手感/暂停态联动/后台标签页停 rAF 是验收硬条件。

## 范围（1 票，2-3 天）

1. **scroll-scrub 区块淡入淡出**（Relay scroll-blur）：IO 启停+**缓存 document offset**（禁每帧 rect，ADR-0027）+CSS var 写 opacity/y；用于 why-pane↔demo↔ecosystem 交界。
2. **reveal 门控统一**（Relay inview-seen+VIEW_MARGIN）：site.js 现有两套 IO 收敛为一套（threshold/rootMargin 约定+大面积触发器+「seen 才播」语义）。
3. **共享单 rAF ticker**（Relay lib/animation/ticker）：引用计数订阅制+每订阅独立帧率节流，替换 usage-flow/demo 标注/grid/frag 各自 rAF（现 4+ 处独立）。
4. **暂停态联动**：与 paneMotion 暂停态联动（用户暂停动画时 scrub 也停）；后台标签页停 rAF（document.hidden）。

## 验收

ego lite 实测：滚动 scrub 无抖帧（缓存 offset 生效）；暂停态联动正确；后台标签 rAF 停；reduced-motion 全静态；owner 手感验收（不过不交付）。

## 不做

Lenis/平滑滚动劫持；content-visibility:auto（与 IO reveal/锚点/find-in-page 有坑）；three.js 粒子流（141KB+双 WebGL 上下文风险+ascii 海洋前科——如 owner 重提先做 2k 粒子 hero 单点验证）。

## 协调

依赖 site-motion-pack 的 text-reveal 同套 IO；ticker 落地后 usage-flow.js 接入（#60 拓扑总览的官网侧同款时钟）。
