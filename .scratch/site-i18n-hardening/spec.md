# Spec: 官网 i18n 加固（parity 去空转 + key 锁）

**Status:** implemented (2026-10-26) — used-key lock plus dead-dictionary-key failure, direct `en.*` and pair-table usage included.
**Issue:** https://github.com/Aafff623/pane/issues/80
**Evidence:** temp/research/20261025-website-reference-sweep.md §一（审计发现 9）+ §二 magpie 节 T1

## 问题

site.test.mjs:31-33 的 i18n parity 断言**空转**：页面无 `data-i18n` 属性 → `keys.size===0` 提前 return → 给虚假安全感（README:13 自承）。同根病：site.js:22-23 `setText/setTexts` 无 key 存在性锁——`en[key]` 缺失时静默不换文案（magpie T() 静默回退同病，magpie 用 worker.test.mjs:117-129 双向锁治它）。

## 范围（1 票）

1. **断言显式化**：空转分支改为显式断言——要么声明「本站无 data-i18n 轨，parity 由词典单测覆盖」，要么给页面补 data-i18n 轨后恢复真断言（二选一，实施时与 owner 定；推荐前者+词典单测）。
2. **词典单测**：从 i18n.js 提取三语 key 集合做双向 parity（缺 key FAIL/多 key FAIL，magpie 双向锁同宗旨），插值变量集合对比。
3. **key 存在性锁**：site.js `setText/setTexts` 加锁——`en[key]` 缺失时 console.warn（dev）而非静默；i18n.js 加启动自检（缺 key 列表打日志）。
4. **回归断言**：site.test.mjs 补「词典三语 parity」真断言（取代空转分支）。

## 验收

`node --test site/*.test.mjs` 绿且 parity 断言非空转（删一个 key 验证会红）；site.js key 锁手测（临时删 key 控制台有 warn）；owner 验收。

## 协调

与 site-legal-pages（privacy 页也要 i18n）——本票先立锁，法务页文案随后进词典天然受检。
