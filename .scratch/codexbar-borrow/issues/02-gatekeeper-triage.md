Status: ready
Type: task
Blocked by: 01

# 02 · gatekeeper 残量填量（行内标记/白名单）

## 做什么

对票 01 扫出的每一个命中，逐个决策：a) 改写为 `providers::<family>::` 转发（优先）；b) 加 `// provider-specific: <理由>`（真外部契约）；c) 进白名单（锚点+expected_families+理由）。禁止无理由豁免。

## 验收

- 扫描零未解释命中；白名单条目都有理由且 expected_families 准确。
- parse-tests 全绿；报告含白名单规模统计（对照 CodexBar 7.8 槽/provider 基线评估健康度）。
