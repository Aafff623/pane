# 01 — 新用户默认只有两个分组（Coding agent / MCP）

**What to build:** 用户拿到 Pane 时，分类栏不应该出现只有开发者本机才有的分组。新用户的默认分组应当只有 **Coding agent** 和 **MCP** —— 例如「生产力工具」这类分组只在本机存在（因为本机启用了闪电说），新用户没有对应 provider，就不该出现。

**Blocked by:** None — can start immediately.

**Status:** needs-triage

- [ ] 先确认口径（见 Comments 的待确认项）：是"**空分组不显示**"，还是"**从内置分组里彻底去掉 productivity**"
- [ ] 额度总览的分类切换行：只渲染**有 provider 的**分类（现在无条件 map 全部 `OVERVIEW_CATEGORIES`）
- [ ] 主看板的分类分节同样只出非空分类（`renderGroupedCards` 已经是"非空才出 head"，需与切换行口径对齐）
- [ ] 新用户（无任何 provider）不应看到任何分类分组
- [ ] 本机回归：有闪电说时应仍能看到它的分组

## Comments

- 2026-10-11 用户报："用户下载咱们这个的时候，就不应该有我自己本地开发调试的那些分组……他那边默认应该就两个分组：1. Coding agent 2. MCP"。
- 现状锚点：`src/main.ts:192` `OVERVIEW_CATEGORIES = ["coding","productivity","mcp"]`；切换行 `≈4150-4178` 无条件 map 三个；`renderGroupedCards()` `≈7906-7924` 已按非空过滤。分类来源：`providerCatalog.ts` 的 `category`（缺省 = coding），`shandianshuo` = productivity，`bocha/tavily/firecrawl/brave` = mcp。
- 本机 `config.json` 里另有**用户自建卡片分组** `layout.groups = [🥚鸡蛋, 🐑羊毛]`（本地配置，不会随安装包分发）—— 若用户指的是这个，则无需改动代码。
