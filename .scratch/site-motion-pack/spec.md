# Spec: 官网动效包（即日件 + 文字揭示）

**Status:** implemented (2026-10-26) — entrance/type/footer decisions recorded in site/DESIGN.md §9.
**Issue:** https://github.com/Aafff623/pane/issues/86
**Evidence:** temp/research/20261025-website-ui-deepdive.md §四（Relay 可移植模式 1/2/4/5/6/7/10/11/12）+§一（magpie 排印/og）

## 范围（2 票）

**票 01 即日件（半天，零新依赖，gsap 已 vendored）**：
1. 数字 count-up（Relay proof/count-up：rAF/gsap 1400ms easeOutCubic+tabular-nums）——hero-stats「78 服务商」、stat-row、download 区。
2. nav 离开顶部出磨砂板（scrollY<height 切 class+backdrop-filter 过渡；header-nav.js 已有 hide-on-scroll，补 atTop 态）。
3. mono 打字标签（clip-path inset steps 动画）——eyebrow chip、demo-status。
4. POP 徽章过冲（gsap back.out(2) scale 0.6→1）——chip、section-index。
5. footer 静态 glow（CSS radial-gradient，不抄 PNG）。
6. 章节节奏 token（--section-gap 变量统一 ruled 区间距）。

**票 02 文字揭示+hero 编排（1-2 天）**：
1. text-reveal.js：blur+mask 扫光（IO 加 class，CSS 变量 --reveal 驱动 mask-image+filter）用于 why-pane/ecosystem 标题。
2. 逐行滑入揭示（每行 overflow:hidden span，IO 触发 translateY；**leading≥1.1 防裁切**）。
3. hero 入场编排表（一张常量表驱动 CSS animation-delay，取代散落 delay）。
4. 排印收口（magpie 九档 weight 规范化：h1 700/h2 680/卡题 640/强调 600/按钮 560/弱化 450——对齐 Pane 现有 Manrope 可变字体五档或引 mid 档）。

## 验收

reduced-motion 直出终态（magpie 枚举式关闭对照）；中英双语都测（leading 裁切）；ego 实测 WebView2 滚动手感；owner 视觉验收。

## 协调

令牌/规范从 site-design-governance 的 DESIGN.md；不引新依赖；与 site-scroll-narrative（scrub/门控/ticker）分开票——本票只做「触发型」动效，scrub 留给下票。
