# 总花费卡恒为上下结构（修复跨 DPI 布局漂移）

**Status:** ready-for-agent
**Feature dir:** `.scratch/spend-stacked-layout/`
**Date:** 2026-10-12
**来源:** 用户报告（同学电脑上左右结构）+ ZCode 定位（2026-10-12）

## Problem Statement

总花费卡在部分用户的电脑上变成**左右结构**：圆环在左，时间范围与图例在水平方向右侧。用户机器（高 DPI 缩放）上从未出现过，因此被当作"别人电脑里的比例不对"。正确形态只有一个：**圆环区在上、时间范围（今天/昨天/7 天/30 天/全部）与折线/图例在下的上下结构**。

根因（已定位并用复现回路证实）：`.donut-wrap` 基础规则默认 flex row，仅 `@media (max-width: 440px)` 才切 column。CSS 视口宽度是否 ≤440px 取决于 DPI 缩放——100% 缩放的电脑 CSS 像素更多，跨过断点即翻成 row。

## Solution

把上下结构定为**唯一**布局：堆叠规则从媒体查询挪进基础规则，删除 440px 断点。任何窗口宽度、任何 DPI 缩放，总花费卡都与用户当前看到的正常形态一致。

## User Stories

1. As a 100% DPI 缩放的用户, I want 总花费卡恒为上下结构, so that 圆环与时间线不会挤成难以辨认的左右两栏
2. As a 高 DPI 用户, I want 布局不随缩放设置变化, so that 升级/换机后看到的样子和我习惯的一致
3. As a 用户, I want 分享 PNG 与官网 demo 里的总花费卡和本地一致, so that 对外展示不出现另一种布局

## Implementation Decisions

- `.donut-wrap` 基础规则改为 `flex-direction: column; gap: 6px`（沿用原媒体查询内的堆叠值），svg 居中、`.spend-cols` 占满宽度同样上提为基础规则。
- 删除 `@media (max-width: 440px)` 中对 `.donut-wrap` / `.spend-cols` 的规则块（该断点对其他选择器的规则不受影响）。
- 不引入新的布局模式或配置项；row 布局按用户定义即为错误，不留开关。

## Testing Decisions

- 回归 seam = 复现回路 `temp/scripts/spend-layout-loop.mjs`：真实 styles.css + iframe 控宽（420px / 480px），断言 `.donut-wrap` computed flex-direction 全宽度为 column。修复前该命令必须为红（已在 2026-10-12 实测红：480px=row），修复后必须转绿。
- CSS 布局无单元测试 seam，回路脚本即回归载体（本地 temp，不入 CI——缺口如实记录）。
- 修复后 `pnpm build`（tsc+vite）门禁照常。

## Out of Scope

- 花费详情浮层（spend-detail-panel）自身的布局。
- 圆环尺寸、图例列数等视觉参数调整。

## Further Notes

- 修复者必须先跑回路看红、再改、再看绿（diagnosing-bugs Phase 5 纪律）。
- 官网 demo 由同一源码构建，随下次 demo 重构建自动一致。
