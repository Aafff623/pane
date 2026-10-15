# 01: 首启全量检测 + 勾选弹窗 + 落配置

**What to build:** 真·首次打开 Pane 时全量探测本机所有 provider 的凭据，弹出「检测到这些工具」清单让用户勾选，确认后勾中的直接出现在额度总览与看板上；关闭弹窗按默认（检测到的全选）落配置，不二次打扰。

**Blocked by:** None (can start immediately)

**Status:** implemented（2026-10-15，待 owner UI 验收——首启流程需清空 layout 才能复现）

- [ ] 新 config 键 setupPicked：后端 config_with_defaults + CONFIG_KEYS；前端 Config 接口 + FRONTEND_CONFIG_KEYS + 默认对象 ×2 + resetAllSettings
- [ ] 首启全量探测：fetch_usage 只带 removedProviders（不套默认禁用名单）
- [ ] detected 判定（status !== no_credentials）与 disabled 计算抽成纯函数
- [ ] 勾选弹窗（appProviderPicker，沿用 confirm 遮罩样式）：detected 默认全勾 + 未检测到的 Claude/Codex 作建议项
- [ ] 确认/Esc/遮罩关闭三条路径都落配置（disabled + setupPicked + layout 标记）
- [ ] 时序：弹窗完成后再起聚光灯引导；面板窗不参与；探测异常时保留现有 refresh() 首启兜底分支
- [ ] i18n 三语新增键（弹窗标题/说明/确认/空态/建议组）
- [ ] tsc + vite build 过；i18n 门禁过
