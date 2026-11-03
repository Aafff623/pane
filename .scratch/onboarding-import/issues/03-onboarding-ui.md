# 03 · 前端：引导第 3 步 + 设置入口

**Status:** done（待实机验证） · **依赖:** 01、02

## 做什么

1. onboarding 由 4 步扩为 5 步：欢迎 → 检测本机 → **从其他工具导入** → 连接确认 → 快捷键；新步骤在进入时调用 `import_scan`，随后后台 `import_probe` 更新每行状态。
2. 行 UI：勾选框 + provider 名 + family 图标/名 + 掩码 key + 主机 + 状态（可用/密钥被拒/需补充/未检测/已存在）；`ok` 默认勾选，`unverified` 可手动勾，`duplicate`/`unsupported` 禁用。
3. 底部「重新扫描」；无候选项时显示空态。
4. 完成/下一步时 `import_apply` 已勾选项，结果写入 toast/状态。
5. 设置 → 常规 → 来源扫描行新增「从其他工具导入…」，复用同一列表。
6. i18n en / zh / ru 三语齐备（i18n-pairing 测试守护）。

## 验收

- `pnpm build` 通过；三语 key 对齐。
- owner 实机：首启新步骤、勾选导入、设置入口、空态与失败态。
