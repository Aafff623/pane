# 02 — 「窗口未起步」提成 metric 级标志，五个表面消费同一结论

**What to build:** 让「这一档压根没用过」在卡片、概览环、折叠卡徽标、悬停提示和 `/v1/usage` 五处给出
**同一个**结论，并且覆盖 weekly/monthly 档，修掉未使用窗口每次刷新倒计时跳回满窗的误导显示。
**Blocked by:** 无
**Status:** done (取证已完成，代码与回归已落) — owner 目检：未使用的长窗口四处不显示倒计时
**GitHub:** [#48](https://github.com/Aafff623/pane/issues/48) §2 · **Priority:** P1 · **量:** 中

## 要点

1. **现状：判定已存在，但只覆盖一处半。** `src/main.ts:2434-2455` 的 `notStarted`：

   ```
   条件 = period_ms <= 6 * 3_600_000 && used <= 1
        && resets_at - now >= period_ms - max(60_000, period_ms / 100)
   ```

   漏网的四处：
   - 概览环 `main.ts:4809` 直接取 `best.resets_at`，渲染在 `:5233-5243` 与 `:5286-5293`，无该判定；
   - 折叠卡徽标 `main.ts:3413` 走 `nearestResetSeconds`（`:2782-2815`），那里只筛
     `resets_at !== null`，**完全不看用量**；
   - 悬停提示 `main.ts:2982-2984`；
   - `src-tauri/src/httpapi.rs:204`/`:215` 原样吐 `resetsAt`，走 HTTP API 的 agent 与脚本拿到的是
     未过滤结论。

2. **标志位落在 Rust 侧**，不要只在前端补 if。理由：只在前端修，`/v1/usage` 会继续输出会跳窗的时间，
   等于把同一个错留第二遍。建议在 `src-tauri/src/pace.rs` 的 `attach`（`:93` 附近）写一个
   `not_started: bool` 进 `Metric`（`#[serde(default)]`，纯增量），前端五处与 HTTP 一起消费。

3. **与 #41 的 just-reset 必须不打架。** 刚重置完的窗口也是「用量 0 + resets_at 在未来」，
   但它该显示倒计时而不是「未开始」。grace 语义要和 `resets::is_reset` / `just_reset_at` 对齐，
   明确优先级：刚重置 > 未起步 > 倒计时。

4. **文案复用现成的**：`card.notStarted` / `card.notStartedTip` 已存在，五处用同一个 key，
   不要再写第二套措辞。

## 开工前先取证（本票唯一的前置研究）

上游 #8631 修的是 **Codex 的 5 小时窗口**：用量为 0 时它返回的重置时间是「现在 + 一个窗口长度」。
本票要把这个判定推广到 weekly/monthly，**必须先确认长窗口是否也有同样行为**，否则会把「真的还剩 7 天」
误标成「未开始」，那比现在更糟。

取证方式（只读，不改任何数据）：
- 看 `%APPDATA%\Pane\last_snapshots.json` 里各家 `period_ms > 6h` 的 metric，在 `used_percent` 为 0
  或 1（Codex 未触碰窗口会报 1%）时，`resets_at - now` 是否恰好 ≈ `period_ms`；
- 连续两次刷新（间隔 ≥1 分钟）比较同一档的 `resets_at`：若**每次都前移回满窗**，即同现象；
- 若本机没有未使用的长窗口，标注「无法取证」，本票范围收窄到 6 小时档 + 其余四处消费点，
  长窗口另开票。

## 取证材料 / 复现步骤

- 折叠徽标这一处最容易当场看出：把某张卡折叠，看它徽标上的倒计时是否来自一个用量 0% 的周档。
- 概览环：找一个未使用的长窗口，看环上「还剩 X 重置」是否等于满窗长度。
- HTTP：`curl http://127.0.0.1:6736/v1/usage`，确认修复后同一 metric 带 `notStarted`（或等价字段）。

## 验收

- [ ] 取证结论写进 Comments（长窗口是否同现象；若否，本票范围如何收窄）
- [ ] `not_started` 判定只有一个实现处；卡片/概览/折叠徽标/悬停/HTTP 五处消费同一结论
- [ ] 与 just-reset 的优先级有测试覆盖（刚重置的窗口不误标为未起步）
- [ ] 未使用窗口不再显示「还剩满窗」的倒计时；已使用窗口行为不回归
- [ ] `grace` 边界测试：`resets_at` 略旧于窗口长度时判为已起步
- [ ] `pace::` 定向测试 + 前端 node 测试 + `pnpm build` 通过
- [ ] owner 目检：一个未使用的 weekly（或 monthly）窗口，在四处表面都不显示倒计时

## Comments

- 2026-10-23：立项，证据来自 #48 与 `temp/research/20261023-ccswitch-usage-gap.md` §9.1-2。
- 2026-10-23 **取证完成（D2 结掉）**：读本机 `%APPDATA%\Pane\last_snapshots.json`，`antigravity / Claude Weekly`（period 168h）在 `used_percent = 0` 时 `resets_at − now = 168.0h`，与同卡 5h 档同形 —— 长窗口确实会回满窗倒计时，扩到所有档有实测依据，不再是外推。反例同样存在：`sensenova / Credits Weekly` 用量 0% 但只剩 70.3h（168h 档已走 98h），这类真在窗口中途的行**必须继续显示倒计时**。
- 2026-10-23 实现：判定收进 `pace::compute`，条件是「用量 ≤1% **且** `|resets_at − now − period| ≤ 60s`」——比上游 #8631 的「用量为 0 就隐藏重置时间」多一道回声识别，正是上面那条反例逼出来的。结论落在 `Pace::not_started`（序列化随 pace 走，`/v1/usage` 自动同结论）。前端删掉卡片脚上那条自己写的 `period_ms <= 6h && used <= 1` 回声算式，`nearestResetSeconds`、`formatHoverReset`、概览环 meta、折叠徽标 meta、汇总行、待重置列表、以及 `data-reset-at` 实时计数钩子，全部改读 `metricNotStarted(m)` / `quota.notStarted`。
- 保留的一处长度比较：`resets_at === null` 的分支（厂商压根没给重置时刻）仍按窗口类型回落到「未起步 / 无重置时间」，那是缺数据的兜底，不是同一判断，已在代码里注明。
- 回归：`pace::tests::untouched_window_is_flagged_at_any_length`（5h/24h/168h/720h 全档 + Codex 下限 1%）、`pace::tests::mid_flight_zero_usage_window_stays_a_countdown`（本机反例形状）、`tests/not-started-window.test.mjs`（禁止本地回声算式复活 + 消费点计数 ≥8）。顺带给 `tests/card-review-regressions.test.mjs` 的函数沙箱补了 `metricNotStarted`，否则抽出的 `extractOverviewQuota` 取不到该符号。Node 全套 129 通过。
