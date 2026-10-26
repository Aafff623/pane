# Spec: 收尾零碎批

**Status:** partial (2026-10-26) — shipped: 1, 2, 11, 12, 13, 14, 15, 16. Deferred with their parent tickets: 3/4/5/6/7/8/9/10/17/18; 19–20 remain long-term.
**Issue:** https://github.com/Aafff623/pane/issues/104
**Evidence:** temp/research/20261026-closing-audits.md §二（覆盖率对账漏网 16 条）+§三（quota-pinkpixel/tokscale 小料）

## 定位

覆盖率对账+漏网仓调研筛出的零碎项合并批。主对话按需取，不要求一次做完。

## 子项清单（每条独立可取）

| # | 子项 | 来源 | 落点 | 量级 |
|---|---|---|---|---|
| 1 | 托盘溢出行：「还有 N 个供应商…」+Win32 助记符 `&`→`&&` 转义 | quota-pinkpixel tray.rs:148-157,189-204（Pane 8 行静默截断 lib.rs:563-596） | lib.rs cached_tray_provider_rows | 0.5h |
| 2 | env 覆盖值去空白/去引号（`$CODEX_HOME` 带引号场景） | quota-pinkpixel local_paths.rs:12-28 | spend.rs 各 root 解析（:1940/2671/3577） | 0.5h |
| 3 | 排名聚合口径随迁：account/key/host/purpose/session 维度+Totals 含 errors/retries | magpie usage.go:725-763（#61-07 被 #64 取代时未随迁） | #64 统计页增量 | 0.5-1 天 |
| 4 | resetDay 账单周期（1-28 clamp 推 periodStart/End 非自然月） | codeburn computePeriodFromResetDay | #72 月度外推扩展 | 0.5 天 |
| 5 | fx 汇率 CNY 显示（缓存+离线兜底） | magpie internal/fx | 前端显示层 | 0.5 天 |
| 6 | 请求归档+spill 大日志防内存尖峰 | magpie request_spill.go | 扫描器改造时搭车 | 条件性 |
| 7 | 模型别名环境变量+日期后缀剥离 | ccusage model_aliases.rs | #56 后续 | 0.5 天 |
| 8 | provider 能力声明表（~70 家调度/新鲜度声明） | agentsview capabilities.go | #65-05 扩展 | 1 天 |
| 9 | skills/hooks/MCP 清单卡片（延伸 agent_skill.rs） | ZCode P1 | auth center 或设置 | 1 天 |
| 10 | 前端视觉 QA harness+PR 录屏（多语言×宽窄×假 Tauri API） | magpie ui-preview | 新工具链 | 2-3 天 |
| 11 | Windows 更新安装 REBOOT=ReallySuppress+先摘托盘 | cc-switch commands/settings.rs:277 | 更新流 | 0.5 天 |
| 12 | 200ms 防抖事件合并 emit | cc-switch usage_events.rs | 快照广播 | 0.5h |
| 13 | 语言双源校正（localStorage 非真相，启动用后端校正） | cc-switch i18n/index.ts:101-122 | i18n 启动链 | 0.5h |
| 14 | config 迁移版本号字段 | QuickClipboard storage.rs | config.json | 0.5h |
| 15 | rc 预发布不给正式频道推送的归队逻辑 | QuickClipboard updater creator.rs:326-330 | updater | 0.5h |
| 16 | 遥测分桶隐私声明文档（「永不外发」明文清单） | codeburn telemetry-snapshot.ts | privacy.md 增量 | 0.5h |
| 17 | 跨视图一致事件（带来源窗口） | cockpit accountSyncEvents | 面板/主窗同步 | 0.5 天 |
| 18 | OTEL 做差还原 resume 区间（copilot） | ccusage copilot loader.rs:164-268 | #61-08 已 deferred，研究项 | 条件性 |
| 19 | CJK FTS/friction（会话检索/质量信号） | agentsview | 远期条件性 | 远期 |
| 20 | 边缘停靠条（EdgeDock 形态） | QuotaBar | 待 #77 桌面卡片后评估 | 远期 |

## 验收

无统一验收——按子项各自验收（每条的落点与量级已列）；主对话排期时从 1/2/11/12/13/14/15/16 这些半小时级先取。

## 协调

子项 3/4/7/8/9/18 是既有票的增量（标注了归属票），实施时并入对应票不单独立流程；其余为新小件。
