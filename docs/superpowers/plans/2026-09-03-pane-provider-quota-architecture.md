# Pane Provider Quota Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不复制 ccSwitch 全部产品的前提下，把 pane 的多 provider 额度监控整理成“provider family → provider instance → query adapter → 统一快照”的可持续结构，并保持已定稿的 API Key 查询范围：5 个必做专属能力、1 个可选 ZenMux 专属能力，以及 1 个通用 billing 能力。必做项覆盖 14 家厂商，ZenMux 纳入后再额外扩展。这里的“API Key 可查询”与“同一 family 支持多账号”是两个独立能力。

**Architecture:** 保留 pane 现有 Rust 原生 provider 查询模块和 `Snapshot`/`Metric` 输出，新增轻量的 provider 能力注册和实例边界。每个 API Key 账号实例拥有独立的查询、缓存、余额基线、卡片和托盘身份；family 只用于能力定义与遥测聚合。页面和托盘继续消费同一轮后端刷新结果。已有 API Key 查询包含 Kimi For Coding、StepFun、SiliconFlow、OpenCode Go（复用现有 `opencode.rs`）、Novita 和 Custom Balance；后者覆盖 9 家 OpenAI 兼容 billing 中转站。多账号实例化当前覆盖 DeepSeek、Kimi Code、StepFun、SiliconFlow、Novita 和 Custom Balance 六个 family；Kimi 额外卡片不合并全局 Moonshot wallet。

**Tech Stack:** Rust 2021、Tauri 2、Tokio、Reqwest、Serde/Serde JSON、TypeScript、Vite、现有 parse-tests harness。

**Spec:** `docs/plans/pane-provider-quota-design.md`

## Global Constraints

- 工作目录是 `D:\code\pane`，当前分支是 `feat-merge-upstream`；保护已有未提交改动，不执行 reset、restore、批量 stage、commit、push 或清理。
- `src-tauri/Cargo.toml` 的 `crate-type = ["rlib"]` 是本地调试值，绝不提交。
- Rust 验证使用 `cargo +stable-x86_64-pc-windows-gnu ...`；完整 DLL 测试受本机 GNU linker 限制时使用 `src-tauri/target/parse-tests` harness。
- `docs/plans/HANDOFF-codex.md` 和本计划属于工作文档，不放入面向上游的 PR。
- 不新增依赖；除非某个阶段的现有实现无法完成明确验收，否则不迁移 SQLite、不引入脚本沙箱、不重写现有 provider 模块。
- 不重复创建 `opencodego` provider：OpenCode Go 已由现有 `opencode` family 的官方 usage API 提供，改动只应围绕其凭据来源和已有 fallback。
- 任何测试数据、日志、错误消息和遥测都不得包含完整 API Key、OAuth token 或用户配置文件原文。
- 每个阶段先写或补测试，再改实现；每个阶段完成后运行该阶段指定验证并记录结果。

---

### Task 0: 建立现场基线并锁定交付边界

**Files:**
- Read: `docs/plans/HANDOFF-codex.md`
- Read: `docs/plans/ui-overhaul-three-phases.md`
- Read: `src-tauri/src/lib.rs`
- Read: `src-tauri/src/accounts.rs`
- Read: `src-tauri/src/providers/mod.rs`
- Read: `src/main.ts`
- No source edits

**Interfaces:**
- Consumes: 当前分支、工作树、handoff、已有 provider 和刷新实现。
- Produces: 一份当前状态记录，以及后续任务必须遵守的文件归属和验证基线。

- [x] **Step 1: 确认工作树和分支状态**

运行：

```powershell
git status --short --branch
git diff --stat
git diff -- src-tauri/Cargo.toml src-tauri/src/lib.rs src/main.ts
```

预期：当前分支为 `feat-merge-upstream`；只看到 handoff 中列出的 `Cargo.toml`、`lib.rs`、`main.ts` 修改和未跟踪工作文档，不触碰这些现有改动。

- [x] **Step 2: 检查冲突标记和 handoff 待核对点**

运行：

