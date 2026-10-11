# 01 — 把「总花费」面板改造成多计量单位的「用量」面板

**What to build:** 把现有「总花费」面板**本身**改造成「用量」面板：顶层按**计量单位**分节（token / 积分 / 分钟），每节一个图 + 明细。token 节沿用现有圆环 + 时段 + 热力图（跨厂商可拼）；积分节按「家」分组、每家自己的逐日消耗；分钟节（闪电说）单环/数字、不做饼图。**不再另起一个并列看板**——原描述里的「与现有总花费面板互补」已作废（见下方 2026-10-11 定稿）。

**Blocked by:** None — can start immediately.

**Status:** needs-triage

- [ ] 先出 `temp/preview/` 静态原型给用户拍板（布局、图表类型、口径），再动正式代码
- [ ] Rust 聚合命令（按日/周聚合额度与积分，复用现有台账，不新建数据源）
- [ ] 自绘 SVG 图表接线（不引重型图表库，对齐现有花费图风格）

## Comments

- 2026-10-11 从退役的 TODO.md 迁入（原任务号 **T-0005**，板由用户拍板退役、账本改为 `.scratch/`）。参考物：`coding-plan-dashboard`（国内 Coding Plan 额度看板，克隆已清理、上游 margrop/coding-plan-dashboard 可重拉）；credits 数据源已随 0.7.0 上线（Qoder CN / Trae CN 按日罗列，`usage_history::credit_extras`）。
- 2026-10-11 **grill-with-docs 定稿设计**（决策记录见 `docs/adr/0003-usage-panel-grouped-by-unit.md`）。本项实际收敛为**改造「总花费」面板本身**，而非另起一个并列看板——需确认这是否取代本条原描述里的「与现有总花费面板互补」。
  - 面板改名「**用量**」；顶层按**计量单位**分节（token / 积分 / 分钟），**无跨单位总数**。
  - 只有 **token 允许跨厂商**拼图；积分 / 分钟按「家」分开；一卡多单位（闪电说 = 分钟 + 积分）**各归各节**、不折算。
  - 成员 = 能给出该单位**绝对量**的 provider（**本地台账**或**服务端快照**）；只报百分比的归「额度总览」。
  - 来源两类并明确标注：本地台账（有逐日趋势 / 热力图）+ 服务端快照（仅当前值，无历史）。
  - **美元 = token 的一个视图**，不是独立计量单位。
  - 词汇已入 `CONTEXT.md`：额度 / 用量 / 计量单位。
  - **待定事实**：MCP 搜索的 credits 是否有绝对量 —— 决定它进不进积分节（后台调查中）。
  - 下一步：`temp/preview/` 静态原型给用户拍板 → `/to-spec` → `/to-tickets`。
- 2026-10-11 **MCP 搜索 credits 实况核查**（读运行中 Pane 的 `/v1/providers` + `searchquota.rs`）。四个 provider 的"每 key credits"能力**不一样**：
  - **Tavily** ✅ 每 key：`GET api.tavily.com/usage` → `key.usage`/`key.limit`（key 级已用）+ `account.plan_usage`/`plan_limit`。实测 3 把 key：424 / 0 / 5 已用。
  - **Brave** ✅ 每 key：无额度端点，靠真实搜索响应的 `X-RateLimit-*` 头；实测 1 把：229 / 2000。
  - **Firecrawl** ❌ 只有**团队**剩余 credits（`/v1/team/credit-usage`），给不出 key 级消耗。
  - **Bocha** ❌ 是**人民币余额**（`/v1/fund/remaining`），不是 credits。
  - **逐日 credits 历史的可行接法**（已核实）：`lib.rs:2971` 对**每个 ok snapshot** 调 `usage_history::credits_used_from_metrics`，命中即写入 `credit_history.json`；判据是「label == "Credits" 且 detail 以 `credits used` 结尾、首个 token 是绝对已用量」。所以 provider 只要发一条该形状的 metric 就自动进台账。
  - **但诚实性有坑，需先拍板**：**多 key 的 Tavily 给不出一个诚实的单一累计值**（代码自己标"可能共享"：key 级已用之和 429 ≠ 账户 424），台账按 card id 存一个数 → 要么改成**按 key 记账**（id 用 `tavily@<keyhash>`），要么只收单 key 情形。**Firecrawl（无绝对量）与 Bocha（¥）不进积分节**（符合 ADR 0003 的成员规则）。
  - 附：`bocha` / `firecrawl` 现在都是**真的耗尽**（¥0.00 / 剩余 0），显示 0 不等于坏了。
