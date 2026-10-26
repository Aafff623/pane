Status: done
Type: task
Blocked by: —

# 08 · 授权中心呈现与反馈小件包（五件）

## 做什么（全部 main.ts authRowHtml 区，原创实现）

1. **stale 额度提示**：账号额度刷新失败 → 保留旧值 + 「缓存刷新失败」副语（含重置时间），不显示错误覆盖（platformAccountPresentation.ts:688-707 语义）。
2. **组/行级进行中互斥**：刷新/登录进行中 → 锁该组操作钮 + 行内 spinner（防连点）。
3. **成功反馈自动消散**：授权/刷新成功提示 1.6s 自动收起。
4. **状态聚合 title**：「检查失败/需重登/禁用原因」合并为一条悬停说明（复用现有 network 区分，main.ts:19110 扩成聚合）。
5. **隐私眼睛钮**：一行开关掩码邮箱/ID（复用 maskedKey 链路）。

## 验收

- 五件逐项 owner 场景验收；断网刷新 → stale 提示出现且旧值保留；三皮肤正常。

## 证据

temp/research/20261025-cockpit-multi-account.md §二安全网/§三清单 2-5。
