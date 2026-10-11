# 03 — 用户自定义分组（内置固定两个，其余用户自己建）

**What to build:** 内置分组**固定两个**：`Coding Agent` 与 `MCP`（名字就这两个）。**其余分组由用户自己建**：右键菜单里能新建分组，并把任意 provider 分进去 —— 例如把「闪电说」放进用户自建的「生产力工具」组。新用户装上就只有那两个内置分组。

**Blocked by:** None — can start immediately.

**Status:** needs-triage

- [ ] 内置分组列表固定为 `coding` / `mcp`（已完成：`src/main.ts:192`）
- [ ] 用户可**新建分组**（右键菜单入口 + 命名），存进 config（建议 `config.categories: {id,name}[]`）
- [ ] 分组选择器 = 内置 + 用户自建；provider 可分配到任意一个
- [ ] 用户自建分组可改名/删除（删除时其成员回落到内置默认）
- [ ] 新用户（无任何 provider）看不到任何分组；只有内置两个 provider 时只显示两组
- [ ] **不要改 provider 的目录默认分类**：闪电说应保持自己的分类，归到哪个组由用户决定
- [ ] 分区渲染要能容纳"目录里出现、但不在内置列表里"的分类（见 Comments 的修正项）

## Comments

- 2026-10-11 用户澄清（原话）："别人下载这个安装包，里面默认就只有两个内置分组：Coding Agent 和 MCP，名字就这两个，不要有多余的名字了……咱们当前有个'MCP搜索'，把那个'搜索'去掉。然后作为一个用户，我自己能把'闪电说'分到'生产力工具'这个分组里……想要其他分组自己去右键菜单添加。"
- 已完成部分：内置列表 = 2（`OVERVIEW_CATEGORIES`）；i18n `category.mcp` 三语从 "MCP 搜索 / MCP Search / MCP Поиск" 改为 **"MCP"**；`category.productivity` 三语已删除。
- **⚠️ 本轮的错误改动（需在实现本票时修正）**：把 `shandianshuo` 的目录分类从 `Productivity` 改成了 `Coding`（Rust `provider_catalog.rs` 的枚举去掉了 `Productivity`、`src/providerCatalog.ts` 的类型收窄为 `"coding"|"mcp"`）。用户明确说"你改我这个闪电说的分组干嘛呢" —— 应**恢复** `Productivity`，并让**分区渲染接受目录里出现但不在内置列表里的分类**（新用户没有闪电说 → 只见两组；有闪电说的用户 → 多出「生产力工具」一组）。i18n 的 `category.productivity` 文案也要一并恢复。
- 参考锚点：`OVERVIEW_CATEGORIES` `src/main.ts:192`；`effectiveCategory()` `≈1612`；`setFamilyCategory()` `≈1618`；额度总览分类切换行 `≈4159-4180`；主看板分类分节 `renderGroupedCards()` `≈7919`；右键分类列表 `≈5056`；拖拽分类 `≈4940`；键盘轮换 `≈4877`；**已存在的"分组管理"面板** `≈4611`（`overview.groupBuiltin`，管的是卡片二级分组 `layout.groups`，可作为自定义分组的交互参考）；config 类型 `≈258`（`categoryOverrides`）。
