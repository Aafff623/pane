# 05: CI 新增 src-tauri 自身 cargo test job（MSVC）

**What to build:** src-tauri 全部 836 个测试（含命令层 43 个数据损失防线断言）在 CI 拥有持续执行载体：windows runner + MSVC toolchain 的 cargo test job——绕开本机 GNU 链接死因，测试不搬动。

**Blocked by:** 04（同一 ci.yml，串行化编辑避免冲突）

**Status:** implemented（2026-10-15，见对应 commit）

- [ ] ci.yml 新增独立 job：windows-latest + stable MSVC toolchain + rust-cache + src-tauri 目录 cargo test
- [ ] job 与 parse-tests job 相互独立，任一失败都能单独定位
- [ ] yaml 语法与 job 逻辑走查通过；首次真实运行须 push 后验证（标注未验证）
- [ ] 若首跑挂：单独修复本 job，不与其它改动混合
