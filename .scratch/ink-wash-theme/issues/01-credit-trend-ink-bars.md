# 01 — 积分消耗趋势柱形图的水墨风美化

Type: task
Status: ready-for-human
Blocked by: None

## What to build

在现有水墨风 HTML 原型中，美化 provider 卡片里的「积分消耗趋势」柱形图。用户提供的 SenseNova / APIGOTO 卡片截图中，绿色实心柱仍像原生组件，应与纸面、山水、墨色主题统一。

- 柱体采用浓淡墨色、轻微飞白和克制的晕染边缘；亮暗模式分别保持足够对比度。
- 柱高、日期、数值、数据口径及原有交互保持准确；装饰不能改变柱高的视觉判断、遮住刻度或拦截交互。
- 「积分消耗趋势」作为标题使用主题书写字体；provider 名称、核心数值继续使用用户自定义正常字体。
- 先交付 `temp/preview/ink-wash/` HTML 原型，用户亲自验收后再考虑正式主题接入。

## Acceptance

- [x] HTML 原型亮暗模式接入墨色渐变、飞白纹理与轻边缘晕染，移除绿色原生柱体。
- [x] 保留原始柱形坐标、宽高和日期；零值与无数据保留独立透明度；装饰层不拦截命中区域，6 个相关场景 × 亮暗模式无横向溢出。
- [x] 提供浏览器检查证据和亮暗截图。
- [ ] 用户亲自 UI 验收。

## Comments

- 2026-10-11 用户要求加入任务队列。本条已排队，柱形图美化尚未实施；当前继续完成已开始的主题字体调整。
- 参考截图：`C:/Users/Lenovo/AppData/Local/Temp/codex-clipboard-3d83e1a3-8cf1-47f7-aa96-8fc174358d3c.png`。
- 2026-10-11 用户要求立即实施，已完成 HTML 原型。同时补齐额度总览圆环的墨色笔圈与朱红耗尽态。证据：`temp/preview/ink-wash/graph-checks.json`、`v4-trend-ink.png`、`v4-rings-ink.png`。未接入正式应用，状态为等待用户审阅。
- 2026-10-11 用户明确要求明显渐变：柱体改为垂直浓墨到淡墨，透明度从 1 / 0.9 / 0.55 渐变至 0.16；飞白层降为 0.15，避免纹理掩盖浓淡。亮暗模式浏览器检查无横向溢出，最新证据：`temp/preview/ink-wash/v4-trend-gradient.png`、`gradient-checks.json`。
