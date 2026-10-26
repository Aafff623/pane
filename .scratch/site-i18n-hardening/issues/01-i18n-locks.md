Status: ready
Type: task
Blocked by: —

# 01 · parity 断言显式化 + 词典双向单测 + setText key 锁

## 做什么

1. site.test.mjs:31-33 空转分支改显式断言（声明无 data-i18n 轨 or 补轨——推荐前者）；新增词典三语双向 parity 单测（key 集合 deepEqual+插值变量集合对比，magpie worker.test.mjs:117-129 同宗旨）。
2. site.js setText/setTexts 加 key 存在性锁（en[key] 缺失 console.warn 不静默）；i18n.js 启动自检打缺失 key 列表。

## 验收

删一个词典 key → parity 测试红（证明非空转）；临时缺 key → 控制台 warn；`node --test site/*.test.mjs` 全绿；owner 验收。
