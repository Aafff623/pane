# WorkBuddy / CodeBuddy 三渠道补齐（v2）

**Status:** ready-for-agent
**Feature dir:** `.scratch/workbuddy-family-v2/`
**Date:** 2026-10-19
**上游参考:** `changexbc/workbuddy-switch` @ `v0.1.62`（只读取证，本地副本在 `temp/research/workbuddy-switch-0162/`）
**前序工单:** `.scratch/codebuddy-family/`（7/7 已 done，本 spec 不重复其已完成项）

---

## 1. 问题陈述

Pane 的 CodeBuddy 家族（`codebuddy` / `codebuddycn` / `workbuddy`）已经跑通扫码登录、多账号、credits 卡面和企业兜底，但对照上游与用户决策，还差三块：

1. **档位不完整**：现有 `workbuddy` 是国内档（`copilot.tencent.com` / platform `workbuddy`）。真正的国际档是 `www.workbuddy.ai` / platform `workbuddy-ai`，Pane 完全没有；而且 CodeBuddy 国际版（`.ai`）**不是** WorkBuddy 国际版。
2. **账号/渠道语义不完整**：同一账号从扫码 / 本地桌面登录态 / 网页 Cookie 进入时，Pane 没有统一的来源标注、没有「当前官方客户端账号 ≠ Pane 监控账号」的显式区分，也没有显式导入入口；多通道还可能产出同 ID 的重复卡。
3. **用量完全没有本地事实**：WorkBuddy CN/AI 的 `~/.workbuddy*/projects/*.jsonl` 与 CodeBuddy CLI 的 `~/.codebuddy/projects/*.jsonl` 一个都没扫；现有 credits 只是「远端累计余额的快照差额」，不是 Token 事实。

本 spec 只做这三块，**不做**切号、会话迁移、签到、Agent Companion、WebUI。

---

## 2. 本地现状（2026-10-19 核实，含证据）

### 2.1 已有能力（不重造）

| 模块 | 现状 |
|---|---|
| `codebuddy_login.rs` | `FAMILIES` 表 3 行；扫码 `state → auth/token` 轮询；`refresh`；`Tenant` 头 |
| `login_accounts.rs` | `family@fnv1a` 稳定卡 ID、按 `account_id` upsert、Vault-first 存 token |
| `auth_center.rs` | 登录型账号行（id/label/email/maskedKey/capturedAt/kind） |
| `providers/codebuddy.rs` | credits 计量条、Reserved、UTC+8 重置、企业兜底、`limitNum=-1` 不限量、UA 强制 |
| `providers/workbuddy_local.rs` | **仅国内档** `workbuddy-desktop.info`，只读、拒 `$wbEncrypted` |
| `providers/workbuddy_web.rs` | 国内网页 cookie 兜底（显式开关、UA 绑定） |
| `usage_history.rs` | credits 台账（按卡 ID、按天 max、「首日无基线」） |
| `spend.rs` | 16 个本地来源；**无** WorkBuddy/CodeBuddy 条目 |

### 2.2 缺口（本次要补）

| # | 缺口 | 影响 |
|---|---|---|
| G1 | 缺 `workbuddyai` 家族（host / platform / catalog / i18n / 图标 / 命令） | 国际档用户完全不可见 |
| G2 | 档位隔离不完整：无 domain 校验、桌面文件只认 CN、cookie 兜底无档位门 | CN/AI token 可能串用 |
| G3 | `PENDING` 是全局单槽，跨三个家族共用 | 第二个扫码会静默废掉第一个 |
| G4 | refresh 无单飞；`write_back` 是整表读改写 | 并发刷新互相覆盖已轮换 token |
| G5 | 错误只有「rejected / 其他」两档 | 401/403/429/WAF/断网不可区分 |
| G6 | `X-Auth-Refresh-Source` 固定 `ide-main` | 国际档刷新被拒（上游实测 `invalid_grant` 12153） |
| G7 | 桌面登录态不落 Pane 账号库、无导入入口、无 source 标注 | 无法在授权中心管理；「官方当前账号」不可见 |
| G8 | 桌面裸卡与扫码账号卡可同 ID 重复 | 同一次刷新出现两条同 ID 快照 |
| G9 | 卡面不显示数据来源，也不显示单个资源包到期 | 用户不知道数据从哪来、哪个包先过期 |
| G10 | 无 WorkBuddy / CodeBuddy 本地 Token 扫描 | 用量面板缺这块真实消耗 |

---

## 3. 三渠道模型（本次设计的核心）

```text
Provider/Variant
  └── Account（稳定身份 = family + account_id）
       ├── AuthSource   : qr | desktop | web
       ├── QuotaSource  : pane-login | desktop-session | web-session
       └── UsageSource  : credits-snapshot | workbuddy-jsonl | codebuddy-cli-jsonl
```

