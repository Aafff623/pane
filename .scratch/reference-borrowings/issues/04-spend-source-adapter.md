# 04 — Spend SourceAdapter、probe/doctor 与增量扫描

Status: done (2026-10-23) — registry, byte-offset resume, Copilot parser, zstd/token_usage_record/compaction tests landed; Gemini remains needs-probe
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

- 2026-10-23（主对话接手）：把 `source_statuses()` 收口为 `spend/sources/mod.rs` 注册表（id/tool/form/collector/roots/feeds/runtime/needs_probe/parse_probe）；`/v1/sources` 新增 `empty`、`unreadable`、`parse_error`、`needs_probe` 状态。
- 新增 `spend/sources/copilot.rs`：`~/.copilot/session-store.db` 的 `assistant_usage_events` 逐调用解析（input_tokens 含 cache；tokens=prompt+output；cache read/write 分开计价），`session-state/*/events.jsonl` 的 `session.shutdown.modelMetrics` 仅补 DB 缺的 session。DB 先连 WAL/SHM 复制再只读打开。Gemini 没有本机真实样本，按要求明确为 `needs_probe`，没有猜解析器。
- Codex/Claude 改为 `file_days_with_state`：`(mtime,size,4 KiB boundary hash)` + 字节 offset 断点；追加只读新行并合并摘要，缩短/边界哈希不符/价格目录换代/状态损坏/末行未写完都回退整文件。Codex `.jsonl.zst` 走完整解压；`token_usage_record` 按 `response_id` 去重并优先于紧随其后的重复 `token_count`；compaction 行不改写累计基准。
- 测试：`cargo test spend::` 83 passed；新增 stateful offset、Codex record/zstd/compaction、Copilot DB/事件兜底、registry 状态测试。前端 `node --test tests/*.test.mjs` 96 passed；`corepack pnpm build` 通过（155 modules）。
- 实测扫描：`spend_live_dump` 首跑 16.93s、二跑 14.62s（同一台机器、缓存已落盘；改造前基线未保存，因此不宣称提速百分比）。
- 全量 `parse-tests`：1017 passed / 0 failed / 14 ignored（串行、去代理，444.95s）。

- 未验证/待 owner：设置页“来源”诊断的视觉呈现（当前只有 `/v1/sources` 数据）；Gemini 解析器；20 个既有来源没有逐源 golden fixture，回归依赖现有单测与全量 scan，不是逐源字节级对账。
- 2026-10-23（收尾）：设置页来源诊断已补到浮窗和面板的 General 页，调用 `fetch_spend_sources`，按 `parse_error > unreadable > empty > needs_probe > detected > not_detected > runtime` 排序显示状态、原因和路径 tooltip；`pnpm build` 通过。Gemini 仍未实现，仍是 `needs_probe`。
- 技术冒烟（2026-10-23）：面板 Settings → 通用实际渲染“花费来源”，可见 Gemini CLI `待校准`、Antigravity/cc-switch/Claude Code/CodeBuddy IDE `已检测`，刷新按钮存在；浮窗同一 mount 代码。owner 视觉验收仍待做。
