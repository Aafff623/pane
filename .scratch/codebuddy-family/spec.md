# CodeBuddy 家族合并：WorkBuddy + CodeBuddy CN

**Status:** ready-for-agent
**Feature dir:** `.scratch/codebuddy-family/`
**Date:** 2026-10-11
**调研底稿:** `temp/research/20261011-workbuddy-provider-research.md`

---

## Problem Statement

用户装了 WorkBuddy（腾讯桌面 AI 办公助手，`www.workbuddy.cn`）并且**已登录、免费档确实有 credits**，但 Pane 里三处都找不到它：不统计、自定义面板搜不到、设置界面没有图标。用户因此判断这是「登录态失效导致的缺陷」。

实况（2026-10-11 逐条核实）：**Pane 里不存在 WorkBuddy 这个 provider**。全仓 `rg -i workbuddy src src-tauri/src docs .scratch site` 仅一处命中，且那是 `site/vendor/magpie-routing/routing.js`——从 magpie 网关 vendored 进来的第三方站点 UI，与 Pane 功能无关。目录现有 70 个 provider，其中 `codebuddy` **只指向国际站 `www.codebuddy.ai`**。

所以三处「找不到」是**同一个原因**：目录里没有条目。不统计是没有代码路径；面板搜不到是候选列表来自目录；没有图标是没有 provider 就没有 `icon_key`。**与登录态无关**——Pane 里没有 WorkBuddy 凭据槽，不存在「登录 / 未登录」这个状态。

同时缺 **CodeBuddy CN**（`www.codebuddy.cn`）。

---

## Solution

把 WorkBuddy 与 CodeBuddy CN 合并进现有的 **CodeBuddy 家族**，作为同一家族的**租户**：三者共用同一套腾讯 CodeBuddy 账号体系（Keycloak realm `copilot`）、同一 `/v2/plugin` 计费网关、同一套「扫码登录 + bearer token」握手，差异只在 **主机名**与 **platform 参数**。

用户在设置里登录一次，卡片按租户各自成卡（沿用 `family@<hash>` 约定），卡面显示该租户的 **credits 计量条 + 套餐名 + 重置时间**；额度与消耗照现有约定进入「额度总览」与「用量」面板（ADR 0003）。桌面端登录态读不到时，回退到 `www.workbuddy.cn` 的**网页会话 cookie**。

三个租户的差异表（全部来自上游一手代码核对）：

| 租户 | 主机 | platform | 备注 |
|---|---|---|---|
| `codebuddy`（已存在） | `https://www.codebuddy.ai` | `ide` | 国际站，现网在用 |
| `codebuddycn`（新增） | `https://www.codebuddy.cn` | `ide` | 中国站 |
| `workbuddy`（新增） | `https://copilot.tencent.com` | `workbuddy` | 桌面 AI 办公助手 |

---

## User Stories

