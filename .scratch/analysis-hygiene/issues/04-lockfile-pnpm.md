# 04: 双 lockfile 收一——删 package-lock，CI 切 pnpm

**What to build:** 依赖锁文件回到单一事实源：仓库只保留 pnpm 锁文件，CI 前端门禁改用 pnpm 冻结安装与构建，CI 验证的依赖解析与开发实际运行的完全一致。

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] package-lock.json 从仓库删除（.gitignore 无需新增条目——目标是消灭而非忽略）
- [ ] CI 前端 job 改为 pnpm 安装（frozen-lockfile）+ pnpm 构建（带 pnpm action-setup）
- [ ] 本地 tsc --noEmit + vite build 通过（node 直调路径）
- [ ] yaml 改动经语法自检；实际 CI 运行须 push 后验证（标注未验证）
