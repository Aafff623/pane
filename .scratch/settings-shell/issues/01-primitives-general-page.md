# 01 — 设置骨架原子组件 + 常规页样板

Type: task
Status: ready-for-agent
Blocked by: None

## What to build

新建 `src/settings/` 目录，落地三个纯函数渲染器（返回 HTML 字符串 + 一次事件委托，不引入框架）：

- `section.ts` → `renderSection({title, desc, children})`：页签式标题条（标题 18/600 + 描述 12/400 同基线；`px-3 py-3`、`border`、`border-b-0`、`rounded-t-xl`、`-mb-px`）+ 内容卡（`px-5 py-5`、`border`、圆角 12）。
- `item.ts` → `renderItem({label, desc, control, stacked})`：左标签 14/600 + 描述 12/400，右控件；`py-3.5`、`border-b`、`last:border-0`；生成 `data-setting-anchor`（`encodeURIComponent(label)`）。
- `tokens.ts`（或直接写进 styles.css 的设置段）：把语义映射到现有 shadcn 令牌——`--card` / `--border` / `--muted-foreground` / `--primary` / `--foreground`。**不新增 `--qc-*` 变量。**

只把「常规」页（`st-general`）迁到新骨架作为样板：语言、外观、开机自启、静默启动、隐藏未配置、显示全部消费、刷新间隔、托盘指标、时间格式等现有项按 `SettingItem` 重排。其余页面一律不动。

## Acceptance

- [ ] `src/settings/section.ts` / `item.ts` 存在，导出纯函数，无框架依赖。
- [ ] 常规页视觉与参照物一致：标题条与卡片之间无缝隙（`-mb-px` 生效）、行间只有 1px 分隔线、最后一行无下边线。
- [ ] 三档文字层级在真实渲染中可量出 18 / 14 / 12。
- [ ] 每个新渲染的设置行都带唯一 `data-setting-anchor`，且同页无重复。
- [ ] 新增 node 测试：`renderItem` 的 `stacked` 分支、`anchor` 编码、`last:border-0` 位置。
- [ ] `native` 皮肤下与改动前对比截屏，字号/间距差异仅限预期项。
- [ ] 常规页所有开关行为与迁移前完全一致（逐个手点一遍）。

## Out of scope

- 不动侧栏、Header、窗口尺寸。
- 不迁其他分区。
- 不加搜索。

## Implementation notes (2026-10-24)

- `src/settings/section.ts` / `item.ts` export pure renderers; `panelBlock()` now builds from `renderSection`, and every panel row goes through `renderItem`. Section heads carry an 18/600 title plus a 12/400 page description on the same baseline.
- The general page is fully migrated: locale, appearance, Latin/CJK fonts, font-pack manager, density, glass, reduced motion, startup animation + replay, overview jump, refresh interval, tray metric, time format, Total Spend, spend tiers (compact inline fields), experimental, overview categories, Qoder check-in, agent skill, shortcuts entry, autostart, silent start, hide-unconfigured, spend-show-all, re-scan, window form, corner resize, size reset, tour, and backup rows.
- Visible descriptions use existing `*Tip` keys plus new `settings.desc.*` keys (en/zh/ru); 41 descriptions render in the live panel.
- `tests/settings-primitives.test.mjs` covers section/item structure, `stacked`, anchor encoding, i18n keys and the 18/14/12 CSS contract.
- Verified live: 18/14/12 computed sizes, `-mb-px` head/card seam, single 1px separators, last-row border, 46 unique anchors, switch round-trip (`hideUnconfigured` true → false → true).
- Pending owner: the per-control click-through and the native before/after screenshot.
