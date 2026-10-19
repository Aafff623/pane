# 08 — QA 矩阵（只读能力收缩版）

**Blocked by:** 01, 02, 03, 04, 05, 06, 07
**Status:** done（自动化 + 手工清单一页；owner 实机勾选待办）
**Issue:** #27（实施，收缩）

**What to build**

一份可执行的验收清单 + 自动化覆盖表，只覆盖**只读**能力：

- 家族/档位：CN/AI/CodeBuddy/CodeBuddy CN 四个 family 的目录四层一致。
- 身份：同 UID 跨档位隔离、同账号重登录去重、桌面导入 upsert。
- 认证：扫码并发、refresh 单飞、401/403/429/WAF/断网文案。
- 额度：多包、冻结、过期、不限量、UTC+8、7 天临期。
- 用量：CN/AI/CLI 三源、去重、unpriced、credits/token 不混算。
- 安全：日志/UI/测试 fixture 无 token；`$wbEncrypted` 不解密；不写官方文件。

**明确排除**：切号、会话复制、Hooks、签到、Agent Companion 的验收项（这些能力不存在，不列假条目）。

**验收**
- [ ] `parse-tests` 全绿；列出新增测试用例清单
- [ ] 手工验收清单一页（owner 按单勾选）
- [ ] 未验证项明确写「待 owner 实机」

## Comments

- 2026-10-19 出自 spec §6。
- 2026-10-19 自动化结果：`parse-tests` 898 通过 / 0 失败 / 13 忽略（新增
  13 条：档位路径、域名匹配、桌面文件分档、身份解析、错误分类、单飞分类、
  资源包明细、三处 usage 位置、缓存拆分与去重、未定价）；`tsc` 0；
  `vite build` 成功；`node --test` 51/51；`cargo check` 通过。
  手工清单见 `../acceptance-checklist.md`。