```powershell
rg -n "^(<<<<<<<|=======|>>>>>>> )" src src-tauri
rg -n "run_usage_fetch|spawn_auto_refresh|apply_main_tray_projection|refreshMinutes|usage-updated" src-tauri/src/lib.rs src/main.ts
```

预期：没有 Git 冲突标记；能够定位 handoff 标注的三个托盘刷新复核点。

- [x] **Step 3: 建立现有验证基线**

运行：

```powershell
npm run build
Push-Location src-tauri/target/parse-tests
cargo +stable-x86_64-pc-windows-gnu test
Pop-Location
Push-Location src-tauri
cargo +stable-x86_64-pc-windows-gnu check
Pop-Location
```

预期：前端构建成功；parse-tests 全部通过；GNU toolchain check 成功。若出现与当前工作树无关的既有 warning，只记录，不顺手重构。

**Checkpoint:** 若基线失败，先判断是否由当前未提交改动引入；未完成归因前不进入后续实现任务。

---

### Task 1: 建立 provider family 能力注册表

**Files:**
- Create: `src-tauri/src/provider_catalog.rs`
- Create: `src/providerCatalog.ts`
- Modify: `src-tauri/src/lib.rs`（注册模块）
- Modify: `src/main.ts`（使用 provider catalog 的 family/icon/account 能力）
- Test: `src-tauri/src/provider_catalog.rs` 内置单元测试

**Interfaces:**
- Consumes: 当前 `run_usage_fetch` 的 provider 列表、`accounts::ACCOUNT_PROVIDERS`、`PROVIDER_ICONS`、`MULTI_ACCOUNT_PROVIDERS`。
- Produces:

Rust：

```rust
pub enum QueryKind {
    NativeSnapshot,
    NativeBalance,
    NativeCodingPlan,
    Composite,
    LocalOnly,
}

pub struct ProviderDefinition {
    pub family_id: &'static str,
    pub display_name: &'static str,
    pub query_kind: QueryKind,
    pub supports_api_key: bool,
    pub supports_extra_accounts: bool,
    pub icon_key: &'static str,
}

pub fn provider_definitions() -> &'static [ProviderDefinition];
pub fn provider_definition(family_id: &str) -> Option<&'static ProviderDefinition>;
pub fn supports_api_key(family_id: &str) -> bool;
pub fn supports_extra_accounts(family_id: &str) -> bool;
```

TypeScript：

```ts
export interface ProviderDefinition {
  familyId: string;
  displayName: string;
  queryKind: "nativeSnapshot" | "nativeBalance" | "nativeCodingPlan" | "composite" | "localOnly";
  supportsApiKey: boolean;
  supportsExtraAccounts: boolean;
  iconKey: string;
}

export const providerCatalog: readonly ProviderDefinition[];
export function providerFamily(id: string): string;
export function providerDefinition(familyId: string): ProviderDefinition | undefined;
export function supportsApiKey(familyId: string): boolean;
export function supportsExtraAccounts(familyId: string): boolean;
```

- [x] **Step 1: 写 registry 失败测试**

在 `src-tauri/src/provider_catalog.rs` 的测试模块中覆盖：六个多账号 family 都被标记为 `supports_extra_accounts = true`；`claude` 不被标记为 API Key 多账号；`provider_definition("deepseek")` 返回 `NativeBalance`；未知 family 返回 `None`。

- [x] **Step 2: 运行 Rust 单元测试确认失败或缺少实现**

运行：

```powershell
Push-Location src-tauri
cargo +stable-x86_64-pc-windows-gnu test provider_catalog
Pop-Location
```

预期：在实现前因模块或函数不存在失败；若测试 harness 不支持该模块，则将相同断言放入 parse-tests 的对应测试入口。

- [x] **Step 3: 实现最小 Rust catalog**

只登记当前 pane 已支持的 family；不要把 ccSwitch 的全部 provider 名称复制进来。将 `accounts::ACCOUNT_PROVIDERS` 改为由 catalog 的能力表派生，或在无法安全删除旧常量时增加一致性断言。

- [x] **Step 4: 实现 TypeScript catalog 并替换重复判断**

