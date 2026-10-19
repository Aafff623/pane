# 05 — 额度来源行、资源包明细与 7 天临期

**Blocked by:** 02
**Status:** done（代码层 + 纯解析测试；UI 观感待 owner）
**Issue:** #11（实施，收缩）

**What to build**

- 非默认额度渠道（desktop / web）在卡面末尾追加 `Source` 文本行；Pane 扫码账号不显示。
- 资源包明细：当 credits 包 ≥2 或任一条未来 7 天内到期，逐包输出一行
  `"{name} · {remain}/{total} left · expires {YYYY-MM-DD HH:mm}"`；
  7 天内额外 `· expires in Nd`；无法解析到期显示 `expiry unknown`。
- 包列表按到期时间升序（未知排最后），仅展示剩余 > 0 的包；总行仍用现有 Credits 计量条。
- 不新建 Dashboard、不改托盘布局。

**验收**
- [ ] 多包 fixture：排序、剩余、冻结、过期包被排除
- [ ] 7 天边界（正好 7 天 / 6 天 23 小时 / 8 天）
- [ ] 未知到期不编造
- [ ] 单包且不临期 → 不产生额外行（避免噪音）

## Comments

- 2026-10-19 出自 spec D7；上游 `credits.rs:35,199-297,680-732`。
- 2026-10-19 已实现：桌面/网页渠道追加 `Source` 行；`package_detail_rows`
  在多包或 7 天临期时逐包输出（剩余/总量/到期/倒计时/未知到期），按到期
  排序、只算剩余 > 0 的 credits 包；单包不临期时不产生噪音行。单测覆盖
  排序、边界与退化。
