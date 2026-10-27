Status: partial (2026-10-26) — main.ts scanned; data-table exemptions not enumerated.
Type: task
Blocked by: 02

# 03 · gatekeeper TS 侧 + 数据表豁免决策

## 做什么

1. 扫描扩展到 src/main.ts（115 行命中基数）与 src/providerMechanisms.ts（77 行）：同规则（TS 侧无 providers:: 转发形态，豁免按「provider 模块文件/映射表」判定——具体边界实施时定，与 owner 确认）。
2. 数据表文件（rtk_savings.rs / snapshot_store.rs 内嵌 id/icon 表、auth_center.rs 分组数组）决策：整文件豁免（等价 generated manifest）还是逐条理由，owner 拍板。
3. i18n/图标等纯数据文件如命中，同上决策。

## 验收

- parse-tests 全绿覆盖 Rust+TS 两面；白名单规模记录在案。
