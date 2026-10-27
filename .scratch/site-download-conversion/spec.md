# Spec: 官网下载转化（平台分流 / 动态版本 / 下载矩阵）

**Status:** ready · **Priority:** P2 · **Area:** Site
**Issue:** https://github.com/Aafff623/pane/issues/85
**Evidence:** temp/research/20261025-website-ui-deepdive.md §一（magpie 平台检测+版本动态+plats 矩阵；Pane data-latest-installer 死属性）

## 问题

Pane 下载区的 `data-latest-installer` 全树无 JS 读取=**死属性**；版本号静态；无平台分流（Windows 用户看到 mac/Linux 卡片平等竞争注意力）。magpie 的正解：UA 判 OS+getHighEntropyValues 修正 arm64+按平台改写所有 dl href 与文案+fetch /api/latest 填版本号。

## 范围（1 票）

1. **平台分流**（magpie M:772-795 语义，原创实现）：UA 判 OS→html[data-os]；arm64 先 UA 猜再 `navigator.getHighEntropyValues(["architecture"])` 修正；按平台重排下载卡（首选平台置顶+CTA 文案「Download for Windows」）。
2. **动态版本号**：fetch `/api/latest`（worker 已有 /api/latest :66-71）填 `.ver`；失败回退静态值。
3. **plats 下载矩阵**（magpie M:229-234）：每平台一张卡=名称+版本要求+架构链接——macOS/Linux 上线后即激活（当前由 site-content-integrity 的 badge 三态兜底）。
4. demo iframe `loading="lazy"`+首屏占位（demo-rest 已隐藏备用）。

## 验收

Windows/macOS UA 下下载卡排序与文案正确（手测+ego 虚拟机）；arm64 修正可测（mock UA）；版本号动态填充；lazy 后首屏不拉 iframe；owner 验收。

## 协调

与 site-content-integrity（静态口径先做，本票做动态）；worker /api/latest 已存在无需新端点；i18n 三语。
