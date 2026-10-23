# Reference borrowings — MCP/生产力扩圈与额度可观测性

Status: in progress — partial slices landed on `main`; next round order written 2026-10-23 14:31
Branch: `main` only (local; 51 commits ahead of `origin/main`, not pushed; no feature worktrees)
Source: owner instruction 2026-10-23; reference repos cloned under `temp/repos/`
Synthesis: `temp/research/20261023-reference-borrowings-synthesis.md`
Reference scan: `temp/research/20261023-reference-repo-clone-shortlist.md`
Next: `temp/handoff/20261023-1431-reference-borrowings-next-round.md`

## Objective

从 7 个参考实现里拿“结构”，不做产品复制。用一轮可验证的改造，把 Pane 从“coding agent 额度条”推进到“可计量 API 的额度层”：

1. **provider 接入成本下降**：新增 MCP/生产力 provider = 一条 descriptor/spec + fixture，不再改三处注册和一段手写 fetch。
2. **agent 能直接调用 Pane**：把现有 `127.0.0.1:6736` 只读 API 薄封装成 MCP server。
3. **额度语义可见**：pace 四态、重置事件、统一热度色，卡片/总览/API 共用一套事实。
4. **本地来源可扩**：`spend.rs` 拆出 SourceAdapter、probe/doctor、增量扫描，先补 Gemini/Copilot 等高价值源。
5. **成本不撒谎**：billing/provenance、stored/missing、时间感知定价；未知不显示为 0。
6. **Windows 第一表面**：托盘严重度染色、系统字体、投影缓存；不改现有模式默认值。

## Non-goals

- 不引入代理/网关；Pane 仍是只读聚合，不要求用户改流量路径。
- 不抓官网 dashboard DOM；只对 owner 自己的账号走 Pane 自有 webview。
- 不做 transcript/聊天归档、远程同步、多用户/云端；MCP 只查额度/用量/来源/状态。
- 不写 vendor 文件、不切号、不复制会话、不装 hooks 改用户配置。
- 不发布、不 push、不打 tag；owner UI 验收和 release gate 仍按项目规则。
- 不把没有额度接口的 provider 做成“看起来能查”的卡。

## Decisions

- 按 `CONTEXT.md` 的词汇：**额度**（有上限+重置窗口）、**用量**（已消耗绝对量）、**计量单位**（token/积分/分钟/字符/页/货币/次数）、**窗口**、**重置**、**余额**。
- 遵守 ADR 0003：按单位分节，**没有跨单位总数**；credits/分钟不与 token/美元折算。
- 单位扩展新增 `characters` / `pages` / `currency` 时必须带明确语义；currency 必须带币种码，不用自由字符串。
- MCP 只读：不暴露 refresh/test/credential/account mutation/transcript；结果带 `asOf`/`stale`/`ageSecs`；loopback-only，无 CORS，secret/PII 脱敏。
- Windows-first；macOS/Linux 不因本轮改造退化。
- 参考代码优先“读设计、重写实现”；MIT/Apache 代码如复用，保留版权与许可声明。`cockpit-tools` 无检测到的 LICENSE，只做设计参考。

## Reference scan

| Repo | Revision | License | 用途 |
|---|---|---|---|
| `temp/repos/CodexBar` | `575d6ad` | MIT | descriptor/fetch plan/typed error/API-key 策略/成本口径/状态页 |
| `temp/repos/QuotaBar` | `2dda67f` | MIT | window 模型/pace 四态/reset 事件/热度色/多表面 |
| `temp/repos/TokenTracker` | `56e21b4` | MIT | 46 工具探测/Windows 托盘/本地 dashboard/新增日志格式 |
| `temp/repos/ccusage` | `06347d3` | MIT | adapter 契约/价格语义/聚合模型/去重/快照测试 |
| `temp/repos/Claude-Code-Usage-Monitor` | `c59a83b` | MIT | 置信标签/预测公式/limit hit/脏值防御 |
| `temp/repos/agentsview` | `074b10a` | MIT | MCP service seam/ReadOnlyHint/asOf/增量索引 |
| `temp/repos/codeburn` | `1a3ce74` | MIT | probe/doctor/增量解析/成本归因/托盘 ambient |

