# 02: 积分小节跟随时段 tab（一账号一行、0 值不显示、名字用备注名）

**What to build:** 花费面板底部的「积分」小节与面板其他行口径一致：按当前时段 tab 取窗口、窗口内按账号求和、一账号一行、合计为 0 的行不显示、名字用面板既有的备注名机制、单行布局（数字右对齐、名字过长省略号）；删掉不受 tab 约束的「本周期已用」行及其数据字段。完成后在真机上切换 今天 / 昨天 / 7 天 / 30 天 / 全部，积分小节应逐 tab 变化，且不再出现 `workbuddy@1aaac1cc…` 这类内部 id。

**Blocked by:** 01（纯函数与窗口求和）

**Status:** needs-info

- [ ] 后端返回**每账号的窗口合计**（今天 / 昨天 / 7 天 / 30 天 / 全部），口径与 token / 美元侧一致；不再返回逐日明细给前端
- [ ] `CreditExtra` 移除 `latest` / `latest_day`；前端「本周期已用」渲染与三语 `spend.creditsUsed` 一并删除（核实过无其他消费方）
- [ ] 前端按当前 tab 选窗口渲染：一账号一行、数字右对齐
- [ ] 合计为 0 的账号整行不渲染
- [ ] 名字用既有 `notedName(id, providerDisplayName(providerFamily(id)))`：用户设过的 ⚙ 备注名优先，其次 provider 显示名
- [ ] 单行布局 + CSS 原生省略号（不截字符串、不算长度）；`.spend-credits` / `.spend-credit-*` 选择器组改造，去掉按天分组容器
- [ ] tooltip 文案改为「跟随所选时段」，中 / 英 / 俄三语同步；旧注释里「不随时段切换」的说明同步改写
- [ ] 多账号：一账号一行，不合并（**待 owner 拍板**；若改为合并，本项与 01 的求和粒度一起调整）
- [ ] 「全部」窗口：账本全部已记录日（**待 owner 拍板**）
- [ ] 积分仍不折算成 token / 美元、不进合计（ADR 0003 口径不变）
- [ ] 验证：tsc + vite build 通过；dev 真机切换五个 tab 逐一看过；分享 PNG 与面板一致；UI 验收由 owner 本人完成
- [ ] 单读数、无前值的账号（如本机 Qoder CN 只有一条 10-10 读数）不显示——属「无数据」，不得当成缺陷修回去

## Comments

- 2026-10-14：由 spec `.scratch/credits-follow-period-tabs/spec.md` 拆分。本票是用户可见行为的完整切片；01 是它的 prefactor。
- 相关：`.scratch/codebuddy-family/issues/04-workbuddy-tenant.md`（done）——WorkBuddy 卡即按 credits 计量，本票对它直接生效。
