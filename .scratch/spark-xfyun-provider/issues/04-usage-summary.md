# 04: 最近 7 天用量摘要 + 收尾

**What to build:** 用 `api-services/usage` 拉最近 7 天用量，聚合成现有卡片能显示的文本行或 `UsageDays`；补 i18n、图标、主题、文档和验收清单。不做按模型明细表、导出或独立用量页。

**Blocked by:** 03

**Status:** done (real usage response calibration pending owner login)

## 要点

- 用量是补充信息，失败不影响免费额度卡片；额度仍是主信息。
- 聚合 `inputTokens`、`cacheTokens`、`outputTokens`；日期边界按本地时区；响应行结构以真实调用为准。
- 如果现有卡片承接不了 7 天趋势，降级为「今日 / 近 7 天」文本行，不新增表格页。
- i18n 覆盖 en/zh/ru；图标/配色按品牌资产规则；文档更新（如果项目的 provider 文档列出 provider）。
- 最后跑 parse-tests、tsc、vite、i18n、node 门禁；owner 实机验收。
- 不做 Token Plan、账单/充值、自动领取、按模型明细表。

## 验收

- [ ] 卡片显示最近 7 天用量摘要，数字与网页控制台/usage API 对得上（需真实登录校准）。
- [x] 用量接口失败时额度卡片仍正常（usage 失败被忽略，额度快照照常返回）。
- [x] 三语言文案、图标、主题正常；相关文档更新。
- [ ] 全量门禁绿；owner 实机验收。

## Comments

- 2026-10-23：用量摘要实现为 7 天 `usage` 接口的分页聚合文本行（`inputTokens + cacheTokens + outputTokens`），失败不影响额度；新图标、机制表、onboarding 和三语文案已补。真实 usage 响应结构尚未登录校准，数字以 API 为准。
