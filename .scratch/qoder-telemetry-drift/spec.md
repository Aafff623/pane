# Qoder 本机用量的读数能力哨兵（qoder-telemetry-drift）

**Status:** 01 已实现（本机实测口径），02 待办
**GitHub:** 无（owner 2026-10-23 口头立项，起因是 Qoder CN 从 0.4.9 升到 0.4.11）

## Problem Statement

Qoder CN 每次升级都可能改掉「本机到底能不能读出用量」这件事，而且**不会有任何东西报错**：

- 0.4.9：`chat_model_preferences.context_window` 被写成 null，估算的分母没了。
- 0.4.11（本轮实测）：本地彻底不存实测 token 数了。证据三处并存 ——
  `chat_session_context_usage.snapshot_json` 自己声明 `tokenCountsAvailable=false`、
  `totalTokens=0`、`maxTokens=0`；`chat_session_messages.payload_json` 里 400 行内
  **0 处**数字型 `input_tokens`/`prompt_tokens`/`cache_read_*`；会话 JSONL 的
  `runtime-config.contextWindow` 也是 null（`transcriptWindows: declared 59 / nullish 98`）。
- 同时窗口只剩 3/12 个模型有值（`dfmodel`/`qfmodel`/`qmodel_38max` = 1,000,000），
  `schema_migrations` 版本 84。

后果不是「读不到」而是「读到的意思变了」：Pane 的 token 总量一直是
`context_usage_ratio × 窗口` 推导值，一旦 model_key 改名或窗口全 null，同一列数字会
悄悄换基准。credits 不一样，它走远端 API 是实测的（`qodercn` 卡状态 ok），所以钱的口径
没坏——**这条线必须和单位分离（ADR 0003）一起守住：credits 实测，tokens 估算且标明基数。**

## Solution

不新增机制，把「读数能力」变成可报告、可对比的事实：

1. **路径发现不再依赖版本号**：`qoder_db_from(data_dir)` 扫 `%APPDATA%\com.qodercn.app*`，
   取最近更新过的 `main.sqlite`；容器改名或新增 stable2 都不用改代码。
2. **能力探测进报告**：`QoderSpend.telemetry`（`dbFound / dbPath / schemaVersion /
   modelsTotal / modelsWithWindow / exactTokenCounts / estimateBasis`），
   随 `/v1/usage` 一起出去，外部消费者拿到同一个结论。
3. **卡片说明基数**：Qoder 日块追加一句「{n}/{total} 个模型给出了窗口（推导值）」，
   窗口全空时说「不可估」。真值缺失仍不补 0（沿用既有 `tokens_unavailable_trips`
   与「—」渲染，本轮没有放宽它）。
4. **升级后一条命令自检**：`node temp/scripts/verify-qoder-drift.mjs`
   （只读：把库连 -wal/-shm 复制到 temp 再开，因为 WAL 库无法就地只读打开），
   与 `temp/qoder-drift-baseline.json` 比对；差异退出码 2，基数不可用退出码 3，
   `--update-baseline` 才写基线。

## Out of Scope

- 不替 Qoder 造 token 真值：不去抓包、不读浏览器 profile、不调用厂商接口估 token。
- 不把 credits 与 tokens 合并成同一个数或同一个进度条。
- 不改 marketplace 插件本体（`credits-inspector`、`token-usage` 装在插件缓存里，
  下次插件更新会覆盖本地改动）；插件收敛到 Pane 的本地 API 属于 02 票。

## 工单

- 01 读数能力探测 + 路径发现去版本化 + 卡片基数说明（本机已实测，含回归）
- 02 插件侧收敛：`credits-inspector` 去掉 200K 静默回退、`token-usage` 判定弃用或换源，
  两者改读 `127.0.0.1:6736`；需要 owner 决定「修还是弃」，并给上游提改动

## Comments

- 2026-10-23：01 实现。基线已写入（schema 84；3/12 模型有窗口；官方 token 计数 false；
  转录 164 个文件、2781 行带 ratio）。02 未开始，等 owner 拍「token-usage 修还是弃」与
  是否允许给插件上游提改动。
