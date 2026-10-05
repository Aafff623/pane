# Plan: cc-switch 作为 Pane 花费数据源（2026-10-05）

> 状态：设计定稿，待拍板后实施
> 依据：temp/reports/20261004-ccswitch-usage-source-analysis.md（机制+实测）
> + 本文档第 3 节对账证据（temp/scripts/ccswitch-vs-pane.mjs 可复跑）

## 0. 一句话

Pane 新增只读扫描 `~/.cc-switch/cc-switch.db`：**claude 换源拿真账（当前 7~24 倍）、codex 等补历史缺日、mcode 首次进面板**；所有工具按「日级差集」合并，永不双计。

## 1. cc-switch 用量机制（v4.0.0 源码 + 本机实测）

写侧两条腿（都写入 `proxy_request_logs` 明细表，`request_id` 主键）：
- **代理记账**：流量过 cc-switch 内置代理时逐请求记（真实 token，即使中转站把 CLI 日志 usage 抹零，代理在中间看得到）；`data_source='proxy'`。
- **会话导入**：每 **60 秒** + 启动首轮 + 手动（`session_auto_sync_enabled` 门控），扫各 CLI 本地日志增量导入（游标在 `session_log_sync`，字节偏移+尾指纹）；导入器 7 家：claude（~/.claude/projects JSONL，message_id 去重、与代理行互斥 `skips_matching_proxy_log`）、codex（~/.codex/sessions rollout，token_count 累计→delta）、gemini、opencode、grokbuild、pi、mcode；`data_source='<app>_session'/'session_log'`。
- **Rollup**：启动 + 周期把 `created_at < now-30d` 的明细聚合进 `usage_daily_rollups`（`date(created_at,'unixepoch','localtime')` 本地日）后**删除明细**；rollups 永久。→ 明细表=近 30 天（含实时），rollups=30 天前历史，**两表 UNION 才是全量**。
- **计价**：`model_pricing` 表 + `cost_multiplier`，落 `total_cost_usd`；明细另有 `input_token_semantics`（0=legacy 含 cr / 1=total 含 cr+cc / 2=fresh 纯输入；`CACHE_INCLUSIVE_APP_TYPES=[codex,gemini,grokbuild]`，新 app 默认 claude 式）。

读侧（Pane 要复刻的口径）：
```
fresh_input = CASE sem=2 THEN i
              WHEN app IN (codex,gemini,grokbuild) AND sem=1 AND i>=cr+cc THEN i-cr-cc
              WHEN app IN (codex,gemini,grokbuild) AND sem=0 AND i>=cr   THEN i-cr
              ELSE i END
tokens = fresh_input + cache_read + cache_creation + output   -- 与 pane 其他扫描器同构
```
单位坑：`created_at`/`last_synced_at` 是**秒**；rollups.date 是本地日 TEXT（与 pane `day_of_utc()`（实为本地日）、ZCode 页面三方对齐）。

## 2. 本机数据现状（2026-10-05 00:1x 实测）

| 数据 | 范围 | 说明 |
|---|---|---|
| codex_session 明细 | 09-05 → **10-05 00:13（分钟级活）** | gpt-6.1-sol 近 2 天 3.07 亿（工作区里 CodeX agent） |
| claude proxy+session | 09-05/09-06 → 10-04 | 代理行是真实 token（CLI 日志被中转抹零） |
| mcode_session | 10-01 → 10-04 | 4,148 行 9.05 亿，model 多为 `unknown` |
| pi / opencode 明细 | → 10-03 | 活 |
| rollups | 至 09-04 | 30 天线，非停摆（设计如此） |
| claude rollups 历史 | **03-10 → 09-04（132 天）** | 106 亿，pane 永远扫不到 |

## 3. 对账证据（cc-switch vs pane spend_history，逐日）

- **codex**：pane 窗口内 10-01→10-04 **比值 1.00 分毫不差**（两源同文件同算法）；10-05 差 = 实时增量滞后；09-03~09-17 pane 缺、cc 有。
- **claude**：cc-switch 是 pane 的 **7~24 倍**（09-29：2.13 亿 vs 0.15 亿）——pane 自扫源（~/.claude 日志）被中转抹零，cc-switch 代理记真账；仅 10-04 两源一致（当天链路未抹零）。
- 结论：**codex 等会话导入型数据与 pane 自扫等价（取谁都不亏）；claude 必须换源。**

