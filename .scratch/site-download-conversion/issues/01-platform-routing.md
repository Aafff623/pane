Status: ready
Type: task
Blocked by: site-content-integrity

# 01 · 平台分流 + 动态版本 + plats 矩阵 + iframe lazy

## 做什么

1. 平台分流（magpie M:772-795 语义原创实现）：UA 判 OS→`html[data-os]`；arm64 先 UA 猜再 getHighEntropyValues(["architecture"]) 修正；下载卡按平台重排+CTA 文案「Download for {OS}」。
2. 版本号：fetch /api/latest（worker :66-71 已有）填 .ver，失败回退静态。
3. plats 下载矩阵（magpie M:229-234：名称+版本要求+架构链接）。
4. demo iframe `loading="lazy"`+首屏占位（demo-rest 备用）。

## 验收

Win/mac 手测排序文案；mock arm64 UA 验证修正；版本动态；lazy 后首屏不拉 iframe；三语；owner 验收。
