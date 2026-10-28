# Pane — Domain glossary

The canonical vocabulary for this repo. This file is the project's **domain
glossary and nothing else**: it defines terms, it does not carry engineering
facts, constraints, failure modes, or decision logs.

- Verified engineering facts, constraints and failure modes →
  [`docs/project-facts.md`](docs/project-facts.md).
- Hard-to-reverse decisions →
  [`docs/adr/`](docs/adr/).
- Work state (tickets, specs, blockers) →
  [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).

If a concept you need is missing here, that is a signal — either it is not
project language (reconsider) or it is a real gap to settle.

## Domain vocabulary

- **安装探测 (install probe)** — 只读检查 PATH/PATHEXT、常见安装目录和环境变量，用来区分“已安装/未安装/未知”，不执行 CLI、不出网。
- **已安装未登录 (installed but not signed in)** — 本机存在工具安装痕迹但没有可用凭据；和“未安装”是两种不同状态，动作也不同。
- **凭据来源 (credential source)** — `env:<变量名>`、系统凭据库、本机登录或 `none`；只显示来源，不显示 secret 值。
- **设置清单 (setup checklist)** — 首启的四步流程；检测步骤展示本机证据，完成状态由真实 `status == ok` 的来源驱动。
- **皮肤包 (skin pack)** — 官方分发的 zip + manifest 资源包，带 id、版本、入口 CSS 和逐文件校验；安装后落在 `%APPDATA%\PaneAafff623\skins\<id>\`。原生外观不是皮肤包。
- **原生外观 (native appearance)** — `skinId: null` 时的内置主题；恢复原生不是删除皮肤包。
- **应用 / 选中 (applied / selected)** — applied 是当前真正渲染的皮肤；selected 是 config 里保存的 `skinId`。选中的包可能在本机缺失，此时界面显示原生外观，但工坊报告 Selected / Not installed，不能把原生说成已应用。
- **皮肤工坊 (skin workshop)** — 设置 → 通用 → 皮肤工坊；商店式画廊与详情页，替代旧的悬浮抽屉市场。悬浮面板调色板、通用页入口和站点 demo 都指向这里。
- **额度 (quota)** — 供应商给出的、有上限且带重置窗口的许可量；语义是"还剩多少、何时重置"（百分比 + 倒计时）。由「额度总览」承载。
- **用量 (usage)** — 已消耗的绝对量；语义是"消耗了多少"，按天累积。由「用量面板」（即原「总花费」面板）承载。与额度是两回事：额度看许可，用量看消耗。
- **计量单位 (unit)** — 用量的计数单位（token、积分、分钟、字符、次数、容量、货币…）。「用量面板」按计量单位分组：只有可通约的项才进同一张图，单位之间不做换算；同名单位（如"积分"）在不同厂商之间未必可通约。
- **本机登录行 (local sign-in row)** — 授权中心按官方客户端本地凭据文件识别出的只读账号行（`kind: "local"`）。它只做展示与启用入口，不复制、不写入官方登录态；与 Pane 管理的账号行（`account`/`slot`/`oauth`）分开。
- **估算 tokens (estimated tokens)** — Qoder CN 不写 token 计数，其计费输入量按 `Σ(上下文占比 × 上下文窗口)` 测算，并在用量面板标「估算」；credits 仍是精确值，二者不是固定汇率，也不折美元。
- **family** — one provider identity shared by all its cards: `claude`,
  `codex`, `cursor`, `kimi`, `onenewapi`, … the full family set in
  `src-tauri/src/provider_catalog.rs`, mirrored in `src/providerCatalog.ts`
  (keep both in sync).
- **card** — one UI card. Id is either the bare family id (`claude`) or an
  account card `family@<32hex>` — two-lane FNV-1a over the identity token;
  same scheme in `accounts.rs`, `antigravity_accounts.rs`, `cursor_accounts.rs`.
- **Snapshot** — one provider query result:
  `{id, name, plan, status: "ok"|"no_credentials"|"error", metrics, stale,
  warning, dashboard_url}`.
- **Metric** — `kind: "progress"` carries `used_percent` / `resets_at` /
  `period_ms`; `kind: "text"` rows are informational.
- **Trae CN credit packs** — `user_entitlement_pack_list` stacks several
  hard-expiry packs (verified 2026-09-17): `usage.credits_amount` exists
  ONLY on the pack currently being drained (others `{}`), so only that
  one can render as a progress row; `end_time` is epoch seconds.
  `entitlement_base_info.available_endpoint` 0/1 = general
  (TraeCode+TraeWork) vs TraeWork-only scope — same-name packs can
  differ in scope, so pack rows merge by display_desc+scope and Work
  rows get a " (Work)" label suffix. The summed "Credits" row carries
  NO resets_at (mixed expiries; a single date would misread) —
  expiry-as-reset on a merged row stays the convention only for
  single-expiry sums (Doubao earliest-expiry, Codex per-credit rows).
  The 免费 pack is feature flags (no `credits_limit`), not credits.
- **AccountEntry** — generic api-key account (`label`, `apiKey`, optional
  `baseUrl` for relaybalance), stored in `accounts/<family>.json`.
- **AgSlot** — Antigravity captured Google OAuth bundle (the IDE keeps one
  token in Windows Credential Manager; slots capture one account each).
  Identity = `refresh_token`. File: `antigravity-accounts.json`.
- **CursorAccount** — Cursor OAuth account. Identity = `access_token`.
  File: `cursor-accounts.json`.
- **Quota Overview (配额总览)** — pinned dashboard section aggregating every
  provider that has a rolling reset window (generalized from the initial 5-hour
  overview in commit `5d03a82`). Header badge = three disjoint chips
  (`● N 可用` green = off-peak available, `● N 满额` red shown only when
  non-zero, `● N 高峰` yellow = available AND inside the family's peak
  window, always rendered even at 0), plus a `5h` / `Weekly` capsule.
  The sectioned board splits the available set into 可用 / 高峰 / 不可用
  (peak section header always renders, even empty); the availability
  chips count the same predicates, so chips, dots and sections can never
  disagree. Default / 5h tab picks the most binding quota
  (5h session first, otherwise shortest-period percent such as
  daily/weekly/monthly). Weekly tab switches ordinary families to a week
  meter when one exists; Z.ai, One/New API, and Copilot keep the default
  binding.   Click a ring to scroll to that card; hover still shows the
  other window. One ring per family — Antigravity/Cursor extra
  accounts stay as separate cards below, but do not get a second
  overview tile. **maxed (满额)** = the shown window at 100%; providers
  without a time window render as `按量/长期` (non-quota) and don't count into
  the availability tally. Independent pools (Cursor Auto vs API, Antigravity
  Gemini vs Claude) are maxed only when every *present* pool is exhausted.
  Cursor's 5h-tab ring follows `Cursor Models` (Auto), not the API pool.
- **Peak hours (高峰)** — time-of-day billing windows for 4 families,
  rules table + Beijing-time (UTC+8) checker in `src/peakHours.ts`
  (verified against official docs 2026-09-15): zai Mon–Fri
  14:00–18:00; commandcode Mon–Fri 09:00–12:00 & 14:00–18:00 (only
  DeepSeek-routed models; weekends never peak); qodercn & traecn daily
  08:00–22:00 (weekend daytime IS peak — both use a "daily" wording, no
  weekday/weekend split). A yellow status dot (.acct-dot/.overview-dot/
  .peak-dot/.trail-badge variants) appears ONLY when the family would
  otherwise read green; red (maxed/error) and gray never turn yellow.
  The overview tile tooltip appends the per-family multiplier rule
  (i18n `peak.rule.*`). Provider edges: doubao/kimi have NO time-of-day
  billing (rolling windows only); GLM/DeepSeek-official weekends are
  all off-peak while Qoder/Trae keep daytime standard rate.
- **Relay site (One/New API)** — one site entry in `onenewapi.json`
  (version 1): `name`, `base_url`, optional dashboard **access token** +
  `New-Api-User` id, and N relay keys (`sk-…`). Sites are ACCOUNTS of the
  `onenewapi` family (`supports_extra_accounts = true` since 2026-09-05):
  the dashboard renders ONE merged card with a tab per site key; the site
  manager lives in the Customize drawer as the family row's account
  section (moved out of Settings). Card ids: `onenewapi@<key_id>`, or
  `onenewapi@<site_id>` for token-only sites (access token + user id but
  no relay key — display is the subscription endpoint only, billing
  fallback impossible without a key). Subscription numbers come from
  `/api/subscription/self` (access token + `New-Api-User` header) with
  silent fallback to the OpenAI-compatible billing endpoints. The bare
  `onenewapi` card on the dashboard is synthesized at render time
  (borrows the first healthy account's snapshot; never in
  `last_snapshots`), and the family layout entry is seeded once from the
  first configured key's layout.
- **Spend** — `ProviderSpend` (today / yesterday / last30 windows + 30-day
  trend) scanned from local CLI session logs (`spend.rs`), priced via
  LiteLLM / models.dev catalogs (`pricing.rs`, daily refresh, hourly while
  unpriced models exist). Tokens are facts even when no price is known
  (⚠ + `unpriced_models`).
- **Usage history** — `usage_history.json`: per card, daily max used-%,
  35-day retention; synthesizes the 30-day trend for cards without local
  logs (`usage_history.rs`).
- **Tray strip** — up to 4 starred metric entries rendered in the tray;
  main-tray projection in `tray_projection.rs`.
- **Kimi monthly anchor** — Kimi's membership-wide monthly limit is never
  reported by the usage API; it only surfaces as failed inference. The
  wall refreshes on the **27th** (owner member-dashboard screenshot,
  2026-09-16; date precision only, modeled at local midnight) —
  `MONTHLY_RESET_DAY` in `providers/kimi.rs`. Probe states: healthy =
  one tiny probe per refresh (the only wall-discovery channel), walled
  with anchor in the future = zero probes (cached monthly row with
  countdown), anchor passed = 30-min re-probe rhythm until quota
  returns. A pane restart clears the memo and forces a fresh probe.
- **uiFont** — custom UI font family in `config.json` (`""` = stock
  stack). Enumerated by `fonts.rs` via font-kit (DirectWrite; localized
  names like 微软雅黑 appear as separate families), validated on write
  (trim, ≤100 chars), applied by `applyUiFont()` in `src/main.ts`
  prepending the family to the stock stack so missing glyphs fall back.
  The Settings dropdown is a body-level fixed menu (the settings
  accordion clips absolutely-positioned children), each option rendered
  in its own face.
