# 01: 按真实 DOM 写 `comic-pixel` 皮肤并本地打包

**What to build:** 把已完成的漫画×像素原型变成能装进 Pane 的皮肤包：`skins/comic-pixel/skin.css`
+ 字体 assets，注册进 `build-skin-packs.mjs`，本地出包并过校验。

**Blocked by:** 无（不碰 `src/**`）

**Status:** done (local verification) — owner 视觉验收未做

## 验收

- [x] 选择器全部命中 Pane 真实 DOM，原型类名不挪用（259 个类名逐个回源对账，0 个查无此类）
- [x] 亮暗双套 token；`:root[data-skin]` 特异性足以压过宿主，`!important` 0 次
- [x] 数据不被装饰扭曲：donut 分段比例、服务商 logo、数值文本、用户字体选择保持不变
- [x] 选中态四类分色（黄=统计维度 / 青=计量单位 / 紫=时间范围 / 红=低额度与错误）
- [x] `node site/scripts/build-skin-packs.mjs` + `check-skin-packs.mjs`（本地）三包 size/sha 一致
- [x] `parse-tests` 的 `skin_packs` 9 通过（含真校验器过官方包）
- [ ] owner 桌面实装看真机效果（窄窗斜切、像素体可读性、中文标题回退）
- [ ] 上架线上皮肤目录（独立授权动作，本票不做）

## Comments

- 2026-10-23：皮肤 257 行 / 161 规则，包体 97947 B（对比 ink 945416 B、signal 1526381 B）。
  字体原样打包不子集化；页面外缘警戒带装饰不进应用。旧方向 `dc520c2:skins/comic-desk/skin.css`
  只作选择器面参照，未合并。
