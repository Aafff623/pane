# 01 — 反复点卡片子账号切换按钮 → 整个窗口持续上移

**What to build:** 修掉这个 bug：在详情卡片里反复点击同一 provider 的子账号切换按钮（如 Codex 的两个子账号），**整个 Pane 窗口会持续向上漂移**，点得越多移得越高。属于用户报告的严重缺陷（虽然触发方式"手闲"，但窗口会跑掉）。

**Blocked by:** None — 需要先按 `diagnosing-bugs` 建回路

**Status:** needs-triage

- [ ] **Phase 1 回路**：Pane 的 WebView 没开调试口（9220/9221 是**闪电说**的，不是 Pane）。需用 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` 重启 pane.exe，再用 CDP 脚本：读 `window.screenY` + 卡片 rect → 连点 `[data-card-account]` N 次 → 断言 `screenY` 不变（变红即复现）
- [ ] Phase 2 复现 + 最小化（确认是**窗口位移**还是**内容滚动**，两者读数不同）
- [ ] Phase 3 出 3–5 个可证伪假设并排序
- [ ] Phase 4 单变量插桩（日志打 `[DEBUG-xxxx]` 前缀）
- [ ] Phase 5 先写回归测试再修（若无正确 seam，把"缺 seam"本身作为结论记录）
- [ ] Phase 6 清插桩 + 复跑原回路 + 写清"哪个假设是对的"进 commit

## Comments

- 2026-10-11 用户报："反复点击的时候，整体窗口竟然一直在往上漂移……在同一个 provider 下对着子账号切换按钮不断切来切去，整体窗口就会一直往上移。"要求按 Matt 的 `diagnosing-bugs` skill 处理。
- **已完成 Phase 1 侦察（未建回路）**：
  - **前端没有任何 invoke 会移动窗口** —— 漂移不是前端发起的定位（`grep 'invoke("…position…")' src/main.ts` 为空）。
  - 唯一的窗口移动代码在 Rust：`lib.rs:5025 toggle_popover()`（召唤时定位，`5054 set_position`）、`lib.rs:5068 toggle_popover_centered()`、`lib.rs:5097-5108`（`current_monitor()` + `set_position`）、`lib.rs:4925 .center()`。**下一步优先读 `5097-5108` 那段**：若它是"把窗口夹回屏幕内"的钳位逻辑，且每次渲染/高度变化都跑一次，就可能累积位移。
  - 子账号切换按钮的选择器：`data-card-account="${family}|${id}"`（`src/main.ts:2505`，`card-account-tab`）；点击后走 `renderAll()` 重渲染。
  - 相关：窗口模糊自动隐藏（`lib.rs:5391` 附近）与"最小化僵尸态"历史坑（见 `docs/project-facts.md` 的已知故障模式）。
- 环境：dev 回路 = `pnpm build` + `temp/scripts/restart-pane-dev.ps1`（Vite :1420 + debug pane.exe :6736）；本机无 MSVC，Rust 走 `cargo +stable-x86_64-pc-windows-gnu` + `PATH=D:\Tools\mingw64\bin`。
