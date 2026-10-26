Status: pending
Type: task
Blocked by: 03

# 10 · 站点设计治理与滚动叙事（#83 / #87）

## #83（采用“整理”档）

- 产出 `site/public/design/tiers.html`：整理/升级/重塑三档对比，记录 owner
  选择“整理”；不重做首页骨架。
- 把选择写进 `site/DESIGN.md` 的决策记录。

## #87

1. 新建 `site/public/ticker.js`：引用计数 + 每订阅帧率节流 + hidden 停 +
   reduced-motion 静态。
2. 统一 reveal 门控：`reveal-text` 与 `.is-visible` 两套 IO 收敛为共享
   observer；seen 才播。
3. scroll-scrub：缓存 section offset（禁止每帧 rect），CSS 变量写 opacity/y；
   覆盖 why-pane ↔ demo ↔ ecosystem 交界。
4. 暂停态：paneMotion 暂停时 scrub/ticker 停；后台标签页停 rAF。
5. ego lite 实测四项硬条件；owner 手感验收后再关票。

## 验收

无抖帧、暂停联动、后台停、reduced-motion 全静态；`node --test site` 绿。
