# Reference repositories

本文件是 **入库的机制笔记**：记录我们借鉴机制的几个上游仓库——它是什么、当前
版本/提交、值得 Pane 借鉴的做法（按价值排序），以及已经过时、不再适用的旧结论。

上游源码本身 **不入库**：只读克隆放在本地 `temp/`（`temp/` 被 `.gitignore` 整树排除，
永不跟踪），我们只在这里写结论，不拷贝上游代码（许可证与体积原因）。

## 这个文件怎么维护

- `.github/workflows/upstream-scan.yml` 定时检查本文件列出的上游仓库：
  - **`on: schedule`**（每周一 03:00 UTC）与 **`on: workflow_dispatch`**；
  - 用 GitHub API 拉每个仓库的最新 release 与默认分支 HEAD，生成报告
    （job summary + `upstream-scan-report` artifact）；
  - **定时运行只报告、绝不写仓库**；只有手动触发且 `update_docs: true` 时，
    才会重写下面的基线块并开一个 PR（避免无人值守时乱写仓库）；
  - 不需要任何仓库 secret：GitHub 自动提供的 token 只用于提高 API 限流额度，
    取不到时回退到公开的匿名端点。
- 机器可读基线（**不要手改**，交给工作流维护；`repos` 的顺序即报告顺序）：

<!-- upstream-scan:begin -->
```json
{
  "checkedAt": "2026-10-14",
  "repos": [
    { "repo": "farion1231/cc-switch", "ref": "main", "sha": "c21106ff20e351d40f412cb51d1f24da177b113d", "release": "v4.0.11" },
    { "repo": "jlcodes99/cockpit-tools", "ref": "main", "sha": "915e7a20b432546660c3cd15e90f9fa527490ecc", "release": "v1.3.68" },
    { "repo": "robinebers/openusage", "ref": "main", "sha": "bae20d9ca7eb5aaab93cdae2673846d24ee5e8d2", "release": "v0.7.14" },
    { "repo": "yetone/magpie", "ref": "main", "sha": "722abfbb0a68f4cec341f4f28ef298e451abd772", "release": "v0.1.1231" }
  ]
}
```
<!-- upstream-scan:end -->

### 最近检查

> 本表由 `upstream-scan` 工作流的 `--update`（手动 `update_docs: true`）运行重写；
> 定时运行只产出 artifact / job summary，不改本文件。`status` 为 `release-moved` 时
> 定时运行会以非零退出码变红。最后一次人工复核：2026-10-14。

<!-- upstream-scan-table:begin -->
| repository | ref | 复核 release | 线上 release | 默认分支 HEAD | status |
| --- | --- | --- | --- | --- | --- |
| farion1231/cc-switch | main | v4.0.11 | v4.0.11 | c21106ff (2026-10-14) | up-to-date |
| jlcodes99/cockpit-tools | main | v1.3.68 | v1.3.68 | 915e7a20 (2026-10-12) | up-to-date |
| robinebers/openusage | main | v0.7.14 | v0.7.14 | bae20d9c (2026-10-14) | up-to-date |
| yetone/magpie | main | v0.1.1231 | v0.1.1231 | 722abfbb (2026-10-14) | up-to-date |
<!-- upstream-scan-table:end -->

---

## 1. cc-switch — `farion1231/cc-switch`

**它是什么。** Tauri 2 + Rust + React 的「一键切换 API 供应商」桌面工具：管理
Claude Code / Claude Desktop / Codex / Gemini CLI / Grok Build / OpenCode /
OpenClaw / Hermes / Pi / MiniMax Code 的连接配置，并附带 MCP / Skills / Prompts
面板、本地路由与故障转移、聚合模式、会话管理、用量看板、云端同步。**技术栈与
Pane 最接近的兄弟项目。**

**当前版本/提交。** `v4.0.11`（2026-10-13 发布）；`main` HEAD `c21106ff`（2026-10-14）。

### 值得借鉴的机制（按价值排序）

