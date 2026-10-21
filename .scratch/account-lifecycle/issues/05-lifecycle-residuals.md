# 05 — 生命周期残留：身份化 archive/set-default、command 测试、外部回滚

**What to build:** 把 review 发现但未收口的生命周期项补齐：归档/置顶也按稳定 card id 解析；命令层覆盖快照/order/layout/usage 清理与写失败回滚；外部（Antigravity/Cursor）归档回滚不误删既有墓碑；确认删最后一个账号的 no-account 状态。

**Blocked by:** 01, 02

**Status:** ready-for-agent

- [ ] `account_archive` / `account_set_default` 的 UI 调用改为按 card id 解析；行号过期时不会操作错账号
- [ ] command 层测试覆盖：删除/归档后快照、provider order、per-provider layout、usage history 的清理与默认接班
- [ ] 写失败注入覆盖：archive/restore 回滚不会留下 active + archived 双份，也不会误删原有墓碑
- [ ] Antigravity/Cursor 外部归档回滚同样不误删既有墓碑
- [ ] 删除最后一个账号后显示明确的 no-account / needs-setup，不依赖旧快照兜底
- [ ] 上述测试在现有 parse-tests / src-tauri CI 中运行，不新建平行 runner

**Comments**

- 2026-10-21 两轴 review 残留：Spec 轴指出 archive/set-default 仍按 index；Standards 轴指出缺 command/写失败测试、外部回滚仍有 `let _` 吞错。
- 已修：删除按 card id 清 active+archived+vault+快照/layout、默认接班与折叠、默认历史迁移、归档列表删除、外部 active 删除、并行族折叠保护、i18n 文案。