1. 作为 Pane 用户，我想在设置里用和 CodeBuddy 完全相同的扫码流程登录 WorkBuddy，这样我不用学第二套登录方式。
2. 作为 Pane 用户，我想登录 WorkBuddy 后立刻看到一张卡片，显示我的 credits 还剩多少，这样我知道免费额度还有没有。
3. 作为 Pane 用户，我想在卡片上看到我的套餐名（例如「体验版」），这样我知道自己在哪一档。
4. 作为 Pane 用户，我想在卡片上看到额度重置时间，这样我知道什么时候回血。
5. 作为 Pane 用户，我想在自定义面板里搜「WorkBuddy」就能搜到它，这样我能像其他 provider 一样启用 / 禁用 / 调分组。
6. 作为 Pane 用户，我想在设置界面的图标列表里看到 WorkBuddy 的图标，这样我能一眼认出它而不是一个空白格。
7. 作为 Pane 用户，我想让 WorkBuddy 的卡片用 WorkBuddy 自己的图标，而不是借用 CodeBuddy 的，这样我不会认错产品。
8. 作为 Pane 用户，我想让卡片标题能区分「CodeBuddy / CodeBuddy CN / WorkBuddy」，这样三个租户同屏时我不会混。
9. 作为 Pane 用户，我想在 auth center 里看到我的 WorkBuddy 账号行，这样我能查看、改名、删除它。
10. 作为 Pane 用户，我想给 WorkBuddy 账号起备注名，这样我能区分多个账号。
11. 作为 Pane 用户，我想让 WorkBuddy 的 token 过期时自动刷新，这样我不用反复重新登录。
12. 作为 Pane 用户，我想在刷新失败时看到「重新登录」的可读提示，而不是一个裸错误码。
13. 作为 Pane 用户，我想在企业账号（个人资源接口返回空）时也能看到真实用量，这样企业版不会被显示成 0。
14. 作为 Pane 用户，我想在不限量的套餐下看到「不限量」而不是一个假数字。
15. 作为 Pane 用户，我想让 WorkBuddy 的 credits 自动进入「用量」面板的积分节，这样我能看逐日趋势，而不用为它单独做一套账本。
16. 作为 Pane 用户，我想让 WorkBuddy 的额度环参与总览与托盘投影，和其他 provider 一致。
17. 作为 Pane 用户，我想在桌面端登录态读不到时用网页会话 cookie 兜底，这样我不至于完全没有数据。
18. 作为 Pane 用户，我想让 cookie 兜底是**显式选择**（而不是偷偷去翻我的浏览器），这样我清楚 Pane 读了什么。
19. 作为 Pane 用户，我想让 WorkBuddy 关闭时（没登录 / 没启用）不产生噪音卡片，和现有 provider 一致。
20. 作为 Pane 用户，我想让 `codebuddycn` 也能登录并显示额度，这样中国站不再缺席。
21. 作为 Pane 用户，我想在三个租户间看到**一致的卡面结构**，这样认知成本最低。
22. 作为 Pane 维护者，我想让新增租户只改**一张租户表**，而不是散落在四个层面各改一遍，这样加第四个租户不会漏。
23. 作为 Pane 维护者，我想让目录四层（Rust catalog / 前端镜像 / 图标 / i18n）的一致性由**可失败的测试**守住，而不是靠记忆。
24. 作为 Pane 维护者，我想让解析逻辑的测试不依赖网络，这样 CI 与本地都能稳定跑。
25. 作为 Pane 维护者，我不想让 Pane 去执行厂商的 Electron 二进制或解密其私有 envelope，这样不会引入脆弱且高风险的依赖。

---

## Implementation Decisions

### D1 — 合并形态：代码层一张家族表，目录层三个条目

**合并发生在代码层**：一张家族描述表（family id → 主机 / platform / 显示名）+ 一个参数化 provider 模块 + 一套登录流程。新增一个家族成员 = 表里加一行 + 目录层加一条。

- 目录层三个条目：`codebuddy` / `codebuddycn` / `workbuddy`，各自有显示名、图标、`disabled` / `categoryOverrides` 语义——面板搜索、设置图标列表、「关掉 CodeBuddy 不连坐 WorkBuddy」全部走既有机制。
- 登录存储沿用 `login-accounts/<family>.json` 约定，卡片 id 沿用 `family@<fnv1a>`。
- 图标按家族解析：`workbuddy` 用 WorkBuddy 自己的 mark；`codebuddycn` 与国际站共用 CodeBuddy 的 mark（一个品牌、两个区域站点）。
- 登录命令沿用前端既有的 `<family>_login_start/poll/cancel` 命名约定，每个成员一组薄包装，前端分发零改动。

