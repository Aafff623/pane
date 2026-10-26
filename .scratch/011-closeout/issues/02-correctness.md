Status: pending
Type: task
Blocked by: 01

# 02 · 正确性收口（#48-3 / #37-03）

## #48-3

1. `volcengine.rs`：签名/凭据类型错误（非网络、非 429）映射为 `needs_setup`，
   附“AK:SK / 控制台入口”提示；保留真实网络错误为 error。
2. `zai.rs`：pay-as-you-go 余额回退只在确认官方端点后实现；确认不了就保留
   `needs_setup` 的“待确认”文案，不猜端点、不造 0%。
3. 测试：错误凭据类型、无套餐、网络失败三分支；detail 文案不变。

## #37-03（窗口来源展示）

- 默认在 Qoder 明细的 tooltip/详情里显示窗口来源（DB 窗口 / 同模型回退 /
  未知），不占主行；三语文案。
- 测试：有窗口、无窗口、同模型回退三分支；旧报告兼容。

## 验收

`parse-tests` 相关测试全绿；Qoder 明细可解释窗口来源；zai/volcengine
不再把可修复状态画成红色错误。
