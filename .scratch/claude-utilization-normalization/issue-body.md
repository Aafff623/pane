Status: ready
Issue: https://github.com/Aafff623/pane/issues/66
Priority: P1
Area: Correctness

# Claude utilization 归一 + 过期窗口置空

## 问题

本机 80 条 progress 行中 26% reset 已过仍显示百分比：pace 把 42% 的过期窗读成 "Ahead"（pace.rs:99 remain.max(0)），alerts 把 99.85% 的过期窗误报「将会用完」（alerts.rs:58 left<0.5 判定在过期检查之前）。CCUM 同端点（api.anthropic.com/api/oauth/usage）做了 utilization 归一（≤1→×100、越界拒绝、过期置空），Pane claude.rs:377 裸透传。

## 方案（详见 spec.md，证据 temp/research/20261025-final-sweep-7repos.md §1-1）

1. `normalize_utilization`：非有限→None；(0,1)→×100（1.0=1% 保守判定）；[0,101] 原样；负/>101→None。
2. push_window/limits 路径：`resets_at<=now_ms` → text 行 "Window expired — awaiting refresh"；归一失败 → text "Usage unavailable"（不钳 100）。
3. pace.rs：used>=99.5 短路前插过期→unknown()。alerts.rs：verdict 过期检查前置 + almost_now 加 !expired。
4. 11 条 parse-tests（fraction/percent/边界/过期/负值/>101/pace/alerts）。

## 验收

parse-tests 全绿+零回归；`cargo check`/`pnpm build` 绿；本机过期窗口不再显示 pct 不再误报；owner UI 验收。半天工时。

## 不做

不动全局 clamp；不碰其他 provider 解析（共享层 >101 护栏另票 spend-parsing-hardening）；不对齐 CCUM 的 5h 窗口本地日志推导（Pane 用官方端点）。