**归属规则**

- 认证来源写进 `LoginAccount.extra.sourceKind`（`qr` / `desktop`），随账号稳定保存。
- 额度来源不新增数据库：**卡面附一行 `Source` 文本**，只在非默认渠道（desktop / web）出现；Pane 扫码账号不显示（它就是默认）。
- 用量来源按 **family 级**归卡：WorkBuddy 日志没有账号字段（上游同样不做日志级账号归因），所以 CN 日志进 `workbuddy` 卡、AI 日志进 `workbuddyai` 卡；卡名注明「本机日志」。不伪造账号归属。
- **Credits 与 Tokens 永不混算**：credits 继续走 `usage_history` 的 `UsageUnit::Credits`；本地 JSONL 走 `spend` 的 tokens/dollars。

**去重规则**

- 账号：同 family 同 `account_id` → 一张卡（已有）。
- 桌面裸卡：若账号库里已有同 uid 的 Pane 登录，**不再产出桌面卡**（本 spec 修复 G8）。
- JSONL：同一文件内用 `(timestamp,input,output,cache_read,cache_write,model)`
  指纹去重；`subagents/` 整体排除。**跨文件整份副本暂不去重**（`file_days`
  逐文件缓存与跨文件集合冲突；上游 `token_stats` 会跨文件去重）。CN/AI/CLI
  三个来源互不跨源去重。

---

## 4. 远端 Issue 重新分类

远端 Epic #7 与 #8–#27 **保持原样，不编辑、不评论、不关闭**（本轮硬约束）。本地裁决如下：

| Issue | 裁决 | 本 spec 落点 |
|---|---|---|
| #8 WorkBuddy AI 国际版 | **实施** | ticket 01 / 02 |
| #9 多账号中心 | **实施（收缩）**：来源标注、桌面导入、当前客户端账号识别、渠道筛选；不做批量复制/跨设备备份 | ticket 04 / 06 |
| #10 OAuth 与 Token 生命周期 | **实施** | ticket 03 |
| #11 资源包与到期 | **实施（收缩）**：包级明细行 + 7 天临期 + 来源行；不新建独立 Dashboard | ticket 05 |
| #12 官方请求用量 | **先验证，后决定**：没有可验证账号就不发数据；只写探针与判定标准 | ticket 09 |
| #13 本地 Token 统计 | **实施** | ticket 07 |
| #27 QA 矩阵 | **实施（收缩到只读能力）** | ticket 08 |
| #14–#19 | **不做**（写官方客户端 / IDE / 插件 / 切号 / 会话） | — |
| #20 | **不做自动轮换**；「建议优先看哪个账号」由 ticket 05 的临期排序承担 | ticket 05 |
| #21 | **不做**（JetBrains 切号） | — |
| #22 | **不装 Hook、不改外部配置**；只保留本地 429 错误文案 | ticket 03（错误分类） |
| #23 | **不做**（签到/成长） | — |
| #24 | **不做**（Agent Companion） | — |
| #25 | **不做**（加密备份属通用账号体系，另案） | — |
| #26 | **不做**（Pane 已有本地只读 API，不为 WorkBuddy 新建宿主） | — |

---

## 5. 实施决策

### D1 — 家族表继续做唯一事实源
`workbuddyai` 只加一行；新增字段仅限真实差异：
- `refresh_source`（`ide-main` / `plugin`）
- `billing_paths()`（国际档先 `/billing/meter/...`，仅 404 回落 `/v2/...`）
- `desktop_auth_file()`、`data_root()`（CN/AI 分离）

### D2 — 档位隔离是硬门
- 扫码落库前校验 token `domain`：AI 必须以 `.workbuddy.ai` 结尾；CN 不能是 `.workbuddy.ai`。不匹配 → 拒绝保存 + 可读错误。
- 桌面文件按档位读各自文件名；AI 不读 CN 文件，反之亦然。
- 网页 Cookie 兜底**只给 CN**；AI 走自己的扫码/桌面通道。

### D3 — 扫码状态按家族隔离
`PENDING` 从 `Option<Pending>` 改为 `HashMap<family_id, Pending>`；同一家族内新扫码覆盖旧的，跨家族互不影响。

### D4 — 刷新单飞
按 `family:account_id` 维护 `tokio::sync::Mutex`；持锁完成「刷新 → write_back」。同一账号并发刷新只发一次网络请求。

