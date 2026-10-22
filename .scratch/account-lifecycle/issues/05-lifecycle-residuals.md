# 05 — 生命周期残留：身份化 archive/set-default、command 测试、外部回滚

**What to build:** 把 review 发现但未收口的生命周期项补齐：归档/置顶也按稳定 card id 解析；命令层覆盖快照/order/layout/usage 清理与写失败回滚；外部（Antigravity/Cursor）归档回滚不误删既有墓碑；确认删最后一个账号的 no-account 状态。

**Blocked by:** 01, 02

**Status:** ready-for-agent

- [x] `account_archive` / `account_set_default` 的 UI 调用改为按 card id 解析；行号过期时不会操作错账号
- [ ] command 层测试覆盖：删除/归档后快照、provider order、per-provider layout、usage history 的清理与默认接班
- [x] 写失败注入覆盖：archive/restore 回滚不会留下 active + archived 双份，也不会误删原有墓碑
- [x] Antigravity/Cursor 外部归档回滚同样不误删既有墓碑
- [ ] 删除最后一个账号后显示明确的 no-account / needs-setup，不依赖旧快照兜底
- [ ] 上述测试在现有 parse-tests / src-tauri CI 中运行，不新建平行 runner

**Comments**

- 2026-10-21 两轴 review 残留：Spec 轴指出 archive/set-default 仍按 index；Standards 轴指出缺 command/写失败测试、外部回滚仍有 `let _` 吞错。
- 已修：删除按 card id 清 active+archived+vault+快照/layout、默认接班与折叠、默认历史迁移、归档列表删除、外部 active 删除、并行族折叠保护、i18n 文案。

- 2026-10-21 本地实现更新：stable-id archive/default、external vault 归档事务、Cursor vault GC 顺序、login store 删除/归档/恢复、只读身份预览与旧查询过滤已完成。parse-tests 927 passed / 14 ignored；详见 [本轮结果](../../panel-ui-20261021/RESULT.md)。完整 Tauri command/IPC 联动和真实最后账号 UI 仍待 owner 验收，未将上方两项虚记为通过。

- 2026-10-22 补充收口：Codex 取消/回调、直接多账号持久化、刷新版本保护和写失败 vault 保留均有回归；934 passed / 14 ignored，前端71 passed。真实UI和完整command/IPC联动仍不以持久化核心测试替代，保留上述未勾选项。Owner已明确授权本地commit，release/tag仍待本人双账号验收。