让 `main.ts` 的 `PROVIDER_ICONS`、`MULTI_ACCOUNT_PROVIDERS` 和 `providerFamily` 通过 `providerCatalog.ts` 读取；保留现有 `providerFamily(id)` 的 `@` 兼容行为。不要改变卡片 HTML 和图标 SVG 内容。

`supports_api_key` / `supportsApiKey` 也由同一目录维护；Rust 的 `set_api_key` 和前端
Customize API Key 区域都从该能力位派生，避免设置入口和后端白名单再次漂移。

- [x] **Step 5: 运行验证**

运行：

```powershell
npm run build
Push-Location src-tauri/target/parse-tests
cargo +stable-x86_64-pc-windows-gnu test
Pop-Location
```

预期：行为不变，catalog 测试和既有 parse-tests 全部通过。

**Checkpoint:** 如果 registry 需要改动大量 provider 查询逻辑才能接入，停止扩大范围，保留“元数据注册表先行、调度暂不重写”的方案。

---

### Task 2: 让 API Key 账号实例拥有自己的快照身份

**Files:**
- Modify: `src-tauri/src/providers/deepseek.rs`
- Modify: `src-tauri/src/providers/stepfun.rs`
- Modify: `src-tauri/src/providers/siliconflow.rs`
- Modify: `src-tauri/src/providers/novita.rs`
- Modify: `src-tauri/src/providers/relaybalance.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Test: 上述 provider 模块现有测试或 parse-tests harness

**Interfaces:**
- Consumes: 现有 `snapshot_with_key` 和 `account_snapshot`。
- Produces:

```rust
pub async fn snapshot_with_key_as(
    key: &str,
    card_id: &str,
    card_name: &str,
) -> Snapshot;

pub async fn snapshot_with_key_at(
    key: &str,
    base_url: &str,
    card_id: &str,
    card_name: &str,
) -> Snapshot; // relaybalance
```

旧的 `snapshot_with_key(key)` 和 `snapshot_with_key(key, base_url)` 保留为 family 默认卡片的兼容 wrapper；relaybalance 的旧 wrapper 调用 `snapshot_with_key_at`。

- [x] **Step 1: 写账号实例身份测试**

为每个纯 Key provider 增加不依赖真实网络的断言：默认 wrapper 仍返回原 family ID；命名 wrapper 返回传入的 card ID 和 card name；余额 meter 使用传入的 card ID 作为 baseline key。对 `relaybalance` 同时断言 base URL 参数没有丢失。

- [x] **Step 2: 运行现有 parse-tests 作为失败基线**

运行：

```powershell
Push-Location src-tauri/target/parse-tests
cargo +stable-x86_64-pc-windows-gnu test
Pop-Location
```

预期：新断言在实现前失败，旧测试继续提供回归基线。

- [x] **Step 3: 把各 provider 的硬编码 ID/名称提取成参数**

为每个 provider 添加 `snapshot_with_key_as`；relaybalance 添加 `snapshot_with_key_at`。将成功快照、无凭据快照、错误快照和 `credit_meter` 的 key 改为使用传入的 `card_id/card_name`。默认 `snapshot_with_key` 调用对应命名版本并传入原 family 常量。

- [x] **Step 4: 修改 account_snapshot 使用命名 wrapper**

在 `lib.rs` 的 `account_snapshot` 中直接传入稳定的 `deepseek@<fingerprint>` 等实例 ID和展示名；删除只负责重新覆盖 `snap.id/snap.name` 的补丁逻辑，但保留对未知 family 和 relaybalance 缺少 base URL 的错误处理。

- [x] **Step 5: 验证实例隔离**

运行：

```powershell
Push-Location src-tauri/target/parse-tests
cargo +stable-x86_64-pc-windows-gnu test
Pop-Location
Push-Location src-tauri
cargo +stable-x86_64-pc-windows-gnu check --tests
Pop-Location
```

预期：账号卡片返回各自 ID；余额基线不会再使用 family ID；没有完整 Key 出现在测试输出中。

**Checkpoint:** 复核证明 `family@n` 位置 ID 会在删除/重排后把旧缓存、布局或禁用状态挂到另一个 Key；因此本阶段动态调整为不改账号文件格式、不引入新依赖，使用 Key（Custom Balance 还包括 Base URL）派生稳定本地 fingerprint ID，并过滤已删除账号的启动缓存。fingerprint 不进入遥测。

---

### Task 3: 统一查询路由和凭据边界

**Files:**
- Create: `src-tauri/src/query_kind.rs`（仅在 Task 1 的枚举无法复用时创建）
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/accounts.rs`
- Modify: `src-tauri/src/providers/kimi.rs`
- Modify: `src-tauri/src/providers/onenewapi/mod.rs`
- Modify: `src/main.ts`
- Test: `src-tauri/src/lib.rs` 现有刷新测试、`src-tauri/src/providers/kimi.rs` 现有测试、parse-tests

