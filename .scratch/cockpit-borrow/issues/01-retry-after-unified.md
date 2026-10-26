Status: done
Type: task
Blocked by: —

# 01 · 统一 Retry-After 头解析（P1，半天）

## 做什么

1. provider HTTP 公共层加 Retry-After 解析（数字秒 + HTTP-date 两种格式，RFC 7231）；解析结果写 snapshot 既有 `retry_after_s=` 通道（snapshot_store.rs:341 bench_after_error 已消费）。
2. 退避合成：`max(指数退避 5<<n 封顶 300s, 服务器提示封顶 3600s)` 取大者。
3. 全部 provider 的 429/503 路径复用；诊断信息（status/code/body_len）走结构化字段不拼串。

## 验收

- parse-tests：两种格式的 Retry-After 解析断言 + 取大者语义；模拟 429 带 Retry-After → bench 按提示延迟。
- 本机：任一 provider 429 后刷新间隔符合提示。
