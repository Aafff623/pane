# Pane TODO

<!-- next-task-id: 0027 -->

This file is the canonical board for cross-session task state.

Keep one line per task. Phrase every task as an observable outcome that can be demonstrated when complete. Completed tasks are removed after validation; this board is not a history log.

Handoff parcels in `temp/handoff/` cite task IDs from this board. The board does not depend on local-only handoff files.

## In progress

- [ ] T-0026 — Firecrawl 等文字余额 Provider 在总览条形/圆环及悬停中展示真实余额；可靠百分比保持实填，分母未知时用中性斜纹条/虚线圆环和 ? 标记；多 Key 分别显示余额与日期，不累加共享余额；构建、选择/真实渲染器回归、浏览器图形检查及规范重启通过，待用户原生 UI 验收（证据 temp/reports/firecrawl-overview-text-20261010.md）

- [ ] T-0025 — 悬浮看板字号按真实行宽及各标题剩余空间独立放大/缩小，随窗口、重绘及字体加载更新；总览百分比/时间/胶囊同步适配，详情 provider 图标内缩 6 px；前端构建、320/450/800 px 浏览器布局检查及规范重启通过，待用户原生 UI 验收（证据 temp/reports/responsive-typography-20261010.md）

- [ ] T-0024 — 开发弹窗黑屏修复：启动脚本按 -Id 正确替换旧 Vite 并检查主脚本/样式；收窄 Vite 扫描与监听目录，加入可见加载及重载提示；前端构建、资源 HTTP 200、实际进程替换及浏览器可见性检查通过，规范重启后待用户原生 UI 验收（证据 temp/reports/black-screen-recovery-20261010.md）

- [ ] T-0023 — 全部 65 家 Provider 查询入口与额度机制审计已落地：移除已确认的缺用量补 0/固定套餐上限/历史余额推额度；修复 Firecrawl/Tavily 范围、OpenRouter 周期与 Brave 凭据缓存；前端/Rust 构建、609 项测试（新增 19）、规范重启及本机 Firecrawl/Tavily/Brave 等接口核验通过；完整边界见 temp/reports/provider-quota-mechanism-audit-20261010.md，待用户 UI 验收
- [ ] T-0020 — MCP/搜索多Key层级管理与卡片排版修复（Firecrawl多Key并列显示与全量额度查询，解决趋势图穿插），及Codex CLI网络重试与错误态诊断
- [ ] T-0021 — Brave 使用「已用 X / Y credits」，Bocha ¥ 余额不动；Firecrawl 按真实余额与重置日期展示，不将 plan credits 当总额度或倒推已用（含额外额度、团队共享场景）；纳入 T-0023 的统一机制修复，待用户 UI 验收
- [ ] T-0022 — 已备份删除保险库指定 3 条 Firecrawl；按用户追加要求将 fc-06e…5f32 加密入库（6→7），设置页可管理，ZCode CLI 副本保留并按值去重，fc-922…839c 环境变量保留；卡片仍为 2 把不同 Key，待用户设置页/UI 验收
- [ ] T-0016 — 官网模块二交互演示右侧布局与模块三/四 GSAP 动效恢复：将 Alt+2/Shift 交互卡片重构至面板右侧第 3 列空置区，彻底恢复双向跑马灯与特性卡片循环播放动效
- [ ] T-0019 — 花费面板时段完整性（通用 bug）：今天 / 昨天 / 7 天 / 30 天 / 全部 各时段在「美元」与「tokens」两种口径下都显示非零数据（官网演示数据已修；真机扫描口径待核对）

## Next

- [ ] T-0003 — 注册表收口：`provider_runtime.rs` 单表驱动 refresh，docs 覆盖基线并入一致性门禁
- [ ] T-0004 — Provider 全量补全（5 批）：Batch 1-4 已提交（共 23 家：10 还原 + MiMo/Trae国际/Qoder国际/Zed + Droid/JetBrains/Groq/HuggingFace/LongCat/sub2api/Mistral/Perplexity + 火山方舟），测试 489→550；**未落地项**：千帆/腾讯TokenHub/华为云/讯飞/360 的额度端点在全部参考库中不存在（需装机抓包，不编造）；CodeBuddy/腾讯需 device-code 登录流（T-0009 型）；通义灵码无现成实现需自研抓包；Batch 5 长尾等用户点单。真实出数验证仍需用户提供各家的 key
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
