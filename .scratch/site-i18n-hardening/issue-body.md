Status: ready
Issue: https://github.com/Aafff623/pane/issues/80
Priority: P1
Area: Site

# 官网 i18n 加固：parity 断言去空转 + setText key 锁

## 问题

site.test.mjs 的 i18n parity 断言因页面无 data-i18n 而**空转**（keys.size===0 提前 return）；site.js setText 无 key 存在性锁（缺 key 静默不换文案）。magpie 同构站用双向锁（缺 FAIL/多 FAIL）治同病。

## 方案（详见 spec.md，证据 temp/research/20261025-website-reference-sweep.md §二）

1. 断言显式化：声明无 data-i18n 轨（推荐）或补轨恢复真断言。
2. 词典三语双向 parity 单测（key 集合+插值变量）。
3. site.js setText/setTexts 加 key 存在性锁（缺失 warn 不静默）+i18n.js 启动自检。

## 验收

parity 断言非空转（删 key 验证会红）；key 锁 warn 可手测；测试全绿。
