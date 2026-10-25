# 04 — 皮肤包调整与重新打包

Status: implemented (2026-10-25, owner approved both) - packs rebuilt at 1.0.1; owner visual acceptance pending
Type: task
Blocked by: 01, 03
**Owner gate:** 打包 zip + 更新 index.json 前需 owner 明确授权；未授权则把露馅与恒红写进遗留说明，本票只改 pack 源码。

## Scope

- signal：settings-skeleton 段（skin.css:279-293 有先例）给 `.set-row__control .tabs` 补轨道皮肤（`var(--signal-inset)` 底 + post-stitch 虚线边），治 01 的轨道实色露馅。
- 恒红 reset 按钮（owner 二选一，实施前确认）：
  - 选项 1（默认）：接受三个包恒红（signal:116、ink:46、comic-pixel:175 特异性 (1,2,1)/(1,3,0) 压顶），写入验收标准与遗留说明；
  - 选项 2：同步改 pack 为 hover 版本（静态中性、hover 才红），与 native 对齐。comic-pixel 若改，顺手补 `#reset-all-settings` 的 2px 圆角分裂（skin.css:170 输给 ID 规则、175 行又赢的既有 bug）。
- ink 的 `.mini-btn.danger` 不覆盖（46 行只管 `.danger-btn`），base 的 hover 规则在 ink 上已有效，不动。
- 若授权打包：重新打包三个 zip + 更新 `site/public/skins/index.json`（当前 generatedAt 2026-10-23，版本号与仓库源核对一致 1.0.0）；打包与上传/发布严格区分，发布授权另走 AGENTS.md 门禁。

## Acceptance

- [ ] signal 分段轨道与皮肤美学一致（亮暗各验）；或未授权时露馅已书面记录。
- [ ] reset 按钮行为符合 owner 选择；comic-pixel 圆角分裂已治或已记录。
- [ ] 授权打包时：zip 与 index.json 同步，`site/public/skins/index.json` 的 generatedAt 与版本如实更新；未授权时仓库源改动留在工作区待后续。
- [ ] 不上传、不发布；打包产物边界在交付说明里写清。
- 2026-10-25：native 侧已全部落地（票 01-03）。本票待 owner 两项决策：① signal 的 .set-row__control .tabs 轨道是否按 279-293 先例补皮肤（var(--signal-inset) + 邮戳虚线边）；② 三个包恒红 reset 按钮（signal:116 / ink:46 / comic-pixel:175）接受恒红写进验收标准，还是同步改 hover 版（comic-pixel 顺带修 #reset-all-settings 圆角分裂）。任何 pack 改动 = 重打 zip + 更新 site/public/skins/index.json，需单独授权。未决策前：signal 分段轨道为 native 灰底（露馅已知晓）、三包 reset 按钮恒红。
- 2026-10-25：owner 拍板「都得做」。signal 的 .set-row__control .tabs 并入 settings-tabs 同款轨道（inset 底 + post-stitch 虚线边，skin.css:148 选择器列表扩展）；三个包的危险按钮改静态中性、hover 才红（signal:116 拆两条、ink:46 加 :hover、comic-pixel:175 加 :hover:not(:disabled)），comic-pixel 另加 #reset-all-settings{border-radius:2px} 修圆角分裂。node site/scripts/build-skin-packs.mjs 1.0.1 重打包 + index.json 更新 + check-skin-packs 全绿（size/sha 一致）。发布（上传 zip + index）仍走单独授权；本地已装 1.0.0 的机器需更新包后才见新样式。
