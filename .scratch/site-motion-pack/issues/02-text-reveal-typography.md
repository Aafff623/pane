Status: ready
Type: task
Blocked by: 01

# 02 · text-reveal.js + hero 编排表 + 排印收口

## 做什么

1. text-reveal.js：blur+mask 扫光（IO 触发+CSS 变量 --reveal 驱动）；逐行滑入（overflow:hidden span，**leading≥1.1 防裁切**）；用于 why-pane/ecosystem 标题。
2. hero 入场编排表（常量表驱动 animation-delay）。
3. 排印收口：weight 档位规范（h1 700/h2 680/卡题 640/强调 600/弱化 450，对齐 Manrope 可变字体）；`:lang(zh)` letter-spacing 特调（magpie M:140-143）。

## 验收

中英双语无裁切；reduced-motion 直出终态；字重档位表落 DESIGN.md；owner 验收。