- 2026-10-11 **已修的显示缺陷**：单 key 的 Firecrawl 卡会把同一行打印两遍（`firecrawl_snapshot_from_results` 的 `count == 1` 分支 clone 了 headline 又 extend rows）——已改为单 key 时只出行本身，测试同步更新（`parse-tests` 3/3 通过）。
- 2026-10-11 **另一个观察（非代码缺陷）**：本机 `keyvault.json` 是 **v3 且 0 条 entries**，所以 bocha/tavily/firecrawl/brave 的 key 全从 legacy 兜底读（环境变量 + `tavily-keys.json`），卡面标签是 key 尾掩码而非用户标签。要恢复"自己的名字"需把 key 加进保险箱（Settings 里操作），不是代码 bug。
- 2026-10-11 **用户澄清 + 设计纠正（MCP 搜索，重要）**：
  - **MCP 类 key 不自动识别环境变量**：必须由**用户手动加进 Pane**；只有 agent 类（Codex 内置配置的登录态等）才自动识别。当前 `searchquota.rs` 的 legacy env 兜底（`FIRECRAWL_API_KEY` / `BOCHA_API_KEY` / `BRAVE_API_KEY`）**与这条冲突，要改**。
  - **上限不用写死**：接口自己返回 —— Tavily `account.plan_limit`、Firecrawl `data.plan_credits`，免费档都是 **1000**；Brave 是 **2000/月**。
  - **显示形态 = 每把 key 一个进度条 + 折叠/展开**（多账号管理式，参考考拉 AI），**不做"总量百分比"**。理由：用户的 key 是**独立账户**（实测 Tavily 3 把各 424 / 0 / 5），合并成一个百分比会抹掉"哪把还有额度"；且与 Pane 已认可的"合并卡 + 账号"模式一致。
  - **保险箱丢失事故（根因）**：本机 `keyvault.json` 从 v2（9 条：tavily×3 / apigoto / clinepass / bocha / firecrawl×3）被重置成 **v3 且 0 条**。Tavily 幸存是因为 `tavily-keys.json` 有明文副本；**Firecrawl 的 key 只存在保险箱里 → 全部消失**，这才是"卡片上只剩 1 把"的真正原因（不是扫描逻辑）。
  - 已用备份 + 主密码解密恢复 **10 把**（含 4 把 Firecrawl：`fc-779…6d45` / `fc-174…d3c8` / `fc-6e3…7924` / `fc-06e…5f32`）→ `temp/secrets/keyvault-recovered-20261011.txt`（另有 `firecrawl-recovered.txt`）。
- 2026-10-11 **用户新要求：按「账号指纹」去重**（多把 key 可能同属一个账号；一个账号只有 1000 credits/月，所以**统计上限按账号算，不按 key 算**；子 key 各自的用量照常显示）。**实测接口没有账号 ID**：
  - Tavily `/usage` 返回 `key{usage,limit,search/crawl/extract/map/research_usage}` + `account{current_plan,plan_usage,plan_limit,…,paygo_usage}`，**无 account id / email**。
  - Firecrawl `/v1/team/credit-usage` 返回 `data{remaining_credits,plan_credits,billing_period_start/end}`，**无 team id**。
  - **可用的指纹信号只有「账户级读数」**：同账号的 key 在同一次抓取里会报出**完全相同**的账户数字 —— Tavily 用 `(current_plan, plan_limit, plan_usage, paygo_usage)`，Firecrawl 用 `(plan_credits, remaining_credits, billing_period_start)`。据此把 key 归到同一账号，账号上限 = 该账号的 `plan_limit`（免费档 1000）。
  - **用户实测即三个独立账号**：3 把 Tavily 各报 424 / 0 / 5 → 上限 3 × 1000 = 3000（对上用户"3 个就是 3000"的说法）。
  - **Firecrawl 无 key 级用量**（只报团队余额），所以 Firecrawl 只能显示"账号剩余"，不能显示"每把 key 用了多少"；Tavily 有 `key.usage`，可以。
- 2026-10-11 **⚠️ 恢复路径的坑（写回保险箱会重演旧痛）**：这 3 把 Firecrawl key 当初就是因为在卡片上显示成 **🔒 保险箱已锁定** 才被用户要求删掉的（见 `temp/handoff/20261009-211646-T-0020-firecrawl-key-cleanup-and-quota-format.md`）。原样写回保险箱 → 🔒 每次开机复现（v3 会话密钥只在内存）。**建议改走"Pane 自管 key 文件"**（`tavily-keys.json` 同款：用户手动加、不用解锁、随时可查），这也正好落实"用户自己把 api-key 接进 Pane"这条要求。
- 2026-10-11 **根治已实现（未提交，待用户 UI 验收）**：MCP 搜索类 key 从保险箱迁到 Pane 自管的**明文文件** `%APPDATA%\Pane\search-keys.json`（`version:1`，`keys:[{id,service,label,key,created_at}]`）。
  - 新增 `src-tauri/src/search_keys.rs`（存储 + 4 条单测）；`keyvault.rs` 的 `list/add/remove/reveal/keys_for_service/locked_service_entries/service_of` 对这四个服务**自动路由**到该文件 —— **设置页零改动**，加/删/查看照旧，且**无需解锁、永不出现 🔒**。
  - `providers/searchquota.rs`：`collect_keys` 改为只读该文件；**删除** env 兜底、ZCode CLI 配置读取、`tavily-keys.json` 读取（落实"不自动识别环境变量"）。
  - 已迁移 9 把：firecrawl×4、tavily×3、bocha×1、brave×1（brave 的 key 只在环境变量里，一次性搬入，此后不再读 env）；保险箱里这 8 条已移除（只剩 apigoto / clinepass）。备份：`keyvault.json.before-vault-add.bak`、`.before-drop-search.bak`。
  - 验证：`parse-tests` **677 passed / 0 failed / 13 ignored**（含新增 4 条）；`cargo check`（真 app crate）通过；`tsc` + `vite build` 通过。
- 2026-10-11 **每 key 进度条（用户点名）**：四张 MCP 卡的 per-key 行从文本改为 `Metric::progress`：
  - **Tavily** = 该 key **所属账号**的 `plan_usage/plan_limit`（一个账号 1000）→ 同账号的 key 条一样长，正好落实"账号指纹归类"。
  - **Firecrawl** = 套餐消耗比 `(plan - remaining)/plan`；余额高于套餐（含赠送）读作 0%，不会出现负数。
  - **Brave** = `(limit - remaining)/limit`。**Bocha** 只有 ¥ 余额、无上限 → 保持文本。
  - 已重编 + 重启（debug build）实测：Firecrawl 4 条 0% / 89.7% / 0% / 0%；Tavily 3 条 42.4% / 0% / 0.5%；Brave 11.55%；**全部无 🔒**。测试同步更新（旧测试名 `..._do_not_fabricate_a_ratio` 已改为 `firecrawl_balance_bar_uses_the_plan_and_marks_extra_credits`）。
