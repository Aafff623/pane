Status: pending
Type: task
Blocked by: 02

# 04 · spend 架构收口（#42 / #43）

## #42 来源适配器

1. 把 `spend.rs` 里的 19 个来源解析拆成 `spend/sources/*` 模块；注册表统一
   `roots/collector/status`，删除重复的路径与状态逻辑。
2. 修正 gemini 注册表的 `needs_probe: true`（已有 fixture 与解析测试）。
3. 扫描耗时对比记录：在 `source_statuses`/诊断里暴露每源耗时或写入本地
   evidence，不新增遥测。

## #43 归因

1. 项目/分支归因：opt-in 配置，默认关闭；从本地日志提取 project/branch，
   账号与路径做假名化；不进入 HTTP API 默认输出。
2. `pricing_mode` 接到 UI 徽标（billing/stored/missing）。
3. web search/flex 分项计费：无可靠价目则保留关闭并文档化，不猜价。

## 核实（2026-10-27，逐行对代码）

本票不是零起点，之前说「未开始」不准确：

- **注册表已经在了。** `src-tauri/src/spend/sources/mod.rs` 有 `SourceSpec`
  （id / tool / form / collector / roots / feeds / runtime / needs_probe /
  parse_probe）和 `registry()` / `statuses()`；`spend.rs:292` 的 `source_statuses`
  就是 `sources::statuses()`，`spend.rs:4725` 也走 `sources::registry()`。
- **但物理拆分只有 1/19。** `spend/sources/` 里只有 `copilot.rs` + `mod.rs`；
  其余来源的 `collector` 仍是 spend.rs 内的函数名（如 `collector: "claude_line"`），
  代码还堆在 7801 行的 spend.rs 里。
- **#42-2 已修（2026-10-27）。** `gemini-jsonl` 的 `needs_probe` 改为 `false`：
  它当初被标记是因为解析器只照着上游 session schema 写、没有样本可校验；
  现在 `parse-tests/fixtures/gemini/` 有 fixture，`spend.rs` 里也有
  `gemini_fixture_conversation_parses_once` 等两个解析测试，标记过期。
  测试同步改成钉住「不再 parked」而不是钉住具体状态——具体状态取决于本机
  `~/.gemini` 是否存在，钉死会变成跨机器的脆弱断言。同时加了一条
  「注册表里不应再有 needs_probe」的守卫。
- **#42-3 已做（2026-10-27）。** `SpendSourceStatus` 增加 `scan_us`，`status_of`
  用 `Instant` 给每次状态探针计时（发现路径 / stat / 打开文件 / 列目录）。
  用微秒而不是毫秒：一次 stat 远低于 1 ms，毫秒级会永远读成 0。
  界面上不占新列，挂在来源行的悬浮 `title` 里（`· probe 0.12 ms`），
  因为「为什么扫得慢」是诊断信息，不是每行都要看的事实。
  **注意它的口径是探针，不是解析**：真正的解析仍在各 provider 自己的 collector 里，
  所以这个数字是成本下界——要拿到按源解析耗时，仍必须先做 #42-1 的注册表驱动采集。
- **#43-2 已补（2026-10-27）。** `pricing_mode` 原来只在 Rust 里算、TS 里声明类型，
  界面上从不渲染，所以用户分不清「厂商显示的价格」和「Pane 自己算的」。现在
  `spendBadgesFor` 在非 `auto` 时输出 `pricing-badge`：`auto` 是目录命中的常态，
  给它挂徽标等于每张卡都挂；`display` / `calculate` / `unknown` 各自说明钱的来源。
  三语键齐备，测试同时钉住「四种模式都有标签」和「后端枚举就是这四个」。
- **#43-3 已记录决定（2026-10-27）。** `pricing.rs` / `spend.rs` 里没有任何
  web-search 或 flex 服务层价目，厂商也没有可机器读取的单价表，同一 slug 在不同
  账号上单价不同。按本票规则「无可靠价目则保留关闭并文档化，不猜价」，
  决定性说明写进 `docs/project-facts.md` 的「待确认」节。
- **#43-1 仍未做**：项目/分支归因（opt-in、默认关、路径与账号假名化、不进 HTTP API）。

所以本票的真实进度是「注册表抽象 ✅ / 迁移 1/19 / gemini 标记 ❌ / 耗时 ❌ / 归因 ❌」。

## 验收

- 每个来源有独立模块与回归测试；`/v1/sources` 状态与实际读取一致。
- 归因默认关闭；开启后本地展示，关闭后字节与现在一致。
