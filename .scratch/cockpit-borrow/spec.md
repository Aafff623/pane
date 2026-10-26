# Spec: cockpit-tools 语义借鉴批次（Retry-After / 身份校验 / 动态间隔 / OAuth 模式 / token 锁 / 语义补强）

**Issue:** https://github.com/Aafff623/pane/issues/63（#54 已加评论补充其验收语义）
**补充（2026-10-25 晚，多账号管理补课）**：追加票 07-09（错误终态性分类 / 授权中心呈现小件包 / 账号发现提示条），证据 `temp/research/20261025-cockpit-multi-account.md`；多账号切换/注入/写配置类语义明确排除。

> 2026-10-25 深挖产出。**许可证 CC BY-NC-SA 4.0：全部为 Pane 原创重写，只学调度语义与情报，零代码移植。** 证据：temp/research/20261025-cockpit-deep-dive.md（含 20 平台端点情报表）。单飞闸门本体已在 #54（refresh-single-flight），本批次是 #54 之外的增量。

## 01 统一 Retry-After 头解析（P1，半天）

现状：只有 providers/claude.rs:293 写 `retry_after_s=` 标记，其余 provider 429 只有字符串兜底（300s/60s min）。
方案：provider HTTP 层加一处公共 Retry-After 头解析（数字秒 + HTTP-date 两种格式），解析结果进 snapshot 的 retry_after_s 既有通道（snapshot_store.rs:341 bench_after_error 已会消费）；各 provider 复用。退避语义：`max(指数退避封顶 300s, 服务器提示封顶 3600s)` 取大者。诊断信息（status/code/body_len）走结构化字段，不拼错误串。

## 02 额度响应身份校验 + 凭据指纹（P1/P2）

现状：换绑守卫只护 onenewapi 一条线（lib.rs:1731-1773）；额度响应「数字属于哪个账号」无校验。
方案：① 带账号标识字段的 provider（响应里的 user/org/account id），snapshot 写入前与账号库绑定标识比对，不符 → alert（IDENTITY_MISMATCH 语义）而非静默入库；双方都有值才比。② 刷新失败日志记凭据指纹（sha256 前 8 位+长度+过期时间），不含原文；生产默认不落盘。

## 03 动态刷新间隔（P2，1-2 天）

现状：spawn_auto_refresh 固定 refreshMinutes（默认 5min，lib.rs:3681/:150）。
方案（原创，不引入活动探测）：`interval = clamp(基线 × f(余量, 临近重置), 下限, 基线)`——余量 <20% 或 5h 窗 30 分钟内重置 → 临时缩到 1min；成功后回用户配置值；失败退避优先于加速（退避期不允许加速插队）。落点：spawn_auto_refresh 睡眠时长计算处。**不做**：session 文件活动探测（强耦合 Codex CLI 私有 sqlite schema，多 provider 看板不值维护成本）。

## 04 OAuth 模式语义（P2）

三条通用纪律（原创重写，供 Cursor PKCE 与未来 OAuth 场景）：
1. **双栈 loopback**：回调服务器同时绑 [::1] 与 127.0.0.1 同端口，两者都成功才广告 localhost，单栈广告对应字面 IP（防浏览器解析到未绑定族）。
2. **手动粘贴回调兜底**：接受完整 URL / 纯 path / 纯 query 三种输入——deep-link 失败场景的保命通道。
3. **pending 态落盘纪律**：0700/0600 原子写+文件名遍历校验；解析失败 quarantine（rename 保留原件）不删除；**device-auth 流永不恢复**（轮询器在进程内，stale device code 不能复活）。
另：全局单 flow（已有 flow 时返回既有 auth_url 防重复开浏览器）。

## 05 账号级 token 刷新锁（P3，依赖 #54 落地）

#54 的 IN_FLIGHT 是 provider 粒度；多账号后台刷新需按**账号**粒度锁，且用双检代际语义防「并发刷同一账号双消费已轮换的 refresh_token」：等锁前记 observed_generation，拿锁后重载账号，generation 推进且 token 已有效 → 直接复用别人的刷新结果；仍过期 → 告警并继续刷。跨进程文件锁不做（Pane 无共享凭据目录场景）。

