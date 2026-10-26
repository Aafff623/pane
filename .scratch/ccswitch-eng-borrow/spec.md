# Spec: cc-switch 工程借鉴批次（自报成本 / i18n parity / 事务保存 / CI 门禁 / 错误与托盘）

**Issue:** https://github.com/Aafff623/pane/issues/62

> 2026-10-25 用户确认「这些都加进去」。证据：temp/research/20261025-ccswitch-deep-dive.md §二（file:line）。参照物 temp/cc-switch（MIT）。五项彼此独立。

## 01 CLI 自报成本优先

日志自带精确成本时入账优先用它，本地价只补分项+漂移告警，partial 视为下界（cc-switch session_usage_grokbuild.rs:33-43，Grok `costUsdTicks` 1 tick=1e-10 USD）。Pane 落点：spend.rs 各源行解析——先 grep 真实日志样本确认哪些源带 cost 字段（Grok updates.jsonl 优先查证），有则接入、无则跳过该源。原则：自报价优先、catalog 补缺、偏离超阈值告警一行。

## 02 i18n 三语 parity 测试

三段断言（localeCoverage.test.ts:10-54 平移）：①flatten 递归压平成 Map<path,string> 做 key 集合对比；②插值变量正则提取排序后 join("\0") 对比（Pane 单括号 `\{([^}]+)\}`）；③品牌词保留：对 en 值跑品牌词表（Codex/Claude/Copilot/GLM…）得 key 集，断言 zh/ru 译文仍含该词。node --test（site.test.mjs 先例），0.5 天。Pane ru 是第一方翻译，三语全量 parity 不分域白名单。

## 03 设置事务性保存

save_settings 模式（commands/settings.rs:64-176）：diff 新旧设置 → 逐字段触发即时副作用（托盘样式切换当场生效）→ 依赖的重写失败则**回滚设置并整次报失败**（「界面显示已改好、客户端还是旧的」=反模式）。Pane 落点：main.ts 设置保存路径 + lib.rs config 写入命令；适用范围=有联动动作的设置项（托盘形态、更新频率、皮肤切换等），逐项盘点。

## 04 CI 路径过滤 + release 文档门禁

① paths-filter 固定 SHA + concurrency.group 带 github.event_name + 缓存精确命中（ci.yml:8-16/66-67）——Pane 现有 codeql/dependabot 之外建 ci.yml 时套用；② **release 文档门禁**：release 前检查 CHANGELOG/what's-new 摘要存在性，缺失直接挂 release（cc-switch release.yml:24-74 有同款 job）——对应 Pane release-format 的「CHANGELOG 对账」做成可失败检查。

## 05 错误结构化 + 托盘指纹重建

① 错误两件套：小 error.rs（thiserror：Io 带 path+source、Localized 双语）+ 需结构化的场景用 JSON 错误串（code/context/suggestion）；Pane 落点=lib.rs 各命令 map_err 聚合点，spend/探测失败给前端错误卡片用结构化串。② 托盘重建按问题态指纹：rebuild_tray_menu（lib.rs:600）重建前比对「问题区状态」指纹（mark_feedback_seen 式记账），问题没变不重建（cc-switch lib.rs:1232-1244）。

## 不抄

TanStack Query（四条理由，只抄 queryKey 树语义=main.ts:15764-15818 监听群加 namespace→refresher 映射，属既有 P0-3）、live/ 配置所有权层、SQLite 全家、React settings hook 结构（取「草稿隔离」行为语义）、WSL2 合同测试、apps 拖拽排序。

## 验收

见各票；02/04 可机械验证（测试绿/CI 挂），01/03/05 需 owner 场景验收。
