# Spec: spend 缓存指纹多点采样升级

**Status:** ready · **Priority:** P1 · **Area:** Spend
**Issue:** https://github.com/Aafff623/pane/issues/102
**Evidence:** temp/research/20261026-closing-audits.md §三（tokscale message_cache.rs:4529-4554 + Pane 自认盲区）

## 问题

Pane 的增量游标校验只哈希 **offset 前 4KiB**（spend.rs:308-316 boundary_hash），且注释自认盲区：「重写前缀但保住尾部 4KiB 会误判为 append」；全量路径仅校 mtime+size（:1241）——mtime 保持的同步工具改写（OneDrive/Dropbox/rsync --times）会漏判，导致**旧解析状态续读新内容=少算/错算**。tokscale 用 5 点×4KiB FNV-1a 全文件采样指纹，前缀重写/中部篡改均可检出。

## 范围（1 票）

1. `boundary_hash(path, end)` 升级为多点采样：`sample_fingerprint(path)`=文件 5 个采样点（0/25%/50%/75%/100%-4KiB）各 4KiB FNV-1a 混合；小于 20KiB 的文件退化为全量哈希。
2. 校验语义：续读条件=offset 前 4KiB 哈希匹配（保留现有续读正确性）**且**全文件采样指纹与上次记录一致（文件身份未变）；指纹不符→该文件全量重解析（替换式，天然幂等）。
3. `FileEntry`/checkpoint 持久化加 `file_fingerprint: Option<u64>`；cursor cache 格式版本+1（旧缓存缺字段=视为不符→首轮全量重解析一次，与 #56 的 v6 迁移同型）。
4. 游标/指纹职责写进注释：光标管续读、指纹管校验（继承关系明确，不与 tokscale 的分片缓存混淆）。

## 验收

- parse-tests 两用例：①「同 size+同 mtime+中部改写」旧实现漏判、新实现重 parse；②「前缀重写保尾 4KiB」旧实现误判 append、新实现重 parse。
- 本机实测：对 spend 覆盖的某个 JSONL 源做一次中部插入/重写，确认下一轮 collect 重解析且总额正确。
- `cargo check`+parse-tests 全绿；首轮迁移一次性全量重解析（发布说明注明）。

## 协调

与 #56（PERSIST_VERSION 迁移机制同型，错峰）；与 #69-WSL（UNC 路径采样成本——UNC 上 5 次读取成本评估，若高则 WSL 源降级为 2 点）；与 #59-03 parser_rev（解析器版本管逻辑变更、指纹管文件变更，两轴独立）。
