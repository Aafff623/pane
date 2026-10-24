# 01 — 模型名归一收成一个共享函数

**What to build:** 把「拿模型名当键去查」这件事收敛成一个共享的归一化，定价侧与来源侧共用，修掉
带大写的模型名静默算未定价（金额算少）的缺陷。
**Blocked by:** 无
**Status:** done (code + regression) — owner 目检：带大写的模型名出金额
**GitHub:** [#48](https://github.com/Aafff623/pane/issues/48) §1 · **Priority:** P1 · **量:** 小

## 为什么要做成共享机制而不是打补丁

本仓已经为同一类问题认过一次账：`src-tauri/src/spend.rs:1476-1482` 的 `split_models` 做大小写不敏感
前缀匹配，注释写着「网关同一个 family 两种拼法（`qwen3.8-max` / `Qwen/Qwen3-235B`），大小写漏了会把
行挂到错的卡上」。定价侧漏了同样处理，所以这是**同一缺陷类第二次出现**。再补第三处 if 就是第三次。

## 要点

1. **先清点，再动手。** 全仓「用模型名当键查」的实例，已知清单（行号为 2026-10-23 工作区现状，
   开工前用符号名重定位）：

   | 位置 | 现状 |
   |---|---|
   | `pricing.rs:628-634` | override 精确 `get`，原样键 |
   | `pricing.rs:599-612`、`:645-653` | override 载入与保存都不折叠 |
   | `pricing.rs:695-700` | memo 用原样键 |
   | `pricing.rs:743`、`:764`、`:822`、`:854-870` | 各价格源精确 `get` |
   | `pricing.rs:817`、`:863` | LiteLLM 模糊分支用 `==` 比较 |
   | `pricing.rs:390-394` | alias 正则 ingest 时**未设** `case_insensitive`；实测本机 supplement 157 条只有 41 条自带 `(?i)` |
   | `pricing.rs:119`、`:903`、`:938` | 已有折叠，但 `:903`/`:938` 只服务 Kimi 支路 |
   | `spend.rs:1234`、`:1249`、`:1277` | claude/codex/grok 家族兜底已折叠 |
   | `spend.rs:1476-1482` | `split_models` 已折叠（本仓先例） |

   清点结果若发现清单外的实例，一并写进本票的 Comments，不要另开票。

2. **共享的是「归一化后的键」，不是匹配算法。** 定价侧是精确查表，`split_models` 是前缀匹配，语义
   不同，强行合并会把两边都写坏。落点建议一个 `normalize_model_key(&str) -> String`
   （小写 + trim + 分隔符统一），两侧各自消费。

3. **不要在查询路径上每次现折叠。** 3 万条 usage 的解析路径上多一次 `to_lowercase` 就是 3 万次分配。
   在 **ingest 时**建小写索引（override 表、各价格源表、alias 规则编译时加
   `RegexBuilder::case_insensitive(true)`），查询路径只做一次归一。

4. **撞键风险必须先测。** 折叠会把 `Foo-3` 和 `FOO-3` 视为同一个——这是我们要的；但如果现网价格表里
   存在**折叠后相同、实际是两个不同模型**的条目，就会静默串价。写一个测试遍历随包内置快照与本机
   supplement，断言折叠后无碰撞；有碰撞就显式列出并决定策略，不许静默取第一个。

## 取证材料 / 复现步骤

- 缺陷形态复现（不碰真实数据）：在 `parse-tests` 里造一个 override `{"GLM-5.3-Flash-EXL3": {...}}`
  与一条日志模型名为 `glm-5.3-flash-exl3`（或反之）的记录，断言当前算不出钱。
- 上游同类修复可引为设计依据：cc-switch v4.0.14 #8413「含大写字母的自定义模型定价不生效，成本一直是
  0……现在查价不区分大小写，新请求和保存后的历史回填都能算上」。
- 本机 supplement 与内置快照路径：`src-tauri/pricing/*.json`（随包）与运行期
  `%APPDATA%\Pane\pricing\`（只读核对，不要改）。

## 验收

- [ ] 清点清单确认完毕，实例全部走同一个归一化（Comments 里留清点结果）
- [ ] 大写 override 键 + 小写日志名（及反向）都能命中价格
- [ ] 不带 `(?i)` 的 alias 规则能匹配带大写的日志模型名
- [ ] 撞键测试存在且通过；若有碰撞条目，显式记录并给出策略
- [ ] 归一化不改变已定价模型的现有结果（前后对比同一份日志的金额）
- [ ] `pricing::` 与 `spend::` 定向测试通过；`parse-tests` 全量绿；`pnpm build` 通过
- [ ] owner 目检：用量面板里一个带大写的模型名出金额而不是 `unpriced`

## Out of scope

- 价格源本身的增删（属 `pricing.rs` 日常维护）。
- override 编辑器只解析 4 字段、手填即丢 200k 档与 1h 写价的问题（`pricing.rs:610-612`/`:651-653`）——
  真实缺口，但属 #43 成本口径面，另议。

## Comments

- 2026-10-23：立项，证据来自 #48 与 `temp/research/20261023-ccswitch-usage-gap.md` §9.1-1。
- 2026-10-23 实现：加了共享的 `pricing::normalize_model_key`（trim + 小写），并把**所有**键入口收过去 —— catalog ingest（litellm/modelsdev/supplement/fast_multipliers）、alias（`RegexBuilder::case_insensitive` 且 alias 目标本身也折叠）、用户 override（`set_overrides` 落键时折叠，`override_for` 查询时折叠）、`SCHEDULES` 前缀（折叠比较，`&'static str` 不动）、memo（在 `lookup`/`lookup_at` 入口折叠后作键）。`spend.rs::split_models` 原本自己写了一份 `to_ascii_lowercase`，现在改为调用同一个函数，规则只留一处。
- 范围比票面写的更宽：核对时发现我自己那批未提交的 override 工作**新增了 3 处原样键查表**（`override_for`、`SCHEDULES.starts_with`、memo）。只修定价查表会留下洞，所以按「同一缺陷类要核完所有实例」扩到全部键入口。
- 回归：`pricing::tests::model_key_spelling_does_not_change_the_price`（同一族大小写两种拼法必须同价；先测 override 再测 catalog）。定价全套 22 通过。
