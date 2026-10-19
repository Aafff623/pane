# 03: provider 卡片 + Settings 登录入口 + i18n

**What to build:** `tokenrhythm` family 卡片（余额型：可用/冻结额度，
人民币）+ Settings 行内「网页登录」按钮与登录态指示；三字典 i18n。

**Blocked by:** 02

**Status:** implemented（2026-10-19；设置行「网页登录/重新登录/清除会话」+ 登录态、三语言 i18n、余额型卡片走通用渲染；图标为波形占位标，待换正主标）

## 要点

- family 注册照现有清单（provider_catalog.rs + providerCatalog.ts +
  providerVisuals.ts 图标/配色 + main.ts 渲染分支），补齐全部注册点
- 余额型卡片：显示余额数字 + 指标名，不进百分比环（参考 DeepSeek 余额
  展示口径）
- 「需重新登录」态：明确可操作的设置入口文案
- i18n 三字典（en/zh/ru）键集对齐，i18n-pairing 门禁必须过
- 图标：自绘或复用品牌资产（temp/icon-pack 优先）

## 验收

- [ ] 卡片在总览正确渲染，余额数字与网页控制台一致
- [ ] 重新登录入口可达且文案三语言齐
- [ ] tsc/vite/i18n-pairing/node 测试全绿；UI owner 实机验收
