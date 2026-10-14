# 字体资源包（按需下载，不占安装包）

**What to build:** 按皮肤资源包的成熟形态做字体包：zip + manifest + 逐文件 sha256，校验通过才落盘，安装后字体出现在字体选择里；设置页给出入口与已装列表。

**Blocked by:** None

**Status:** ready-for-agent

- [ ] 字体包格式与校验复用 skin_packs 的既有模式（不新造轮子）
- [ ] 安装后字体下拉出现该字体并可直接应用；卸载后回落系统字体
- [ ] 损坏/篡改的包整包拒装，不留残骸
- [ ] Rust 侧纯逻辑（manifest 解析/校验/路径）进 parse-tests 并有测试
- [ ] 设置页入口在「常规」或字体行附近，三语文案

## Comments

- 2026-10-14：由 spec `.scratch/release-082/spec.md` 拆分（owner 批次需求）。
