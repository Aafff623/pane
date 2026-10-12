# ZCode Weekend 额度行（智谱 / GLM 卡片）

**Status:** needs-triage — 有 2 个待确认点（见 Open Questions）
**Feature dir:** `.scratch/zcode-weekend/`
**Date:** 2026-10-12
**接口配方:** 已实测拉通（见下"已核实事实"），非推测

---

## Problem Statement

ZCode（智谱的编码客户端）会给账号发一份**周末赠送额度**（计划名 `ZCode Weekend Build`，GLM-5.3-Flash 共 3 亿 token）。用户希望 Pane 把它显示出来——挂在**已有的智谱 / GLM 卡片**上，作为详情卡片里的**第三栏**。

用户原话（2026-10-12）：

> 如果用户有智谱的 API key，并且接到了智谱的 provider（咱们面板现在已经有它了），只需要在后续详情卡片的后面再补上一栏。咱们现在有两栏（会话和 web search），可以在后面再补一栏，作为 ZCode 的 weekend。
> 如果用户没有智谱的 API Key，但在自定义面板里开启了 z.ai 的 provider，那就只显示 ZCode 的 Weekend。
> 当然，这需要读取到本地 ZCode 的登录态，并获取到实际的数字。如果现在不是周末，或者读取不到这个参数，这块也是不显示的。

参照截图：GLM 卡片现有两行「会话」「Web Searches」，第三行加「ZCode Weekend」。

---

## 已核实事实（2026-10-12 实测，全部跑通）

### 1. 接口

```
GET https://zcode.z.ai/api/v1/zcode-plan/billing/balance?app_version=3.14.3
```

请求头（**缺 `X-Device-Mid` 必回 `400 / code 3001 "parameter error"`**——magpie 踩过这个坑，其注释明说服务端只校验该头存在）：

| 头 | 值 |
|---|---|
| `Authorization` | `Bearer <JWT>` |
| `X-Device-Mid` | 任意 UUID v4（服务端只校验存在；可复用 `~/.config/magpie/zcode-device-mid`） |
| `HTTP-Referer` | `https://zcode.z.ai` |
| `X-Client-Language` / `X-Client-Timezone` | `zh-CN` / `Asia/Shanghai` |

响应（`code == 0` 才算成功）：

```
data.plans[]    → { plan_id, user_plan_id, name, status, ends_at, entitlements[] }
data.balances[] → { plan_id, user_plan_id, entitlement_id, show_name,
                    total_units, used_units, remaining_units, expires_at,
                    period_start, period_end }
```

- `total_units` 可能是字符串也可能是数字（magpie 用 `zcodeNum` 兼容两种）。
- 计划已过期但 `status` 仍写 `active` 时，**要按 `ends_at` 自行判过期**（magpie `zcodeStartBalance` 就是这么做的），并把该 plan 对应的 balance 桶剔除。

### 2. 登录态来源（本地）

| 来源 | 说明 |
|---|---|
| `~/.zcode/v2/credentials.json` | `zcodejwttoken` 一项即会话 JWT |
| 密文格式 | `enc:v1:` + base64url(iv) `.` base64url(tag) `.` base64url(ct) —— **AES-256-GCM** |
| 密钥 | `sha256(seed)`；`seed` = 环境变量 `ZCODE_CREDENTIAL_SECRET`，否则 `"zcode-credential-fallback:win32:<home>:<用户名>"`（Windows 平台串是 `win32`，用户名取 `DOMAIN\user` 最后一段） |
| 设备号 | Pane 自己生成并持久化一个 UUID v4（**不要**去读 magpie 的），沿用 `~/.config/magpie/zcode-device-mid` 的存放思路 |

> 参考实现（Go）：`temp/magpie-yetone/internal/provider/zcode.go`（`zcodeSecret`/`zcodeDecrypt`/`zcodeCredentialsPath`）、`zcode_start.go`（`zcodeStartBalance`）、`zcode_device.go`（`X-Device-Mid`）。
> 本机实测脚本（Node，只读、不打印 JWT）：`temp/scripts/pull-zcode-weekend.mjs`。

### 3. 本机实测读数（2026-10-12 上午）

- 计划：**ZCode Weekend Build**（`plan_id=zcode-v3-start-plan-1010`，status=active，`ends_at=2026-10-12T01:00:00Z`）
- 桶：**GLM-5.3-Flash · 115,465,592 / 300,000,000 · 剩余 61.5%**（周期 2026-10-10T12:07:24Z → 2026-10-12T01:00:00Z）

---

## Solution（待确认）

### 显示位置

| 情况 | 显示 |
|---|---|
| 有智谱 API key 且已接智谱 provider（`zai`） | 在**智谱卡片**的详情里追加第三行「ZCode Weekend」（现有两行：会话、Web Searches） |
| 无智谱 key，但自定义面板启用了 `zai` provider | 只显示「ZCode Weekend」这一行 |
| 无智谱 key 且 `zai` 未启用 | 不显示 |

### 显示条件（全部满足才显示该行）

1. 能读到本地 ZCode 登录态（`~/.zcode/v2/credentials.json` 的 `zcodejwttoken` 且能解密）；
2. balance 接口 `code == 0`；
3. 存在**未过期**的周末计划（计划名含 `Weekend`，或该桶属于该计划）——即"现在不是周末 / 没有周末赠送"时不显示；
4. 任一环节读不到 → **静默不显示**（不报错、不留空行、不产生噪音卡片），与用户要求一致。

### 行的形态（沿用现有详情行的视觉约定）

- 标签：`ZCode Weekend`
- 主值：进度条（剩余百分比）+ 明细 `已用 / 总量`（如 `115,465,592 / 300,000,000`）
- 副值：重置时间（`period_end`，按本地时区渲染"X 小时 Y 分后重置"，与「会话」行同款）
- 悬浮：核心计量单位（token）

---

## Open Questions（2026-10-12 用户已答，全部关闭）

1. ~~"只显示 ZCode Weekend"时挂在哪个卡片上？~~ → **仍然是一张 ZAI 卡片，后面补一行就行**（不新建 provider、不新建卡片形态）。
2. ~~本地 ZCode 多账号时显示哪个？~~ → **本机 ZCode 没有多账号，先不用管**；只处理**第一个账号**（`~/.zcode/v2/credentials.json` 里第一个可解出 JWT 的项）。

## 待确认的依赖（已关闭）

- ~~第二个 ZCode 账号来源~~ → 用户确认：**ZCode 账号来源就一个，就第一个账号**。截图里那两组读数不来自本机登录态，本次不处理。

