# Spec: magpie 思想借鉴批次（反事实重放 / 百分比反推 / 代码版本哈希缓存）

> 用户确认的三个 magpie 思想级借鉴（2026-10-25）。证据：temp/research/20261025-magpie-deep-dive.md §一「最被低估的 3 个」与 §二。三个功能彼此独立，可并行。

## 01 反事实重放（counterfactual replay）

**magpie 做法**（internal/tune/tune.go）：用用户历史调用逐条重放"若压缩时机/缓存 TTL 不同会花多少 token"，给出最优参数。

**Pane 只读版（分三步）**：
1. 数据可行性盘点：zcode `~/.zcode/cli/db/db.sqlite` turn_usage（每轮聚合 token，schema 见 temp/repos/ZCode session-store/schema.baseline.sql:283）；Claude Code `~/.claude/projects/**/*.jsonl`（每请求 token+ts+isCompactSidechain 标记）；Codex sessions。确认哪些源有"会话内轮次级"粒度。
2. MVP 单场景「压缩时机」：检测会话内 context 增长曲线，找"若在 T 压缩，后续请求 input token 减少 Σ"→ 输出「本会话若在 14:02 压缩，省 ~38 万 token / ~$0.85」。只展示，不改任何用户配置。
3. 扩展场景（按价值排序）：大上下文模型选择（哪几轮用贵模型性价比最低）、重复读文件（同路径反复全量读）。

**UI 落点**：花费页或会话列表加「洞察」徽章；点击展开推演卡。三语 i18n。

**验收**：MVP 在本机真实数据出数且手算可复核；无数据/粒度不足的源显示「该工具暂不支持」而非编造。

## 02 百分比反推绝对用量（percent → absolute）

**两种口径（用户确认）**：
- token 套餐（精确）：remaining_tokens = plan_limit − Σ_model consumed。数据源：spend.rs 每模型每日消耗 + provider 卡片 plan limit 字段。
- 黑盒窗口（估算）：从窗口起点（resets.rs 检测）累计本地消耗 X，厂商报已用 p% → 剩余 ≈ X × (1−p)/p。UI 必须标「≈ 按你目前的用法」；每模型细分只在精确口径下提供。

**护栏（不编数）**：本地日志不覆盖窗口起点 → 不显示；p≥99% 或 ≤1% → 不推算（分母不稳）；pct 字段异常（CCUM 教训：>101 视为泄漏）→ 丢弃。

**UI 落点**：卡片窗口行加「≈ N tokens / 精确 N tokens」副行；hover 解释口径来源。

**验收**：parse-tests 两种口径各一组断言（含护栏边界）；本机对智谱/Kimi（精确）与 Claude（估算）实测。

## 03 扫描缓存代码版本哈希失效（parser-version salt）

**magpie 做法**（internal/usage/request_disk.go:50-77）：缓存指纹=CallCacheVersion+全部源码 hash，升级即失效但靠版本哈希避免脏读，且未变源不重扫。

**Pane 做法**：spend 光标（byte-offset + boundary_hash，spend.rs:295）与 spend_cache PersistEntry 增加 `parser_rev`（per-source 版本常量，解析逻辑变更时 +1）；load 时 parser_rev 不匹配 → 仅重扫该源（现状是 PERSIST_VERSION 全局全弃）。与 pricing-effective-time#56 的迁移机制协调：56 改定价口径走探针，03 改解析口径走 parser_rev，两轴独立。

**验收**：改某源 parser 逻辑 + bump 其 parser_rev → 只有该源重扫、其余源 cursor 原样；parse-tests 断言。

## 风险

- 01 的"省多少"数字是估算，文案必须带「≈」与口径说明，避免变成第二个定价争议。
- 02 估算口径在厂商改倍率时会系统性偏移——已由「按目前用法」标注覆盖。
- 03 与 #56 的 PERSIST_VERSION 迁移有交互（56 升 v6 时全量重解析一次，03 的 per-source rev 在其上建立），实施顺序：#56 先、03 后。
