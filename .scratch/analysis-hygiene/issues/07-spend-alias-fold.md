# 07: 花费热力图别名折叠对齐

**What to build:** 花费热力图上同一 provider 的别名双写（kimi 与 moonshot 同天各出一行）折叠为一行，明细视图与汇总视图口径一致，日消耗不再视觉双计。

**Blocked by:** None (can start immediately)

**Status:** implemented（2026-10-15，见对应 commit）

- [ ] 别名归一化步骤从汇总视图内联提升为明细行与汇总共用的归一化点
- [ ] 明细行构造对同天同源别名折叠为一行（保留原聚合语义，不丢模型明细）
- [ ] 新增测试：同一天 ledger 双 id 输入 → 明细一行、汇总口径一致
- [ ] harness 全绿；`cargo check` 过
