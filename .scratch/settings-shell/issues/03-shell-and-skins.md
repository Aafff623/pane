# 03 — 外壳改造：无边框窗口、Header、200px 文字侧栏（含皮肤兼容层）

Type: task
Status: ready-for-agent
Blocked by: 01

## What to build

**先做皮肤兼容层，再动外壳**（顺序不能反）。

1. **皮肤兼容层**：`skins/{signal,ink,comic-pixel}/skin.css` 里针对 `#panel-nav`、`.st-card`、`.setting-row`、`#panel-head`、`#panel-scroll` 的选择器，追加新类名选择器（`.set-shell__nav`、`.set-card`、`.set-row`、`.set-shell__head`、`.set-shell__scroll`），并各补一条新强调色覆盖（新骨架的激活态用 `--primary`，皮肤要能改它）。旧类名保留。
2. **窗口**：Rust `panel` 窗口改 `decorations(false)` + `transparent(true)`；默认 980×700、最小 860×620；保留 `panel-window.json` 尺寸记忆。前端在根节点加 5px 留白 + 8px 圆角 + `0 0 5px 1px rgba(0,0,0,.3)` 阴影。
3. **Header**：整条挂拖动区（Tauri `data-tauri-drag-region`），高 56、左右 20；左：齿轮 20 + 标题 16/600；右：02 的搜索框 + 三个 40×40 窗控（最小化/最大化禁用/关闭，关闭键 hover 变红）。
4. **侧栏**：72px 图标栏改为 **200px 文字栏**——`padding 12`、项间距 2、项内 `px-3 py-2.5`、`rounded-md`、图标 18 + 标签 14/500；激活态强调色底 + 白字 + 阴影；未激活 hover 底色 + 轻微右移。窗口宽度 ≤1024 时收窄 170 并隐藏项内描述。
5. 迁移完成后删除 `styles.css` 中旧面板外壳的重复覆盖规则（`9281-9298` 那两处靠后覆盖的标题 11px、卡片圆角 12）。

## Acceptance

- [ ] 设置窗口无系统边框、可拖动、可缩放、不可最大化；缩放后圆角与阴影仍在。
- [ ] 四个皮肤 × 明暗共 8 种组合下，侧栏激活态、卡片底色、分隔线、文字三档颜色都跟着皮肤走，没有原生蓝穿透。
- [ ] `native` 皮肤下与参照物并排比对：Header 56、侧栏 200、内容内边距 24、卡片圆角 12、行高一致。
- [ ] 关闭按钮能关闭窗口；最小化正常；最大化按钮置灰或不显示。
- [ ] 窗口尺寸记忆仍生效（关闭再开恢复上次尺寸）。
- [ ] 旧类名保留、旧 sheet 仍 `display:none`，`initSettings()` 无报错。
- [ ] 无新增全局 `!important`。

## Out of scope

- 不动分区内容（除 01 已迁的常规页）。
- 不做背景图/液态玻璃。
- 不删旧 sheet 的 DOM（06 处理）。

## Implementation notes (2026-10-24)

- Skin compatibility landed before the shell change: signal/ink/comic-pixel each gained new `.set-section__head`, `.set-section__card`, `.set-row`, `.set-shell__nav`, `.set-shell__head` rules and a skin-owned active-state rule; legacy selectors are still present because the panel elements still carry both class sets.
- Window: `panel` is now borderless, transparent, non-maximizable, default 980×700, min 860×620, and still restores `panel-window.json`. The front-end shell paints a 5px frame, 8px radius and the two-stop shadow.
- Header: 56px drag region, 20px gear + 16/600 title, 320px search, 30px save, minimize and close at 40×40 (maximize intentionally hidden because the panel is not maximizable).
- Rail: 200px text rail (170px at ≤1024), 18px icons, 14/500 labels, primary active pill + shadow, hover tint and 2px slide.
- Content: 24px scroll padding, 820px reading column, 12px card radius, 28px section gap. One scoped `#panel-shell [hidden]{display:none!important}` fixes the skeleton's `display:flex` beating the UA hidden rule.
- Verified live: translucent frame, active pill, page switching, search, minimize→reopen, and the narrow 868px CSS viewport at the default Tauri size.
- Pending owner: four skins × light/dark screenshots (at least native + comic-pixel), close-button click, and the side-by-side comparison with the reference.
