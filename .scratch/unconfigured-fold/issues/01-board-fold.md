# 01: 看板未配置折叠区 + 总览过滤 + 空状态

**What to build:** 打开看板不再被几十张「未配置凭据」卡刷屏：它们收进底部默认收起的折叠区（claude/codex 引导卡除外），总览不再显示「需凭据」条目，一张可见卡都没有时显示带「去添加」按钮的引导空状态。

**Blocked by:** None (can start immediately)

**Status:** implemented（2026-10-15，待 owner UI 验收）

- [ ] 后端 config 新键 hideUnconfigured：config_with_defaults or_insert(true) + CONFIG_KEYS 白名单
- [ ] 看板：isBareHintCard 且非 claude/codex 的卡进「未配置的 Provider（N）」折叠区（默认收起、会话内可展开）；分类计数只数可见卡
- [ ] 总览：同一判定过滤「需凭据」条目；hideUnconfigured=false 时看板与总览均恢复旧行为
- [ ] 空状态：无可见卡时渲染引导块，CTA 打开 Customize
- [ ] tsc + vite build 过