> **2026-10-11 实现时修订**：本节原定「一个 `codebuddy` 目录条目 + 租户维度」（对应用户的「合并」）。落地核对发现前端按卡片 id 前缀**直查目录**（图标 / 显示名 / 面板搜索 / 启用禁用全以 family 为键），单条目方案需额外造一层「租户→家族」别名机制，收益为零；三条目方案全部走既有机制，且让「关掉 CodeBuddy 不连坐 WorkBuddy」自然成立。用户在被问及推荐后未持异议，按此实现。原被否方案（一个条目 + 租户）作为替代保留：若日后要合并成一张卡，只需改本节与目录层。

### D2 — 认证主通道：Pane 自管登录（扩展现有握手）

复用现有 `codebuddy_login` 的扫码登录流程，把**主机名**与 **platform** 参数化（现为硬编码 `www.codebuddy.ai` + `ide`）。握手形态与 `codebuddy_login::start/poll/refresh` 一致：

- `POST {host}/v2/plugin/auth/state?platform={platform}`（带 `X-No-*` 头与浏览器 UA）
- 轮询 `GET {host}/v2/plugin/auth/token?state={state}`
- 刷新 `POST {host}/v2/plugin/auth/token/refresh`（refresh token 走 `X-Refresh-Token` 头）
- 账号落库时把租户、`uid`、`enterpriseId`、`domain` 写进 `extra`（沿用现有 `Tenant::from_extra`）

**UA 是硬约束**：计费网关对额度接口做 UA 存在性校验，缺 UA 返 `403` / `code 10085`。

### D3 — 桌面客户端登录态：明确不做

本机实测 `%LOCALAPPDATA%\CodeBuddyExtension\Data\Public\auth\workbuddy-desktop.info` 存在且是明文 JSON，但其中的 `accessToken` / `refreshToken` / `nickname` / `phoneNumber` 都是 `{ "$wbEncrypted": 1, "envelope": "<base64>" }` 形状的**应用私有信封**（实测 envelope 长度 2740）。

上游 cockpit-tools 能解，代价是：**调用官方 WorkBuddy 的 Electron 原生绑定取密钥**（`process._linkedBinding('electron_browser_workbuddy_storage')` → `atRestSecretKey` → SHA-256），再走 AES-256-GCM + 自定义 AAD；整套 628 行，且需处理密钥轮换、5s 超时、密钥不匹配重试。

**决策：不读、不解密桌面客户端会话。** 理由：需要**执行厂商二进制**、密钥非 DPAPI/os_crypt 而是随客户端版本变化，属于脆弱且高风险的依赖；Pane 走自管登录（D2）已能拿到同类 token。CodexBar 亦明确选择不读。

> **2026-10-11 用户确认**：token 走 **Pane 自管登录**；不引入厂商私有信封解密。本决策已定，无需再评估。

### D4 — 兜底通道：网页会话 cookie（显式启用）

当自管登录不可用 / 用户未登录时，可用 `www.workbuddy.cn` 的网页会话兜底：

- `POST https://www.workbuddy.cn/billing/meter/get-user-resource-summary`（body `{}`，**根路径、不在 `/v2` 下**）
- 可选 `get-user-resource-free-packages` / `paid-packages` 取重置时间（须带 `PackageCodes`，否则 `400` / `code 10001`）
- **cookie 与浏览器 UA 强绑定**：换任何其他 UA（含旧版 Chrome 大版本）直接 `401`。需读取本机 Chrome 大版本号拼 UA，被拒时用上一个大版本重试一次。
- **必须是用户显式动作**（Chrome 导入或手贴 Cookie），不静默扫描浏览器。

复用 `doubao` 已有的 cookie 提取 seam（Chromium `Cookies` SQLite → 去 `v10` 前缀 → AES-256-GCM，密钥走 `platform::dpapi_unprotect` 解 `Local State` 的 `os_crypt`）。所有 OS 相关解密**必须经 `platform/` seam**（ADR 0002），不得内联 `#[cfg]`。

### D5 — 额度读法

主通道（bearer token）：

