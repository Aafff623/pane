Status: ready
Issue: https://github.com/Aafff623/pane/issues/85
Priority: P2
Area: Site

# 官网下载转化：UA+高熵平台分流 / 动态版本 / plats 矩阵 / iframe lazy

## 问题

`data-latest-installer` 全树无 JS 读取=死属性；版本号静态；无平台分流。magpie 正解=UA+高熵双保险分流+/api/latest 动态版本+plats 矩阵。

## 方案（详见 spec.md，证据 temp/research/20261025-website-ui-deepdive.md §一）

1. 平台分流：UA 判 OS→html[data-os]；arm64 高熵修正；首选平台置顶+CTA 文案按平台变。
2. 版本号 fetch /api/latest 动态填充（失败回退静态）。
3. plats 下载矩阵（macOS/Linux 上线即激活，当前 badge 兜底）。
4. demo iframe lazy+占位。

## 验收

Win/mac UA 排序文案正确；arm64 可测；版本动态；lazy 生效；owner 验收。
