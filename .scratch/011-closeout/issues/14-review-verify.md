Status: pending
Type: task
Blocked by: 全部

# 14 · 总验收与 code-review

1. 对 `codex/0.11.1-closeout` 跑 `code-review`；所有发现修复后复跑。
2. `pnpm build` + `cargo check`；`parse-tests` 串行去代理 0 失败；
   `node --test` + `site/site.test.mjs`；demo 重建；live smoke。
3. 生成 owner 验收清单：三皮肤/明暗/380–1440/#27/#29/#58/#60/#61/#84/#87。
4. 清理 worktrees；汇报 integration branch 与未关闭项。

## 验收

测试全绿；review 无未处理项；owner 签字后才进入发布票。