- `POST {host}/v2/plugin/billing/meter/get-user-resource`
  body `{"PageNumber":1,"PageSize":100,"ProductCode":"p_tcaca","Status":[0,3],"OnlyValidPeriod":true}`
  headers `Authorization: Bearer <token>`、`X-User-Id`、企业版加 `X-Enterprise-Id` + `X-Tenant-Id`、`X-Domain`
- 响应路径需多形态兜底：`/data/resources`、`/data/data/resources`、`/data/Response/Data/Accounts`、`/data/data/Response/Data/Accounts`、`/Response/Data/Accounts`
- **企业成员兜底**：`get-user-resource` 对企业成员返回**空 Accounts**，须改调 `POST {host}/v2/billing/meter/get-enterprise-user-usage`（body 空，企业 ID 走 `x-enterprise-id`）→ `{credit, limitNum, cycleStartTime, cycleEndTime, cycleResetTime}`；已用 = `credit`，剩余 = `limitNum − credit`；`limitNum == -1` 读作**不限量**
- 另可读 `get-dosage-notify`（套餐提示）与 `get-payment-type`（订阅类型）作为文本行

### D6 — 卡面形态

按 CodexBar 已发布的展示口径：

- **Credits 计量条**：总量 / 剩余（`CycleTotalCapacity` / `CycleRemainCapacity`），副行 `450 / 500 credits left`；已用 = 总量 − 剩余（**结转高于发放时读作 0%，不出现负百分比**）
- **套餐名**：`SubscriptionPackageName`（免费档为「体验版」）
- **重置时间**：`CycleEndTime` **无时区，按中国标准时间 UTC+8 解释**，取最早的未来周期
- **Reserved 行**：`CycleFrozenCapacity` 大于 0 时显示
- 总量为 0 时退化为 **Left / Total 文本行**（而不是一条 0/0 的条）
- 只累加 `CapacityUnit == "credits"` 的包

### D7 — 复用现有台账，不新建数据源

按 ADR 0003，WorkBuddy 有**绝对量** credits → 既进卡面，也进「用量」面板的积分节。实现上**不新建台账**：只要发出的 metric 满足既有形状（`label == "Credits"`、detail 以 `credits used` 结尾、首个 token 是绝对已用量），`lib.rs` 的既有钩子会自动写入 `credit_history.json`。这是本项**唯一**的台账接线点。

### D8 — 图标与 i18n

- WorkBuddy mark 本机已有：`temp/icon-pack/codexbar-full/workbuddy.svg`、`temp/icon-pack/magpie-full/workbuddy-color.svg`（CodeBuddy 的亦有：`temp/icon-pack/magpie-full/codebuddy-color.svg`）。落地为 `src/assets/providers/workbuddy.svg` 并在 `providerVisuals.ts` 注册。
- 新增租户名进 i18n 三语（现有 CodeBuddy 的条目为参照）。

### D9 — 机械闸门

`parse-tests/src/catalog_consistency.rs` 已断言「Rust catalog ⇔ 前端镜像」与「每个 `iconKey` 都能在 `providerVisuals.ts` 解析到」。本项必须让它继续通过——这是目录四层不漂移的强制约束，不得靠人工检查替代。

---

## Testing Decisions

**什么算好测试**：只测**外部行为**与**纯函数解析**，不测实现细节；不联网。

**在哪个 seam 上测**：沿用既有 `parse-tests` 测试夹具（用 `#[path]` 编译**真实源码** + `tauri-stub`，见 `parse-tests/src/lib.rs`）。这是项目已有的、唯一的 provider 测试 seam，**不新增测试通道**。`codebuddy_login` 已在夹具中注册，新租户沿用同一入口。

**要覆盖的用例**（纯解析，喂固定 JSON 夹具）：

