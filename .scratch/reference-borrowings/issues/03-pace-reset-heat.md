# 03 — Pace 四态、重置事件与统一热度色

Status: ready-for-agent
GitHub: #41
Priority: P1
Blocked by: None（01 的 `pace_capability` 可后置）
Reference: `temp/repos/QuotaBar` (`Pace.swift`, `Resets.swift`, `UsageRamp.swift`, `QuotaComponents.swift`), `temp/repos/Claude-Code-Usage-Monitor` (`snapshots.py`, `official.py`)

## Scope

Pane 的 `alerts.rs` 已有线性投影，但只服务通知；卡片/总览看不到 pace，重置没有事件检测，热度色有三套阈值（卡片 60/75、总览 80、告警 90）。本轮把“预测”变成可见、可机读的事实。

1. **`pace.rs`**：
   - `PaceState { ahead, close, over, spent, unknown }`。
   - 输入：`used`、`limit`、`window_start`、`resets_at`、`now`；输出：`projected_used`、`run_out_at`、`state`、`settled`。
   - settled 门：窗口已过 ≥5% 且 ≥15 分钟；年轻窗口返回 `unknown`，不报 Close/RunOut。
   - `run_out_at` 只在“按当前均速会在重置前用完”时给；`used ≥99.5%` 直接 `spent`。
2. **`resets.rs`**：
   - 重置检测：两次读数跌幅 ≥10 点，且 `resets_at` 已过或前移 ≥10 分钟/窗口 1/10。
   - 账号/凭证/来源变化直接否决，避免换号/充值误判。
   - 到点后 20 秒宽限触发单 provider 刷新；重置后 10 分钟挂 `just_reset`。
3. **统一热度**：一个函数、固定锚点（如 50–90% 已用），卡片/总览/API 共用；告警阈值独立，不再各写一套。
4. **数据/API**：`Metric`/`Snapshot` 增 `pace`/`reset` 字段（serde default）；`/v1/usage` 暴露；MCP `get_usage` 可读。
5. **UI**：进度条画 even-pace 竖线；hover 解释“窗口已过 X%、已用 Y%、按均速到重置约 Z%（估算）”；just-reset 标记。

## Acceptance

- [ ] 同一 provider 的 pace 数字在卡片、总览、`/v1/usage` 三处一致。
- [ ] 窗口开始 5 分钟内/15 分钟内不报 Close/RunOut；`unknown` 有明确文案。
- [ ] 重置检测测试：正常重置、换号、充值、重置时间抖动、读数下跌但未重置。
- [ ] just-reset 只出现 10 分钟；到点刷新只打一次，不循环。
- [ ] 热度色只有一个来源；告警阈值仍可独立配置。
- [ ] `alerts.rs` 现有通知行为不回归；`parse-tests` + `pnpm build` 全绿。

## Evidence / notes

- QuotaBar `Pace.swift`：`P≤90 ahead`、`≤99 close`、`>99 over`、`used≥99.5 spent`；要求窗口已过 5% 且 ≥900s。
- Pane `alerts.rs`：`frac=(period-max(reset-now,0))/period`；`projected=used/frac`；缺 reset 是否落在窗口内的校验，重置过期会被钳成 `frac=1`，可能把旧读数报成 Close/Ok。本 ticket 修这个边界。
- Claude monitor 的 official TTL/置信标签可在 05 复用；本轮只借“未知/过期不当实测”的原则，不借固定 5h/P90 猜限额。

## Out of scope

- iCloud/CloudKit、Quota Run 中继、端到端加密。
- 菜单栏/刘海/屏幕边缘多表面（Pane 已有 tray/popover/panel）。
- 从日志反推官方套餐限额（Pane 信任厂商百分比）。

## Verification

```powershell
corepack pnpm build
cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu test
```

Owner UI 目检：年轻窗口、临近重置、刚重置、耗尽四种状态；确认 tick/tooltip 不误导。
