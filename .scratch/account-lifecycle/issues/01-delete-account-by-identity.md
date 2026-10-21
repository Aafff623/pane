# 01 — 按 card id 删除账号，清掉归档/置顶/快照

**What to build:** 删除 active 或已归档账号时，用稳定 `card id` 找到它，并从 active store、archived store、快照缓存、布局顺序和默认/置顶位置中一次清干净；删除默认账号后，下一个 active 账号自动成为 index 0；卡片和账号胶囊立即消失，旧账号不能从任何路径回来。

**Blocked by:** None（可立即开始）

**Status:** in_progress（store+command+UI 已实现；store 测试绿；command 层测试与 owner 实机待补）

- [ ] 删除 active 账号后，同一 `card id` 的 archived 记录（内部归档与外部归档）一并删除
- [ ] 删除后该 `card id` 不在 active、archived、快照缓存、provider order、per-provider layout 中
- [ ] 删除默认账号（index 0）后，下一个 active 账号成为 index 0，并接管 bare family 卡片
- [ ] 删除非默认账号时，默认账号和剩余账号顺序不变
- [ ] 已归档列表提供删除入口，走同一条按 `card id` 的清理路径；删除后墓碑消失，且不会误删 active
- [ ] 删除时立即移除卡片/胶囊/快照，不等待下一次成功刷新
- [ ] 删除最后一个账号后，provider 卡片显示明确的 no-account / needs-setup 状态，不显示旧账号
- [ ] 当前 active + archived 同 card id 的重复状态可以一次删除清掉；删除旧 MiniMax 账号后只剩新账号
- [ ] 删除确认文案明确说明凭据、历史用量和卡片各自会发生什么
- [ ] store 层单测覆盖：active + archived 重复清理、默认接班、非默认不动、无匹配 no-op
- [ ] command 层测试覆盖：快照/order/layout/usage 清理（CI 的 src-tauri 测试）
- [ ] UI 手工验收：删除后胶囊/卡片立即消失，刷新/重启不回来

**Comments**

- 2026-10-21 owner 报告：MiniMax 归档后删除不干净，重加后出现两个账号；旧账号占 index 0 的置顶位。
- 数据核对：active store 两行，其中旧 card id 同时存在于 archived_accounts；旧行不在 layout order 但仍是 index 0；快照只属于新账号。