1. **用量查询走沙箱脚本，而不是每个上游写一段 Rust** —— `src-tauri/src/usage_script.rs`
   用 `rquickjs`（QuickJS）执行「模板 / 用户脚本」，脚本只负责拼一个请求；沙箱给了
   5 秒超时、16 MiB 内存上限、栈限制，`base_url` 做 https + 同源校验，密钥经模板
   变量注入而非写死在脚本里。**借什么**：Pane 每新增一个中转/长尾供应商都要加一段
   Rust + 测试，脚本路径能把这类「只是换个端点查额度」的供应商从编译期移到配置期。
   **落在哪**：`src-tauri/src/providers/` + 新增一个受控的 `usage_script` 模块。
   **注意**：Pane 的密钥姿态更严（见 `docs/project-facts.md`），脚本路径绝不能把
   明文密钥写进日志/错误，同源 + https + 超时/内存上限要照抄。

2. **写库即失效缓存的事件机制** —— `src-tauri/src/usage_events.rs`：代理日志落库时
   emit 一个 `usage-log-recorded` 事件（**200 ms 防抖合并**，通知失败只 warn 不阻塞
   写入），前端据此立刻 invalidate 查询，而不用等轮询周期。**借什么**：Pane 的刷新
   是固定循环（`lib.rs`，默认 5 min），本地 spend 变化要等下一拍才反映；事件驱动
   能让本地账本更新「即时可见」。**落在哪**：Pane 的 snapshot/refresh 循环 +
   `spend.rs` 扫描器写回处。

3. **按工具族拆分的会话用量解析器** —— `services/session_usage*.rs`（codex / gemini /
   grokbuild / mcode / opencode / pi 各一个，形状统一）。**借什么**：Pane 的
   `spend.rs` 是一个大扫描器，按来源拆成独立解析器能让「新增一个来源」的成本更低。
   **落在哪**：`src-tauri/src/spend.rs`（结构调整，不改语义）。

4. **外部配置文件的双向 live 同步 + 归属模型** —— `src-tauri/src/live/`：把 App 的
   数据库与工具自己的配置文件对齐，**归属只由配置来源决定、不由 provider id 或认证
   类型决定**；外部改了文件，刷新即自动同步、不再弹「配置已变，请确认」。**借什么**：
   Pane 读厂商文件时基本是只读，缺少「谁拥有这个值」的明确模型，容易出现冲突提示。
   **落在哪**：Pane 的账号存储（`accounts.rs`、`login_accounts.rs`）。

5. **聚合模式 + 本地路由/故障转移**（`src-tauri/src/proxy/`：`provider_router`、
   `circuit_breaker`、`failover_switch`、`model_mapper`、格式互转、`thinking_*`
   整流器）。**借什么**：Pane 的 relay 子系统（`providers/onenewapi/`）只做只读的
   计费/额度，不是路由器；这是一整个更大的产品面，**列为远期方向，不是近期采纳**。

6. **云端同步（WebDAV / S3）+ 原子写 + 自动备份** —— Pane 是本地优先，自身更新走
   Gitee 镜像；跨设备配置同步是 Pane 目前没有的机制。

### 已过时 / 不再适用

- Pane 现有笔记（`docs/project-facts.md`，2026-10-05「cc-switch ledger source」）把
  cc-switch 当作**spend 账本来源**读 `~/.cc-switch/cc-switch.db`（`proxy_request_logs`
  UNION `usage_daily_rollups`）。这条集成仍然成立、也仍是 Pane 的主要 cc-switch 用法，
  **但「cc-switch = 配置切换器 / 账本」的定位已过时**：4.x 已扩展到本地路由、聚合模式、
  多账号（Beta）。另外读 db 前应重新核对 4.0.11 的 `proxy_request_logs` /
  `usage_daily_rollups` 字段映射是否仍与 2026-10-05 的结论一致。

---

## 2. cockpit-tools — `jlcodes99/cockpit-tools`

