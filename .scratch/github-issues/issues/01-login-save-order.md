# 01: login_accounts save_to 事务顺序修复（GitHub #3）

**What to build:** 账号文件写入失败时，已有账号与 Token 保持可用，下次加载不消失；保存成功后，被删除的账号不残留无效凭据。失败注入回归测试覆盖故障路径，不访问用户真实凭据库。

**Blocked by:** None

**Status:** implemented（2026-10-18，77aa007；红绿实测：旧次序 load 返回 1 而非 2）

**来源**：<https://github.com/Aafff623/pane/issues/3>

## 验收标准（issue 原文照抄）

- [x] 使用合成凭据和测试目录复现或排除上述故障路径
- [x] 账号文件保存失败时，已有账号与 Token 保持可用
- [x] 保存成功后，删除的账号不会残留无效凭据
- [x] 补充故障注入回归测试，不访问用户真实凭据库

## 已知事实（已对照 v0.8.6 源码核实）

`src-tauri/src/login_accounts.rs:128-165` `save_to()` 顺序：
1. 逐行 `put_triple` 写 vault，成功的从内存行里清空 token（134-148）；
2. GC：对不在 keep_scopes 的旧行 `remove` vault slot（149-157）；
3. `write_json_atomic` 落盘（158-164）——失败即返回 Err，但 1、2 已生效。

候选方案：(a) 先落 JSON 再 GC（JSON 失败凭据原样保留；需处理「新凭据已写但 JSON 失败」的孤儿凭据）；(b) 三阶段 + journal 可回放。按测试暴露的边界定。

## 验证

- parse-tests harness：`cd parse-tests; $env:PATH = "D:\Tools\mingw64\bin;$env:PATH"; cargo +stable-x86_64-pc-windows-gnu test`（去代理跑：`env -u HTTP_PROXY -u HTTPS_PROXY`）
