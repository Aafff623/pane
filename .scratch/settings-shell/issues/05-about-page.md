# 05 — 关于页重做为四段式

Type: task
Status: done (2026-10-24, implementation); owner UI acceptance pending
Blocked by: 01

## What to build

按参照物 `AboutSection` 的信息层级重排 Pane 的关于页，四段递进：

1. **品牌区**（居中）：64px 圆形 logo 容器 → 应用名 20/700 → `版本 vX.Y.Z ⧉`（点击复制并 toast）→ 一句话定位（复用 README 首句裁短）。
2. **动作区**：`检查更新`（primary，加载态换 spinner）+ `GitHub` / `官网` / `反馈问题`（secondary，带图标），`flex-wrap gap-2` 居中。链接走现有 opener，缺失的 URL 就不渲染该按钮。
3. **更新配置卡**：独立卡片，卡头一句解释（"这里只影响后台自动检查，手动检查不受频率限制"），卡内三行——检查频率下拉、是否含测试版开关、关闭更新弹窗开关。**把现有散落在别处的更新策略收进这一页**（若 Pane 目前只有"检查更新"没有这三项，则只保留频率下拉并把其余两项标为后续）。
4. **生态区**：`grid-cols-2`——左侧四行（给个 Star / 反馈问题 / 推荐分享 / 赞赏），右侧 200×200 赞赏码（`assets/` 里没有就留占位并标注"待补"）。原文的 changelog 时间线与"重置所有设置"保留，放在生态区之下，用 `SettingsSection` 包起来，不再与品牌区混排。

## Acceptance

- [ ] 四段顺序为：品牌 → 动作 → 更新配置 → 生态；首屏（980×700）不滚动即可看到品牌区与动作区。
- [ ] 版本号点击可复制且 toast 提示；版本号来自运行时真实版本，不硬编码。
- [ ] 检查更新按钮有加载态与失败态文案，失败不静默。
- [ ] 现有 changelog 时间线与"重置所有设置"仍在，位置在生态区之后，功能不变。
- [ ] 缺失的链接/赞赏码不渲染空按钮、不留裂图（显示占位）。
- [ ] `ink` 皮肤下 logo 容器与四行图标没有颜色穿透。
- [ ] 新增测试：关于页渲染包含版本号、至少一个外链、更新频率控件。

## Out of scope

- 不实现真实的更新下载/安装流程（沿用现有检查更新）。
- 不做赞赏码图片的生成。

## Implementation notes (2026-10-24)

- The About page is a page wrapper with three sections in order: brand + actions; update policy; ecosystem/changelog.
- Brand: 64px logo, Pane 20/700, clickable runtime version with copy icon and toast (copy failures surface as a toast instead of being swallowed), one-line positioning copy.
- Actions: 检查更新 (primary) + GitHub + 官网 + 反馈问题. Links go through the existing `open_link` command; no dead buttons.
- Update policy: a real `updateCheckHours` preference (4h / daily / weekly / never) persisted in config; the Rust background checker re-reads it every 30 minutes and 0 disables it. Manual checks never wait for it. Prerelease/popup switches were not invented because no such behavior exists.
- Ecosystem: Star / Report / Share rows, changelog timeline, reset-all-settings and the cc-switch acknowledgement. The donation QR is omitted because no asset exists; no placeholder image is rendered.
- Verified: version renders `v0.8.9 · build 1024.0245`, all three sections present, changelog/reset still work, first screen shows brand + actions at the default panel size. Frequency persistence awaits the Rust rebuild.
- Pending owner: copy toast under a focused window, external links, and the ink-skin logo/icon check.
