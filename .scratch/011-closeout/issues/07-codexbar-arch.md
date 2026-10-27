Status: pending
Type: task
Blocked by: 06

# 07 · CodexBar 架构八票（#65）

1. **01/03 基线**：Rust 侧 gatekeeper 基线/记账；enumerate 数据表豁免；
   JS changed-lines gate 与 Rust 规则不打架。
2. **02 triage**：对现有共享文件做一次分类清单，标出合法 provider 特判。
3. **04 FetchStrategy**：把“代理→直连→缓存回退”从各 provider 提成流水线；
   错误分类统一（auth/rate/server/network/other）。
4. **05 capability typing**：`PaceRule`、`MenuBarMetrics`、capability 表 typed；
   与 #104-8 共用。
5. **06 attempts 审计**：每次刷新的尝试路径/耗时/结果可诊断，不落遥测。
6. **07 candidate families**：候选 family 归一（多账号/别名/历史 id）。
7. **08 窗口前缀/离线回退**：窗口标签前缀归一；离线时保留 last-good 并
   标 stale，不造新值。

## 核实（2026-10-27，逐行对代码）

本票不是零起点，之前说「未开始」不准确：

- **#65-04 FetchStrategy 有骨架但没有接线。** `src-tauri/src/fetch_strategy.rs`
  （142 行）已经有 `FetchFailureKind` / `FetchFailure`（含 retry_after、
  is_transient）/ `retry_once_allowed` / `trait FetchStrategy`，并且
  `lib.rs:6` 声明了 `mod fetch_strategy;`。但 `providers/mod.rs` 里**没有任何引用**，
  它自己的文档也写着「The app still calls providers directly」——所以这是一个
  准备好了但没被使用的接口，不是劳动成果为零。
- **其余七票未见实现痕迹**：没找到 capability 表（`providers/mod.rs` 里没有
  capability 相关定义）、没有每源 attempts 审计、没有 candidate family 归一、
  没有窗口前缀归一/离线 stale 标记、没有 gatekeeper 基线、没有共享文件分类清单。

所以本票的真实进度是「1/8 有骨架、0/8 接入」。

## 验收

- 每票专项测试；gatekeeper CI 绿；共享文件新增特判必须委托或标记理由。
- 行为对现有 provider 输出字节不变（除新增诊断字段）。
