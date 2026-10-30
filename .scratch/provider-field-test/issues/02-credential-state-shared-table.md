# Ticket 02 — credentialState 共享凭据表

Status: done 2026-10-30 | Priority: P1 | Depends: 01

## 问题

`/v1/providers` 与首启扫描用 `source_probe::local_hint`（21 家），gear 面板用 `get_credential_status`（63 家）。43 家只在后者，导致 qodercn、orcaterm、shandianshuo、stepfun-plan、traecn、sensenova、zcode 七张有本地会话的卡被报成 `absent`，与卡片状态矛盾。

## 处理

- 把 gear 的整张 family 表移进 `source_probe::local_credential_hint`，HTTP probe、首启扫描和 gear 共用同一个函数；`get_credential_status` 直接读 `probe.local_hint`，每次调用只执行一次 probe。
- `zcode` 补 presence-only 提示：只判断 `~/.zcode/v2/credentials.json` 存在且是普通文件，不解密、不读值。
- 新增 `tests/provider-credential-hints.test.mjs`：锁定 gear 使用 probe 的 hint、共享表保留 7 家与 zcode、旧 `fn local_hint` 不再出现。

## 耗时（本机，39 张卡，5 次）

| 版本 | 单次耗时 | 中位数 |
|---|---|---|
| 修复前（v0.12.2 安装版） | 0.0915 / 0.0878 / 0.0859 / 0.0793 / 0.1879 s | 0.088 s |
| 修复后（debug） | 0.1390 / 0.1498 / 0.1541 / 0.1444 / 0.1529 s | 0.150 s |

绝对增加约 40–60 ms，仍远低于注释里记录的 ~9 s 风险；本轮不引入缓存，避免凭据变化后读到过期的 present/absent。

## 验证

- 7 张卡：`credentialState=present`（source=local），与卡片状态一致；`grok`、`qwen`、`qoder` 仍是 `absent`，对应 `no_credentials` 且有明确引导文案。
- `pnpm build` 通过；根测试 258/258；parse-tests 1207 passed / 0 failed / 18 ignored；`cargo check` 通过。
- gear 与 HTTP 现在走同一 probe，结构上不再存在两张表。

## 未做

- owner 重登 7 项后的复测与 `docs/provider-credentials.md` 回写；第 2 轮未开始。
- 未引入缓存；如果后续 provider 数量或 hint 成本上升，再单独评估。
