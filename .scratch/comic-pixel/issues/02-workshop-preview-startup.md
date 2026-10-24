# 02: 工坊预览、开场动画与分享 PNG 接入

**What to build:** 让 `comic-pixel` 在应用里能被预览、被装上、并有专属开场：工坊三页（额度/花费/设置）
真实控件样张、`comicStartup` 三分镜→字标→印章（3.3s 可跳过、减少动画降级）、分享 PNG 导出在该皮肤下回归。

**Blocked by:** 01；以及主对话 `src/main.ts` / `src/i18n.ts` 那批改动落地提交

**Status:** pending-tree（被未提交工作树挡住，本轮按禁改清单没动前端）

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
