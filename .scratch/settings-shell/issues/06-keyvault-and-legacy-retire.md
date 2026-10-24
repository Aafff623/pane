# 06 — 密钥与额度页适配新骨架，并淘汰旧 sheet

Type: task
Status: ready-for-agent
Blocked by: 01, 04

## What to build

这是最大的一页，也是本 spec 的收尾。

1. **密钥与额度页**迁到新骨架：顶部主密码库栏、A–Z 索引、Provider 密钥卡、MCP/搜索密钥卡，保持现有交互（解锁/改密/恢复码/锁定、查看/复制/替换/删除/添加账户、内联 key 编辑器、多账户与归档）。只换外框与行节奏，不改行为。
   - Provider 卡内的行比普通设置行更密，允许一个 `compact` 变体（`py-2`、标签 13、无描述行），但分隔线与圆角规则与全局一致。
   - A–Z 索引栏宽度与 200px 文字侧栏共存时不能互相挤压，窄窗（≤1024）下索引收成悬浮条。
2. **淘汰旧 sheet**：删除 `index.html` 中已无入口的旧设置 DOM（约 `175-394`），删除 `setSettings()` 与 `#settings` 相关 CSS（含 `settings-open` 分支）、删除 `moveRow()` / `moveToCard()` 的搬运路径与 `initSettings()` 里对旧 DOM 的绑定。
3. 删除 03 期保留的旧类名兼容选择器（`#panel-nav`、`.st-card`、`.setting-row` 在 `skins/*/skin.css` 中的旧分支），只留新类名。

## Acceptance

- [ ] 密钥页所有既有交互逐个回归：解锁 → 查看 → 替换 → 添加账户 → 归档 → 删除 → 锁定，无一失效。
- [ ] 多账户、内联编辑器、A–Z 跳转在 980 与 860 两个宽度下都不溢出、不折行错位。
- [ ] 全仓搜索 `setSettings`、`settings-open`、`#settings`、`moveRow`、`moveToCard` 均无残留引用。
- [ ] 旧 sheet 的 i18n key 若不再被引用，在 04/06 结束后统一列出（不立即删，避免漏项）。
- [ ] 四个皮肤 × 明暗下设置面板全页无颜色穿透；至少 native 与 comic-pixel 两套截屏核对。
- [ ] 设置面板打开耗时与改动前相比不劣化（记录改动前后各一次冷启动到可交互时间）。

## Out of scope

- 不改密钥存储、解密、账户模型。
- 不改 provider 列表数据与排序规则。
- 不改 Auth Center 视图。

## Implementation notes (2026-10-24)

- The keyvault page keeps both cards (provider keys + MCP/search keys), the vault bar, recovery panel, provider rows, inline account editor and A–Z rail. The rail now floats at the window's right edge (3px inset, bottom bar under 1100px) instead of sitting under the old 72px rail.
- Legacy sheet retired: the entire `<aside id="settings">` was removed from `index.html`; its controls now live in `src/settings/controls.ts` as an inert hidden source that the panel adopts. `#settings`, `settings-open`, `#settings-close`, `#changelog-btn`, `setSettings()`, `moveRow`/`moveToCard` and the floating-sheet CSS are gone.
- `#settings-toast`/`#settings-keys-rows`/`#settings-scale-status` were renamed to `#panel-toast`/`#panel-keys-rows`/`#scale-status`; only the sidebar `#settings-btn` entry point keeps the old prefix.
- Verified live: no `#settings` element, source is hidden, 67 provider rows render, A–Z C jumps 506px, unlock prompt opens and cancels, inline add-account editor opens and cancels, all pages switch correctly in both the 868px and default viewports.
- Not yet verified: a full unlock → view → replace → archive → delete → lock pass (needs the owner's master password), and the two-skin screenshot.
- i18n audit: no key was deleted; the removed sheet's controls still use their existing keys. A later pass can list keys that only the source (not the panel) references; none are unreferenced yet.
