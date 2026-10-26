Status: done (2026-10-26) — stage/wires/hub, S beziers, ResizeObserver, live dash flow.
Type: task
Blocked by: 01

# 02 · 拓扑舞台与连线（三明治 + 手算贝塞尔 + FLIP）

## 做什么

1. `.stage` grid 三列 + `svg.wires`/DOM 卡片（z1）/`svg.sky`（z2）三层；每 provider 行一张卡 + 模型端点行。
2. layout()：getBoundingClientRect 换算 stage 本地坐标；S 形贝塞尔 d 公式照搬（client.js:1415-1444：`M {src} {src.cy} C {mx} {src.cy} {mx} {hub.cy} {hub左缘} {hub.cy}`；窄屏/折叠左侧车道 Q 圆角）；ResizeObserver 触发。
3. measure() WeakMap 缓存 path 长度与采样点（getTotalLength/getPointAtLength 极贵，magpie 实测 40s 里 10s 耗在这两个 API）。
4. 折叠/重排 FLIP（记旧 top→translateY→reflow→清空→CSS transition .5s cubic-bezier(.2,.8,.2,1)）。
5. 活跃线：`stroke-dasharray:3 5 + dashoffset .7s linear infinite` 纯 CSS；品牌色走 providerVisuals.ts。

## 验收

- Resize/折叠/展开连线贴合无断裂；隐藏页切回重算正确（offsetWidth=0 坑）。
- 性能面板：动画只走 compositor；rAF 只在 busy 续帧；面板隐藏全停（visibilitychange 门）。

## Comments

- 2026-10-26: 实施与验证见 `temp/handoff/20261026-magpie-borrow-batch.md`；未 commit/push，UI 由 owner 验收。
