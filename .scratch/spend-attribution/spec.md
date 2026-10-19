# 花费归因与模型明细（ZCode / Others）

**Status:** ready-for-agent
**Feature dir:** `.scratch/spend-attribution/`
**Date:** 2026-10-19
**来源:** 用户 2026-10-19 反馈：总花费面板出现 `Step 5 Preview`、`Group Context Default`、`Other`，且看不到逐 provider 的模型 ID。

---

## 1. 已核实的事实

- `Other` 是甜甜圈的小花费折叠：成本模式下单日 < $5 或 30 天 < $10 折成一个楔形（`src/main.ts:3184-3255`）；悬浮只列 provider 名，不列模型。
- `Step 5 Preview` 是真实模型 `step-5-preview`，ZCode 的 `new-provider` 消耗；ZCode 配置里该 provider 叫 `🐑StepFun`（`~/.zcode/v2/provider_config.json` 的 `providerRules`）。
- `deepseek-v4.1-flash-1017` 是 ZCode `new-provider-4` 的消耗，provider 名 `基元律动`。
- Pane 的 ZCode 扫描器只按 ZCode 一张卡记账（`spend.rs:zcode()`），**没有读** `providerRules` 的 `providerId → providerName`，所以所有经 ZCode 的第三方额度都被吞进 ZCode 卡。
- `Group Context Default` 在本机所有 Pane 花费数据源中不存在；只出现在用户当前消息里，疑似 ZCode 自己的用量面板分组。

## 2. 目标

1. ZCode 消耗按真实 provider 归因：`StepFun / 基元律动 / cline / 万象中转站 / 商汤 / MiniMax / DeepSeek` 等分别落到对应卡；映射不到时留在 ZCode 卡但显示 `providerName · modelId`。
2. 模型标签统一为 `Provider · model`（必要时附显示名），不再出现裸 ID 无法辨认来源的情况。
3. `Others` 楔形可展开：列出每个被折叠 provider 及其模型明细；或在成本模式提供「显示全部」开关。
4. Credits 与 Tokens 仍不混算。

## 3. 设计

### D1 — 读取 ZCode provider 映射

`~/.zcode/v2/provider_config.json` → `config.providerConfigRules.providerRules[]`：
- `providerId`、`providerName`、`config.api.baseUrl`、`config.access.type`。
- 只读；文件缺失或损坏时退回「全部归 ZCode」。

### D2 — 归因规则

- `baseUrl` host 映射到 Pane family：
  - `tokenrhythm.studio` → `tokenrhythm`
  - `token.sensenova.cn` → `sensenova`
  - `api.stepfun.com` → `stepfun-plan`
  - `api.cline.bot` → `clinepass`
  - `api.minimax.cn` → `minimax`
  - `api.deepseek.com` → `deepseek`
  - 其余 → ZCode 卡（但模型标签带 providerName）
- 同一模型在 ZCode 卡与目标卡不得重复计数：ZCode 扫描器把这些行**移出** ZCode 卡。
- 目标卡可能已有自己的 spend 源（如 MiniMax Agent 的 SQLite），两者是不同客户端，允许并存；本轮只做归因，不合并。

### D3 — 模型标签

- 归入目标卡时：`model_id` 原样（目标卡已代表 provider）。
- 留在 ZCode 卡时：`providerName · model_id`。
- 价格目录的 display name 作为 hover 补充，不替换 ID。

### D4 — Others 展开

- `showSpendPop` 对 `__others__` 额外渲染每个 part 的模型行（受现有 tip 的条目上限约束）。
- 新增设置项 `spendShowAll`（默认 false）：成本模式下关闭折叠。

## 4. 工单

| # | 内容 |
|---|---|
| 01 | ZCode provider 归因（读 providerRules + 分卡 + 不重复） |
| 02 | 模型标签与 Others 展开/显示全部 |

## 5. 验收

- StepFun 的 `step-5-preview` 出现在 StepFun 卡；基元律动的 `deepseek-v4.1-flash-1017` 出现在基元律动卡。
- ZCode 卡不再包含已归因 provider 的 token。
- `Others` 悬浮能看到被折叠 provider 的模型明细。
- 总 token 数在归因前后不变（只换卡，不增删）。