1. 个人资源包 → credits 总量 / 剩余 / 已用正确
2. 多包只累加 `CapacityUnit == "credits"`，非 credits 包被忽略
3. 结转高于发放 → 已用读作 0，不出现负值
4. 企业成员：`get-user-resource` 空 Accounts → 走 `get-enterprise-user-usage` 兜底，`limitNum − credit` 正确
5. `limitNum == -1` → 不限量
6. `CycleEndTime` 无时区字符串 → 按 UTC+8 解释，取最早未来周期
7. `CycleFrozenCapacity` → Reserved 行（为 0 时不显示）
8. 总量为 0 → 退化为 Left / Total 文本行
9. 缺 UA 的请求形状被拒绝 / UA 头存在性（对照 `403` / `code 10085` 的既有行为）
10. 桌面客户端的 `$wbEncrypted` envelope **不被误读**（防回归：解析器不得把 envelope 当 token）

**门禁**：`catalog_consistency` 两个测试必须绿（Rust⇔TS 家族一致、iconKey 可解析）。

**先例**：`parse-tests/src/lib.rs` 已注册 `codebuddy_login`；目录一致性测试已存在。命令与 PATH 前置见 `AGENTS.md`。

---

## Out of Scope

- **读取 / 解密桌面客户端的 `$wbEncrypted` 会话**（D3 已给理由与实测代价）
- **WorkBuddy 本地会话历史**（`~/.workbuddy/projects/*.jsonl` 的 `providerData.rawUsage`）→ spend 面板。credits ≠ 美元，且只覆盖本机跑过的会话，需单独决策
- **签到 / 切号 / 多实例**等 cockpit 的写操作能力（Pane 是只读额度面板）
- **付费档位与 `get-user-request-usage`** 的响应形态（上游只在免费档实测过，标为待确认）
- **官网 `site/` 同步**（站点是 1:1 复刻演示，另案）
- CodeBuddy 家族的**账号切换**（把 Pane 的登录写回官方客户端）

---

## Further Notes

**一手来源**

- Pane 源码：`src-tauri/src/provider_catalog.rs`、`src-tauri/src/providers/codebuddy.rs`、`src-tauri/src/codebuddy_login.rs`、`src-tauri/src/login_accounts.rs`、`src/providerCatalog.ts`、`src/providerVisuals.ts`
- `temp/cockpit-tools` @ `b0d8862`（2026-10-11）：`crates/cockpit-core/src/modules/workbuddy_oauth.rs`、`codebuddy_cn_oauth.rs`、`src-tauri/src/modules/workbuddy_auth_crypto.rs`（D3 的代价来源）、`CHANGELOG.md:762`
- CodexBar 0.71.2 / PR #4227（2026-10-04）：`docs/workbuddy.md`（D4/D6 的展示口径与 UA 绑定）
- 腾讯计费口径：<https://intl.cloud.tencent.com/document/product/1316/84380>（credits 为 CodeBuddy 统一计费单位）；免费档 250 credits / 14 天见 <https://www.workbuddy.ai/docs/ide/Account/Subscription>

**本机实测（2026-10-11）**

- `~/.workbuddy` 存在且当日活跃（`app/` 下有 `session` / `sessions.json` / `app-config.json`）
- `%LOCALAPPDATA%\CodeBuddyExtension\Data\Public\auth\workbuddy-desktop.info` 存在，8897 字节明文 JSON，**token 字段为 `$wbEncrypted` 信封**（结构探针：`temp/scripts/probe-workbuddy-auth.mjs`，只打印类型与长度、绝不打印值）

**待确认（不阻塞实现）**

- 付费档位 / `get-user-request-usage` 的响应形态
- CodeBuddy CN 与 WorkBuddy 的登录是否互相授权对方 origin（上游明确标注未验证）
- 免费档在 Pane 上的实际读数需用户登录后实机核对

**一个顺带观察**：Pane 的 `site/vendor/magpie-routing/routing.js`（从 magpie 网关 vendored 的第三方站点 UI）里已出现 WorkBuddy 的 credits 计量与拒绝提示——说明它在生态里已是公认的一等 provider，Pane 缺席。该文件与 Pane 功能无关，勿据此以为站点已支持。
