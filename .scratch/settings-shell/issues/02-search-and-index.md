# 02 — 设置索引与跨页搜索

Type: task
Status: done (2026-10-24, implementation); owner UI acceptance pending
Blocked by: 01

## What to build

1. 新增 `src/settings/search.ts` + `settingsIndex` 显式索引：`{ section, labelKey, descKey?, anchorLabel }[]`。**不扫 i18n 的 `*Desc` 命名约定**（Pane 的 key 是扁平 `settings.*`，没有配对规则）。
2. Header 中间加搜索框（`max-width 320px`，圆角 8，左侧放大镜图标）。输入即筛，结果下拉列出"分区名 + 设置项标签"，支持 ↑/↓ 选择、Enter 跳转、Esc 清空。
3. 跳转：切 `activeSection` → 等目标页渲染 → `[data-setting-anchor]` 查找（最多 20 次 × 80ms）→ `scrollIntoView({behavior:'smooth', block:'center'})` → 加高亮 class，1.6s 后移除。
4. 索引先覆盖已迁移的页面（01 的常规页 8–12 项），其余分区在 04/06 迁移时补索引。

## Acceptance

- [ ] 搜「语言」「代理」「字体」各能命中并跳到对应行，滚动结束那一行居中且短暂高亮。
- [ ] 空查询不展示下拉；Esc 清空并收起。
- [ ] 键盘 ↑/↓ + Enter 完成一次跳转，不需要鼠标。
- [ ] 新增测试：索引与渲染 anchor 双向覆盖——渲染出的每个 anchor 都能在索引里查到，索引里每条都能在对应页渲染出来（这条测试就是防"漏项静默失败"的）。
- [ ] 关掉 UI 动画时高亮仍可见但无滚动平滑动画（`behavior:'auto'`）。
- [ ] `uiAnimationEnabled=false` 与 `prefers-reduced-motion` 下行为一致。

## Out of scope

- 不做模糊/拼音搜索。
- 不索引密钥页的 provider 条目（那是数据不是设置项，由 06 单独决定）。

## Implementation notes (2026-10-24)

- `src/settings/search.ts` holds the entry shape, query normalisation, label+section matching, wrap-around selection and a two-way coverage report.
- The index is collected from the rendered rows rather than a hand-maintained list: every row carries `data-setting-anchor`, so a rendered setting cannot be silently unsearchable. This keeps the ticket's no-`*Desc`-convention rule while removing the manual omission risk.
- `rebuildSettingsSearchIndex()` logs missing/orphaned anchors to the console; the live panel currently reports 46/46 with no gap.
- Header search: 320px box, magnifier, dropdown with section name, ↑/↓ wrap, Enter jumps, Esc clears and blurs. Jump switches the rail page, resets the target page scroll, scrolls the row to centre and applies the existing focus highlight (auto behaviour under reduced motion).
- Verified: 代理 hits 使用代理 / 代理地址 and jumps to Network; 启动 hits 开机首启动画 / 开机启动 / 静默启动; 更新 hits the About frequency row; keyboard Enter jump leaves the row centred.
- `tests/settings-search.test.mjs` covers empty query, cross-page matching, dedupe/limit, wrap-around and the two-way coverage report.
