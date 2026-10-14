# 积分小节跟随花费面板的时段 tab

**Status:** ready-for-agent（owner 已拍板两条，见「已拍板」节）
**Feature dir:** `.scratch/credits-follow-period-tabs/`
**Date:** 2026-10-14（首稿）／2026-10-14 复审补缺
**来源:** 用户 handoff（2026-10-14）+ ZCode 两轮只读核实（渲染块、Rust 数据源与窗口先例、i18n、CSS、demo bridge、token 侧窗口实现）

## Problem Statement

花费面板底部的「积分」小节（Qoder CN、WorkBuddy 等按积分计费的 provider）有两个毛病：

1. **显示冗长**：名字显示成内部卡 id（如 `workbuddy@1aaac1ccd6737be2ad12d2e731baff5`），一行折成好几行；同一个名字在每行重复一遍。
2. **不跟随时段 tab**：选「今天」照样列出 10-10 / 10-13 / 10-14 的数据；消耗为 0 的条目也被列出来——实际截图里 4 行只有 1 行是真数据，其余 3 行是 0。

用户看到的是：面板里其他每一行（token / 美元）都随 tab 变，只有这一节纹丝不动、又长又重复，读不出「今天花了多少积分」。

## Solution

把积分当作 **tokens 的另一种单位**：同一面板、同一套时段聚合、零消耗滚出去。具体：

- 按所选 tab 取窗口（今天 / 昨天 / 7 天 / 30 天 / 全部）；
- 窗口内**按账号求和，一账号一行**；合计为 0 的行不显示，一行不剩时整个小节（含「积分」标签）不渲染；
- 名字走面板既有的备注名机制（用户自己设过 ⚙ 备注名的优先）；
- 单行布局：名字在左、数字在右，名字过长用 CSS 原生省略号收尾；
- 删掉不受 tab 约束的「本周期已用」行（它留着会和 tab 打架）。

## User Stories

1. As a 看花费面板的用户, I want 积分小节跟随「今天/昨天/7天/30天/全部」tab 取窗口, so that 它和上面每一行 token/美元的口径一致，不用在心里换算
2. As a 用户, I want 选「今天」时只看到今天的积分消耗, so that 我不用在一堆历史日期里找今天
3. As a 用户, I want 消耗为 0 的账号不显示, so that 有数据的行不被 0 值行稀释
4. As a 用户, I want 所有账号在该时段都是 0 时整节消失, so that 面板不出现一个空壳标题
5. As a 有多个 WorkBuddy/ClinePass 账号的用户, I want 每个账号各占一行, so that 我能看出是哪个账号在消耗（各账号额度池与周期不同，合并会造出假总数）
6. As a 有多个账号的用户, I want 同一家的账号行相邻、顺序稳定, so that 每次刷新不会跳来跳去
7. As a 用户, I want 行首显示我认得的名字（我设过的备注名优先，其次是 provider 显示名）, so that 不再出现 `workbuddy@1aaac1cc…` 这种内部 id
8. As a 用户, I want 名字过长时尾部省略号而不是折行, so that 每行保持一行高度、数字列对齐
9. As a 用户, I want 数字靠右、与上方 token 行同样的对齐, so that 扫读时不用左右跳
10. As a 用英文/俄文界面的用户, I want 积分小节的说明文案是本地化的, so that tooltip 不夹生、不出现「不随时段切换」的过时说明
11. As a 用户, I want tooltip 说明「跟随所选时段」, so that 我不再以为这一节是固定的
12. As a 用户, I want 只有一个读数、还没有前值可比时该账号不显示, so that 不会把累计值当成一天的花销（这是刻意的，不是缺数据）
13. As a 用户, I want 我的 token/美元合计与卡面不受积分改动影响, so that 修这一节不动其他口径
14. As a 分享面板截图的用户, I want 分享 PNG 与面板一致, so that 对外展示不出现另一种排版
15. As a 维护者, I want 窗口求和是可单测的纯逻辑, so that 以后改窗口定义时能立刻验证
16. As a 维护者, I want 这次改动带着回归测试, so that 「积分小节不跟 tab」「0 值行」「单读数误报」这三类问题不会复活

## Implementation Decisions

