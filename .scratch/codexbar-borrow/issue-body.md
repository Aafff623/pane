Status: ready
Priority: P1
Area: Architecture

# CodexBar 借鉴：family gatekeeper 最小版 / FetchStrategy 流水线 / capability typed 化 / 109 家端点情报

## 背景

CodexBar（steipete/CodexBar，MIT，v0.81.0+，110 家 provider）是 provider 架构的标杆。第三轮深挖两路：①**Gatekeeper**（4557 行防特判漂移测试）抠到 Pane 最小版设计完成；②**流水线与端点情报**——FetchStrategy 四段协议+错误 8 分类+109 家全清点（Pane 缺 B 组 typed 16 家 + D 组插件 55 家）。证据与完整设计：`temp/research/20261025-codexbar-deep-dive.md`。**MIT 可抄码带署名**；gatekeeper 明确做最小版（完整版实测 7.8 豁免槽/provider，维护超线性）。

## 范围（8 票）

- **01-03 family gatekeeper 最小版**（P1，架构 P0）：parse-tests/src/family_gatekeeper.rs 新 mod；扫描共享代码 family 字面量，强制 `providers::<family>::` 转发或 `// provider-specific: <理由>` 或白名单；family 真源运行时读 provider_definitions()；v1 先开 5 文件+derived 纯转发豁免（不做则需 700+ 槽）；TS 侧与数据表豁免后续递进。
- **04 FetchStrategy 流水线骨架**（大票）：四段协议 trait + 8 分类错误枚举 + retryAfter≤10s 单次重试 + 降级诊断保留；77 家分批迁移。
- **05 capability typed 化**：PaceRule 枚举+MenuBarMetrics struct 替代布尔+match（编译器漏出漏配点）。
- **06 fetch attempts 审计**：per-source 时间线一行日志 + 诊断导出 schema。
- **07 候选 family 扩展**（依赖 04）：109 家清点中 Pane 缺 71 家（typed 16 + 插件 55），按用户量挑 5-10。
- **08 多窗口前缀+离线兜底**（小票）：双窗口数值加 `5h/周` 前缀；全策略失败返回旧快照+降级解释。

## 明确不做

gatekeeper 完整版三件套（指纹/聚类/AllowedConstruct）与非扫描门禁 11 个（维护超线性，上游自述"防诚实错误"即够）；插件式 web provider 运行时（rquickjs/boa/TOML 声明式——大架构决策，备查等需求出现）；macOS 专属手段直接照搬（/proc 探测/Keychain——Windows 换进程枚举+凭据管理器）。

## 验收

见各票；01-03 机械可验（parse-tests 红/绿）；04-06 单测+分批迁移验收；07/08 owner 场景验收。