**它是什么。** Tauri + Rust + React 的 **AI IDE 多账号管理器**：一键切号、配额监控、
**唤醒任务（wake）**、应用多开、**每账号独立出口代理（Mihomo）**、本地 API 服务网关、
Codex 会话工具；18 种语言。

**当前版本/提交。** `v1.3.68`（2026-10-13 发布）；`main` HEAD `915e7a20`（2026-10-12）。

### 值得借鉴的机制（按价值排序）

1. **唤醒 / 自动触发调度器** —— `modules/wakeup*.rs`、`codex_wakeup*.rs`、
   `workbuddy_auto_checkin.rs`。**这正是 Pane 的 W8 需求**（GLM / MiniMax 等 5 小时
   刷新的套餐，必须被请求一次才进入计费周期）。它已经解决的细节：
   - 调度配置覆盖「每天定点 / 每周 / 固定间隔 / crontab」；
   - **`wake_on_reset`**：在额度窗口重置后自动发一次轻量请求（默认 prompt `hi`），
     带 **10 分钟触发冷却**（`RESET_TRIGGER_COOLDOWN_MS`）与 **2 分钟安全边界**
     （`RESET_SAFETY_MARGIN_MS`）；
   - 每账号记录 `scheduled_date` / `last_checked_date` 防止重复触发；
   - 失败指数退避（5 min → 上限 60 min）、单飞 guard、状态落盘
     （`wakeup_tasks.json`）。
   **落在哪**：Pane 的 W8 自动触发 + 一个新的 scheduler 模块。**注意**：Pane 已决定
   W8 **默认关闭、需用户明确同意**（会消耗一次真实请求）；cockpit 默认值不同，Pane
   要保持 opt-in，不能照抄默认开。

2. **配额刷新调度器（单飞 + 去重）** —— `codex_quota_refresh_scheduler.rs`：队列按
   account id 键控，**重复刷新加入已在飞的请求**而不新建网络请求；Manual / Background
   各自一个信号量；每账号最多 16 个 waiter；队列等待与刷新各 120 s 超时；worker 在
   **所有退出路径（含 panic）都清理自己的条目**。**借什么**：Pane 的刷新循环可能对
   同一账号并发触发多次刷新；这是「每账号一次网络刷新、多个等待者」的成熟模式。
   **落在哪**：Pane `lib.rs` 的 `refresh()` / 后台循环 + 账号快照缓存。

3. **账号回收站** —— `codex_account_recycle_bin.rs`：删除账号先进回收站而非立即抹除。
   **借什么**：Pane 目前直接删账号，回收站是廉价的误删兜底。**落在哪**：
   `accounts.rs` / `login_accounts.rs`。

4. **主动 token keeper** —— `provider_token_keeper.rs`：在过期前主动刷新。
   **与 magpie 的结论一致**（见第 4 节），是 Pane 刷新姿态的第二个佐证。

5. **每账号出口代理（Mihomo 内核）** —— 把某个账号绑定到特定出口 IP/隧道。
   **借什么**：仅当 Pane 未来需要「按账号走不同出口」时相关；体量大（要按需安装
   内核），**列为远期方向**。

6. **本地 API 服务网关的请求日志 / 输出速度 / 服务档位（Standard/Fast）** —— Pane 的
   本地 API 是只读用量；cockpit 跑的是完整请求网关。**范围不同**，只把「请求日志 +
   输出 token 速度 + 档位证据」的 UX 记作「若 Pane 将来代理请求时可借鉴」。

### 已过时 / 不再适用

- Pane 的 `docs/project-facts.md` 引「cockpit 参考为 v1」来判定 Trae 国际版用量端点
  版本（`/trae/api/v1/...`）。**需对 1.3.68 重新核对**：cockpit 的 Trae 支持已扩展到
  Trae CN / SOLO / SOLO CN，v1 的结论可能已过时。

---

## 3. openusage — `robinebers/openusage`

