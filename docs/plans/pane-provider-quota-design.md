# Pane Provider Quota Architecture Design

## 目标

Pane 的当前核心目标是可靠地监控多个 AI provider 的用量和额度，尤其是通过官方 API Key 查询余额或套餐额度。当前定稿的 API Key 查询条目包括 5 个必做专属能力、1 个可选的 ZenMux 专属能力，以及 1 个通用 billing 能力。必做专属能力是 Kimi For Coding、StepFun、SiliconFlow、OpenCode Go、Novita；通用 Custom Balance 覆盖 9 家中转站，因此主覆盖为 14 家厂商，ZenMux 纳入后再额外扩展。DeepSeek、OpenRouter、Z.ai、MiniMax、Moonshot 和 AihubMix 已有查询实现，不在这批新增 provider 里。

“能通过 API Key 查询”和“同一 family 支持多个 API Key 账号”分开处理：前者覆盖上述全部查询能力；后者当前对 DeepSeek、Kimi Code、StepFun、SiliconFlow、Novita、Custom Balance 六个 family 开放。一个支持多账号的 provider 家族可以有多个 API Key；每个 Key 都是一个独立的 provider 实例，应当在卡片、缓存、余额基线、自动刷新和托盘展示中保持独立。

本设计吸收 ccSwitch 的核心思想，但不复制 ccSwitch 的完整产品范围。OAuth 管理中心、通用 JS 用量脚本编辑器、完整 provider CRUD 和 SQLite 迁移不是本阶段的必需交付，只保留它们未来可接入的边界。

## 核心概念

### Provider family

描述供应商家族和查询规则，例如 `deepseek`、`stepfun`、`siliconflow`。它包含显示名、查询适配器、图标和能力信息，但不包含某个用户的具体 Key。

### Provider instance

描述一个可独立监控的实体，例如：

```text
deepseek                 主配置中的 DeepSeek Key
deepseek@<fingerprint>   额外账号文件中的一个 Key
onenewapi@provider-a     One/New API 的一个独立 Key 卡片
```

实例拥有自己的凭据来源、展示名称、查询结果、缓存和余额基线。额外 API Key 使用由 Key（Custom Balance 还包括 Base URL）派生的稳定本地 fingerprint ID；标签和账号文件顺序变化不会改变实例身份。fingerprint 只用于本地缓存、布局和禁用状态，不进入遥测。

### Capability catalog

Provider family 的设置能力也由同一份目录声明：查询类型、是否接受普通 API Key、是否允许额外 API Key 账号和图标 key。前端设置入口与 Rust 命令白名单都从这份能力声明派生；真正的网络请求仍由各 provider 模块显式分发，目录不承担动态执行。

### Query adapter

已知官方接口继续使用 Rust 原生 provider 模块；各模块负责请求和解析，统一返回 pane 现有的 `Snapshot`/`Metric`。不要为了模仿 ccSwitch，提前把所有查询改成脚本或引入新依赖。

当前 API Key 查询适配器边界：Kimi 有 Pane 中保存的或环境变量提供的 Coding API Key 时优先使用 API Key，没有 API Key 时才使用 CLI OAuth；StepFun、SiliconFlow、Novita 使用官方余额接口；OpenCode Go 复用已有 `opencode.rs` 的官方 usage API 和本地数据库回落；Custom Balance 使用用户提供的 Base URL 查询 OpenAI 兼容 billing 接口。ZenMux 仍是可选扩展，不作为当前主链路的硬依赖。

动态调整记录：ccSwitch 的 `query_zenmux` 已确认属于其 coding-plan 服务中的“用户提供端点 + Bearer Key”查询路径，并不是 Pane 当前 provider family 的现成原生 Snapshot 适配器。若纳入 Pane，需要新增 `zenmux` family、原生 Snapshot 映射、设置字段、图标/fallback、错误矩阵和真实端到端凭据验证；这会扩大本轮已经定稿的 14 家主覆盖范围。因此本轮只保留接口边界和借鉴结论，不把 ZenMux 计入已完成能力，后续单独立项。

