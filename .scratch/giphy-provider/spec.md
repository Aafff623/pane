# GIPHY 调用量卡片（giphy-provider）

**Status:** implemented（2/2 票，2026-10-15；parse-tests 798:0:13、tsc/vite、i18n 门禁、目录一致性全绿；UI 验收待 owner）
**来源**：owner 2026-10-15 要求——把 GIPHY 的每日调用量做进看板，落在「额度总览」的 MCP 分组里。
**前置**：接口已逆向并实测（详见记忆 `giphy-no-quota-api` 与 `temp/scripts/probe-giphy-*.ps1`）。

## 已逆向的接口事实（实测，非推测）

- 主机 `https://partner-hub-api.giphy.com/api/v1/partner`，鉴权 `Authorization: Bearer <面板 access_token>`（cookie 里的会话令牌，**不是 api_key**；api_key 当 Bearer 会 403 `Not a Partner Hub User`）。
- `GET /me/apps` → 账号下全部应用（`apps[].id / name / api / status`）。owner 账号实测 2 个：`localdev`(673068，全 0) 与 `hermes-search`(657549，有数据)。
- `GET /apps/{id}/stats/api-calls-count?start_date=YYYY-MM-DD&end_date=YYYY-MM-DD` → **按天**序列 `{"stats":[{"date":"2026-10-14","count":17}, …]}`。
- `GET /apps/{id}/stats/total-counters?同参数` → `{"total_api_calls":21,"total_media_loads":16}`。
- **粒度限制**：只有日桶（`interval=hour` 无效），且**当日数据为空**（统计按天离线聚合、次日才落）。→ **「100 次/小时」的实时剩余做不出**，本票只做「每日调用量」。

## Solution

新增 provider 家族 `giphy`（category = mcp）：

1. **凭据**：用户在「密钥与额度」页粘贴 **GIPHY 开发者面板的 access_token**（提示文案写明不是 api_key）。走既有 provider key lane（钥匙串 + env 兜底），复用 `stored_api_key`。
2. **不存 appId**：每次刷新调 `/me/apps` 自动枚举账号下全部应用，**每个应用一行指标**（label = 应用名，value = 近 7 天调用次数），避免「key→app 映射接口不存在」的问题。
3. **卡片内容**：plan 显示「Beta key · 官方限额 100 次/小时」；每应用一行 text 指标；warning 说明「统计按天聚合，当日数据次日更新」；`dashboard_url` 指向面板。
4. **TTL 缓存 30 分钟**：刷新循环 5 分钟一次，避免把 partner-hub 打爆（且统计本来就是按天的）。
5. 归类与默认值：`category = Mcp`（额度总览的 MCP 分组）、进 `DEFAULT_DISABLED_PROVIDERS`（新装不默认铺开）；粘贴凭据后按既有逻辑自动启用。

## Implementation Decisions

- `provider_catalog.rs` 加定义：`family_id: "giphy"`、`display_name: "GIPHY"`、`QueryKind::NativeBalance`、`supports_api_key: true`、`category: ProviderCategory::Mcp`、`icon_key: "giphy"`；同步 `src/providerCatalog.ts` 镜像（catalog_consistency 测试会双向校验）。
- 新模块 `src-tauri/src/providers/giphy.rs`：`pub async fn giphy_snapshot() -> Snapshot`；纯函数 `rows_from(apps: &Value, series: &BTreeMap<i64, Vec<(String,u64)>>) -> Vec<Metric>` 便于单测；TTL 缓存与 `searchquota` 同款（Mutex + Instant）。
- `providers/mod.rs` 注册模块；`lib.rs` 两处分派（snapshot match + guarded 列表）+ `DEFAULT_DISABLED_PROVIDERS` 追加 `"giphy"`。
- 图标 `src/assets/providers/giphy.svg`（自绘中性标记，不搬官方 logo）+ `providerVisuals.ts` 注册。
- `PROVIDER_CRED_INFO`（main.ts）加 giphy 提示：粘贴面板 access_token、从浏览器 cookie 取。
- 日期：近 7 天 = 今天往前 6 天（与 stats 端点实测行为一致）。

## Testing Decisions

- `giphy.rs` 的纯解析函数单测进 parse-tests（harness 已收录 `providers/mod.rs`，新模块自动随行）：apps JSON + 日序列 → 指标行数/文本正确；空 stats 不崩。
- `catalog_consistency` 双向校验（Rust ↔ TS）与 icon 注册校验必须过。
- 前端 tsc + vite。
- **UI 验收由 owner 亲自**：MCP 分组里出现 GIPHY 卡、粘 token 后出数据、日数字与面板一致。

## Out of Scope

- 不做「本小时剩余 100/小时」（统计按天聚合，物理上拿不到）。
- 不做 refresh_token 续期（未实测，怕轮换失效）；遇 401 走既有错误展示，提示用户重新粘贴。
- 不改 search_keys / searchquota 那条 lane。
- 不搬 GIPHY 官方 logo（避免商标问题，用自绘中性图标）。