**它是什么。** macOS 菜单栏 AI 用量追踪器（原生 Swift，macOS 15+）。**Pane 的直接对标
上游**：Pane 的 `/v1/usage` 线格式与 `pricing.rs` 的 LiteLLM / models.dev 目录都已
沿用它的做法（见 `docs/local-http-api.md`、`CONTEXT.md`）。

**当前版本/提交。** release `v0.7.14`（2026-10-06；`/releases/latest` 不含预发布，
本地 CHANGELOG 已到 `v0.7.15-beta.1`）；`main` HEAD `bae20d9c`（2026-10-14）。

### 值得借鉴的机制（按价值排序）

1. **规范化的 `/v1/limits` 线格式** —— `schema: "openusage.limits.v1"`，`providers[]`
   按稳定 ID 键控 `resources`，值是带显式单位的原始标量，并带 `stale` / `expiresAt`
   / `errors`。Pane 目前只提供 **legacy** 的 `/v1/usage`（面向 UI 行）。**借什么**：
   给 agent / 脚本一个不随 UI 行变化而变的稳定契约。**落在哪**：
   `src-tauri/src/httpapi.rs`（新增 `/v1/limits`，保留 `/v1/usage`）。

2. **一次性 CLI（与应用共享同一引擎/缓存/凭据）** —— `openusage [provider] [--force]`
   打印 limits JSON 后退出，**不需要菜单栏应用在运行**；复用同一份 5 分钟缓存；
   退出码 0 / 2 / 4。**借什么**：Pane 是 GUI + 只读 HTTP API，没有 CLI；有了 CLI，
   agent 无需应用在跑也能读限额。**落在哪**：`src-tauri` 新增一个 bin，或对同一
   snapshot 缓存做一层薄封装。

3. **追加式日志扫描 + 解析缓存** —— 缓存键是 (path, size, mtime, parser version)；
   文件只增长时从**最后一个完整行**续读、只读新增字节；前缀变化 / 截断 / 追加行不完整
   时**回退整文件流式解析**（不丢记录）；写入防抖；身份 35 天淘汰。**借什么**：Pane 的
   `spend.rs` 每轮整文件重解析；续读 + 缓存能显著降开销。**落在哪**：`spend.rs` 扫描器。
   （上游注明该扫描器改编自 Runway / mstallone，MIT。）

4. **分层定价 + 可实时发布的 supplement** —— supplement（发到 GitHub Pages）> LiteLLM
   > models.dev；三个来源都随包内置快照（离线可用）；运行时约每小时带 ETag 重新校验；
   **别名规则 + 模糊名称解析**（provider 前缀、日期后缀、分隔符差异）+ fast 变体倍率；
   无法定价的模型从美元统计中剔除并给警告。**借什么**：Pane 已用 LiteLLM/models.dev
   （`pricing.rs`，日更 / 有未定价模型时小时级）；**增量借鉴点是「实时发布的 supplement
   + 小时级 ETag 重校验 + 别名/模糊解析」**。**落在哪**：`src-tauri/src/pricing.rs`。

5. **stale-while-revalidate 的 UX 细节** —— 磁盘快照启动即显示；「新鲜」只认**本会话内
   取到的**值（所以新版本一启动就刷新，而不是等旧区间到期）；缓存里记住产出它的账号，
   换账号即丢弃旧值；数据超过约 10 分钟显示 **「Outdated」** 标签；**刷新失败绝不清空
   已有数据**；单 provider 120 s 超时。**落在哪**：Pane 的快照缓存 + 卡片渲染。

6. **iCloud 历史同步** —— 只同步规范化的每日 tokens/spend/模型合计（**永不含凭据**），
   设备 id 存在 Keychain，最终一致合并。Pane 本地优先、无跨设备同步；**列为远期方向**。

7. **每个 provider 一页文档 + `adding-a-provider.md`** 的文档约定。

### 已过时 / 不再适用

