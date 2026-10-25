Status: ready
Type: task
Blocked by: 03

# 04 · 回扫估算标记 + 显式重建命令 + UI 标识

## 做什么

1. `PersistEntry` 新增 `estimated: Vec<(day, model)>`；spend_history 的 `ModelDay` 新增 `estimated: bool`（旧记录默认 false）。
2. 首次 collect：ensure_fresh 先拉 catalog → 在用 (scope, model) 落首条观察，observed_at = catalog 拉取时刻（state.json fetchedAt，非 now）；ts 早于首条观察的事件 estimated=true。一旦计过即冻结。
3. UI：History/Daily 视图对 estimated 记录显示「≈」。
4. 新 Tauri 命令 `rebuild_spend_pricing(scope: Option<String>)`：清 spend_cache（可选清 spend_history 指定范围）→ 强制全重解析；UI 二次确认。时间线不可变使结果除目标变化外逐字节一致。
5. Settings 文案：改价只向前生效；修历史走显式重建入口。

## 验收

- parse-tests：回扫 estimated 标记落两处；后到观察不改旧日；显式重建无变化时前后美元一致。
- UI 验收（用户本人）：≈ 标识可见；重建入口二次确认。
