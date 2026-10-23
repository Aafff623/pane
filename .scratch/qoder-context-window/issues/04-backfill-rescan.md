# 04: 修复后回填重扫

**What to build:** 修复窗口解析后，允许把当前 31 天扫描窗口内的 Qoder
转录重新计算一次 credits / tokens，并在界面/弹窗里说明归档与已删除会话的
可读范围。不是重写历史，也不是迁移数据。

**Blocked by:** 01, 02

**Status:** done (owner UI acceptance pending)

- [x] 解析缓存带版本号；升级后的代码不会继续使用旧窗口的 ParsedFile。
- [x] 手动重扫会清空 Qoder 转录缓存、重新 collect，并返回最新 credits/tokens。
- [x] 只读查询 Qoder `chat_sessions` 的 archived/deleted 状态，按稳定
  session id 对照扫描窗口内的主转录文件，报告可读数。
- [x] Qoder 明细里新增「重新扫描」按钮；完成后弹窗显示会话/请求数、
  归档可读 x/y、已删除 n 个（其中 m 个仍有转录），并说明删除且无转录
  的会话无法恢复、tokens 仍是估算。
- [x] 数据库缺失/锁定/缺表时统计降级为“不可用”，不 panic；重扫仍能
  从转录重新计算。
- [x] en/zh/ru 文案齐全。
- [x] Qoder 定向测试、`pnpm build`、parse-tests 通过。
- [ ] owner 真机验收：重扫前后 Qoder 当天 token 恢复、归档/删除提示正确。

## Comments

- 2026-10-23: 现场数据（最近 7 天 50 个会话）：35 个归档，其中 28 个仍有
  主转录；5 个已删除，主转录 0 个。Qoder DB `chat_sessions` 有
  `archived` / `deleted_at`，`chat_model_preferences.context_window` 已用于
  01/02。回填只覆盖当前 31 天扫描窗口；删除且无转录的会话不能恢复。
