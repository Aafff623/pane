# 「复制卡片为图片」：先选范围，再出信封模板

**What to build:** 把当前无 handler 的死按钮接成流程：点击弹出时间范围菜单（今天/昨天/7天/30天/全部）→ 选定后经既有 shareCard 管线产出一张卡片信封模板图。视觉细化等 owner 给参考。

**Blocked by:** None

**Status:** in-review（骨架可用；信封最终视觉待 owner 给参考后细化）

- [x] 点击按钮弹出范围菜单（复用既有菜单样式），Esc/点外部可关
- [x] 选定范围后产出图片并复制到剪贴板，失败有明确提示
- [x] 选中的范围与该图内的数字一致（不出现「选了今天却画 7 天」）
- [x] 骨架先落地，视觉细化留后续票；不得留下死按钮

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
- 2026-10-14（侦察修正）：**总花费分享按钮并非无 handler**——dev 机实测原按钮会直接截整卡并写剪贴板（状态行「已复制到剪贴板」）。owner 的「死按钮」体验应指「点了界面上没有任何反馈、且拿到的是整卡截图而非期望的选范围信封」。因此本票不是接死线，而是**改语义**：总花费按钮改为「先选范围 → 出信封模板」；provider 卡保持即时整卡快照不变。
- 2026-10-14（实现）：
  - 抽出共享快照管线 `copyElementSnapshot(el, W)`（CSS 采集/动画冻结/离屏测量/SVG 包裹/2x 光栅化/剪贴板），`shareCard` 与新信封共用，避免第二份 80 行重复。
  - 新增 `openShareRangeMenu(anchor)`：复用 `.group-menu` 样式族，5 项（今天/昨天/7天/30天/全部），当前面板选中的窗口打勾；Esc 与点外部关闭；贴按钮右缘下方并钳在窗口内。
  - 新增 `shareSpendEnvelope(range)`：自建 `.envelope` 模板（标题 + 范围 chip + 总额 `$ · tokens` + 前 6 名带进度条 + `+N` 溢出 + 快照页脚 logo），数值取自**同一批** `envelopeEntries(range)`（与面板同源、跟随按工具/按模型切换），所以「选了 7 天就一定是 7 天的数」；空窗口出「无用量」而非空白。
  - `shareCard` 收敛为 provider 卡专用（`__total__` 分支已无调用者，删除死路径）。
  - styles.css 新增 `.envelope-*`（仅快照文档内渲染）；i18n 三语新增 `share.rangeTitle`。
- 验证证据：tsc 0 错；vite build 过；site 13/13；check-readme ALL PASSED；demo 已重建。
- 真机走查（CDP + 剪贴板回读，temp/scripts/share-range-walkthrough.mjs / share-range-capture2.mjs / share-copy-once.mjs）：菜单打开=5 项且标题「复制为图片 · 选择时间范围」、当前项打勾；Esc 关闭；选「今天」→ 状态「已复制到剪贴板」、零异常；剪贴板回读 PNG 视觉确认（temp/shots/082-04-envelope.png）：标题/chip/`$31.35 · 412.0M tokens`/ZCode 行+满宽条/页脚 logo 全部正确；菜单截图 temp/shots/082-04-range-menu.png。回归：provider 卡分享仍即时复制且不弹菜单。
- 待办（不在本票）：信封最终视觉（配色/留白/是否带迷你趋势）等 owner 给参考后细化；当前为可用骨架。

- 2026-10-14（接续复核）：独立review发现并修复mtok条长与打印值口径不一致（复用spendVal），分享模板补入同范围credits独立行，与面板共享creditEntriesForRange。新字体截图测量等待document.fonts.ready。浏览器复验五项范围菜单及Esc关闭；原生剪贴板证据沿用此前本票记录，本轮未重写剪贴板。
