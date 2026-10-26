Status: pending
Type: research
Blocked by: 01

# 11 · 反事实重放 research spike（#59-01）

## 问题

Pane 没有可控的压缩前后成对样本；不能凭空宣称“反事实重放”的收益。
先调查本机 cc-switch / RTK / 本地日志是否能构造有依据的对照。

## 做什么

1. 盘点可用数据：`temp/cc-switch`、RTK 日志、`proxy_request_logs`、
   本地会话样本；确认是否有同请求的 before/after。
2. 若样本成立：写最小算法与离线回放测试，再决定是否进统计页。
3. 若不成立：记录“不做”的证据与替代方案（#59-02/03 保留）。

## 验收

spike 结论落 `.scratch/011-closeout/` 或 `temp/research/`；不为了凑功能造数据。
