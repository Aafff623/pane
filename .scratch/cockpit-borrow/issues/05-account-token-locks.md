Status: done
Type: task
Blocked by: #54（refresh-single-flight 落地后）
Priority: P3

# 05 · 账号级 token 刷新锁（双检代际复用）

## 做什么

多账号后台刷新时，token 刷新按账号粒度加锁（进程内注册表 Arc<Mutex>），并用双检代际语义：等锁前记 observed_generation，拿锁后重载账号——generation 已推进且 token 不再过期 → 直接复用他人刷新结果返回（防并发双消费已轮换的 refresh_token）；仍过期 → 告警并继续刷。跨进程文件锁不做（Pane 无共享凭据目录场景）。

## 验收

- 单测：两并发任务同账号 → 只一次真实刷新、后者复用结果；token 仍过期 → 第二次刷新 + 告警。

## 证据

temp/research/20261025-cockpit-deep-dive.md §二-6（codex_account_runtime_switch.rs:268-297 语义）。
