# 01 — 分类栏按内容自适应分配宽度（当前被省略号截断）

**What to build:** 分类栏的分组标签（编程 agent / 生产力工具 / MCP 搜索）目前会被省略号截断，**即使可用空间足够**。改成**按内容长度动态分配宽度**：优先完整显示，只有总宽真的不够时才按比例压缩；悬浮显示完整信息（悬浮详情已有，保留）。

**Blocked by:** None — can start immediately.

**Status:** resolved (harness-verified; user UI acceptance pending)

- [x] 复现：正常宽度下 `Coding Agent` 仍被截断（实测 `clientWidth 72` / 需要 `74`，`max-width: 72px`）——**是硬编码上限，不是空间不够**
- [x] 改成内容驱动的自适应分配（不是等分）
- [x] 总宽不足时按比例压缩，且悬浮仍能看到完整文本（`title` 原有，未动）
- [x] 窄窗口下不引入横向溢出/裁切（实测 bar `scrollWidth == clientWidth`，胶囊仍钉右侧）

## Comments

- 2026-10-11 用户报：分类栏分组"明显没显示全，带了个省略号"，但"目前空间其实是比较足的"，要求做自适应分配机制；用户注明"当前先这样，没有太大问题"，属**延后处理**，非当前阻塞。
- 参考：`.category-head` 的渲染在 `src/main.ts` 的 `renderGroupedCards()`（分类分节）附近；顶部总览的分类切换行见 `switchOverviewCategory` / `data-overview-cat`。

- **2026-10-12 已实现**（用户再次点名要求）
  - **真实位置**：被截断的是**额度总览顶部的分类切换 chip 行**（`data-overview-cat`，`src/main.ts:4160-4176`），**不是**卡片分节标题 `.category-head`——后者 CSS 里没有宽度上限/省略号，未截断（本票原描述里"参考 .category-head"是猜的）。另注：内置分类现为 **2 个**（`OVERVIEW_CATEGORIES = ["coding","mcp"]`，见 `feat(ui): two built-in dashboard groups only`），不再是票里写的三个。
  - **根因**：`src/styles.css` 的 `.quota-overview .overview-cat-tabs .tab` 上有一条硬编码 `max-width: 72px` + `overflow: hidden; text-overflow: ellipsis`——不管行内还剩多少空间，chip 一律截到 72px。实测 `Coding Agent` 需要 74px，**只差 2px 就被截**，而分类栏内部当时还有约 158px 空余。
  - **修复**：把该规则换成内容驱动宽度——`flex: 0 1 auto; min-width: 0; max-width: none;`（保留 `white-space: nowrap` / `overflow: hidden` / `text-overflow: ellipsis`）。空间够 → chip 按内容全宽；不够 → 按基准宽度比例收缩并出省略号；容器原有 `overflow-x: auto` 作为兜底。`.full`（完全显示分类）模式的 3 列网格规则特异性更高，行为未变。
  - **证据**（度量工具 `temp/scripts/pane-catbar-metrics.mjs`，CDP 实测；`TIGHT=<px>` 可强制压缩分类栏来验证退化路径）：

    | 场景 | coding chip client/need | mcp chip client/need | 截断 | bar scroll/client |
    | --- | --- | --- | --- | --- |
    | 修复前（正常宽） | 72 / 74 | 34 / 34 | coding 截断 | 270 / 270 |
    | 修复后（正常宽） | **74 / 74** | 34 / 34 | 无 | 270 / 270 |
    | 修复后 TIGHT=140 | 74 / 74 | 34 / 34 | 无 | 140 / 140 |
    | 修复后 TIGHT=90 | 56 / 74 | 28 / 34 | 按比例压缩 | 90 / 90（不溢出） |
    | 修复后 TIGHT=60 | 34 / 74 | 20 / 34 | 继续压缩 | 60 / 60（不溢出） |

  - 视觉证据：`temp/catbar-postfix.png`（切换行裁切截图，`Coding Agent` 完整显示，`5h/7d/月` 胶囊仍钉右）。
  - 校验：`tsc` exit 0、`vite build` 成功（本 shell 的 `pnpm` 解析坏，直调本地 tsc/vite）。
  - **待用户 UI 验收**：正常窗口下分类 chip 应显示 `Coding Agent` 全称；把窗口缩到最窄，chip 应压缩出省略号但行内不溢出、胶囊不被挤走；悬浮仍能看到完整名称。
  - 未提交：改动只在工作区（`src/styles.css`）。