- **窗口口径与 token 侧同源，且全部落在后端**（核实：token 的今天/昨天/last30 来自 `fetch_spend` 的按日桶，7 天与全部来自 `fetch_spend_history` 的 d7/all 行——即 token 侧本来就是后端聚合）。积分不再由前端逐日遍历，改为后端返回**每账号的五个窗口合计**，前端只按当前 tab 选对应窗口。
- **窗口定义必须逐字对齐 token 侧**（否则同一 tab 下积分与 token 的数字会互相矛盾）：今天 / 昨天 = 本地日的日偏移桶；7 天 = 含今天的 7 日滚动窗口；30 天 = 含今天的 30 日窗口（与 token 侧 last30 同义）；全部 = 账本全部已记录日。today 取与 `credit_trend_map` 同一个本地日来源（`today_string()`）。
- **数据形状**：`CreditExtra` 从 `{ latest, latest_day, days[] }` 改为携带窗口合计（今天 / 昨天 / 7 天 / 30 天 / 全部）。`latest` / `latest_day` 字段与「本周期已用」渲染一并删除（核实过：前端只有这一处消费，删除后无其他引用）。
- **抽纯函数**：日差计算与窗口求和拆成纯函数，形态对标同文件已有的 `credit_trend_for`（入参 = 账本条目 + today，返回可直接断言的数字），从而进入 parse-tests 覆盖范围。
- **前端只做选择与渲染**：按当前 tab 选窗口 → 一行一账号 → 名字用既有 `notedName(id, providerDisplayName(providerFamily(id)))`（面板里已有三处同样写法）→ 0 值行不渲染、全 0 则整节不渲染 → 单行 flex 布局 + CSS 原生 `text-overflow: ellipsis`（不测长度、不截字符串）。
- **行顺序**：按账号显示名排序（同一家的多账号自然相邻，且每次渲染顺序稳定）。
- **样式**：沿用 `.spend-credits` / `.spend-credit-*` 这一组选择器，改成「名字左（可收缩、省略号）+ 数字右（不收缩、tabular-nums）」的单行结构；去掉按天分组容器与 `.spend-credit-day` 这类日期子元素。
- **i18n**：改写 `spend.creditsTip`（tooltip 语义从「不随时段切换」改为「跟随所选时段」），中 / 英 / 俄三语同步；**删除 `spend.creditsUsed`（本周期已用）**。注意：`metric.creditsUsed`（卡面指标文案，i18n 的 672 / 1711 行）是**另一个键**，不在本次范围，别误删。`spend.credits`（积分）/ `spend.creditsUnit`（积分）保留。
- **不做折算**：积分仍不折算成 token 或美元、不进合计（沿用 ADR 0003 的口径）。
- 本改动直接作用于 WorkBuddy / CodeBuddy 家族（卡面即按 credits 计量）与 Qoder CN / Trae CN。

## Testing Decisions

- **唯一新 seam = Rust 纯函数**（日差 + 窗口求和）。对标同文件 `credit_trend_for` 的既有测试形态：喂固定账本条目 + 固定 today，断言各窗口数字。
- 用例至少覆盖：① 五个窗口各自的求和；② 窗口边界（今天 0、昨天 >0 时今天不显示而昨天显示）；③ 0 消耗行被过滤、全 0 时无行可渲染；④ **单读数无基线 → 各窗口全 0（不显示）且必须标注「不是回归」**；⑤ 多账号互不串行（同一 provider 两个卡 id 各自求和）；⑥ mid-day 重置的日差算法保持不变（沿用现有语义）。
- 前端该区块**现状测试覆盖为零**；本次不新建前端测试 seam（项目无前端单测载体）。验证方式 = tsc + vite build + dev 真机（读真实账本）+ 官网 demo 截图；**UI 验收由 owner 本人做**。
- 先补红：新增纯函数测试在实现前应失败（旧实现没有该函数），实现后转绿。不得改断言、降规则或跳过检查来让结果变绿。
- 改完必须重建官网 demo 包（`node site/scripts/build-demo.mjs`）并跑 `node --test site/*.test.mjs`——改了 `main.ts` / `i18n.ts` 会让 provenance 闸门变红。

## Out of Scope

- 把积分折算成 token / 美元或并入花费合计（ADR 0003 明确不做）。
- **花费详情浮层里的积分趋势图**（`fetch_credit_history` → 30 天逐日折线 + `spend.creditTrend` / `spend.creditTrendTip` 文案，以及卡片在无 token 花费时回落到 credits 趋势的那条路径）——那是另一个 surface，本身按天呈现，本次不动。
- 卡面上的 credits 计量条（`.scratch/codebuddy-family/issues/02-credits-card-face.md` 已完成）。
- 官网 demo 的积分 fixture（demo 目前不提供 `fetch_credit_extras`，本次不改；见 Further Notes）。
- 账本采集频率、历史深度、积分对账等数据侧问题。

## 已拍板（2026-10-14，owner）

1. **同一家多个账号：各占一行**（不合并）——各账号额度池与周期不同，合并会造出假总数。
2. **「全部」窗口：账本全部已记录日**——与 token 侧「全部」同义；消耗为 0 的日子不改变合计。

（owner 另外明确过的口径：积分跟 tokens 完全对齐——同一面板、同一 tab、同一套聚合、零就滚出去；不为「首条读数无基线」这类特例设计新分支。）

## Further Notes

- **Qoder CN 账本只有 10-10 一条读数**：没有前值可 diff，按新规则不会显示。这是「无数据」，不是 bug——代码刻意不让首个读数充当当日消耗（避免把累计值算成一天的花销）。测试要覆盖这条并注明，防止后人当回归处理。
- `credit_extras()` 读真实账本文件，没法直接单测；同文件 `credit_trend_for` 是纯函数先例，正是因此要求把日差与窗口求和抽成纯函数后再测。
- 该区块此前的设计注释与 tooltip 明写「不随时段切换」——那是旧的设计决定，本次是**有意的口径变更**，注释需同步改写，避免后人以为是 bug 修回来的。
- 相关票：`.scratch/codebuddy-family/issues/04-workbuddy-tenant.md`（done，WorkBuddy 卡按 credits 计量，本次直接生效）。