### Live projection

Pane 的监控配置、账号文件和运行时快照是内部状态；页面、托盘和 HTTP API 是这些状态的展示投影。所有投影应读取同一批刷新结果，不能分别维护一套查询逻辑。

## 必须保持的约束

1. 任何页面、日志、遥测和快照都不能输出完整 API Key。
2. 多账号查询必须把实例 ID传入余额基线，不能用 family ID 覆盖所有账号。
3. 网络超时、连接失败和 5xx/429 属于瞬时失败，应保留上一次成功快照并标注 stale/warning。
4. 明确的无凭据、401/403、不可解析响应和未知 provider 属于确定性失败，不应继续显示过期的成功额度。
5. Kimi 的 OAuth 与 API Key 来源必须继续共存，但状态展示要标明当前生效来源；One/New API 动态卡片与六个 API Key 多账号 provider 必须继续共存。
6. telemetry 可以按 family 聚合，但运行时卡片、缓存、基线、禁用列表和布局必须按实例 ID 处理。
7. 不改变用户已有的客户端配置格式，不把 pane 的内部元数据写进 provider 的外部配置。
8. Windows 使用 `cargo +stable-x86_64-pc-windows-gnu`；`src-tauri/Cargo.toml` 的 `crate-type = ["rlib"]` 只是本地调试值，不得提交。
9. Custom Balance 的 Base URL 必须使用 HTTPS；仅私有、回环或链路本地 IP 允许使用明文 HTTP，且不得携带 userinfo、query 或 fragment。

## ccSwitch 借鉴边界

直接借鉴：

- SSOT 与展示/客户端投影分离；
- provider family、provider instance、credential binding 分层；
- 原生 quota adapter 与通用 fallback 分层；
- 元数据驱动的能力判断和图标展示；
- 账号级缓存、刷新锁和错误分类。

暂不直接复制：

- ccSwitch 的完整 SQLite schema；
- UniversalProvider 的跨客户端配置生成器；
- OAuth 多账号管理中心；
- rquickjs usage-script 沙箱和可视化脚本编辑器；
- 图标搜索和图标选择器。

## OAuth 扩展边界

当前 Pane 的 OAuth 是独立于 API Key accounts 的身份来源：`codex` 和 `grok` 使用 Pane 自己的 device-code 登录，令牌保存在 `%APPDATA%\Pane\oauth\<provider>.json`；Claude/Codex 的额外 CLI 配置则来自各自的本地配置目录。它们不能被混成同一个 `accounts/<family>.json` 数据结构。

未来如果需要 OAuth 多账号，应先固定三层接口：

```text
AuthProvider: codex_oauth | grok_oauth | future provider
AccountId:    稳定的受管账号身份，不等同于 access token
AuthBinding:  provider instance → AuthProvider + AccountId
```

OAuth 令牌刷新、撤销和 quota 查询应共享账号身份，但令牌仍只留在本地安全存储中；页面、日志、遥测和 Snapshot 只使用脱敏后的账号标签。当前阶段不让这套未来接口成为 API Key 额度交付的硬依赖。

## 完成标准

- 六个 API Key 多账号 provider 的每个账号都能独立查询并显示；
- 定稿范围内的 API Key 查询能力保持可用：5 个必做专属能力加 1 个通用 billing 能力覆盖 14 家厂商；ZenMux 作为可选扩展单独评估；
- 同一 family 的两个账号不会共享余额基线或缓存结果；
- Kimi 双回落、One/New API 动态卡片、页面刷新、隐藏窗口刷新和托盘刷新都不回归；
- 失败分类和 stale/warning 行为有自动化证据；
- 前端构建、parse-tests、GNU toolchain 检查通过；
- 每个代码阶段都能单独说明修改文件、验证命令和未覆盖边界。
