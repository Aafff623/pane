# 03: 两处手搓 RNG 换 OsRng

**What to build:** 登录流 PKCE verifier 与遥测匿名 UUID 的随机源从手搓的时种/哈希种子推导换成操作系统级随机源，随机熵回到 OS 级；与实现相反的两条注释（「无新依赖」「rand 不在依赖树」）随实现一并删除。

**Blocked by:** 01（telemetry 模块测试进 harness 后，其 13 个断言可覆盖本次改动）

**Status:** implemented（2026-10-15，见对应 commit）

- [ ] cursor 登录流的随机字节生成改用 rand_core OsRng，xorshift 展开实现删除
- [ ] 遥测 UUID 的随机填充改用 OsRng，splitmix64 实现与「rand isn't in the tree」注释删除
- [ ] harness `cargo test` 全绿（含新复活的 telemetry 测试）
- [ ] `cargo check` 过
