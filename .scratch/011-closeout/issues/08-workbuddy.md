Status: in progress — #29 文案已补、#27 清单已刷新；#31 后端 ledger 已实现且真机验证，缺面板 UI
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

## 结果

### #29 已落地

多账号收尾里那四件（最近使用排序、健康 pill、多选删除、资源包排序）确实都在，
本票只补了唯一缺的那条：**重新登录无法指定账号**的明确文案。原因不是猜的——
OAuth 总是认证厂商网页当前已登录的那个账号，Pane 无从指定；不说清楚，失败会被
当成 Pane 的 bug。授权中心工具栏现在带一个 `?` 帮助卡（`help-tip` +
`data-help-title/body`），三语齐备，并给出两条真正可行的绕法：先在浏览器里退出
该厂商再重登，或用各自的浏览器配置档分别登录。

### #27 已落地

`.scratch/workbuddy-family-v2/acceptance-checklist.md` 刷新了：

- 顶部写明 **本版只验国内档**（owner Gate 0 决定 AI 档不做），并明确 A 节里
  WorkBuddy AI 档的旧条目**不在本次勾选范围**——保留原文只为不丢设计依据。
- 「等官方流水接入后再验」这类过期前提已改掉。
- 自动化基线换成当前真实数字（parse-tests 1187 / node 215 / site 50）。
- 新增一条验收铁律：**无账号必须是空态，不能是 `0`**。

### #31 后端已实现且真机验证过；缺的是面板 UI

**更正 2026-10-27：本票此前写「#31 未做」是错的。** 逐行核对代码后：

- `src-tauri/src/providers/workbuddy_official_usage.rs`（369 行）已经有完整的纯函数层：
  `parse_page` / `decide_next`（分页推进）/ `merge_pages`（合并去重）/
  `detail_rows` / `aggregate` / `sweep_round_limit_reached`（轮次上限）。
- `src-tauri/src/providers/codebuddy.rs:697` 的 `official_usage()` 已经把它接进请求循环，
  注释写明 **2026-10-24 在 CN 主机真机验证**：HTTP 200 / code 0 / 30 天窗口 26 行，
  并带 `partial` 标记（扫到轮次上限就标部分，不假装完整）。
- 也就是说本票第 1 项（ledger 来源）和第 3 项（纯函数测试：分页合并/去重/聚合/partial）
  的主体已经存在。

**真正缺的只有第 2 项：面板 UI。** `src/main.ts` 里没有任何 `officialUsage` 相关渲染
（全仓 grep 只在 `providers/mod.rs:68` 的声明和 `codebuddy.rs` 的请求循环里出现），
所以账号筛选、每日趋势、模型/客户端分组、请求明细分页这几件在界面上还看不到。
缓存与手动刷新是否已由卡片刷新路径覆盖，需要单独确认——这次没有验证到。

### 待办


1. 用真实 CN 账号登录，确认卡片出现且数字与官网一致。
2. 无账号状态下确认是空态而非 0。
3. 授权中心工具栏点 `?`，确认三语说明可读、Tab 可达。