已有参考（不重复拉）：`temp/openusage`（`26132ff`）、`temp/cc-switch`（`e4873bc`）、`temp/cockpit-tools`（`915e7a2`，设计 only）、`temp/magpie-yetone`（`8c68b68`）。

## Priority and issue map

| 优先级 | Issue | 方向 | 主要参考 | 依赖 | 现状（2026-10-23） |
|---|---|---|---|---|---|
| P0 | `01-provider-descriptor-apikey` | descriptor + 通用 API-key 额度适配器 + 单位扩展 | CodexBar | None | **部分**：adapter + 3 家试点 + Characters/Pages 已落；70 条目录能力位、脱敏 fixture、SSRF 测试待补 |
| P0 | `02-pane-mcp-server` | Pane MCP server over 6736，只读工具 + asOf | agentsview / codeburn | None | **完成**：`mcp/pane-mcp/` Node stdio shim，6 个只读 tool + 6 tests；Phase 2 `api_payload.rs` 未做；客户端接入待 owner 验 |
| P1 | `03-pace-reset-heat` | pace 四态、重置事件、统一热度色 | QuotaBar / Claude monitor | 01 的 descriptor 能力位可后置 | **部分**：pace/reset 检测/热度阈值/even-pace 竖线已落；just-reset、到点刷新、总览统一待补 |
| P1 | `04-spend-source-adapter` | SourceAdapter + probe/doctor + 增量扫描 + 新源 | ccusage / TokenTracker / codeburn | None | **部分**：`root/reason` 与 Gemini/Copilot 探测行已落；SourceAdapter、byte-offset、20 源迁移、真正解析器待做 |
| P2 | `05-cost-provenance-attribution` | billing/provenance、时间感知定价、项目/分支归因 | ccusage / codeburn | 04 | **部分**：billing/pricing mode + badge 已落；时间感知、stored/missing、override、归因待做 |
| P2 | `06-tray-ambient` | 托盘严重度染色、系统字体、投影缓存 | TokenTracker / codeburn / QuotaBar | None | **部分**：severity + 内存 last-good 已落；系统字体、磁盘持久化、125%/150% 验收待做 |

