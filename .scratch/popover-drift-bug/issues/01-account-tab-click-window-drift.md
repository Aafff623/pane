# 01 — 反复点卡片子账号切换按钮 → 整个窗口持续上移

**What to build:** 修掉这个 bug：在详情卡片里反复点击同一 provider 的子账号切换按钮（如 Codex 的两个子账号），**整个 Pane 窗口会持续向上漂移**，点得越多移得越高。属于用户报告的严重缺陷（虽然触发方式"手闲"，但窗口会跑掉）。

**Blocked by:** None

**Status:** resolved (harness-verified; user UI acceptance pending)

- [x] **Phase 1 回路**：`temp/scripts/restart-pane-dev-cdp.ps1` 让 pane.exe 带 `--remote-debugging-port=9333` 起；回路 = `node temp/scripts/popover-drift-loop.mjs [clicks]`，断言**原生窗口矩形 + `#providers.scrollTop`** 不变（退出码 0/1/2）。灵敏度自证：真移窗口 60px 读数跟着变。
- [x] Phase 2 复现 + 最小化：**不是窗口位移**，是 `#providers` 内容每次点击上滚 ~3px 累积（10 次 +31.3px）；未受信的 `el.click()` 也复现，同按钮连点也复现。
- [x] Phase 3 假设：4 条假设逐条证伪/证实，**结论 = Chromium scroll anchoring**。
- [x] Phase 4 插桩：滚动 API 追踪 0 命中 → rAF 采样抓到 `scrollHeight` 瞬时 −9px → 几何差分定位到 `.overview-item-name` 文本度量抖动。
- [x] Phase 5 修复：`src/styles.css` 的 `#providers` 加 `overflow-anchor: none;`（回归回路先红后绿）。
- [x] Phase 6 清插桩（应用源码零插桩残留）+ 复跑原回路 + 事后总结（见下方 Comments）。
- [ ] **待用户 UI 验收**（见 Comments 末尾清单）

## Comments

- 2026-10-11 用户报："反复点击的时候，整体窗口竟然一直在往上漂移……在同一个 provider 下对着子账号切换按钮不断切来切去，整体窗口就会一直往上移。"要求按 Matt 的 `diagnosing-bugs` skill 处理。
- **已完成 Phase 1 侦察（未建回路）**：
  - **前端没有任何 invoke 会移动窗口** —— 漂移不是前端发起的定位（`grep 'invoke("…position…")' src/main.ts` 为空）。
  - 唯一的窗口移动代码在 Rust：`lib.rs:5025 toggle_popover()`（召唤时定位，`5054 set_position`）、`lib.rs:5068 toggle_popover_centered()`、`lib.rs:5097-5108`（`current_monitor()` + `set_position`）、`lib.rs:4925 .center()`。**下一步优先读 `5097-5108` 那段**：若它是"把窗口夹回屏幕内"的钳位逻辑，且每次渲染/高度变化都跑一次，就可能累积位移。
  - 子账号切换按钮的选择器：`data-card-account="${family}|${id}"`（`src/main.ts:2505`，`card-account-tab`）；点击后走 `renderAll()` 重渲染。
  - 相关：窗口模糊自动隐藏（`lib.rs:5391` 附近）与"最小化僵尸态"历史坑（见 `docs/project-facts.md` 的已知故障模式）。
  - 环境：dev 回路 = `pnpm build` + `temp/scripts/restart-pane-dev.ps1`（Vite :1420 + debug pane.exe :6736）；本机无 MSVC，Rust 走 `cargo +stable-x86_64-pc-windows-gnu` + `PATH=D:\Tools\mingw64\bin`。

