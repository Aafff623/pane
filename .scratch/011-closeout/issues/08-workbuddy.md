Status: pending
Type: task
Blocked by: 06

# 08 · WorkBuddy 用量与多账号（#31 / #29 / #27）

## #31 CN 用量面板（AI 档不做）

1. `workbuddy_official_usage` 增加 ledger 来源：缓存、手动刷新、错误态；
   不写入 credits 账本。
2. UI：账号筛选、每日趋势、模型/客户端分组、请求明细（最新 N 行）；
   大列表虚拟化或分页，避免卡顿。
3. 纯函数测试：分页合并、去重、聚合、partial 标记。

## #29 多账号收尾

- 最近使用、健康 pill、多选删除、资源包排序已存在；补批量重登的明确限制
  文案（OAuth 无法指定账号），owner 3 账号验收。

## #27 验收矩阵

- 更新 `.scratch/workbuddy-family-v2/acceptance-checklist.md` 的过期描述
  （官方流水/IDE 已接入）。
- 提供 owner 真机勾选清单；代码自动化证据附后。

## 验收

CN 账号真实数据可出面板；无账号时为空态而非 0；owner 真机验收。
