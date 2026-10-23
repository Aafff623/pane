# 04 — Spend SourceAdapter、probe/doctor 与增量扫描

Status: in progress — probe reasons/new source inventory landed; SourceAdapter trait and incremental offsets pending
GitHub: #42
Priority: P1
Blocked by: None
Reference: `temp/repos/ccusage` (`rust/adapters`, `common`, `core`), `temp/repos/TokenTracker` (`scan-roots.js`, `codex-usage-record.js`), `temp/repos/codeburn` (`src/providers/types.ts`, `src/doctor.ts`, `src/codex-cache.ts`)

## Scope

`spend.rs` 已经覆盖 20 个来源，但 5.7k 行单文件、整文件重读、来源状态只有 `detected/not_detected/runtime`，加一个新 agent 要改大函数。本轮先做结构，不追求一次补齐 42 个源。

1. **SourceAdapter 契约**：每个来源一个模块，至少 `paths` / `probe` / `parse` / `report`；共享扫描、窗口过滤、去重、聚合留在公共层。
2. **probe/doctor**：`source_statuses()` 由注册表生成；`/v1/sources` 返回 root、state、reason（missing/unreadable/empty/parse_error/runtime）；设置页能解释“为什么没数据”。
3. **增量扫描**：size/mtime/hash + byte offset/checkpoint；先做 Codex/Claude（长 rollout 追加不全扫），保留整文件回退作为安全网。
4. **新源优先级**：
   - Gemini CLI：`~/.gemini/tmp/**/chats/*.{json,jsonl}`。
   - Copilot 本地：`~/.copilot/session-state/*/events.jsonl` 或 `session-store.db`（以真实样本为准）。
   - Codex 增强：`.jsonl.zst`、`token_usage_record`、compaction 去重（TokenTracker `codex-usage-record.js` / ccusage replay）。
   - 后续再评估 Cline、Zed、Cursor local `state.vscdb`、Goose、Warp、Kiro。
5. **保留 Pane 独有**：cc-switch、WorkBuddy CN/AI、CodeBuddy CLI/IDE、Qoder transcript、Trae CN、ZCode 路由、Antigravity、DSH、Cursor CSV。

## Acceptance

- [ ] 新增一个来源 = 新模块 + fixture + 注册一行；`source_statuses()` 不再手写 19 条。
- [ ] `/v1/sources` 能区分 missing/unreadable/empty/parse_error/runtime；错误不吞。
- [ ] Codex/Claude 追加日志只读新增字节；扫描耗时对比有记录；整文件回退可用。
- [ ] Gemini CLI 与 Copilot 至少一家落地并有真实脱敏 fixture；另一家若样本不足，明确 `needs-probe`。
- [ ] Codex `.jsonl.zst`、compaction 去重有测试；不改变现有 20 源的日/模型总量。
- [ ] 不读 prompt/对话正文；不写 vendor 文件；`parse-tests` 全绿。

## Evidence / notes

- Pane `spend.rs` 的 `source_statuses()` 目前是硬编码列表；`fetch_spend_sources` 只暴露 detected/not_detected/runtime，UI 未接。
- ccusage `rust/adapters/README.md` 的五类文件（paths/parser/loader/report/types）是可抄的契约；TokenTracker `scan-roots.js` 区分 missing/unreadable；codeburn `probeRoots` 列路径。
- ccusage 有 18 个源，Pane 有 20 个但形态不同；先对照 `docs/token-spend-coverage.md`，不要丢 Pane 独有源。
- 增量扫描失败时必须回退整文件，宁可慢不可漏。

## Out of scope

- 一次补齐 42 个源；project/branch 归因（05）。
- 读取对话正文/工具调用内容；远程同步/SSH；写 vendor 文件。
- 把“发现文件”宣称成“所有请求都已记录”。

## Verification

```powershell
corepack pnpm build
cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu test
```

回归：现有 20 源各一份 fixture，比较改造前后的日/模型总量；记录扫描耗时。Owner 目检设置页“来源”诊断。

## Comments

- 2026-10-23：`SpendSourceStatus` 增加 `root/reason`，`source_status` 区分 missing/empty/unreadable/detected；新增 Gemini CLI 与 Copilot local 的探测行（解析器待接）。`/v1/sources` schema 向后兼容并带原因。SourceAdapter trait、byte-offset checkpoint 和 20 源迁移未完成。