## 4. 数据源决策表（每 provider 主源 + 差集互补，防双计）

| pane provider | 主源（两源都有该日时） | cc-switch 贡献 | pane 自扫保留职责 |
|---|---|---|---|
| claude | **cc-switch** | 全部日期（含 3 月起历史） | 仅补 cc-switch 完全没有的日期（如 cc-switch 未装/未开时代） |
| codex | pane 自扫（已验证等价） | 仅 pane 缺的日期（09-03~09-23 段） | 当前实时 |
| opencode | pane 自扫（已全覆盖 01-25 起） | 无（差集为空） | 全部 |
| grok | pane 自扫 | 07-14/15 两天 | 其余 |
| pi | cc-switch（pane 无扫描器） | 全部 | 无 |
| **mcode（新）** | cc-switch 独有 | 全部 | 无 |
| gemini/claude-desktop | cc-switch（微量，一并收） | 4 天≈0 / 1 行 | 无 |
| zcode/kimi/antigravity | pane 自扫 | 不涉及（cc-switch 不记） | 全部 |

去重实现：日级合并——`cc_days` 与 `pane_days` 按 (day) 差集，重合日按上表主源取舍；与 zcode() 的 jsonl legacy `if day >= min_day { skip }` 同模式。

## 5. 实施设计（src-tauri/src/spend.rs）

1. **新函数 `ccswitch_db_data()`**：只读开 `~/.cc-switch/cc-switch.db`，(db,wal) 指纹缓存（复刻 `zcode_db_data` 模式，含锁库回退上次好结果）。查询：
   ```sql
   -- 明细（近 35 天，容 rollup 延迟）
   SELECT created_at, app_type, model, input_tokens, output_tokens,
          cache_read_tokens, cache_creation_tokens, input_token_semantics
   FROM proxy_request_logs WHERE created_at >= ?  -- 秒！cutoff/1000
   -- 历史
   SELECT date, app_type, model, ...同列 FROM usage_daily_rollups
   ```
   逐行算 `fresh_input`（semantics CASE 复刻）→ tokens；`add_event`（明细行 created_at×1000 → 本地日；rollups 行 date 文本 → NaiveDate → day_ce）。
2. **app_type→provider 映射 + 主源规则**（§4 表），merge 进 `collect()` 的既有 providers：claude/codex/opencode/grok/pi 走差集合并；mcode/gemini/claude-desktop 走 `build_spend` 新行（id 待定：`mcode`）。
3. **claude() 改造**：`claude()` 自扫结果仅保留「cc-switch claude 日集没有的日期」；cc-switch 的 claude 行（proxy+session+rollups）并入 claude provider。成本：用 pane 自己的 `probe_lookup` 定价（与面板口径一致）；cc 的 `total_cost_usd` 不采信（它按它的价目表，pane 有自己的）；`unknown` 模型走 note_unpriced。
4. **spend_history**：新 provider（mcode 等）自然经 merge_daily 落账；VERSION 不必再 bump（新行是新增不是改旧账）；但 claude 换源会让**历史日数字变大**（补真账），需要一次 v2→v3 迁移清 claude 格子（同 zcode 模式，仅清 claude）。
5. **source_status**（lib.rs:137 附近）：新增 `ccswitch-sqlite` 源登记。

## 6. 前端

- provider 目录加 `mcode`（名称/图标拍板项；无图标回落色点/「?」）。
- claude 数字变大属预期（真账替换抹零账），向用户交底。

## 7. 测试计划（parse-tests）

- semantics CASE：sem=2/1/0、守卫失败兜底、cache-inclusive 名单外 app 带 sem=1（兜底 ELSE）
- 秒→毫秒、rollups.date 文本→本地日、created_at 恰好跨本地午夜
- 差集合并：两源重合日取主源（claude→cc / codex→pane）、cc 缺 pane 补
- claude 换源后 spend_history v3 迁移只清 claude
- cc-switch.db 不存在/锁库 → 回退 pane 自扫、不 panic
- 对账基线（一次性脚本）：ccswitch-vs-pane.mjs codex 段比值恒 ≈1.00

## 8. 拍板项

1. 实施与否（建议：是；工作量约半天，含测试）
2. mcode 显示名：**MaxCode**（用户截图用词）/ MiniMax Code / 其他
3. claude 换源确认：历史 claude 日数字将显著变大（真账），预期内？
