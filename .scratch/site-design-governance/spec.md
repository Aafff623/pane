# Spec: 官网设计治理（DESIGN.md + 三档对比）

**Status:** ready · **Priority:** P1 · **Area:** Site
**Issue:** https://github.com/Aafff623/pane/issues/83
**Evidence:** temp/research/20261025-website-ui-deepdive.md §三（oil-ui-pro DESIGN.md 机制+visual-polish 36 项）

## 定位

oil-ui-pro 技能的核心机制：existing-project 流程=规范体检（品牌资产 vs 偶然默认值）→「整理/升级/重塑」三档推翻程度对比页→写项目设计规范文件 DESIGN.md。**本票是后续所有官网 UI 票（7-12）的验收标准与令牌锚，必须先立。**

## 范围（1 票）

1. **规范体检**：盘点 site/ 现有设计决策——哪些是品牌资产（ink-mark、section-index 编号、blueprint 语言、comic-pixel 皮肤）哪些是偶然默认值（散落的 cubic-bezier、五档散字重、临时间距）；对照 oil-ui-pro visual-polish 36 项清单过一遍。
2. **三档对比页**：整理（只收拢不动结构）/升级（令牌化+动效统一）/重塑（按 oil-ui-pro 首屏八骨架重评 hero）三档各出对比（文字+关键区截图或原型），交 owner 选档。
3. **DESIGN.md**：落 site/DESIGN.md——设计北极星、令牌表（色/字/距/圆角/动效曲线，对齐 site.css 现有 token）、section 级规范、验收清单（36 项裁剪版）、「禁自创版式」约束记载。
4. **协同锚**：DESIGN.md 与 #60 拓扑总览的母题/色板/GSAP 时钟约定互相引用（官网与应用一张皮）。

## 验收

DESIGN.md 落盘且被后续 UI 票引用为验收标准；三档对比页交付 owner 拍板（档位选择记入 issue 评论）；oil-ui-pro 原型代码许可结论记录在案（未确认前不进公开仓库）。

## 协调

本票先于 site-ui-trust-specs / site-motion-pack / site-scroll-narrative / site-oil-ui-cards / site-comic-pixel-web 实施（或至少其规范章节先行）。
