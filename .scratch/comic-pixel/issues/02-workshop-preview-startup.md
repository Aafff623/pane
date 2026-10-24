# 02: 工坊预览、开场动画与分享 PNG 接入

**What to build:** 让 `comic-pixel` 在应用里能被预览、被装上、并有专属开场：工坊三页（额度/花费/设置）
真实控件样张、`comicStartup` 三分镜→字标→印章（3.3s 可跳过、减少动画降级）、分享 PNG 导出在该皮肤下回归。

**Blocked by:** 01；以及主对话 `src/main.ts` / `src/i18n.ts` 那批改动落地提交

**Status:** done (code + browser fixture) — owner 实机与动效手感待验

## 要点

- 起点是现成的，不用从零想：`git show dc520c2:src/main.ts` 里有 +45/−6 的真实接入
  （`renderComicSample`、`playComicApply`、`data-comic-page`），`src/comicStartup.ts` 133 行 +
  `.css` 46 行也是完整文件；`git show dc520c2:src/i18n.ts` 有 `skin.page.*` 三语 9 行。
  这些要重新落到当前 `main.ts` 的结构上，**不是** cherry-pick 或合并文件。
- 现在 `renderSkinSample()`（`src/main.ts` 里）只渲染花费卡，所以额度条、开关、账号页签
  在皮肤工坊里没有预览位 —— 本票要补的就是这个缺口，否则「安装前看一眼」这件事不成立。
- 样张必须只吃固定演示数据，不读真实账号；且不得污染其它皮肤的 `.skin-sample`。
- 开场接入要尊重既有开关（`settings.startupAnim`、`reduceAnimations`）与
  每开机一次的门禁（`lastStartupBootId`）。

## 验收

- [ ] 工坊三页样张在亮暗、380/450px 下完整可见，像素体与 Bangers 的实际字号可读
- [ ] 应用皮肤 → 开场播放 → 跳过按钮/Escape/点击都能终止；减少动画时退化为静态构图
- [ ] 重启与双窗口下皮肤选择保持；分享 PNG 导出在该皮肤下版式不塌
- [ ] 真实动效速度手感由 owner 本人判定（自动化无法代替；此前两轮独立评审都播不了录像）

## Comments

- 2026-10-23 实现：样张走**能力驱动**（`skinId + page`，三页共用模板），开场**复用** `themeStartup`
  唯一入口而不是并行（并行会双层 overlay），失败返回 `undefined` 让原生 splash 留下；
  `document.fonts` 就绪后再导出分享 PNG。
- 验证：`tsc`/`vite build` 通过；Node 142 通过（新增 13 条）；ego-lite 夹具用「永不返回的 fetch_usage」
  证明 pending 只有 skeleton、无「添加服务」，拒绝态显示「读不到」+ 重试；三页样张的账号页签/
  低额度 `.fill.low`/自绘开关/六段 donut/像素字体在夹具里成立；`signal` 皮肤仍走原开场。
- 未验：真机 WebView2 的分享 PNG 导出、开机门禁、动效手感（需 owner 本人）。
