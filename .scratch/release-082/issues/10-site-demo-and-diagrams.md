# 官网演示与本机一致 + 设计区嵌入架构图

**What to build:** 演示面板由当前前端重建（含本批次新面）；设计区嵌入 archify 生成的自包含精细架构图（深/浅主题可用）。

**Blocked by:** None

**Status:** ready-for-agent

- [ ] site/public/demo 由 build-demo.mjs 从当前源码重建，source.json 版本与 package.json 一致
- [ ] 官网设计区嵌入至少一张 archify 架构图（自包含 HTML），可交互、深浅主题都可用
- [ ] 架构图内容与真实代码结构一致（有源码证据），不臆造模块
- [ ] site 测试全绿

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
