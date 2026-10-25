Status: done (2026-10-25)
Type: task
Blocked by: 01, 02, 03, 04

# 05 · 15 条验收断言落库 + 迁移发布说明

## 做什么

1. parse-tests 按 spec.md §7 落全部 15 条断言（涨价/降价不追溯、日内分段、日级日末、回扫估算、填充一次、overflow 美元中性、CORRECTIONS_REV bump、override 仅向前、stamp 空转、alias/倍率、checkpoint 续扫、merge_daily 幂等、显式重建一致）。
2. 迁移说明：v5→v6 一次性全重解析 + 老用户历史按最早观察价重估标 ≈——写进发布说明（release note）与 CHANGELOG  reconciled 流程。
3. 全量验证：`cd parse-tests; $env:PATH="D:\Tools\mingw64\bin;$env:PATH"; cargo +stable-x86_64-pc-windows-gnu test`；`cargo +stable-x86_64-pc-windows-gnu check`（src-tauri）；`pnpm build`。

## 验收

- 15 条断言全绿；`cargo check`/`pnpm build` 通过；UI 验收由用户本人确认（≈ 标识、Settings 文案、重建入口）。

## 待用户最终确认的产品决策（落地前）

1. 日级源日内调价近似：默认「≤日末最后观察」（spec §①）；备选「日初观察、次日生效」。调价日落有语义张力，需拍板。
2. 首次安装回扫按首条观察价估算并标 ≈。
3. override 不再能修历史（Settings 文案变更）。