## 06 provider 窗口语义补强（P3，按情报表逐家核对）

依据 research 文件 §一情报表，逐家核对 Pane 解析是否已含：① codex 双窗字段（used_percent/window_minutes/reset_time 显式化，wham/usage）；② copilot premium_models⇄premium_interactions 档位+entitlement≥1500 高档判定；③ cursor hourly+weekly 双 reset+billingCycleEnd；④ antigravity Gemini 4 Argon 周窗（v1.3.74 新）；⑤ grok unified billing 单池语义+credits 不算独立额度。每家：情报核对→补解析→fixture 测试。另记教训：kiro 403→BANNED 状态机批量跳过。

## 风险分级总表（Pane 对等实现时标注）

低=官方 API（copilot/kiro/windsurf/zed/trae/codebuddy/qoder 直接做）；中=仿客户端头（grok npm 版本探测/antigravity UA/cursor cookie——对等实现前 owner 拍板）；高=解密他人凭据存储（claude cookie/zcode enc:v1:/workbuddy safeStorage——**保持拒绝**，workbuddy_local.rs:20-23 先例）。

## 验收

见各票；01/02/06 有 parse-tests 断言；03/05 需 owner 场景验收。

## 实施结果（2026-10-26）

| 票 | 状态 | 落点 |
|---|---|---|
| 01 | done | `providers::retry_after_secs`（delta + HTTP-date + 1h cap）+ `retry_hint` + tests；claude/apikey 与全部 `providers/*.rs` 的 HTTP 状态错误统一追加 `retry_after_s=`，bench 取“指数/下限”与服务器提示的较大值（parse-tests 570 provider 测试通过）。 |
| 02 | done | `identity_guard.rs`（三值比对 + SHA-256 指纹）与 tests；额度响应没有账号标识字段时按规约走 Missing 分支，Codex/Grok/Copilot 的取数本身按 account_id 重载账号，结构上不会把别号数字挂到本卡；身份字段一旦出现即可接入该守卫。 |
| 03 | done | `adaptive_interval.rs`（20% / 30min / 退避优先 / 60s 下限）+ tests；接入 `spawn_auto_refresh`，成功后回用户配置值。 |
| 04 | done | `parse_pasted_callback`（URL/path/query）+ tests；Codex 登录框新增“浏览器没返回就粘贴回调链接”，后端校验 state 后交给原轮询交换；Cursor 复用既有 auth_url；`advertised_callback_host` 双栈广告规则 + tests。pending 全在内存、从不落盘，隔离纪律天然满足；未改 codex/kiro 已注册的 redirect URI。 |
| 05 | done | Codex 的 `login_refresh_lock` 已是账号粒度锁 + 拿锁后重载账号（token 仍有效就直接复用，不再消费 refresh_token）；本轮补通用 `account_token_lock.rs`（共享锁 + 双检代际 + 两并发只刷一次测试），供其余刷新路径接入。 |
| 06 | done | 审计：codex 双窗字段已显式解析；cursor 已用 billingCycleStart/End；antigravity 已有 Gemini/3p weekly；grok 用量为单池 progress、credits 为 text 行；copilot 补 `premium_models` 回退 + entitlement≥1500 高档标签与测试；kiro 403 走凭据失效而非批量超时。 |
| 07 | done | `credentialPolicy::credentialFinality` + node test；`authNeedsReauth` 只对撤销类判“需重登”；Auth 中心展示沿用该判定。 |
| 08 | done | stale 额度副语（保留旧 bar + 缓存 chip）、行/组进行中互斥与禁用、成功 1.6s 自动消散（web session 补齐）、状态聚合进 title、隐私眼睛钮（localStorage 显示偏好）。 |
| 09 | done | `accountDiscovery.ts` 指纹 + 基线（首见静默/变化提示/消失删除）+ node tests；Auth 中心横幅只提示不导入，local-only。 |