**Interfaces:**
- Consumes: Task 1 的 family catalog、Task 2 的实例查询 wrapper、现有 Kimi 和 One/New API 分支。
- Produces:

```rust
pub fn family_of(id: &str) -> String;
pub fn query_kind_for_instance(id: &str) -> Option<QueryKind>;
```

这些函数只负责判断和路由，不读取或返回完整凭据。

- [x] **Step 1: 写路由矩阵测试**

覆盖以下事实：

```text
deepseek       → NativeBalance
deepseek@<fingerprint> → NativeBalance
relaybalance@<fingerprint> → NativeBalance + instance base URL
kimi           → Composite
onenewapi@x    → onenewapi dynamic-card path
unknown@1      → no query route
```

- [x] **Step 2: 检查前后端 family 规则一致性**

运行：

```powershell
rg -n "providerFamily|family_of|ACCOUNT_PROVIDERS|MULTI_ACCOUNT_PROVIDERS|kimi|onenewapi" src/main.ts src-tauri/src/lib.rs src-tauri/src/accounts.rs src-tauri/src/providers/kimi.rs src-tauri/src/providers/onenewapi
```

预期：所有实例归属只在明确的 `family_of` 处解析；Kimi fallback 和 One/New API 动态卡片不被统一 family 逻辑误删。

- [x] **Step 3: 把账号枚举接到 catalog 能力上**

`run_usage_fetch` 仍可保留现有显式 provider future 列表，但额外账号的遍历必须通过 `supports_extra_accounts(family)` 判断。不要通过“是否存在 @”猜测一个 ID是不是账号卡。

- [x] **Step 4: 固定凭据读取顺序**

主卡继续使用各 provider 的配置文件/环境变量读取；额外账号只使用 `accounts/<family>.json` 的对应条目；relaybalance 额外账号必须使用自身 base URL。任何 query adapter 不得读取另一个账号的 Key。

- [x] **Step 5: 验证 Kimi 和 One/New API 共存**

运行：

```powershell
Push-Location src-tauri/target/parse-tests
cargo +stable-x86_64-pc-windows-gnu test
Pop-Location
npm run build
```

预期：有 Kimi Code 凭据时 Moonshot fallback 仍按现有规则处理；没有 Kimi 成功卡片时仍能使用 key 查询；One/New API 的动态卡片仍按自己的 generation 过滤。

---

