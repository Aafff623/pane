# Spec: web/ 实验站清理（B 路，owner 决策门）

**Status:** ready · **Priority:** P2 · **Area:** Site
**Issue:** https://github.com/Aafff623/pane/issues/90
**Evidence:** temp/research/20261025-website-reference-sweep.md §一-10 + 20261025-website-ui-deepdive.md（web/ 死产物事实）

## 背景（待 owner 拍板）

web/（Relay 模板改造的实验站）**源码 10-23 被删**（app/src/README/package.json/next.config.ts 全不在，无备份、从未进 git；10-23 14:31 的交接件还写着「web/ 也不要删除」）。现只剩 out/（10-22 构建 19.3MB）+node_modules/.next（约 480MB 尸体）。模板副本在 temp/template-analysis/relay。

**第一决策点（本票执行前必须确认）：10-23 删除是有意还是事故？**
- 有意 → 执行 B 路清理（本票范围）。
- 事故 → 先讨论抢救（从 relay 副本+out 产物重建 ≈2 天重做，且需立即 git add web 防再丢），B 路挂起。

## 范围（B 路，1 票）

1. **提取 demo 产品窗外壳**：从 web/out/index.html 提取 demo 外壳（mono 标题栏+状态+玻璃卡的纯 CSS）移植到 site/public/demo-frame.css（site/ 已有 1KB demo-frame.css——增强而非新建）；这是 web/ 唯一独有的可移植资产。
2. **清理尸体（需 owner 明确授权）**：删 web/ 剩余（node_modules 455MB+.next+out+logs，约 480MB）；保留 temp/template-analysis/relay 与 temp/sites/ 参考。
3. **worker /en 分支记录**：site/worker.js:179-185 的 /en 重写保留（web/ 死后无冲突，记录结论即可）。
4. **git 状态**：web/ 本就未跟踪，清理后 git status 干净；无需提交（temp/ 全树本地）。

## 验收

demo-frame.css 增强落地（owner 视觉验收）；480MB 清理完成（授权后）；relay 模板副本保留；结论记录进 issue 评论。

## 不做

不重建 web/；不切生产站；不删 relay 副本。
