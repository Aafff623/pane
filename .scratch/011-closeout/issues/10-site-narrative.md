Status: done (owner 手感验收待做)
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

## 结果

### #83

- `site/public/design/tiers.html`：三档各带 目标 / 会改 / 不改 / 成本 / 风险，
  `<section class="tier" data-chosen="true" data-tier="organize">` 标出选中档，
  决策日期 2026-10-26，`noindex`（治理页不进搜索）。
- `site/DESIGN.md` §11 记同一裁决：**Chosen: 整理 (organize)**，并写明升级到
  「升级 / 重塑」档必须先改设计契约再动首页。

### #87

- `site/public/ticker.js`：全页唯一 rAF 环。订阅者申报帧率，环在最后一个订阅者
  离开或标签页 hidden 时停，`visibilitychange` 恢复。接入方：demo 标注跟随、
  采集器光束、数字滚动。60 fps 订阅带 1 ms 容差——否则真实 60 Hz 时间戳会落在
  自己的 interval 内侧，隔帧被跳过，实际只跑一半帧率。
- `site/public/reveal.js`：唯一入场门控。observer 按 `(threshold, rootMargin)`
  共享（不再是每个标题一个），节点自带类名；GSAP 入场用 `className: null` +
  `onReveal` 注册，因为它的 threshold/margin 与区块入场完全相同。命中即
  unobserve，回滚不重播。
- `site/public/scrub-curve.js` + `scroll-scrub.js`：`#why-pane` / `#demo` /
  `#ecosystem` 带 `data-scrub`，其文案块带 `data-scrub-fade`；`--scrub` 在
  交界各用半个视口从 1 落到 0，CSS 换算成
  `opacity: calc(.55 + .45 * var(--scrub, 1))`。
- 曲线拆到 `scrub-curve.js`（零 import），`motion.js` 改为惰性读全局——这是
  ticker / 门控 / 曲线能在 `node --test` 下真跑起来的前提。
- **只做 opacity，不做 transform**：`#why-pane-title` 与 `#ecosystem-title` 已带
  `.reveal-text`，transform 归它；同一元素上再叠一个 transform 会打架。

### 浏览器实测发现并修掉的两个缺陷（读代码看不出来）

1. **缓存 offset 会被晚到的布局变化甩掉。** demo iframe 懒加载，落地后把
   `#demo` 高度从 ~654 撑到 1267，初始化时缓存的 offset 就此失效：17 步走完
   叙事区，写入值与「用实时几何重算的曲线」只在 2/17 个位置吻合。改为
   `ResizeObserver` 观察每个 scrub 区块 + `body`，任何盒子变化就重测（同时覆盖
   晚到字体与 ecosystem 的 `<details>`），修完 17/17 全中（容差取模块自己的
   epsilon 0.002）。
2. **IntersectionObserver 门控必须去掉。** 原实现只画被 observer 标记为 near 的
   区块；机器有负载时 observer 明显落后于滚动位置，已在屏上的区块仍带着 CSS
   兜底值（全不透明），随后一次性暗下去——肉眼可见的跳变。改成每帧算全部区块：
   离交界远的区块本来就饱和在 0 或 1，epsilon 判断下不产生任何写入，比那个 bug
   便宜。
3. `--scrub` 改写在**用到它的文案块**上而不是区块上：继承型自定义属性会脏整棵
   子树，而 `#demo` 里就是内嵌面板。

## 测试

- `node --test site/*.test.mjs`：**50 passed / 0 failed**（`site.test.mjs` 29 项，
  其中新增 3 项；其余四个文件 21 项）。
- 新增测试不止查文本：ticker 用假 rAF 真跑（60 fps 每帧都到、30 fps 隔帧、
  最后一个订阅者离开后 `tickerState()` 回 `{subscribers:0, running:false}`）；
  reveal 门控用假 IO 真跑（同配置复用同一 observer、命中一次不重播、无 IO 时
  立即揭示）；`scrubValue` 钉住两端为 0、中段为 1、两条边单调。
- `node --test tests/*.test.mjs`：212 passed / 0 failed。
- `node site/scripts/sync-site-version.mjs --check`：v0.11.0 / 78 providers 同步。
- demo bundle 已按 `src/main.ts` 变化重建（ticket 02 的改动让它过期，
  freshness gate 会红）。

## ego lite 实测（wrangler dev 127.0.0.1:8791，Chrome）

| 硬条件 | 结果 |
| --- | --- |
| 缓存 offset 生效、无每帧 rect | 按 receiver 归因统计 `getBoundingClientRect`：scroll-scrub.js **0 次**；同一段路上 `line-sidebar.js:61` 183 次（阅读进度轨，早于本票，按滚动事件合帧，已写进 DESIGN.md §12 免得下一个人查错文件） |
| 曲线正确 | 17 步走完叙事区，写入值与实时几何重算值 **17/17** 吻合 |
| 暂停联动 | 点暂停后三段全 settle 到 `1`，继续滚动仍保持 `1`，opacity 回 1.000；两个暂停按钮（采集器 / 轨道）标签与 `aria-pressed` 一致，`localStorage` 落 `paused`；恢复后立刻回到真实曲线值 |
| 后台标签页停 | 伪造 `document.hidden = true` + `visibilitychange` 后滚动，`--scrub` **完全不动**（frozen=true），采集器自己进 `is-paused`；恢复后同一位置继续更新 |
| reduced-motion 全静态 | 9 个滚动位置上 `--scrub` 恒为 `1`、所有 fade 块 computed opacity 恒为 `1.000`、采集器 paused、两个 `.reveal-text` 标题 opacity 1 |
| 真实观感 | 帧钟健康时（median 14.2 ms）滚动到 y=700 读到 `--scrub 0.746` / opacity 0.886，y=1000 起稳定在 `1.000` / `1.000`；标题入场与 scrub 互不干扰 |

**测量方法上踩的坑（留给下一次）**：ego lite 窗口最小化时 rAF 掉到 ~1 Hz、
IntersectionObserver 基本不投递，那一轮所有读数作废；`Page.bringToFront` +
`Browser.setWindowBounds(windowState:"normal")` 之后仍可能因为机器负载停在
median 48 ms。**先量帧钟，median > 25 ms 就不要下结论**，改成按超时等待（而非
等 rAF）并用「实时几何重算」做对照，才能把机制验对。

## owner 手感验收步骤

1. `cd site && npx wrangler dev --port 8791`，打开 <http://127.0.0.1:8791/>。
   （8787 已被本机别的服务占着，别用。）
2. 慢慢滚过 「为什么用 Pane → 实时工作台 → 生态」三段：文案应在交界处变暗到
   约 55% 再回到全亮，像接力而不是三块同时打光；不应有任何一次性跳变。
3. 点任一「暂停动画」：三段文案立刻全亮并保持在 100%，继续滚动不再变化；
   两个暂停按钮的文案与按下态应同步。再点一次恢复，滚动重新起效。
4. 切到别的标签页几秒再回来：不应看到文案在回来的一瞬间「补帧」跳动。
5. 系统开「减少动态效果」后刷新：所有文案恒为全亮，采集器与轨道不动，
   两个大标题直接是清晰状态（不模糊、不上移）。
6. 打开 <http://127.0.0.1:8791/design/tiers.html>：三档对比可读，「整理」档
   标为已选，日期 2026-10-26。

