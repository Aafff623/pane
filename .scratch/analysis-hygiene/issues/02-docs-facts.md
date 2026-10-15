# 02: 治理与隐私文档对齐事实

**What to build:** 修掉两处持续误导读者的事实失真：项目地图对前端单文件体量的描述不再带 3 倍失真的行数；隐私文档对「遥测实现在哪可审计」的描述覆盖真实的两处实现，并披露每日汇总的有损性。

**Blocked by:** None (can start immediately)

**Status:** implemented（2026-10-15，见对应 commit）

- [ ] AGENTS.md 项目地图中 main.ts 条目不再含具体行数（改相对描述）
- [ ] privacy.md 遥测审计入口从「单一文件」改为指向 telemetry 模块与命令层 payload 构造两处
- [ ] privacy.md 披露 provider 每日汇总发送失败即丢弃的有损性
- [ ] 纯文档改动，无代码变更
