Status: ready
Type: task
Blocked by: —

# 01 · 多点采样指纹 + 校验语义 + cache 版本迁移

## 做什么

1. spend.rs：`sample_fingerprint(path)`（5 点×4KiB FNV-1a，<20KiB 退化全量）；续读条件加「全文件指纹一致」；指纹不符→全量重解析。
2. FileEntry/checkpoint 持久化 `file_fingerprint: Option<u64>`；cache 版本+1（旧缓存缺字段→首轮全量重解析一次）。
3. UNC 源采样成本评估（WSL 上线后可降级 2 点）；注释写明「光标管续读、指纹管校验」。

## 验收

parse-tests 两用例（中部改写漏判修复/前缀重写保尾误判修复）；本机实测中部插入后重解析且总额正确；`cargo check`+parse-tests 全绿；发布说明注明一次性迁移。
