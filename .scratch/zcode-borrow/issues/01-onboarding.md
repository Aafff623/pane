Status: done
Type: task
Blocked by: —

# 01 · 首启引导向导（三步）

## 做什么

1. 触发判定：config.json 加 `onboardingSeen/dismissed` 双布尔（沿用 tourSeen 模式）；**判定失败保守不弹**（防误扰老用户）；settings 加「重看引导」入口。
2. 三步向导（全屏覆盖含标题栏、三段进度条+返回+关闭、每步可跳过=显式答案、可返回）：①欢迎+权限/托盘/Alt+2 呼出说明 ②快捷键设置（内嵌现有 recorder，完成即写 config；若票 02 未实施先放只读键位展示）③provider 扫描等待/完成页。
3. 实现：renderOnboarding() 模板函数 + 单个 document keydown capture（Esc 关闭且记 dismissed）；复用 tour 的 overlay 定位与动画基调；三语 i18n；明暗+三皮肤适配。

## 验收

- 首启（清 config）弹向导、中途退出不再弹、settings 重看可进；三步跳过/返回/完成全路径走通。
- owner 视觉验收（对照 ZCode 首启向导截图级观感）。

## 证据

temp/research/20261025-zcode-ui-borrow.md §引导（OccupationOnboarding.tsx:35-419 语义）。
