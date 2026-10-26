Status: ready
Type: task
Blocked by: #63-06（Argon 周窗解析）

# 01 · Argon 现价落地 + 12-01 变价预案

## 做什么

1. supplement pricing 补 gemini-4-argon 现价（input $2/cache_read $0.10/output $10 每百万）+parse-tests 断言。
2. 12-01 变价预案：新价（$4/$0.20/$20）排进 supplement 更新队列；issue 评论+日历留 12-01 当天验证提醒（本机实测 Argon 卡片金额）。
3. 若 #63-06（周窗）未落地则先只补价，窗口到位后验证归属。

## 验收

现价断言绿；变价预案记录在案；12-01 实测金额正确（或确认上游 catalog 自动带新价则只需验证）。
