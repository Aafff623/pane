# 02: 前端镜像 + 图标 + 凭据提示

**What to build:** 前端认识 giphy 家族：目录镜像一致、MCP 分组里卡片有图标、密钥页给出正确的粘贴提示（面板 access_token，不是 api_key）。

**Blocked by:** 01

**Status:** implemented（2026-10-15，commit 7dbfbdd；UI 验收待 owner）

- [ ] `src/providerCatalog.ts` 加镜像条目（category: "mcp"）
- [ ] `src/assets/providers/giphy.svg` 自绘中性图标 + `providerVisuals.ts` 注册
- [ ] `PROVIDER_CRED_INFO` 加 giphy 提示文案（三语键）
- [ ] catalog_consistency 测试过（Rust ↔ TS 双向 + 图标注册）
- [ ] tsc + vite 过；i18n 门禁过
