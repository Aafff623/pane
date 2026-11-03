# 02 · 后端：连通性探测 + 导入落盘

**Status:** done（待实机验证） · **依赖:** 01

## 做什么

1. `provider_dispatch::key_snapshot_for` 复用为探测通道：每候选 8 秒超时，状态 `ok` / `needs_setup` / `rejected`（401/凭据类错误）/ `error` / `unverified`。
2. `providers::save_api_key()`：从 `set_api_key` 抽出的共享写入（凭据库优先、失败落明文、base URL 校验），命令与导入共用。
3. 命令：`import_probe(ids)`（并发上限 4，串行保底）、`import_apply(ids)`（跳过 duplicate/unsupported，逐项返回结果）。
4. 安全：不回传密钥；导入后清空本次缓存。

## 验收

- 单测：save_api_key 的空 key 删除、base URL 拒绝、导入结果分支。
- 实机：从本机 cc-switch 导入一条可用 key，卡片能出数。