### Task 4: 复核并收紧刷新、缓存和托盘一致性

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/main.ts`
- Modify: `src-tauri/src/providers/mod.rs`
- Test: `src-tauri/src/lib.rs` 刷新/失败/缓存相关测试；parse-tests；手动隐藏窗口验收

**Interfaces:**
- Consumes: Task 2 的实例级 Snapshot、现有 `run_usage_fetch`、`spawn_auto_refresh`、`usage-updated` 事件和 `last_ok`/fail-state 逻辑。
- Produces: 同一轮刷新结果驱动页面、托盘、HTTP API；实例级状态隔离，family 级遥测聚合。

- [x] **Step 1: 先补刷新不变量测试**

测试必须覆盖：

```text
一轮刷新只允许一个 run_usage_fetch；
disabled provider 不会启动网络 future；
瞬时失败保留旧 Snapshot 并设置 stale/warning；
确定性失败不会伪装成旧的成功额度；
隐藏窗口时后端仍能产生 usage-updated；
刷新间隔从配置重新读取。
```

- [x] **Step 2: 逐段复核 run_usage_fetch**

按 handoff 顺序检查 base provider、Claude/Codex 额外账号、One/New API 动态卡片、六个 API Key 账号、禁用过滤、last-good 缓存、telemetry、alerts 和 HTTP API publish。任何新增实例必须在这一轮完整走完。

- [x] **Step 3: 保持后台与前台事件口径一致**

后台调用 `run_usage_fetch` 后，用同一批 snapshots 更新主托盘图标并发出 `usage-updated`；前端 `visibilitychange` 只负责恢复时主动刷新，不重新实现 provider 查询。

- [x] **Step 4: 保持实例级键，family 级聚合只放在遥测边界**

检查 `last_ok`、fail-state、layout/disabled、tray selection 和 HTTP API payload：运行时使用完整实例 ID；telemetry 使用 `family_of(id)`，不上传账号位置或 Key 指纹。

- [x] **Step 5: 执行验证**

运行：

```powershell
npm run build
Push-Location src-tauri/target/parse-tests
cargo +stable-x86_64-pc-windows-gnu test
Pop-Location
Push-Location src-tauri
cargo +stable-x86_64-pc-windows-gnu check --tests
Pop-Location
```

手动验收：启动开发实例，记录一个主卡和两个额外账号卡的 `fetchedAt`，隐藏窗口至少观察两个刷新周期，再恢复窗口；页面卡片和托盘必须继续更新，且三个卡片的余额/错误状态不能互相覆盖。

---

### Task 5: 整理前端展示元数据和本地图标 registry

**Files:**
- Create: `src/providerVisuals.ts`
- Modify: `src/main.ts`
- Modify: `src/styles.css`（仅在 registry 替换造成样式回归时）
- Test: `npm run build`；手动卡片、trail、托盘视觉检查

**Interfaces:**
- Consumes: Task 1 的 `ProviderDefinition`、现有 `src/assets/providers/*.svg` 和 `PROVIDER_ICONS`。
- Produces:

```ts
export interface ProviderVisual {
  iconKey: string;
  iconSvg: string;
  iconColor?: string;
  invertOnDarkTray?: boolean;
  recolorOnTray?: boolean;
}

export function providerVisual(id: string): ProviderVisual | undefined;
```

- [x] **Step 1: 写图标完整性检查**

在构建前通过静态检查确认当前展示 provider 都能解析到既有 SVG 或明确 fallback；检查 `minimax`、`novita`、`copilot` 的现有反色/重染色规则不丢失。

- [x] **Step 2: 把图标内容从主渲染流程移到 registry**

只迁移映射，不更换 SVG，不引入远程图标 API，不改变 fallback initials 和 family 继承行为。额外 DeepSeek 实例应继续继承 `deepseek` 图标。

- [x] **Step 3: 验证视觉投影**

运行 `npm run build`，再手动检查卡片、侧边 trail、暗色托盘和分享卡片。若只有颜色或尺寸变化，优先修正 registry 数据，不改业务逻辑。

**Checkpoint:** 图标 registry 是展示整理，不得阻塞额度查询主链路；视觉回归超出当前改动范围时保留现有 `main.ts` 映射并延期。

---

### Task 6: 为 OAuth 保留 ccSwitch 式扩展边界

**Files:**
- Read: `src-tauri/src/oauth.rs`
- Read: `src-tauri/src/providers/codex.rs`
- Read: `src-tauri/src/providers/grok.rs`
- Read: `src/main.ts`
- Modify only if a concrete OAuth quota bug is found

**Interfaces:**
- Consumes: pane 当前 Codex/Grok device-code 登录和现有单账号存储。
- Produces: 一份不改变当前行为的扩展设计，未来可使用 `auth_provider + account_id + credential binding`，但不在本阶段强行实现完整 OAuth 多账号中心。

- [x] **Step 1: 记录当前 OAuth 的真实边界**

确认 pane 自有 Codex/Grok 登录、Claude/Codex CLI 额外账号发现、API Key accounts 三者不是同一套身份来源。

- [x] **Step 2: 定义未来兼容接口**

设计文档中固定以下概念：

```text
AuthProvider: codex_oauth | grok_oauth | future provider
AccountId: stable managed account identity
AuthBinding: provider instance → auth provider + account id
```

- [x] **Step 3: 不把 OAuth 作为当前额度交付的硬依赖**

现有 OAuth 额度查询和 Key fallback 继续按 pane 当前行为运行；只有发现 OAuth 卡片无法正确查询额度时，才单独创建 OAuth quota 修复任务。

---

### Task 7: 建立端到端验收矩阵和 PR 边界

**Files:**
- Create: `docs/plans/pane-provider-quota-verification.md`
- Read: `docs/plans/HANDOFF-codex.md`
- Read: `docs/providers.md`
- No source edits unless verification finds a concrete regression

**Interfaces:**
- Consumes: Task 0–6 的实现和验证结果。
- Produces: 可复用验收记录、上游 PR 拆分边界和未覆盖风险清单。

- [x] **Step 1: 建立 provider 维度矩阵**

记录以下组合：

```text
主 Key：DeepSeek / StepFun / SiliconFlow / Novita / Custom Balance
API Key 查询覆盖：Kimi For Coding / StepFun / SiliconFlow / OpenCode Go / Novita / Custom Balance（覆盖 9 家中转站）；ZenMux 作为可选项单独评估
额外 Key：每个支持多账号的 family 至少两个账号
复合路径：Kimi OAuth、Kimi Key fallback、One/New API 多卡
状态：成功、无 Key、401/403、429/5xx、超时、空响应、重启恢复
投影：主页、隐藏窗口、托盘、HTTP API、遥测
```

- [x] **Step 2: 运行最终静态验证**

运行：

```powershell
npm run build
Push-Location src-tauri/target/parse-tests
cargo +stable-x86_64-pc-windows-gnu test
Pop-Location
Push-Location src-tauri
cargo +stable-x86_64-pc-windows-gnu check
cargo +stable-x86_64-pc-windows-gnu check --tests
Pop-Location
git diff --check
```

- [ ] **Step 3: 做完整运行时验收**

记录隐藏窗口连续两个刷新周期的页面时间、托盘状态和两个账号卡的独立结果；检查失败恢复、设置刷新间隔变更和手动 Refresh 不会产生并发重复请求。

已补充的部分运行时证据：为默认 MSVC toolchain 安装 GNU target 后，使用 GNU toolchain 与 `D:\Tools\mingw64\bin` 启动 Tauri 成功；`pane.exe` 有响应窗口，本地 API `GET http://127.0.0.1:6736/v1/usage` 返回 200，Vite 页面 DOM 能加载 Settings、API Keys 和 One/New API 区块。真实 `src-tauri` lib test 二进制即使使用相同 GNU target 仍在启动阶段返回 `STATUS_ENTRYPOINT_NOT_FOUND`，所以行为断言继续使用 parse-tests harness。当前 CUA 能控制浏览器，但原生 `@oai/sky` 的 `list_apps()` / `initialize()` 返回 `Trusted RPC service is not configured: sky`，所以没有原生窗口/托盘控制能力；普通 Chrome 页面也没有 Tauri `invoke/listen` runtime。因此原生按钮、托盘、多账号卡片交互和隐藏窗口周期仍未完成验收，Step 3 保持未勾选。默认 MSVC 启动仍会因缺少 `link.exe` 失败。

- [x] **Step 4: 拆分后续 PR**

建议顺序：

1. 托盘后台刷新修复，单独复核，不包含 `Cargo.toml` 本地调试改动；
2. 六个 API Key provider 的实例级查询和余额基线隔离；
3. provider capability registry 和 Kimi/One-New API 路由保持；
4. 图标 registry 和界面整理；
5. OAuth 多账号管理，另立独立计划。

每个 PR 从 `upstream/main` 建立独立分支，只携带一个明确主题。当前不执行 commit、push 或正式提 PR。

**Final checkpoint:** 只有当 Task 7 的静态、运行时和实例隔离证据齐全，才能声称本阶段完成；否则保留计划未完成状态并根据实际失败调整下一阶段。
