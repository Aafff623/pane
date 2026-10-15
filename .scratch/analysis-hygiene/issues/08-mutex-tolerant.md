# 08: mutex 中毒容忍写法全树统一

**What to build:** 单个 mutex 意外中毒不再级联毒化整个刷新循环与全部命令入口：全树非测试的 `lock().unwrap()` 统一为既有的容忍写法（中毒时取回内部数据继续），模式收一，无行为变化。

**Blocked by:** None (can start immediately)

**Status:** implemented（2026-10-15，见对应 commit）

- [ ] 全部非测试 `lock().unwrap()` 换为 `unwrap_or_else(|e| e.into_inner())` 容忍写法（以实现时实测清单为准，核实时点为 ~49 处、8+ 文件）
- [ ] 已验证不可达的防御性 unwrap 不动；测试模块内不动
- [ ] 换完后非测试区 grep 该反模式归零
- [ ] harness 全绿；`cargo check` 过