GitHub issues（2026-10-23 创建）：[#39](https://github.com/Aafff623/pane/issues/39)、[#40](https://github.com/Aafff623/pane/issues/40)、[#41](https://github.com/Aafff623/pane/issues/41)、[#42](https://github.com/Aafff623/pane/issues/42)、[#43](https://github.com/Aafff623/pane/issues/43)、[#44](https://github.com/Aafff623/pane/issues/44)。

## Current status and next order

执行顺序（owner 2026-10-23 确认，交主对话执行）：**04 (#42) → 05 (#43) → 01 (#39) → 03 (#41) → 06 (#44) → owner UI 验收 → release gate**。

- 04 → 05 是硬依赖：先稳定 SourceAdapter/pricing 结构，再做成本口径，避免迁移两遍。
- 01 不堵任何人：只碰 provider 层和 fixture，排第三顺手补完。
- 03/06 代码先收尾，UI 验收必须 owner 亲自做（远程会话不能签收）。
- 02 已完成，保持现状；#45 已记录扩展面决策（HTTP 6736 是事实源、MCP 薄适配、不做完整 Skill、CLI 暂缓），不要擅自扩写。
- push/tag/release/skin 线上部署均未授权；owner 验收前不合并、不发布。

每张 ticket 的剩余项、文件起点、验证命令和验收门在对应 `issues/*.md` 与交接件里。

## Workstream notes

### A. Provider layer (01)

- `ProviderDefinition` 增加最小能力位：`auth_kind`、`probe_kind`、`quota_scope`、`reset_semantics`、`pace_capability`。
- `ApiKeySpec` 只支持 GET + 固定 HTTPS host + header 模板 + JSON pointer + unit；禁止 user-controlled URL、重定向、超大 body。
- 先接 2 家试点：SerpApi（requests/月）与 Stability AI（credits/余额），验证“requests + credits”两种形态；DeepL 作为 characters 试点可放同一 ticket。
- fixture：`parse-tests/fixtures/<family>/` + pure parse + golden；禁止真实 key/账号标识。

### B. Agent surface (02)

- 先把 `httpapi.rs` 的 payload 纯函数抽到共享模块，HTTP/MCP/Tauri 共用一份 schema。
- Phase 1 用独立 `pane-mcp` stdio shim 调 6736，避免给 Tauri 引入新 HTTP 依赖；Phase 2 再评估 HTTP transport。
- tools：`list_providers`、`get_usage`、`get_provider_quota`、`list_sources`、`get_status`、`list_resets`（有则加）。
- 不暴露：refresh/test、keyvault、账号增删、原始 config、对话正文、email/maskedKey 默认不返回。

### C. Quota UX (03)

- `pace.rs`：`PaceState { ahead, close, over, spent, unknown }`；settled 门（窗口过 5% 且 ≥15 分钟）；`runOutAt` 只在会于重置前用完时给。
- `resets.rs`：跌幅 ≥10 点 + 重置时间已过/前移 ≥10 分钟或窗口 1/10；账号/凭证变化否决；到点后 20s 宽限刷新；just-reset 标记。
- 单一热度函数：卡片/总览/API 同一组数字；告警阈值独立。不要保留 60/75、80、90 三套热度。
- 年轻窗口必须 `unknown`，不能把噪声报成 Close/RunOut。

### D. Spend source layer (04)

- `SourceAdapter`：`paths` / `probe` / `parse` / `report`；`source_statuses()` 由注册表生成；`/v1/sources` 暴露 root/state/reason。
- 增量：size/mtime/hash + byte offset/checkpoint；先做 Codex/Claude，保留整文件回退。
- 新源优先级：Gemini CLI、Copilot 本地（`session-state`/`session-store`）；Codex `.jsonl.zst`、`token_usage_record`/compaction 去重。
- 不丢 Pane 独有：cc-switch、WorkBuddy CN/AI、CodeBuddy CLI/IDE、Qoder transcript、Trae CN、ZCode 路由。

### E. Cost semantics (05)

- `BillingMode`：subscription/API/local/payg/unknown；`PricingMode`：display/auto/calculate。
- per-model `stored_cost`/`missing_pricing`；`unpriced` 不变成 0；时间感知 `lookup_at`；用户 override。
- 可选 project/branch/session 归因：opt-in、只读 cwd/git 元数据、默认假名化、绝不读 prompt。
- 与已关闭 #33 的“实测/估算分开”保持一致，不重新引入混合口径。

### F. Tray ambient (06)

- 严重度 tint；系统字体数字；持久化最近投影；保留现有模式为默认。
- 不做桌面小组件、Dock、宠物、成就、排行榜。

## Verification

- `corepack pnpm build`
- `cargo +stable-x86_64-pc-windows-gnu test --manifest-path parse-tests/Cargo.toml`（串行、无代理）
- `node --test tests/*.test.mjs`
- 01/02 额外：security review（SSRF/redirect/size/secret redaction）；MCP 只读测试。
- 03/06 额外：owner UI 目检（卡片 tick、reset 标记、托盘 tint）。
- 04/05 额外：现有 20 个 spend 源回归样本 + 扫描耗时对比。
- 不运行真实 key 的网络测试；fixture 全部脱敏。

## Out of scope

- 代理/网关、dashboard scraping、跨单位总数、provider 数量填充。
- transcript/聊天归档、remote sync、多用户/云端、写 vendor 文件、切号/会话复制。
- 发布、push、tag、deploy；owner acceptance 前不合并。

## Handoff

- **当前有效**：`temp/handoff/20261023-1431-reference-borrowings-next-round.md` — 冷启动事实、逐 ticket 剩余项、执行顺序、文件起点、验证命令、owner 验收门、可直接粘贴的主对话启动 prompt。
- **历史**：`temp/handoff/20261023-reference-borrowings-handoff.md` — 调研阶段的原始交接；其中“独立 worktree / 基于 origin/main”的建议已被 2026-10-23 的 main 合并覆盖，读现状以新件为准。
