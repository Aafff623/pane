# 02: 设置开关 + 三语文案

**What to build:** 设置-通用页新增「隐藏未配置的 Provider」开关（默认开），带说明 tip；所有新文案 en/zh/ru 三语齐备。

**Blocked by:** 01（共用 config 键与判定函数）

**Status:** implemented（2026-10-15，待 owner UI 验收）

- [ ] st-general 页照 silentStart 先例加 panelSwitchRow，变更即 patchConfig
- [ ] i18n 三字典补齐全部新增键（折叠区标题/空状态/开关+tip）
- [ ] tests/i18n-pairing.test.mjs 本地跑过（三字典对齐）
- [ ] tsc + vite build 过
