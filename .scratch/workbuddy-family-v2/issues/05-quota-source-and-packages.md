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
- 2026-10-19 已实现（**实机修正后**）：桌面/网页渠道追加 `Source` 行；资源包
  明细改为「临期单独行 + 其余汇总行」——7 天内到期的包各一行（最多 3 行），
  其余合成 `N packages · X credits left · next expiry …`。原因：真实账号有
  13 个 credits 包，逐包一行会把卡片撑成 13 行；修正后在 `127.0.0.1:6736`
  实测为 `Credits` + `Packages` 两行。单测覆盖单包、双包、临期、13 包、已耗尽。
