# Pane TODO

<!-- next-task-id: 0020 -->

This file is the canonical board for cross-session task state.

Keep one line per task. Phrase every task as an observable outcome that can be demonstrated when complete. Completed tasks are removed after validation; this board is not a history log.

Handoff parcels in `temp/handoff/` cite task IDs from this board. The board does not depend on local-only handoff files.

## In progress

- [ ] T-0016 — 官网模块二交互演示右侧布局与模块三/四 GSAP 动效恢复：将 Alt+2/Shift 交互卡片重构至面板右侧第 3 列空置区，彻底恢复双向跑马灯与特性卡片循环播放动效
- [ ] T-0019 — 花费面板时段完整性（通用 bug）：今天 / 昨天 / 7 天 / 30 天 / 全部 各时段在「美元」与「tokens」两种口径下都显示非零数据（官网演示数据已修；真机扫描口径待核对）

## Next

- [ ] T-0003 — 注册表收口：`provider_runtime.rs` 单表驱动 refresh，docs 覆盖基线并入一致性门禁
- [ ] T-0004 — 分批接入 23 家新 provider adapter（按 auth taxonomy A→B→C 顺序，逐家实测端点）
- [ ] T-0005 — Analytics 用量面板：先出 `temp/preview/` 原型拍板，再接线（Rust 聚合命令 + 自绘 SVG）
- [ ] T-0006 — CI 门禁补齐：加 PR 触发的 `ci.yml`（前端 build + parse-tests 去代理运行）
- [ ] T-0007 — 把 4 个模块的 84 个「死测试」挂进 parse-tests harness（`lib.rs`/`tray_projection`/`telemetry`/`httpapi`）
- [ ] T-0008 — 收尾欠账：长墙镜像白名单逐家实测扩展、ru 语言 38 键、`docs/providers.md` 16 节
- [ ] T-0009 — Qoder 配额卡：用户完成一次 device-code 登录后按实测响应实现（端点情报已在 handoff）
- [ ] T-0010 — 发布资产与杂项：`install.ps1` 上游指向 6 处、`ROADMAP`/`README` 计数、`stash@{0}` 处置、项目 skill 落位与入库、**装机目录自启 exe 换正式构建**（实测为依赖 Vite 的开发构建——无 Vite 时开机自启只显示报错页；换正式版后走应用内更新，不再卸载重装，并在换装后实测一次应用内更新（v0.5.0 的 latest.json 通道首次可验））

- [ ] T-0012 — 官网模块二重做「使用流程」区块：快捷键（Alt+2/Shift/Ctrl+S/Esc）+ 基本功能（多账号/总花费/额度查询）+ 生态扩展声明（不止 Coding Agent，生产力工具皆可接入）
- [ ] T-0013 — 官网第三模块 provider 图标深浅主题可见性修复（claude/hermes 等暗色看不清；深浅主题下全部可辨，验收双主题逐图肉眼过）
- [ ] T-0014 — 复核官网第六轮成果（艺术字/默认配置卡/字号/keyvault 模拟），按审美与一致性问题重做或收敛
- [ ] T-0015 — 官网首屏深度美化：重新设计艺术字「让每一次调用 / 都看得见」的字形、层次、动效与首屏构图，完成桌面与移动端视觉验收

## Rules

- Task format: `- [ ] T-0001 — <demoable outcome>`
- Optional dependency format: `Blocked by: T-0002`
- Task IDs are immutable and MUST NEVER be reused. `next-task-id` is a monotonically increasing allocator; removing a completed task does not make its ID available again.
- Add or update board state only for changes that matter across sessions: a task is created, enters or leaves `In progress`, materially changes scope, becomes blocked or unblocked, or is validated complete and removed. Do not record agent micro-steps (reading files, planning an edit, partial implementation).
- Remove a completed task only after its acceptance condition is satisfied; if another live task's `Blocked by:` edge points at it, remove that edge in the same board mutation.
- `TODO.md` owns task state. Handoff parcels may explain a task but MUST NOT maintain a second task list or contradict the board.
- Read this file at session start and immediately before any board mutation. All writes follow the TODO optimistic concurrency protocol (see the `handoff-to-agent` Skill); a hash conflict is a reconciliation event, never permission to overwrite newer state.