- **2026-10-12 根因定位并修复（`diagnosing-bugs` 六阶段全程走完）**

  **结论先说**：漂移**不是窗口位移**，是 **Chromium 的 scroll anchoring（`overflow-anchor`）** 把"每次重渲染的瞬态布局抖动"换算成了**列表滚动位置的永久累积位移**——卡片列表每次点击净上移 ~3 CSS px。修复 = `#providers { overflow-anchor: none; }` 一行 CSS。

  **Phase 1 回路**
  - Pane 的 WebView2 原本没开调试口。新增 `temp/scripts/restart-pane-dev-cdp.ps1`（Vite 仍由官方 `restart-pane-dev.ps1` 起，pane 走 WMI + `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333 --remote-allow-origins=*`）。
  - 回路 = `node temp/scripts/popover-drift-loop.mjs [clicks]`：CDP 驱动 `[data-card-account]`，断言**原生窗口矩形（物理 px，`pane-window-json.ps1`）+ `#providers.scrollTop` + `#trail.scrollTop`** 不变；退出码 0 绿 / 1 红 / 2 跑不起来。
  - 灵敏度自证（不然绿没有意义）：`pane-move-window.ps1 -Dy -60` 真移窗口 → 原生 Top 209→149、页面 `screenY` 186→133，读数跟着动。
  - **教训（本 bug 的关键坑）**：回路第一版每次点击前 `scrollIntoView({block:'center'})`，把目标卡片强行居中，**正好掩盖了漂移**，给出假 GREEN。现在默认不再居中（`SCROLL=1` 才恢复），并在 verdict 里同时看 scrollTop / scrollHeight / trail。

  **Phase 2 复现 + 最小化**
  - 原生窗口矩形全程 `621,209 465x649` 不动、`screenY` 恒 186、窗口尺寸不变 → **排除窗口位移**。
  - 真正在动的是 `#providers.scrollTop`：每次点击 **+2.95~3.54 CSS px**，单调累积（10 次 +31.3px、15 次 +47.8px），被点按钮的 `rect.top` 同步上移 272→227 —— 这正是用户描述的"整体窗口一直往上移"。
  - 最小化：**未受信的 `el.click()`**（不经输入管线、不改变焦点）同样复现；`SAME_TAB=1`（同一按钮连点，选中态不变）同样复现 → 不是"切换账号"引起，而是"**点击 → renderAll**"本身。`scrollHeight` 不变 → 不是内容增长。

  **Phase 3 假设（按证据排序）**
  1. ~~窗口被 Pane 代码移动~~ → **证伪**：全仓仅 `toggle_popover`/`toggle_popover_centered` 会移动主窗，且只挂在托盘点击/托盘菜单/全局快捷键/二次启动；前端无任何窗口几何 API（`setPosition`/`setSize`/`moveTo` 全无）。
  2. ~~账号按钮区是可拖动区（HTCAPTION）→ Windows 模态移动循环~~ → **证伪**：直接发 `WM_NCHITTEST`，账号按钮处返回 **HTCLIENT(1)**；CSS 里也没有 `app-region`。
  3. ~~卡片入场动画 / prefers-reduced-motion~~ → **证伪**：关掉 `#providers>article` 动画仍漂移（17.70px）；模拟 reduce 仍漂移（21.24px）。
  4. **✅ Chromium scroll anchoring（`overflow-anchor`）** → **证实**：`#providers{overflow-anchor:none}` 让漂移**精确归零**；同一页面把它设回 `auto` 立刻恢复 19.47px。
  5. （环境限制，非假设）真·鼠标输入在本机不能用作回路：pane.exe 由 WMI 从 agent 会话启动、**无前台权限**，真·点击会被**覆盖在 popover 之上的 ZCode 窗口**抢走（`WindowFromPoint` 实测命中 `Chrome_WidgetWin_1`，PID = ZCode 自己），popover 随之失焦自动隐藏。故回路以 CDP 合成点击为准（合成点击同样触发真实 handler，且不受 Z 序影响）。

  **Phase 4 机制（插桩全部为 CDP 运行时注入，应用源码零改动）**
  - 滚动 API 追踪（页面内挂钩 `scrollIntoView`/`scrollTo`/`scrollBy`/`scrollTop` setter 并抓调用栈）：点击路径 **0 次 JS 滚动调用** → 是**引擎**自己滚的。
  - rAF 级采样抓到瞬态：渲染过程中 `#providers.scrollHeight` **瞬时 4635→4626（−9px）再回 4635**，而 scrollTop 恰在这一瞬 **+2.95px 并永久保留**（瞬态恢复后位移不撤销）。
  - 几何差分定位到瞬态中缩小的元素：`span.overview-item-name`（含 `.is-note`）**18→14→18**（另有 18→17），多个元素累加 ≈9px —— 整体重建 DOM 后**文本度量/样式尚未稳定**造成的布局抖动。

  **机制一句话**：每次 `renderAll()` 整体重建列表 → 新插入节点第一帧文本度量未稳、`scrollHeight` 瞬时缩 ~9px → **scroll anchoring 对这一瞬态做补偿、把 scrollTop 往下推 ~3px** → 瞬态恢复但位移保留 → 每次点击净上移 ~3px，无限累积。

  **Phase 5 修复（回归回路先红后绿）**
  - 回归 seam = 上述浏览器级回路。修复前 RED：`scrollTop d=31.27`（10 次点击）。
  - 修复：`src/styles.css` 的 `#providers` 加 `overflow-anchor: none;`（带说明注释）。理由：该列表由应用**整体重建**，且应用**自己持久化滚动位置**（`config.mainScrollTop`，实测页面重载后恢复生效）——scroll anchoring 的"保住锚点节点"前提在这里不成立，只会把瞬态换算成永久位移。

  **Phase 6 验证证据**
  - 修复后 `FAMILY=codex` 20 次点击：`scrollTop d=0`、`trail d=0`、原生矩形/尺寸/`screenY` 全不变 → **GREEN**。
  - 修复后 `FAMILY=clinepass`（7 账号）10 次点击：`scrollTop d=0` → **GREEN**。
  - 同页 A/B（`pane-drift-experiments.mjs`，单命令铁证）：as shipped **0.00** / 重开锚定 **19.47** / 重开锚定+关动画 **17.70** / 回到 shipped **0.00**。
  - `tsc` exit 0、`vite build` 成功（本 shell 的 `pnpm` 解析坏，直调 `node node_modules/typescript/bin/tsc` + `node node_modules/vite/bin/vite.js build`）。

  **验收清单（用户亲自做）**
  - [ ] 打开 popover，对着 Codex 两个子账号来回点 20+ 次：卡片列表不应再整体上移。
  - [ ] 确认滚动记忆没回归：滚到列表中段 → Esc 关闭 → Alt+2 重开，应回到原位置。
  - [ ] （可选）换另一个多账号 provider（clinepass 7 卡）重复一次。

  **遗留 / 单独值得记**
  - **仓库没有可提交的浏览器测试 seam**：本 bug 的正确 seam 是浏览器级（CDP），而仓库只有 Rust `parse-tests`。回路与全部取证脚本暂存 `temp/scripts/`（本地、未入库）：`popover-drift-loop.mjs`（回路）、`restart-pane-dev-cdp.ps1` + `pane-cdp-launch.cmd`（带调试口启动）、`pane-window-json.ps1`/`pane-window-rect.ps1`/`pane-move-window.ps1`/`pane-hit-test.ps1`/`pane-real-click.ps1`（原生取证）、`pane-layout-diff.mjs`/`pane-scroll-series.mjs`/`pane-drift-experiments.mjs`（诊断）。若要长期防回归，需要一个可提交的 CDP 冒烟脚本位。
  - **同类风险**：`#trail`、`#drawer-body` 也是"整体重建 + 自管滚动"的容器，理论上同一机制。本次实测 trail 未漂（`d=0`），未改；日后若复现，同样一行 `overflow-anchor: none` 即可。
  - **会话中观测到一次 pane.exe 崩溃**（与本改动无关，**无法确认归因**）：`thread 'main' panicked at tao-0.35.3/src/platform_impl/windows/event_loop/runner.rs:371: cannot move state from Destroyed`，发生在我测试期间；同一时段 Vite 进程也一起消失（页面报 `ERR_CONNECTION_REFUSED`），疑似整机/桌面环境瞬态。重启后一切正常，最终验证是在**重启后的干净实例**上复跑通过的。**待确认**：是否与"二次实例召唤"（single-instance 插件路径）或 CDP `Page.reload` 有关。
  - **未提交**：改动只在工作区（`src/styles.css`，+8 行），等用户验收/授权后再提交。
