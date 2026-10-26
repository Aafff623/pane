# Spec: 用量拓扑总览（吉祥物枢纽拓扑图）

> 用户 2026-10-25 确认立项。只读版 magpie 路由页：借画法与交互，不搞路由。移植级机制证据：`temp/research/20261025-magpie-ui-mechanics.md` §一。

## 架构（四层）

1. **数据层**：静态底座=provider 卡片快照（Snapshot，main.ts:111）作 provider 分组行 + ModelSpend（main.ts:129-152，spend.rs 聚合）作每 provider 的模型端点行；活跃事件=`listen("usage-updated")`（lib.rs:2638/3616 已有 Tauri event）+ provider metrics 变化检测；LINGER 8-12s 过期降级（magpie client.js:1284 LINGER=12s 同思路）。
2. **布局层**：`.stage` grid 三列（124px/124px/1fr 起步，可调）；每 provider 一行（分组头+模型子行，FOLD_AT 式自动折叠可后置）；ResizeObserver(stage) 触发 layout()。
3. **绘制层**：`svg.wires`（底层）+ DOM 卡片（z1）+ `svg.sky`（顶层，粒子可选）；每行一条 path，d 手算——S 形三次贝塞尔 `M {src右缘} {src.cy} C {mx} {src.cy} {mx} {hub.cy} {hub左缘} {hub.cy}`（hub→行反向同形）；窄屏/折叠走左侧车道（Q 圆角+竖线，client.js:1431-1440 公式照搬）。
4. **动画层**：活跃线 `stroke-dasharray:3 5 + @keyframes {to{stroke-dashoffset:-8}} .7s linear infinite`（纯 CSS compositor）；静态高亮只变色；端点圆点三态（空心/实心/脉冲 box-shadow）；`prefers-reduced-motion` 全降静态。

## 吉祥物与主题（用户点名项）

- 源形：`src/assets/brand/pane-mark.svg`（唯一品牌主形，禁手改派生 PNG；改动走 pane-brand-icons skill 管线）。
- 主题绑定：hub 节点底色/描边走 CSS 变量（--card/--line/--accent），三皮肤（signal/ink/comic-pixel）+ native 自动生效；hub 副标「本地只读 · 不代理」。
- 动效（替换 magpie 喜鹊飞行）：**首选零依赖 CSS/SVG**——hub 呼吸/漂浮（transform 微动）+ 活跃时 hub→线的能量脉冲衔接；**Lottie 为可选**（引 lottie-web 依赖，需 owner 拍板才引入；Pane 目前零运行时依赖哲学）；实施时用 **oil-ui-pro 技能**做动效打磨（用户点名）。
- 喜鹊飞行层（rt-flier 沿路径飞+翅膀拍动）不搬；若 owner 想要"飞行物"，可后置做一个沿 path 运动的光点（circle.pkt，pose() 每帧按路径百分比定位——注意配合 measure 缓存）。

## 边界与不做

- 不做路由配置/切换/排序（Pane 只读；magpie 的路由判决、tries 重试飞行、replay 时间轴、rest 文案全不搬）。
- Alt+A 快捷键：magpie 仓库内无注册点（窗口级，待确认）——Pane 如需快捷键走自己的体系，非本票必做。
- 拓扑页数据只到「provider→模型」粒度；会话级（哪个会话在用哪个模型）作为后续扩展（依赖 magpie-ideas-batch#01 的会话数据盘点）。

## 验收（owner 本人）

- 静态：provider 分行、模型端点、品牌色正确、三皮肤+明暗正常。
- 活跃：usage-updated 到来 → 对应线流动+端点脉冲 → LINGER 后降级；无事件时零动画零 CPU（busy-only rAF）。
- 性能：Resize/折叠/事件突发下无掉帧；面板隐藏时全停。
