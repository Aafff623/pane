# 03: 卡片呈现 + i18n/图标/文档 + owner 验收

**What to build:** 把 02 取到的五类余量在卡片上表达清楚，补齐 en/zh/ru 文案、图标、机制说明和 `docs/providers.md` 段落，并通过全部门禁，最后交 owner 实机对照验收。

**Blocked by:** 02

**Status:** ready-for-agent

## 要点

- 主信息是**剩余量**，不是百分比：五个服务单位不同（次 / 小时），不能汇成一张进度条。每个服务一行 `Metric::with_usage`，另加「到期」文本行；`maxConnection`（路数）是独立事实，不并进用量行。
- 试用池（`threshold/remainCount`）与套餐包（`left`）要分别标注，不能相加成一个池子展示。
- 空态与错误态分开：未开通渠道 = 空态；`code=80000` = 重新登录；取到但字段缺失 = 「未取到」，不补 0。
- 到期不进 `resets_at`（一次性包，非周期重置；同 #38 的判断）。
- 若某个 app 一个包都没有，卡片显示「未领取资源包」并给出领取入口提示（领取页 `www.xfyun.cn/free`，个人/企业两档，额度不同）；Pane 不代点领取。
- 文案走 `src/i18n.ts` 三字典（en/zh/ru），不新增只有中文的硬编码字符串；`tests/i18n-pairing.test.mjs` 会拦。
- 前端 `providerMechanisms.ts` / `providerOnboarding.ts` / `providerVisuals.ts` + `src/assets/providers/<family>.svg` 按现有规则补，`tests/provider-credential-flow.test.mjs` 要求每个 provider 有 `MECHANISMS` 条目。
- `docs/providers.md` 新增一节，逐条写清「读哪些接口、发什么头、字段含义、哪些含义待确认」；CONTRIBUTING 要求每个新 provider 都有这一节。未核实项按 AGENTS 规则标 `待确认`，同步一份到 `docs/project-facts.md`。
- 注意 `parse-tests/src/catalog_consistency.rs` 的 disabled-count 断言会因新增 family 变化，按实际数字更新，不要为了让断言通过而把 family 改成默认启用。

## 验收

- [ ] 卡片五个包的剩余、单位、到期与网页控制台逐项一致（owner 本人对照，不是截图或 agent 判断）。
- [ ] 会话过期表现为「重新登录」引导，绝不出现 0% 或空数字卡片。
- [ ] 三语文案齐全，图标与机制说明存在，暗色/亮色下可读。
- [ ] `parse-tests`、`node --test tests/*.test.mjs`、i18n 配对、catalog 一致性、`tsc`/`vite build` 全绿。
- [ ] `docs/providers.md` 新段落写完，含待确认清单；Git 跟踪文件内无 cookie、无 uid、无 APPID、无真实订单号。
- [ ] owner 明确点头后才算 UI 验收通过。

## Comments

- 2026-10-23：调研阶段未动 Pane 任何代码。owner 账号的当前实测数字写在 `temp/xfyun-console-quota-analysis-20261023.md`（不入库），本文件只留对照方法，不复述账号数据。
