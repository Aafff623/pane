# 02 — 归档/恢复的一致性：不留 active + archived 双份

**What to build:** 归档一个账号时，它必须立即从 active 卡片/胶囊消失，同时作为墓碑保留凭据和历史供恢复；归档写 active store、archive store、快照/布局的任一步失败，都不能留下同一个 `card id` 同时在 active 和 archived 的半状态。恢复时回到原 `card id`，不产生第二份；如果它已经 active，明确拒绝。

**Blocked by:** 01（复用按 card id 的 store helper 与归档列表删除入口）

**Status:** in_progress（回滚+默认历史已实现；archive/set-default 仍按 index，见 05）

- [ ] 归档后 active store 不再包含该账号，archived store 包含一条同 card id 的墓碑
- [ ] 归档后卡片、账号胶囊、默认/置顶标记立即移除；归档账号不参与默认/置顶
- [ ] 归档保留凭据与历史用量，可在归档列表恢复
- [ ] 归档任一写入失败时回滚，最终状态要么仍 active，要么已归档，不能双份
- [ ] 恢复后 active store 包含原 card id，archived store 不再包含它
- [ ] 恢复时如果同 card id 已 active，返回清晰的 already-exists 错误，不产生第二份
- [ ] 恢复写入失败时回滚，不留下双份
- [ ] 归档当前默认账号（index 0）后，下一个 active 账号成为默认
- [ ] store 层单测覆盖归档/恢复成功、重复恢复、写入失败回滚
- [ ] command/UI 手工验收：归档后刷新/重启不显示；恢复后原位置/历史回来
