# 02: 积分小节跟随时段 tab（一账号一行、0 值不显示、名字用备注名）

**What to build:** 花费面板底部的「积分」小节与面板其他行口径一致：按当前时段 tab 取窗口、窗口内按账号求和、一账号一行、合计为 0 的行不显示（全为 0 时整节消失）、名字用面板既有的备注名机制、单行布局（数字右对齐、名字过长省略号）；删掉不受 tab 约束的「本周期已用」行及其数据字段。完成后在真机上切换 今天 / 昨天 / 7 天 / 30 天 / 全部，积分小节应逐 tab 变化，且不再出现 `workbuddy@1aaac1cc…` 这类内部 id。

**Blocked by:** 01（纯函数与窗口求和）——frontier 规则：01 完成后才可领

**Status:** done（落地 main，真机验证通过；UI 终验归 owner）

- [x] 后端返回**每账号的五个窗口合计**（今天 / 昨天 / 7 天 / 30 天 / 全部），窗口定义与 token 侧逐字一致；不再把逐日明细交给前端
- [x] `CreditExtra` 移除 `latest` / `latest_day`；前端「本周期已用」渲染与 `spend.creditsUsed`（三语）一并删除
- [x] **不要误删 `metric.creditsUsed`**（卡面指标文案，另一个键，i18n 672 / 1711 行附近）——本次不动卡面
- [x] 前端按当前 tab 选窗口渲染：一账号一行、数字右对齐
- [x] 合计为 0 的账号整行不渲染；**该时段全部账号为 0 时，整个「积分」小节（含标签）不渲染**，不留空壳标题
- [x] 行顺序按账号显示名排序：同一家的多账号相邻，且每次渲染顺序稳定
- [x] 名字用既有 `notedName(id, providerDisplayName(providerFamily(id)))`：用户设过的 ⚙ 备注名优先，其次 provider 显示名
- [x] 单行布局 + CSS 原生省略号（不截字符串、不算长度）；`.spend-credits` / `.spend-credit-*` 选择器组改造，去掉按天分组容器与日期子元素
- [x] tooltip 改写为「跟随所选时段」语义，中 / 英 / 俄三语同步（`spend.creditsTip`）；旧注释里「不随时段切换」的说明同步改写
- [x] 多账号：一账号一行，不合并（已拍板）
- [x] 「全部」窗口：账本全部已记录日（已拍板）
- [x] 积分仍不折算成 token / 美元、不进合计（ADR 0003 口径不变）
- [x] 单读数、无前值的账号（如本机 Qoder CN 只有一条 10-10 读数）不显示——属「无数据」，不得当成缺陷修回去
- [x] 验证：tsc + vite build 通过；**重建官网 demo 包**（`node site/scripts/build-demo.mjs`，改了 main.ts/i18n.ts 会触发 provenance 闸门）并跑 `node --test site/*.test.mjs`；dev 真机切换五个 tab 逐一看过；分享 PNG 与面板一致；UI 验收由 owner 本人完成

## Comments

- 2026-10-14：由 spec `.scratch/credits-follow-period-tabs/spec.md` 拆分。本票是用户可见行为的完整切片；01 是它的 prefactor。
- 相关：`.scratch/codebuddy-family/issues/04-workbuddy-tenant.md`（done）——WorkBuddy 卡即按 credits 计量，本票对它直接生效。

- 2026-10-14 实现并真机验证（dev 应用 + 注入合成账本）：五档逐一切换——今天 60/30/150、昨天只剩 40（其余 0 值账号隐藏）、7 天与 30 天 160/30、全部 160/30，均与后端窗口合计逐一吻合；备注名生效（注入长备注的账号按备注显示）；超长名 clientWidth 288 / scrollWidth 680 + computed ellipsis 确认省略号；恢复真实账本（全 0）后整节消失无空壳。tsc + vite build 绿，官网 demo 包已重建，site 测试 13/13。验证脚本：temp/scripts/credits-verify-fixture.mjs、credits-verify-ui.mjs。
