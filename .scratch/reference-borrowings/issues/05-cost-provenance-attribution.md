# 05 — 成本口径与归因：billing、stored/missing、项目/分支

Status: in progress — billing/pricing modes and badges landed; time-aware/override/attribution pending
GitHub: #43
Priority: P2
Blocked by: 04
Reference: `temp/repos/ccusage` (`core/src/pricing.rs`, `cost.rs`, `types.rs`), `temp/repos/codeburn` (`src/models.ts`, `src/branch-spend.ts`), `temp/repos/CodexBar` (`CostProvenance.swift`)

## Scope

Pane 的 `Total Spend` 目前把订阅内用量也按 API 价计入，缺少 billing 语义；`unpriced` 有标记但不够结构化；没有 project/branch/task 归因。本轮把“这笔钱是厂商计量、API 等价、估算还是未知”说清楚。

1. **BillingMode**：`subscription | api | local | payg | unknown`，落在 `ProviderSpend`/日桶/模型行。
2. **PricingMode**：`display | auto | calculate`；`stored_cost`（日志自带）优先，`missing_pricing` 不变成 0。
3. **时间感知定价**：`lookup_at(timestamp, model)`，处理 DeepSeek 时段价/价格变更；保留 Pane 现有 LiteLLM + models.dev + supplement + PriceProbe。
4. **补费用项**：web search（如 $0.01/次）、flex/priority、长上下文倍率；只在有官方依据时启用。
5. **用户 override**：设置页可为私模/中转模型指定价格；优先级明确（stored > override > catalog > unknown）。
6. **可选归因**：opt-in 读 cwd/git 元数据，产出 project/branch/session 维度；默认假名化，绝不读 prompt。
7. **UI/分享图**：徽标区分“厂商计量 / API 等价 / 估算 / 未知”；未知不显示为 $0。

## Acceptance

- [ ] 同一张卡的总额能说明口径：vendor-metered / API-equivalent / estimated / unknown 至少四态。
- [ ] `missing_pricing` 的 token 仍进 token 总量/趋势，美元留空并标 `unpriced`；不伪造价格。
- [ ] 时间感知定价测试：同一模型跨价格档期取对价格；override 优先级测试。
- [ ] web search/flex 费用有官方依据才启用；没有依据时保持关闭。
- [ ] 归因 opt-in、默认假名化；关闭时行为与现在一致。
- [ ] 与已关闭 #33 的“实测/估算分开”一致；不重新引入混合口径。
- [ ] `parse-tests` + `pnpm build` 全绿。

## Evidence / notes

- ccusage `core/src/pricing.rs` 有 LiteLLM/models.dev 快照 + ETag + exact-only + alias + override + fast/flex + DeepSeek 时段价；`ModelBreakdown` 带 `missing_pricing`/`stored_cost`。
- codeburn `src/models.ts` 有 `BillingMode`、web search $0.01/次、flex/priority、长上下文；`branch-spend.ts` 有 project/branch 归因。
- CodexBar `CostProvenance.swift` 的 listPrice/vendorMetered/mixed/unknown + priced/unpriced/unmetered/estimated 可直接借命名。
- Pane 现有 `pricing.rs` 的目录/缓存比参考仓库更强，不要为了“统一”降级。

## Out of scope

- 发票、税率、按量结算、预算审批流。
- 默认开启项目/分支归因；读取 prompt/代码内容。
- 把 credits/分钟折算成美元/token（ADR 0003）。

## Verification

```powershell
corepack pnpm build
cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu test
```

Owner 目检：总额 tooltip/分享图的四态徽标；关闭归因时无变化。

## Comments

- 2026-10-23：`ProviderSpend` 增加 `billing_mode`（subscription/api/local/payg/unknown）和 `pricing_mode`，按 family/API-key 能力派生；spend 行显示口径 badge。时间感知定价、stored/missing 细节、用户 override、project/branch 归因尚未做。
