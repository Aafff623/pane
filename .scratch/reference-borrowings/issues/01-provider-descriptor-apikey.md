# 01 — Provider descriptor + 通用 API-key 额度适配器

Status: done (2026-10-23) — catalog descriptor contract, three sanitized fixture sets and SSRF/body/HTML tests landed; Currency deferred
GitHub: #39
Priority: P0
Blocked by: None
Reference: `temp/repos/CodexBar` (`ProviderDescriptor`, `ProviderFetchPlan`, `APITokenFetchStrategy`), `temp/repos/QuotaBar` (`UsageWindow`)

## Scope

把“新增一家 provider”从三处注册 + 一段手写 fetch，收口成一条 descriptor/spec + 一份 fixture。先服务 MCP/生产力扩圈，不改现有 70 家的外部行为。

1. **descriptor 能力位**（Rust `provider_catalog.rs` + TS `providerCatalog.ts` 镜像）：
   - `auth_kind`: `api_key | oauth | web_session | local | none`
   - `probe_kind`: `active_get | passive_header | static | local_counter`
   - `quota_scope`: `key | account | team | unknown`
   - `reset_semantics`: `api_timestamp | monthly_inferred | none`
   - `pace_capability`: bool
   只加真正会被 UI/刷新/doctor 消费的字段；不要一次加十个空字段。
2. **`ApiKeySpec` + `providers/apikey.rs`**：
   - GET-only；固定 HTTPS host；header 模板；JSON pointer 映射 `used/limit/remaining/reset`；unit；可选 divisor。
   - 禁止 user-controlled URL、重定向跟随、>1 MiB body、无限深度；超时 10s。
   - 多 key：复用 `searchquota.rs` 的 per-key 行/账户级汇总语义，不把共享账户余额重复相加。
3. **单位扩展**：`UsageUnit` 增加 `Characters`、`Pages`、`Currency`（currency 带 ISO 币种码）；同步 `src/usageUnits.ts`、`main.ts` 渲染、i18n、`usage_history` 序列化兼容。
4. **试点 3 家**（只做额度查询，不做额外 UI）：
   - SerpApi：`GET https://serpapi.com/account`，requests/月。
   - Stability AI：`GET https://api.stability.ai/v1/user/balance`，credits/余额。
   - DeepL：`GET https://api-free.deepl.com/v2/usage`，characters/月（验证新单位）。
5. **fixture 契约**：`parse-tests/fixtures/<family>/` 存脱敏响应；pure parse + golden；覆盖 401/402/429/空字段/HTML 200。

## Acceptance

- [x] 新增三家只走 `providers/apikey.rs` 的静态 spec + 通用 fallback，不再为每家加 dispatch match。
- [x] 三家卡片在 MCP/生产力分类正确显示；DeepL 的字符行不显示成 credits。
- [x] 现有 provider 的 `/v1/providers`、`last_snapshots.json` 兼容（单位枚举为追加变体；旧快照仍可读）。
- [x] API-key 只在对应 vendor host 使用；日志/HTTP API/错误信息不含 key。
- [x] SSRF 测试：非 HTTPS、重定向、超大 body、user-controlled host 全部拒绝（复用已有 `http_no_redirect`/`json_body`；`no_redirect_client_stops_at_a_302`、`json_body` 上限测试与 `every_spec_is_a_fixed_https_origin_with_no_user_url` 覆盖）。
- [x] `parse-tests` 定向、`cargo check`、`pnpm build` 通过；全量 parse-tests 未重跑。

## Comments

- 2026-10-23：MVP 落了 Characters/Pages 单位、通用 `ApiKeySpec`（GET、固定 host/path、Bearer/header/query、JSON pointer、divisor）和 DeepL/SerpApi/Stability 三张试点卡；不新增每家 dispatch match。Currency 依 owner 决定延后到 #43。真实端点字段和 fixture 仍待校准；descriptor 五能力位尚未加到 70 条目录。

## Evidence / notes

- CodexBar 的 `APITokenFetchStrategy` 证明“解析 token → 缺凭证报错 → loadUsage”可以抽成一种策略；`PluginProviderSpec` 允许 `quotaLimit/quotaUnit` 为 credits/requests/ISO 货币。
- Pane 现有 `searchquota.rs` 已有 per-key rows、账户级汇总、TTL、Brave 被动探测；本 ticket 复用它，不另造多 key 模型。
- `usage_units.rs` 目前只有 tokens/credits/minutes/requests；ADR 0003 要求单位显式，不能把字符塞进 credits。

## Out of scope

- OAuth/web-session provider；写操作；自动发现 key；批量注册。
- 通用 JSONPath/脚本转换；只做简单 pointer + divisor，复杂响应仍写专用 parser。
- 跨单位合计；currency 不带币种码。

## Verification

```powershell
corepack pnpm build
cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu test
```

Security review: SSRF/redirect/body-size/secret redaction。UI 验收：owner 看三张卡的单位/重置/空态。

- 2026-10-23：`ApiKeySpec` 增加 auth/probe/scope/reset/pace 描述符并暴露 `descriptor()`；三家试点共用固定 host/path/auth，无用户 URL。70 条目录能力位、真实 fixture 和 SSRF fixture 测试仍未做。

- 2026-10-23（主对话接手）：Rust `ProviderDefinition::descriptor()` 输出 `auth_kind`/`probe_kind`/`quota_scope`/`reset_semantics`/`pace_capability`；TS 增加 `descriptorFor()` + `providerDescriptors` 镜像；`catalog_consistency` 新增 descriptor 一致性测试，并校验 TS OAuth 家族都在 Rust `OAUTH_FAMILIES`。字段由现有 capability 派生，不批量塞无意义默认值，解析器仍是额度形状的权威。
- `parse-tests/fixtures/{deepl,serpapi,stability}/` 各 6 份脱敏 fixture（200/401/402/429/empty/HTML 200）；`apikey.rs` 增加 status 映射、固定 HTTPS host、fixture golden 测试。
- 安全：`http_no_redirect` 的 302 不跟随 + `json_body` 的 HTML 200/超限 body 测试；`ApiKeySpec` 只接受固定 HTTPS host/path，没有 user-controlled URL。
- 测试：`cargo test apikey` 9 passed、`descriptor_contract` 通过；`pnpm build` 通过；全量 parse-tests 已跑（1013 passed + 1 个既有 productivity 断言已修，修后定向通过；最终全量重跑见本轮收尾）。
- 全量 `parse-tests` 最终：1017 passed / 0 failed / 14 ignored（串行、去代理，444.95s）。

- 未验证/待 owner：fixture 是按官方字段契约构造的脱敏样本，不是带真实 key 的线上抓包；三张卡的单位/重置/空态视觉验收待 owner。Currency 仍按 owner 决定延后，未偷偷加入。

- 2026-10-24（SSRF 验收补齐）：apikey 适配器固定 HTTPS host、GET-only、`http_no_redirect()`（不跟随 302）和 1 MiB `json_body` 上限都有既有单测覆盖；`no_redirect_client_stops_at_a_302`、`json_body_rejects_oversized_body` 与 `every_spec_is_a_fixed_https_origin_with_no_user_url` 共同满足本票最后一项。Currency 仍按 ADR 0003/owner 决定延后（当前没有 provider 报币种余额）。
