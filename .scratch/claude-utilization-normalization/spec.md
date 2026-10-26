# Spec: Claude utilization 归一 + 过期窗口置空

**Status:** ready · **Priority:** P1 · **Area:** Correctness
**Issue:** https://github.com/Aafff623/pane/issues/66
**Evidence:** temp/research/20261025-final-sweep-7repos.md §1-1（函数级设计+11 条测试+本机数据实证）

## 问题

CCUM 去重调研实证：本机 `%APPDATA%\Pane\last_snapshots.json` 80 条 progress 行中 **21 条（26%）reset 已过仍显示百分比**（commandcode Weekly 42.14% 过期 16.57 天等）。过期窗口被 pace 读成 "Ahead"（pace.rs:99 remain.max(0)→frac=1.0），被 alerts 误报「将会用完」（alerts.rs:58 left<0.5 判定在过期检查之前）。契约判定：utilization 实为 0-100 percent（本机无 fraction 形态），「≤1×100」仅前向兼容。

## 范围（1 票）

1. claude.rs 新增 `normalize_utilization(raw)`：非有限→None；(0,1) 开区间→×100（1.0=1% 保守判定）；[0,101] 原样；负/>101→None。
2. push_window（:375-380）加 now_ms 参数：resets_at<=now_ms → `Metric::text(label,"Window expired — awaiting refresh")`；归一失败 → text "Usage unavailable"（**不钳 100**）；limits[] 路径（:321-328）补同一过期判断。
3. pace.rs compute（:70-88）在 used>=99.5 短路**之前**插过期→unknown()（无 resets_at 的 100% 保持 Spent）。
4. alerts.rs verdict（:45-73）过期检查移到 left<0.5 之前；evaluate :118 almost_now 加 !expired。
5. parse-tests 11 条（fraction/percent/1.0 边界/过期/负值/>101/limits 过期/pace×2/alerts×2）。

## 验收

- parse-tests 全绿且 pace/alerts 现有测试零回归；`cargo check`+`pnpm build` 绿。
- 本机实证：过期窗口不再显示 pct、不再报「将会用完」；1.0 显示 1%。
- UI owner 验收：刷新前 boot cache 旧值+chip；刷新后 text 行。

## 协调

#55-01 同向（text 行需人工验收一行；备选 progress+used_percent=None 先例 codex.rs:542）；mirror_week_wall 过期 Weekly 降级 text 后镜像自然不触发。存量：下次刷新即覆盖。遗留决策：×100 分支本机无法取证（未登录 claude），默认启用，后续实测可移除（一行）。
