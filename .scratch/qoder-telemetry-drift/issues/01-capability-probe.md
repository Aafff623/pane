# 01: 读数能力探测 + 路径去版本化 + 卡片基数说明

**What to build:** 让 Pane 在 Qoder CN 升级后仍然「说得清自己读到的是什么」：数据库位置靠发现而不是硬编码版本；`/v1/usage` 与卡片一起给出本机读数的基数；一条命令可与上次基线对比。

**Blocked by:** 无

**Status:** done (code) — owner 目检基数文案待做

## 要点

- `qoder_db_from(data_dir)`：`%APPDATA%\com.qodercn.app*` 里取最近更新过的 `main.sqlite`，不写死 `com.qodercn.app.stable`，也不碰 `.qoder-versions/<版本>`。
- `read_window_facts`：一次读三件事——`schema_migrations` 最大版本、`chat_model_preferences` 的**全部**行数与非空窗口数（null 窗口要计入，否则「今天没用」和「更新把窗口拿走了」长得一样）、`chat_session_context_usage` 最新快照的 `tokenCountsAvailable`。表缺失/库被锁一律降级成「什么都不知道」，绝不猜窗口。
- `QoderSpend.telemetry`（camelCase 出网），卡片日块加基数说明；`qoder.basisPartial` / `qoder.basisNone` 三语齐全。
- `temp/scripts/verify-qoder-drift.mjs`：只读（复制库连 -wal/-shm 再开），差异退出码 2、基数不可用 3、`--update-baseline` 才落基线。

## 验收

- [x] 新增回归 `db_discovery_follows_the_newest_container`、`window_facts_count_null_windows_and_report_the_official_token_flag`
- [x] 哨兵脚本在本机可跑，基线记录 schema 84 / 3 个模型有窗口 / 官方 token 计数 false
- [x] `node --test tests/*.test.mjs`、`corepack pnpm build`、`parse-tests` 全量绿
- [ ] owner 目检：Qoder 日块那句基数说明是否看得懂；升级一次 Qoder 后跑哨兵脚本，确认可比出差异

## Comments

- 2026-10-23：实现完成。既有诚实行为（无窗口时计入 `tokens_unavailable_trips`、token 列显示「—」而不是 0）未放宽，本轮只补「为什么」和「基数从哪来」。
