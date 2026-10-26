# Spec: comic-pixel 皮肤官网化（漫画七件套）

**Status:** ready · **Priority:** P2 · **Area:** Site
**Issue:** https://github.com/Aafff623/pane/issues/89
**Evidence:** temp/research/20261025-website-ui-deepdive.md §二（aigcnews comic-pages.css 七件套，前 6 件零素材依赖）

## 定位

aigcnews 的 comic-pages.css（320KB）提供一整套手绘漫画 CSS 手法。Pane 有 comic-pixel 皮肤（skins/comic-pixel/skin.css，应用内）+皮肤 CDN——本票把漫画手法以**最小子集（~200 行）**注入官网样式层，comic 皮肤激活时官网呈现漫画风。

## 范围（1 票）

**七件套最小子集（前 6 件零素材依赖，全纯 CSS）**：
1. 手绘感边框：`border-width:2px 2.5px 3px 2px`（四边不等）+`border-radius:6px 8px 5px 9px`（四角不等）。
2. 硬阴影：`box-shadow:4px 4px 0 var(--comic-shadow)` 无模糊。
3. Ben-Day 网点：`.comic-panel:after` radial-gradient 局部铺点（右上角 190×110px，暗色降 opacity——注意性能克制）。
4. 荧光笔标题：`.comic-title:before` 色块 rotate(-3deg) skew(-18deg) 盖文字下部 68%。
5. 爆炸角标：clip-path polygon 小星形+标题 italic 950。
6. 贴纸 chip：1.5px 边+2px 硬阴影+rotate(-3deg) skew(-6deg)+italic 800——用于服务商徽章。
7. 描边字：-webkit-text-stroke .45-.6px（细节，低优先）。
**不做（需素材，二期）**：页脚贴纸探边（webp）、对话泡（webp 日夜双图）、漫画字体子集（Bangers/站酷庆科黄油体——中文子集 8 字符 trick 记录备查）。

## 实施纪律

- 抽 200 行内最小子集（原 320KB 全桌面生效是反面教材）；令牌挂 site.css 现有 token 派生（--comic-shadow 等进 DESIGN.md）。
- 与皮肤切换架构协同：comic 手法仅 comic 皮肤激活时生效（skin.css 体系内 or body[data-skin=comic] 作用域——实施时定）。
- 性能：网点层 opacity 克制；reduced-motion 无关（静态手法）但尊重对比度。

## 验收

comic 皮肤激活时官网卡片/标题/徽章呈现漫画风且无布局错位；切回 signal/ink 皮肤零影响；owner 视觉验收（对照 aigcnews 漫画页截图）；site.test.mjs 不涉及（样式层）。

## 协调

令牌进 site-design-governance 的 DESIGN.md；与 magpie-settings-polish 票 04（皮肤包 1.0.1 已重打包）错开——本票只动官网样式层不动皮肤包本体。
