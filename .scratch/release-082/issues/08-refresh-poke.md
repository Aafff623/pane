# 刷新点自动触发（默认关，用户显式开启）

**What to build:** 为「必须被请求一次才进入计费周期」的 5 小时套餐（GLM / MiniMax 等）加调度：到下次刷新时刻自动发一次轻量请求，并在卡片上给出「已自动触发」标识。默认关闭。

**Blocked by:** None

**Status:** ready-for-agent

- [ ] 「何时该触发」抽纯函数（窗口/刷新点计算）并进 parse-tests
- [ ] 默认关闭；开启需用户在设置中明确同意（会消耗一次真实请求）
- [ ] 触发成功/失败都在卡片上有可读状态，不静默
- [ ] 同一刷新点只触发一次（幂等），应用重启后不重复触发
- [ ] 覆盖 provider 可配置（GLM / MiniMax / 其它同形态厂商）

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
