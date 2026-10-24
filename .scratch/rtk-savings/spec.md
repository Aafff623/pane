# RTK 命令输出节省（v0.10）

**Status:** implemented (2026-10-24) — merged into Total Spend; owner acceptance pending
**Issue:** #50（Related #42 / #43）
**Date:** 2026-10-24
**Owner decisions:** 项目 Top 过滤临时目录 + 「未归属」桶；RTK 作为总花费的第三种查看口径；缺失时显示安装与 Harness 接入引导；先做 RTK，再 OrcaTerm，最后 CodeArts。

## 1. 问题

RTK 已经在本机节省了约 45.9M token，但 Pane 看不到。用户需要在总花费卡里回答「RTK 省了多少、哪些项目省得最多」，而不是另起一张卡，也不能把它混进账单、额度或日报总量。

## 2. 数据与口径

- 全局：`rtk gain --all --format json`，顶层 `summary/daily/weekly/monthly`；60 秒后端缓存。缺失、非 JSON、旧版不支持 `--format json` 都降级。
- 项目：复制 `%LOCALAPPDATA%\rtk\history.db` 与 `-wal`/`-shm` 到 Pane 临时目录后只读查询 `commands`；不直接锁运行中的库。
- `saved_tokens` 是 RTK 的**字节/4 估算**，不是厂商账单 token；卡片标「估算」，不参与 spend/quota。
- 当前窗口：today/7d/30d 从 `daily` 按本地日期汇总；all 用 `summary.total_saved`。项目窗口用本地日期边界。
- 长期：`%APPDATA%\Pane\rtk_savings_history.json`，按日期保存 `saved_tokens/commands/input_tokens/output_tokens/savings_pct`；合并是**覆盖同日**，不是累加。

## 3. 已核实的坑

- 本机 909 行里 `%TEMP%\2026-10-24\f628eddd` 占 44.07M（95.9%），严格按路径显示 Top 会出现 GUID。
- 因此项目区：过滤 `%TEMP%`、`%LOCALAPPDATA%`、`%APPDATA%`、`.qoder-cn`、`resources/app`、纯日期/长 GUID 目录、用户根目录；剩余按 basename 显示 Top 5，另给「未归属（临时目录）」桶。
- RTK 的 90 天保留期未实测；快照按日期覆盖以补缺口。

## 4. 卡片

- `RTK 节省` 是 Total Spend 里与美元、tokens 并列的第三口径；不进入 `lastSpend`/donut/额度。
- 头部保留 Total Spend 的标题、折叠和 chrome；当前口径显示 RTK 节省值或 `—`。
- 主体：四窗口 tabs（今天/7d/30d/全部），saved tokens、命令数、压缩率；30 天 sparkline；项目 Top 5 + 其他项目 + 未归属。
- 已安装：RTK tab 图标高亮；点击进入原有节省明细，并提供「管理 RTK 接入」。
- 缺失态：RTK tab 置灰，点击进入安装引导；显示官方 `winget` 安装入口和 17 个 Harness 接入目标。未安装时 Harness 按钮禁用。

## 5. 架构

- `src-tauri/src/rtk_savings.rs`：CLI 调用 + 60s 缓存、DB 复制/查询、快照读写、纯解析函数。
- `lib.rs`：`fetch_rtk_savings` 命令注册（独立于 `fetch_spend`）。
- `src/main.ts`：总花费的 RTK metric、节省明细、安装/接入面板；`refresh()` 并行拉取。
- `src/i18n.ts`：en/zh/ru 三语；`src/styles.css`：卡片局部样式。
- 测试：纯 JSON/DB/快照解析；CLI/DB 失败降级；同日重复刷新不重复；项目过滤。

## 6. Out of scope

- 不做 `--quota`、账单/美元折算、跨机聚合、命令/路径展示或写 RTK 文件。

## 7. Risks

- RTK JSON 字段未来变化 → parser 只取已知字段，缺字段给空态而非 panic。
- DB 锁/复制不完整 → 项目区单独降级，全局仍可用。
- 旧快照/时区边界 → 统一本地日期，快照按日期覆盖。

## 8. Implementation deltas (2026-10-24)

- 「Top + 未归属 = 全局」在超过 5 个可识别项目时不成立；新增 `otherSaved` 行，变成 **Top 5 + 其他项目 + 未归属 = 全局**。
- 全局数在 `fetch_rtk_savings` 落地后立即写入并重绘，不再等 spend 扫描。
- `RTK_BIN` 视为权威覆盖（不存在即 missing），用于可复现三态与排障。
- `.qoder-cn` 过滤改为不要求尾随斜杠，避免用户根下的 `.qoder-cn` 目录本身漏进 Top。
- 坏历史文件改名为 `.json.bad`，卡片显示一行「本地快照不可用」，不静默覆盖旧快照。
- 项目过滤与 `unattributed` 的口径：临时/应用数据/工具内部目录，加上历史库没有项目信息的旧记录；两者共用一行「未归属」，tooltip 写清。
- 2026-10-24 设计修订：移除独立 RTK 卡与独立折叠状态；RTK 跟随 `showTotalSpend`，因此关闭总花费时不会留下孤立卡。
- Windows 一键安装使用官方 winget 包 `rtk-ai.rtk`；Harness 接入执行固定白名单中的 `rtk init -g ... --auto-patch`，不接受前端任意参数。
