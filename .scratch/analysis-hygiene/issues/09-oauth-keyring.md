# 09: OAuth/登录令牌进系统钥匙串（owner 拍板 A 案）

**What to build:** 寿命最长、权限最高的凭证（device-code/PKCE 刷新令牌、登录账号令牌、中转检索 key）不再明文落盘：写入 Windows 凭据管理器，沿用 API key 已验证的三步迁移（先写钥匙串 → 读回确认 → 删明文文件），旧装机明文文件启动后自动迁移删除，全程用户无感。

**Blocked by:** 08（oauth 模块锁点最多，机械统一先行让本票 diff 纯净）

**Status:** implemented（2026-10-15，见对应 commit）

- [ ] 五处明文存储点（oauth 令牌、登录账号、cursor/codex 账号、检索 keys）接入既有 secretstore 钥匙串设施，条目命名沿用 account 作用域语义
- [ ] 读取路径：钥匙串优先，明文文件兜底（迁移前旧态）；写入路径：钥匙串成功且读回确认后才删明文，失败保留原文件并降级不 panic
- [ ] 新增测试覆盖：钥匙串优先级、三步迁移语义、迁移失败保留明文
- [ ] harness 全绿；`cargo check` 过；真实凭据迁移效果留给用户 UI 验收
- [ ] 存储策略写入 SECURITY.md（含「哪些凭证允许明文」的边界声明）
