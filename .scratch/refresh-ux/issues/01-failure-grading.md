# 01 — 失败分级：开机中性态 + 连续失败降级 + 行内重试

Type: task
Status: ready-for-agent
Blocked by: None

## What to build

- `Snapshot` 增 `boot_cache`（serde 默认 false）；`cached_usage` 置位；前端对它渲染中性 chip（zh「缓存数据 · 刷新中」），不再挂 ⚠。
- Rust 按快照 id 维护连续失败计数：失败（error/降级透传）+1，成功清零；`restore_last_success_after_error` 的 stale 判定改为「超 3 分钟宽限**且**连续 ≥2 次」。
- 前端 ⚠ chip 可点击=单卡重试（复用 data-card-refresh），悬浮原因保留。
- 三语文案；计数判定纯函数进 parse-tests。

## Acceptance

- [ ] 启动首屏不再出现 ⚠，显示中性「缓存数据 · 刷新中」；首轮成功后消失。
- [ ] 偶发一次失败不亮 ⚠；连续两次失败后亮，点击即重试该卡。
- [ ] 成功一次计数清零，⚠ 消失。
- [ ] parse-tests / tsc / vite 全绿，等待用户 UI 验收。
