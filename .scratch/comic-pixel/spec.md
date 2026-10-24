# Comic Pixel — 漫画×像素皮肤（进应用）

**Status:** P0/P1/P2 均已实现并通过门禁 — owner 实机目检与动效手感待验
**GitHub:** 无（owner 2026-10-23 口头立项；方向由 owner 选定）
**分支:** 建议 `codex/comic-pixel`

## 冷启动纠错（重要，先读这段）

上一段交接说「工作区被删、产物靠日志重放恢复、旧 `skins/comic-desk/skin.css` 全盘无副本」。**这条不成立**：
对象库里存在 `dc520c2`（父 `9e15708`，2026-10-23 21:19，15 文件 +961/−6），不被任何分支包含
（`git branch --contains dc520c2` 为空），但仍被 `refs/codex/snapshots/dd608a9…` 引用，因此完整可读：

- `skins/comic-desk/skin.css` 279 行（黑白朱红旧方向）、`assets/fonts/display.ttf` 93148 B
- `src/comicStartup.ts` 133 行 + `src/comicStartup.css` 46 行
- `tests/comic-skin.test.mjs` 135 行、`scripts/build-comic-skin.mjs` 82 行
- `src/main.ts` +45/−6（`renderComicSample`、`playComicApply`、`data-comic-page`）、`src/i18n.ts` +9、
  `src/skins.css` +4、`src/themeStartup.ts` +2
- `.scratch/comic-desk/{spec.md, issues/01-comic-desk-redesign.md, issues/02-loading-empty-state.md}`

分支 `codex/comic-desk` 确实零提交（reflog 只有 `Created from HEAD`），这半句是真的。
教训写进本票：**「确认丢失 / 无副本」这类否定判断必须自己去 `git cat-file` / `for-each-ref` 复核**，
不能采信日志重放的覆盖率。旧件按只读取用（`git show dc520c2:<path>`），**不 checkout、不合并**。

## 命名与素材决定

- 皮肤 id 用 **`comic-pixel`**，不用 `comic-desk`：旧名字描述的是被 owner 否掉的黑白朱红方向，
  而 id 会进目录名、ZIP 与 manifest，两者混在一个 id 下会分不清。历史票与 spec 里的
  `comic-desk` 措辞保留为旧方向记录。
- 字体**不子集化**：`PressStart2P-Regular.ttf` 118204 B + `Bangers-Regular.ttf` 93148 B 原样打包，
  整包 97947 B（zip 后）。参照系是官方包 `ink.zip` 945416 B、`signal.zip` 1526381 B ——
  新包反而是最小的一个，`MAX_PACK_BYTES` 是 32 MB。做子集化要引新工具链（仓库里没有 woff2/子集化脚本，
  `build-font-packs.py` 的 `FONTS` 是空列表），收益为零。
- 概念稿里的**黄黑警戒带页面外缘装饰不进应用**，也不做开关：380–450px 窄浮窗上它是噪声，
  多一个开关就多一份皮肤 token 面。

## 已完成

- `skins/comic-pixel/skin.css`（257 行、161 规则、55 KB）：按**真实 DOM** 重写选择器，
  原型类名（`.panel` `.seg` `.meter` `.chapter`）一个都没照搬。
  选中态分工落成四类互不共用：统计维度=黄、计量单位=青、时间范围=紫、低额度/错误=红；
  像素体只给纯数字展示位，带本地化词与自适应字号的数值（`.left-val`、`.donut-total`）明确不用。
- token 作用域 `:is(:root[data-skin="comic-pixel"],.skin-sample[data-skin="comic-pixel"])`，
  堵掉旧版「根 token 被 `:root[data-theme=light]` 压过」的坑；`!important` 出现 0 次。
- 挑刺报告逐条对照：错误文本落在 `.placeholder`（`.provider-error` 这个类现在根本不存在）、
  `.toggle input` 是自绘的（只给 `accent-color` 无效）、`.card-account-tab` 补上、
  样张不污染其它皮肤。
- P1：`build-skin-packs.mjs` 注册 `comic-pixel`，本地出 ZIP + manifest（逐文件 sha256），
  `check-skin-packs.mjs`（不带 --live）三个包 size/sha 全 ok；`skin_packs` Rust 测试 9 通过。
- 上架**没做**：`site/public/skins/index.json` 只加了本地生成条目，线上发布是独立授权动作。

## P2（已落地，2026-10-23）

- 工坊三页样张预览（额度/花费/设置真实控件）、`comicStartup` 接入（三分镜→字标→印章，
  3.3s 可跳过、减少动画降级）、分享 PNG 导出回归。起点用 `dc520c2` 的 +45/−6，
  但要重新落到当前 667 行未提交（现已提交）的 `main.ts` 结构上。
- 另有一张独立挂账：首次加载同时显示空态与「去添加 Provider」（`issues/03`），属现存渲染状态逻辑，不是皮肤问题。

## 验证边界（不许越界宣称）

- 皮肤 CSS 的视觉**从未被肉眼验收**：斜切 + 硬阴影在真机 WebView2 的表现、Bangers 在中文标题上回退、
  像素体在 9–11px 的可读性、380px 窄窗下的 `--cp-cut: 6px`，全都需要 owner 桌面上判定。
  截图与选择器对账 ≠ 独立评审 ≠ owner 验收。
- 整包重建会改写 ink/signal 两个已提交 zip（Compress-Archive 内嵌时间戳）。本轮已把它们恢复到提交态、
  并按磁盘实算回写 index 条目，所以本地自洽；下次谁整包重建还会再漂 —— 要不要给脚本加按包过滤，待 owner 定。
