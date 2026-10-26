Status: blocked (owner login)
Type: task
Blocked by: owner #38/#47 login

# 12 · 讯飞真实响应校准（#38 / #47）

## 做什么

1. #38 Spark-X2.6：owner 在 Pane 登录一次，抓真实 `free-quota`/usage 响应；
   校准 `claim`/`quotaStatus` 大小写、`expiresAt` 格式、cookie-only 是否 4001。
2. #47 xfyun-speech：owner 打开 console，给出资源包余量与 Pane 卡片数字对照；
   校准 `left + trial` 口径。
3. 更新 fixture/parser/文档；无法确认的字段标“待确认”，不猜。

## 验收

真实响应与 fixture 对齐；owner 控制台数字一致；三语文案。
