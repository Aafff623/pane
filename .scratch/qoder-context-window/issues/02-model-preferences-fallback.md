# 02: Qoder 模型偏好窗口回退

**What to build:** 当转录和同模型最近已知窗口都给不出窗口时，只读读取 Qoder CN 模型偏好中该模型的 `context_window`，用于新会话的 token 折算；按模型隔离、按扫描缓存、失败降级。用户在更新后新建的、整场 `runtime-config` 全为 `null` 的会话里也能看到 token。

**Blocked by:** 01

**Status:** done (owner UI acceptance pending)

- [x] 临时数据库 fixture：`dfmodel=1000000`；整场只有 `null` 的新会话按 1M 折算出 token。
- [x] 数据库中该模型窗口为 `NULL` 或缺失 → tokens 不可用，credits 精确，`tokens_unavailable_trips` 增加。
- [x] 数据库不存在、被占用或缺列 → 不 panic，Qoder 账本仍可生成，只是 tokens 不可用。
- [x] 不同模型不串窗口；没有 200K 或任何全局固定窗口回退。
- [x] 适配器只读；一次扫描内缓存，不重复打开数据库，不写 Qoder 文件。
- [x] parse-tests 全绿；面板新会话显示恢复后的 token。

## Comments

- 2026-10-23：新增只读 `chat_model_preferences.context_window` 适配器（`model_windows_from`），每次 `collect()` 只读一次，数据库/表/列缺失或查询失败一律降级为空映射。回归覆盖：整场 null 的新会话按 DB 1M 恢复、DB 中 NULL 模型保持不可用且 credits 精确、缺库/缺表不 panic；Qoder 定向 13 passed / 1 ignored。
