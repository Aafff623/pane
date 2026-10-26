Status: done (2026-10-26) — extended key/interpolation/brand parity
Type: task
Blocked by: —

# 02 · i18n 三语 parity 测试

## 做什么

新建 tests/i18n-parity.test.mjs（node --test）：① en/zh/ru flatten 后 key 集合互相对比（缺/多都报）；② 插值变量 `\{([^}]+)\}` 提取排序 join("\0") 对比；③ 品牌词保留：品牌词表（Codex/Claude/Claude Code/Copilot/GLM/ZCode…按 src/i18n.ts 实际盘点）在 en 出现的 key，断言 zh/ru 译文仍含。挂进 package.json test 脚本。

## 验收

- 三个测试全绿；故意删一个 key/改一个变量/翻掉一个品牌词 → 分别红。
- 与 cc-switch localeCoverage.test.ts:10-54 的断言结构对照留注释。
