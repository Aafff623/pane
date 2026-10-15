# 02: 重新检测入口

**What to build:** 设置 → 常规加入「重新检测本机工具」入口，点开重跑全量探测并弹出同一勾选清单——用户后来装了新工具时不用去定制面板一家家找。

**Blocked by:** 01（复用同一探测与弹窗函数）

**Status:** implemented（2026-10-15，待 owner UI 验收——首启流程需清空 layout 才能复现）

- [ ] 设置 → 常规加行（照 silentStart / hideUnconfigured 先例）
- [ ] 点击重跑全量探测 + 打开同一勾选弹窗；落配置路径与首启一致
- [ ] i18n 三语键；tsc + vite build 过
