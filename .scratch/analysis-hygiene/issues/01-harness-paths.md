# 01: parse-tests harness 收录 tray_projection 与 telemetry

**What to build:** 39 个自带测试但从不执行的源模块（托盘投影、遥测）进入 parse-tests harness 的 `#[path]` 编译清单，其既有断言从此在本地 `cargo test` 与 CI 里真实运行——数据损失防线之外的另一批静默死测试复活。

**Blocked by:** None (can start immediately)

**Status:** implemented（2026-10-15，见对应 commit）

- [ ] harness 清单新增 tray_projection 与 telemetry 两个模块的 `#[path]` 条目
- [ ] parse-tests `cargo test`（去代理环境）较 673 基线净增 ≥39 个通过（26+13）
- [ ] 无新增失败/忽略；`cargo check` 过
