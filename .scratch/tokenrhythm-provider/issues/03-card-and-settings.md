# 03: provider 卡片 + Settings 登录入口 + i18n

**What to build:** `tokenrhythm` family 卡片（余额型：可用/冻结额度，
人民币）+ Settings 行内「网页登录」按钮与登录态指示；三字典 i18n。

**Blocked by:** 02

**Status:** resolved（2026-10-19；owner 实测验收通过——「基元律动这个我测试没什么问题了」。设置行「网页登录/重新登录/清除会话」+ 登录态、三语言 i18n、余额型卡片走通用渲染；首登实测出「可用额度 ¥142.31 / 冻结额度 ¥0.00」；**品牌标已换成正式「TR」标志**，随主题适配深浅色）

## 要点

- family 注册照现有清单（provider_catalog.rs + providerCatalog.ts +
  providerVisuals.ts 图标/配色 + main.ts 渲染分支），补齐全部注册点
- 余额型卡片：显示余额数字 + 指标名，不进百分比环（参考 DeepSeek 余额
  展示口径）
- 「需重新登录」态：明确可操作的设置入口文案
- i18n 三字典（en/zh/ru）键集对齐，i18n-pairing 门禁必须过
- 图标：自绘或复用品牌资产（temp/icon-pack 优先）

## 验收

- [x] 卡片在总览正确渲染，余额数字与网页控制台一致 —— 2026-10-19 首登实测
  出「可用额度 ¥142.31 / 冻结额度 ¥0.00」，与网页控制台一致
- [x] 重新登录入口可达且文案三语言齐 —— 设置行「网页登录 / 重新登录 / 清除会话」
  三个状态 + 三语言 i18n；按钮处理函数挂在**设置面板窗口自己的 DOM** 上
  （初稿挂在定制抽屉的分发器上——那是另一个窗口，点击落不到任何地方）
- [x] tsc/vite/i18n-pairing/node 测试全绿 —— tsc 无错、51 node、38 site 全绿
  （i18n-pairing 覆盖 en/zh/ru 键集一致）
- [x] **UI owner 实机验收** —— 2026-10-19 owner 实测通过
- [x] **品牌标**：`src/assets/providers/tokenrhythm.svg` 已换成基元律动的正式
  「TR」标志。做法：对 owner 提供的 PNG 做轮廓提取（`cv2.findContours` +
  `approxPolyDP`）得到 3 个多边形，与原图 **IoU 0.9897**（差异仅抗锯齿），
  再归一化到 24×24 viewBox、`fill="currentColor"`，随主题自动适配深浅色