- Pane 已采纳 openusage 的 **legacy `/v1/usage`** 线格式与同一个 **`6736`** 端口
  （`docs/local-http-api.md`）。但 openusage **已把 `/v1/usage` 标为 deprecated**、改推
  `/v1/limits`——Pane「compatible with the macOS OpenUsage API」的说法仍成立，但指向的是
  已弃用路由。
- **端口冲突**：两者默认都用 `127.0.0.1:6736`，同一台机器上二者的 HTTP API **不能同时
  运行**（后启动者静默禁用该功能）。Pane 应显式记录/处理这一点。

---

## 4. magpie — `yetone/magpie`

**它是什么。** yetone 的 Go 网关/路由工具（多供应商 LLM 网关 + GUI）。Pane 把它用作
**README 章节骨架的来源**，以及**刷新机制的参考**（2026-10-12 已做过一轮逐条对比，
见 `temp/research/20261012-magpie-refresh-mechanisms-vs-pane.md`，本文件不重复其表格）。

**当前版本/提交。** release `v0.1.1231`（2026-10-14；release 落后于提交，本地 HEAD 已在
该 tag 之后）；`main` HEAD `722abfbb`（2026-10-14）。

### 值得借鉴的机制（多数已在推进）

1. **带余量的主动刷新**（`wbFresh` 60 s；codex/copilot/kiro 2–5 min）—— Pane 已对
   codex/claude/grok/antigravity 采纳 60 s 主动刷新；**剩余缺口**：codebuddy 家族 /
   Kiro / Cursor 仍是「先发一次、被 401/403 才刷」。2026-10-12 笔记的建议 #1–#3 仍开放。
2. **让厂商 CLI 自己刷**（跑 `cursor-agent` / `grok models` / `kiro-cli debug
   refresh-auth-token`，而不是写厂商凭据文件）—— 降低 sign-out 竞态风险；笔记建议 #4，
   建议单独开票评估。
3. **无 refresh token 时用旧 token 直到被拒**（更宽容）—— 笔记建议 #3。
4. **README 章节骨架** —— 已在 2026-10-14 复制进 Pane 的 README（`docs/project-facts.md`）。

### 已过时 / 不再适用

- 2026-10-12 笔记里的悬置项「任务描述提到的 `workbuddy_local.rs` 在本仓不存在」——
  **仍然成立**：WorkBuddy 的实现全在 `providers/codebuddy.rs` + `codebuddy_login.rs`。
- magpie 的 release 明显滞后于 `main`（例如复核时 `main` HEAD 已在最新 tag `v0.1.1231`
  之后），**扫描工作流对 magpie 应同时看 release 与默认分支 HEAD**，不能只看 release。

---

## 本次新增发现（2026-10-14）

相对 Pane 已有笔记，本轮拉新后**新出现、值得单独记一笔**的点：

1. **cc-switch 4.x 的聚合模式 + 本地路由/故障转移 + 沙箱用量脚本**，是相对 Pane
   2026-10-05「cc-switch = spend 账本来源」旧定位的实质新增；其中 **沙箱用量脚本**
   与 **写库即失效缓存的事件机制** 是两个可直接落地的借鉴项。
2. **cockpit-tools 的 `wake_on_reset`**（重置后触发 + 10 分钟冷却 + 2 分钟安全边界 +
   每账号日期去重）是 Pane W8 需求**最接近的现成实现**，可直接作为 W8 调度的设计参考。
3. **openusage 已把 `/v1/usage` 标为 deprecated、改推 `/v1/limits`**，而 Pane 跟随的是
   被弃用的那条路由；同时**两者默认端口同为 `6736`**，同机不能并存。
4. **openusage 的追加式日志扫描 + 解析缓存**（续读 + 按 (path,size,mtime,parser) 缓存 +
   回退全解析）可直接用于 Pane 的 `spend.rs`。
5. **magpie 有 release 但明显滞后于提交**（本地 HEAD 领先最新 tag 34 个提交），提醒扫描
   工作流不要只看 release。
