Status: deferred (optional; no per-request source)
Type: task
Blocked by: 07
Priority: P3

# 08 ·（可选）请求明细直读 cc-switch.db

## 做什么

仅当本机存在 cc-switch.db 且用户启用时：新增 `fetch_request_logs`（只读，复用 ccswitch_db_read 连接模式 spend.rs:3145）出明细表（时间/应用/供应商/模型/成本/token 四分量/状态码，服务端分页 100 行/页）；无库或未启用 → 隐藏该 tab。列可配置后置。

## 验收

- 有库：分页翻页正确、状态码筛选可用；无库：tab 隐藏不报错。
- 只读打开不持锁（busy_timeout(ZERO) 同 #53 加固）。

## 注意

这是 Pane 唯一能出 per-request 明细的源（自身聚合不留明细）；定位为「cc-switch 用户的加成」而非全用户功能。
