# 04 分享卡端到端验收

**Status:** needs-info  
**Blocked by:** 01, 02, 03  
**关联:** #106，`../spec.md`

## What to build

在 Windows Tauri/WebView2 验证从总花费入口到选范围、选风格、预览、复制和保存的完整流程，记录客观证据并交 owner 目检。

## Acceptance criteria

- [ ] 七种风格与五个范围核对预览、剪贴板 PNG 和保存 PNG；100% / 125% 缩放、2x 输出无裁切、模糊、错字或缺图。
- [ ] 测试空范围、长模型名、时间边界、不同单位及估算标记；与面板来源和合计一致。
- [ ] 检查最终 SVG/PNG 不含 API key、邮箱、账号 ID；provider 卡即时复制、Esc、错误与回退不回归。
- [ ] 记录 `pnpm build`、相关测试和实际环境结果；交 owner 逐套检查，记录其验收结果；未获授权不提交、推送或发布。

## Comments

- 2026-10-27：依赖实现、原型素材和 owner 视觉验收；静态检查不能替代真实桌面验证。