### D5 — 错误分类（可读、不泄密）
`billing_post` 返回 `status` 与业务 `code`，映射为：
- 401/403 → `session expired — sign in again`
- 403 + code 10085 → `vendor rejected the browser fingerprint (UA required)`
- 429 → `rate limited — try again later`
- 其他非 0/200 → 原始 `message`
- 传输失败 → `network: …`
不记录、不打印 token。

### D6 — 官方客户端账号识别只读
- 桌面文件顶层 `account` 即「官方当前账号」。
- 「Pane 监控账号」是账号库里的一张卡。
- 显式「导入当前客户端登录」：读明文 token → `record_login`（upsert uid）→ Vault；**绝不回写官方文件**。`$wbEncrypted` 只报状态，不导入。

### D7 — 数据来源与新鲜度
- `Metric` 增加一行 `Source`（仅降级渠道）。
- 7 天临期：`expireAt - now <= 7d && remaining > 0` → 行尾 `· expires in Nd`。
- 未知到期不编造，显示 `expiry unknown`。

### D8 — Token 统计口径
- 单位：Tokens。credits 不加进 tokens。
- 字段：fresh input = `input - cache_read`；output；cache read；cache write；reasoning 单独取但在 total 中按上游口径计入 output 侧。
- 无价格目录 → `unpriced`（token 仍计数，美元不猜）。
- 31 天窗口，与 `recent_jsonl_files` 一致。

### D9 — 不新增测试通道
所有解析/路由测试进 `parse-tests/`（`#[path]` 编译真实源码 + tauri-stub）。

### D10 — 官方流水先验证
`get-user-request-usage` 的形状来自上游，但 Pane 没有可用账号实测。ticket 09 只产出**探针脚本与判定标准**；不接 UI、不落库。

---

## 6. 验收标准（最小可用闭环）

1. 设置里能扫码登录 WorkBuddy AI（`workbuddy-ai`），出现独立卡；CN 与 AI 同 UID 不互相覆盖。
2. AI 账号的 token 若带 CN domain 被拒绝保存并显示可读错误；反向同理。
3. 同一时刻对 CN 与 AI 各发起一次扫码，两边都能完成或各自超时，不互相报 `no pending sign-in`。
4. 同一账号并发刷新只产生一次 refresh 请求，写回后 token 是新的。
5. 授权中心能看到账号来源（qr / desktop）与企业名；能显式把本机 WorkBuddy 明文登录态导入为 Pane 账号。
6. 卡面在桌面/Cookie 渠道出现 `Source` 行；多资源包时每个包有剩余与到期，7 天内到期有提示。
7. 用量面板出现 WorkBuddy CN / AI / CodeBuddy CLI 的 token 行（本机存在日志时），credits 仍在独立积分节，两者不混算。
8. `parse-tests` 全绿；`tsc` + `vite build` 通过；`cargo check` 通过。

---

## 7. 明确不做

- 修改 WorkBuddy / CodeBuddy 官方客户端的任何文件、进程、登录态。
- 会话复制 / 同步 / 迁移。
- 自动切号、自动轮换、签到、活动、猫猫旅行。
- 安装或修改任何第三方 Hook / 插件配置。
- Agent Companion 悬浮栏、独立 WebUI、新的高权限本地 API。
- 解密 `$wbEncrypted`。
- 修改远端 GitHub（issue / PR / release / push）。
- 未经 owner 验收就发版。

---

## 8. 风险

| 风险 | 缓解 |
|---|---|
| 国际档 OAuth platform / billing 路径未经 Pane 实机验证 | 只按上游 v0.1.62 已发布代码落地；真实扫码由 owner 验收；失败给可读错误，不假装成功 |
| 本地 JSONL 字段随客户端版本变化 | 解析器容错 + fixture 覆盖；缺字段不记账 |
| 刷新单飞增加锁 | 锁粒度仅到单账号，不阻塞其他账号 |
| 新增 spend 扫描拖慢刷新 | 31 天窗口 + 目录上限（复用 `recent_jsonl_files`） |
| 桌面导入把凭据写进 Pane | 仍走 `record_login` 的 Vault-first；失败保留旧账号 |

---

## 9. 上游一手来源

- `variant.rs` — CN/AI 域名、platform、认证文件名、数据根、billing 路径候选
- `auth_file.rs` — 认证文件四段结构、uid/email/企业字段、`$wbEncrypted`
- `refresh.rs` — `X-Auth-Refresh-Source: plugin`、`invalid_grant` 12153
- `credits.rs` — 资源包字段别名、`EXPIRING_SOON_DAYS=7`、企业 `limitNum=-1`
- `official_usage.rs` — `get-user-request-usage` 请求/分页/去重（仅作验收依据）
- `token_stats.rs` — JSONL 字段路径、指纹去重、CN/AI/CLI/IDE 四个独立来源
