# 04 — 迁移其余简单分区到新骨架

Type: task
Status: ready-for-agent
Blocked by: 01, 03

## What to build

把「通知」「隐私」「网络」「快捷键」四个分区改成 `renderSection` + `renderItem` 组合，沿用 01 的样板；每页迁移时同步把对应条目补进 02 的 `settingsIndex`。

- 通知：4 个开关（快用完 / 接近上限 / 将耗尽 / 5 小时窗口即将重置），每项补一句描述。
- 隐私：3 个开关（匿名统计 / 共享屏幕时隐藏托盘数字 / 卡片显示用量趋势）。
- 网络：使用代理开关 + 代理 URL 输入（用 `stacked` 变体铺满）+ 一段说明。
- 快捷键：8 行（唤醒、分类、6 个本地动作），每行右侧为"当前键位 + 重置 + 录制 + 清除"的组合控件，整体走 `stacked` 或右侧自适应容器；保留现有录制逻辑，只换渲染外壳。

同时删除这四个分区在原 `index.html` 中的手写行，并从 `moveRow()` 的搬运列表中移除对应 id。

## Acceptance

- [ ] 四页视觉与常规页同节奏：标题条、卡片、行分隔、三档文字一致。
- [ ] 快捷键录制/重置/清除行为与迁移前完全一致（逐个按一遍，含冲突提示）。
- [ ] 网络页代理 URL 输入铺满整行且可编辑、可保存。
- [ ] 四个分区内每个设置行都有 `data-setting-anchor`，且已被 `settingsIndex` 覆盖（02 的双向覆盖测试通过）。
- [ ] `index.html` 中对应旧行已删除，`moveRow` 列表中不再引用；没有"改了新的、旧的还在"的双绑。
- [ ] `native` 与 `ink` 两套皮肤下截屏核对。

## Out of scope

- 不动「密钥与额度」「关于」（分别在 06 / 05）。
- 不改快捷键的默认值与冲突检测算法。

## Implementation notes (2026-10-24)

- Notifications: note paragraph + four switches (almost out / cutting close / will run out / reset soon), each with a visible 12/400 description.
- Privacy: note paragraph + telemetry, hide-while-sharing and usage-trend switches.
- Network: proxy switch, proxy URL row and the restart note.
- Shortcuts keeps its dynamic `#local-shortcuts` block; its hints and the wake shortcut row remain wired to the existing handlers.
- The pages use the shared `panelSwitchRow`/`panelRowDesc` helpers, so the labels, descriptions and separators match the general page.
- Verified: pages render, switches round-trip through `patchConfig`, and search finds 使用代理 / 代理地址.
- Pending owner: click-through of the notification switches and the shortcut recorder.
