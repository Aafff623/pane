# 06 — 托盘 ambient：严重度染色、系统字体与投影缓存

Status: ready-for-agent
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
