# 05 — 成本口径与归因：billing、stored/missing、项目/分支

Status: in progress (2026-10-23) — stored/missing provenance, override file, time-aware lookup and unpriced UI landed; project/branch attribution and Currency still pending
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

- 2026-10-23（主对话接手）：`ModelSpend` 增加 `stored_cost` / `missing_pricing`；`FileData` 增加 per-model `stored`，Claude costUSD、Pi、OpenCode、MiniMax/Hermes `cost_usd` 走 `add_stored_event`；未定价模型不再被 top-5 折叠隐藏，美元显示为 `unpriced` 而不是 `$0`，spend 行增加 unpriced 徽标。
- `pricing::lookup_at(timestamp_ms, model)` + `PriceProbe::LookupAt` 已接 Codex/Copilot 计价；缓存按同一时间戳复验。时间窗表默认为空：没有第一方历史价目，不编造 DeepSeek 时段价、web search 或 flex/priority 费用。
- 用户 override：`%APPDATA%\Pane\pricing_overrides.json`，USD/百万 token，优先级 `stored > override > catalog > unknown`；`lookup()` 与 `lookup_at()` 都会先查 override。没有做设置页编辑器，文件是当前入口（需重启生效）。
- 测试：`pricing::tests::lookup_at_prefers_override_then_window_then_catalog`、`spend::tests::stored_cost_and_missing_pricing_survive_finalize`、`split_models_carries_stored_cost_with_the_rows` 通过；node 96 passed；`pnpm build` 通过。
- 全量 `parse-tests` 最终：1017 passed / 0 failed / 14 ignored（串行、去代理，444.95s）。

- 未做/待 owner：project/branch/session 归因（opt-in、不读 prompt）尚无 UI 与数据面，保持关闭；Currency 按 ADR 0003/owner 决定继续延后；设置页 override 编辑器、真实 key 的成本对比、四态徽标视觉验收均未验证。
- 2026-10-23（收尾）：设置页模型价格覆盖编辑器已补到浮窗和面板 General 页；新增 `get_pricing_overrides` / `save_pricing_overrides`，严格校验 JSON（input/output 必填、不能为负），保存即写 `pricing_overrides.json` 并清空定价 memo，无需重启。`override_editor_validates_required_rates` 通过，`pnpm build` 通过。剩余仍是 project/branch 归因、Currency、真实历史价目表。
- 技术冒烟（2026-10-23）：面板 Settings → 通用实际渲染“模型价格覆盖”，textarea 读到 `{}`，点保存返回“已保存，新价格立即生效。”；后端 `parsed_overrides` 校验测试通过。owner 视觉验收仍待做。

- 2026-10-24（尾巴裁决）：project/branch/session 归因需要新的数据面 + opt-in UI，且默认假名化后展示价值要重新定义；本机日志没有稳定 cwd→展示名映射，未在未确认的情况下新增。Currency 继续按 ADR 0003 与 owner 既有决定延后。真实 key 的成本对比也没有可复核的第一方价目，不编造。此票保持 partial，不标 done。
