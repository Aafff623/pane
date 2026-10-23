# 06 — 托盘 ambient：严重度染色、系统字体与投影缓存

Status: in progress (2026-10-23) — persistent projection, system-font digits and five-tone colors landed; owner scale/taskbar acceptance pending
GitHub: #44
Priority: P2
Blocked by: None
Reference: `temp/repos/codeburn` (`tray_status.rs`, `tray_badge.rs`, `status.json`), `temp/repos/TokenTracker` (`TrayApplicationContext.cs`), `temp/repos/QuotaBar` (`UsageRamp.swift`)

## Scope

Pane 的托盘已有多 provider 数字条、pin/lock、共享时隐藏，这部分比参考仓库更强。本轮只补“第一眼可读性”，不重做托盘：

1. **严重度 tint**：按额度状态（ok/close/maxed/error/stale）给数字或小点染色；颜色与卡片/总览同源，不另造阈值。
2. **系统字体数字**：托盘数字改用系统 UI 字体/更清晰的字号，避免 32px 像素字在 125%/150% 缩放下糊。
3. **投影缓存**：持久化最近一次托盘投影，重启/刷新失败时先显示 last-good；过期要有 stale 标记。
4. **设置开关**：保留现有模式为默认；tint/font 可作为显示选项，默认开启安全的那部分。
5. **验证**：Windows 100%/125%/150% 缩放、深浅色任务栏、多 provider 溢出、单 provider、错误态。

## Acceptance

- [ ] 托盘在 ok/close/maxed/error/stale 五态可区分，且与卡片颜色语义一致。
- [ ] 125%/150% 缩放下数字不糊、不裁切；深浅任务栏都可读。
- [ ] 重启后先显示 last-good，不闪空；过期有 stale 标记。
- [ ] 现有 pin/lock/共享隐藏行为不回归；单 provider/多 provider 溢出正常。
- [ ] 不新增桌面小组件、Dock、宠物、成就、排行榜。
- [ ] `pnpm build` + `parse-tests` 全绿；owner 目检截图。

## Evidence / notes

- codeburn `tray_status.rs` 按严重度染色，`tray_badge.rs` 用 16px 系统字体，`status.json` 有老化；TokenTracker `TrayApplicationContext.cs` 有 Windows 托盘生命周期。
- Pane 的 `tray_projection.rs` 已有多 provider strip；本 ticket 只加 tint/font/cache，不改投影数据结构。
- 颜色必须复用 03 的统一热度函数；如果 03 未落地，先用现有阈值但集中到一个函数，避免第三套。

## Out of scope

- 桌面小组件、Capacity Dock、宠物、成就、排行榜。
- 托盘交互重构、右键菜单大改、自动更新。
- macOS/Linux 托盘重做（保持现有）。

## Verification

```powershell
corepack pnpm build
cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu test
```

Owner 目检：五种状态 × 三种缩放 × 深浅任务栏；重启/断网 last-good。

## Comments

- 2026-10-23：新增 `TraySeverity`（ok/close/maxed/stale/error）到 main tray projection，数字图标按同一 used-percent 热度染色；`LastMainTray` 保存 severity，刷新失败/重绘时先画 last-good。27 个 tray projection 测试通过。系统字体/125–150% 缩放验收和磁盘持久化投影尚未完成。

- 2026-10-23（主对话接手）：`LastMainTray` 增加 `saved_at` 并持久化到 `%APPDATA%\Pane\tray_projection.json`；启动时先读上次投影，直接以数字图标/tooltip 建 tray，不再先闪 logo；超过 15 分钟的旧投影以 `stale` 灰态显示。`apply_main_tray_projection` 成功绘制后原子写回。
- 数字改用系统 sans-serif：font-kit 取字形轮廓，二次/三次贝塞尔展平后在 4x 超采样画布填充，再降采样出抗锯齿覆盖率；黑色描边保持深浅任务栏可读；字体加载/轮廓失败回退原 4x6 像素字。
- 五态颜色：ok 白、close 琥珀、maxed 红、error 橙（与总览错误态一致）、stale 灰；阈值对齐卡片/总览 50/90。
- 测试：`tray_projection` 定向测试通过（含 50/60/92/100 阈值）；`cargo check` 主 crate 通过；`pnpm build` 通过。
- 全量 `parse-tests` 最终：1017 passed / 0 failed / 14 ignored（串行、去代理，444.95s）。

- 未验证/待 owner：100%/125%/150% 缩放、深浅任务栏、单/多 provider 溢出、重启 last-good 与断网 stale 的真实托盘目检；系统字体渲染只过了编译，没有运行时截图。
