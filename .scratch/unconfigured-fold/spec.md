# 看板清爽化：未配置 Provider 折叠 + 空状态引导（unconfigured-fold）

**Status:** implemented（2/2 票落地 codex/unconfigured-fold，2026-10-15；tsc/vite/parse-tests 794:0:13/i18n 门禁全绿；UI 验收待 owner）
**来源**：GitHub issue [UX/UI] 新用户满屏未配置 Provider（v0.8.2 实报）。
**分支**：`codex/unconfigured-fold`。

## Problem Statement

新用户（或从旧版本升级、或历史 config 的 `disabled` 列表为空的用户）打开额度总览时，看板铺满 50+ 张「未配置凭据」死卡：黄色警示图标刷屏、有效信息为零、新用户误以为自己漏配了什么。已有的首启检测（layout 为 null 时自动禁用无凭据者）与默认禁用名单（DEFAULT_DISABLED_PROVIDERS）覆盖不到升级路径——旧 config 已固化空 disabled 列表时 or_insert 永不生效，而补救分支只收「layout 里没有的新 provider」。

## Solution

视图级治理（不动 disabled 数据语义，零迁移风险）：

1. **未配置折叠区**：看板把「未配置凭据」的裸 family 卡收进看板底部一个默认收起的「未配置的 Provider（N）」折叠区；claude/codex 两张引导卡豁免（现有 onboarding 设计不变）。
2. **总览过滤**：额度总览不再显示「需凭据」条目（它们不携带任何数据）。
3. **全局开关（默认开）**：设置-通用加「隐藏未配置的 Provider」开关；关掉则完全恢复旧行为（卡片原地平铺、总览照旧）。
4. **空状态**：看板没有任何可见卡片时，显示引导块（「还没有可用的 Provider」+ 去添加按钮，直达 Customize）。

## User Stories

1. 作为新用户，我打开 Pane 第一眼看到的是干净看板（最多两张引导卡），未配置的几十家收在折叠区，不会误以为自己漏配。
2. 作为升级用户，我的旧 config 不会被迁移改写，但看板同样清爽。
3. 作为想浏览全部可接入服务的用户，我能在折叠区一键展开所有未配置卡，或关掉设置开关恢复平铺。
4. 作为禁用了全部 Provider 的用户，我看到友好的空状态引导而不是空白。
5. 作为俄语/英语用户，新增文案三语齐备（CI 门禁守护）。

## Implementation Decisions

- 新 config 键 `hideUnconfigured`（bool，默认 true）：后端 config_with_defaults or_insert(true) + CONFIG_KEYS 白名单；前端 Config 接口加可选字段。
- 折叠判定复用现有 `isBareHintCard`（裸 family + no_credentials）；claude/codex 豁免常量与首启检测的 starters 一致。
- 折叠区复用 card-group-head/card-group-fold 既有样式与交互形态；展开态为会话内存（不持久化）。
- 分类分节计数只数可见（已配置）卡。
- 总览条目构建处按同一判定过滤（开关关闭时不过滤）。
- 空状态在 renderGroupedCards 无可见卡时渲染，CTA 打开 Customize 抽屉。
- 设置开关照 silentStart 的 panelSwitchRow 先例放进 st-general。

## Testing Decisions

- 前端：tsc --noEmit + vite build。
- i18n 三字典键集对齐由 tests/i18n-pairing.test.mjs CI 门禁守护（新增键漏翻即红）。
- 后端键默认值与白名单走代码走查（parse-tests 不含 lib.rs；MSVC CI job 会跑到 lib.rs 测试）。
- UI 效果由 owner 亲自验收（项目规则），交付时附验收清单。

## Out of Scope

- 不做勾选式新手引导向导（issue 方案 A）——与「claude/codex 卡即引导」的现有设计冲突。
- 不动 disabled 数据语义、不做 config 迁移——视图级方案已覆盖症状，迁移无法区分「故意全启用」。
- 「查询失败」（error/stale）卡不在折叠范围——那是有凭据但查询失败，另一个问题域。
- Customize 抽屉内的展示不变。

## Further Notes

- 根因链：DEFAULT_DISABLED 只在 disabled 键缺失时注入 → 旧版 config 已固化 `disabled: []` 的升级用户全量启用；首启检测只在 layout===null 生效一次。
- renderGroupedCards 分区：先 partition（configured/unconfigured），分类节用 configured，折叠区垫底。
