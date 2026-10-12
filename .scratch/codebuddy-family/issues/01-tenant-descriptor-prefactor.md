# 01 — 预重构：CodeBuddy 家族的租户描述表

**What to build:** 把 CodeBuddy 家族里「主机名 + platform + 显示名 + 图标」抽成**一张租户描述表**，现有 `codebuddy`（国际站 `www.codebuddy.ai`，platform `ide`）改为从表里取值。**行为完全不变**——登录、卡片、额度读数必须与改动前一模一样，目录一致性门禁与既有解析测试全绿。

这是后续加租户的铺垫：让「加一个租户」只改表里一行，而不是在登录流程、provider 模块、目录四层各改一遍。

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] 租户描述表存在，`codebuddy` 的全部调用点都从表取值，无残留硬编码主机名 / platform
- [ ] 现有行为零变化：登录、卡面、额度读数与改动前一致
- [ ] `parse-tests` 全绿（含 `catalog_consistency` 的两个门禁）
- [ ] 表里以注释写明「新增租户 = 加一行」的约定

## Comments

- 2026-10-11 出自 `.scratch/codebuddy-family/spec.md`（决策 D1 / D2）。
- 2026-10-12 **已实现**（worktree `codex/codebuddy-family`）：`codebuddy_login::Family` + `FAMILIES` 表 + `family()`/`known_family()`；`start`/`refresh`/`fetch_account` 与 provider 模块全部改为按描述符取值；lib.rs 三处分发点改为**表驱动**（登录卡片、裸卡、auth center），裸卡列表由表生成——加成员只改一行。**实测**：`parse-tests` 686 通过 / 0 失败 / 13 忽略；`cargo check`（真 app crate）exit 0；`tsc` + `vite build` exit 0。
  - 注：本机 `pnpm` 已损坏（PATH 上的 shim 指向不存在的 `pnpm.cjs`），前端门禁改用主树已装依赖（`cp -a node_modules`）直跑 `./node_modules/.bin/tsc` 与 `vite build`，非 `pnpm build`。
  - 合并形态按 spec D1 的 2026-10-11 修订落地：**代码层一张表，目录层三个条目**。
