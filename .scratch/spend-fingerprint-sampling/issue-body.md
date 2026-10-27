Status: ready
Issue: https://github.com/Aafff623/pane/issues/102
Priority: P1
Area: Spend

# spend 缓存指纹多点采样升级（堵同步工具改写漏判）

## 问题

Pane 增量游标只校验 offset 前 4KiB 哈希（注释自认「前缀重写保尾 4KiB 会误判为 append」），全量路径仅校 mtime+size——OneDrive/rsync 类同步工具改写文件会漏判，旧状态续读新内容导致少算。tokscale 用 5 点×4KiB FNV-1a 全文件采样指纹。

## 方案（详见 spec.md，证据 temp/research/20261026-closing-audits.md §三）

1. sample_fingerprint=5 采样点各 4KiB FNV-1a 混合（<20KiB 退化全量）。
2. 续读条件=offset 前 4KiB 匹配**且**全文件指纹一致；不符→全量重解析（替换式幂等）。
3. FileEntry/checkpoint 加 file_fingerprint 字段+cache 版本+1（首轮一次性全量重解析，发布说明注明）。
4. UNC（WSL）源评估采样成本，可降级 2 点。

## 验收

parse-tests 两用例（中部改写/前缀重写保尾）；本机实测重解析正确；cargo check+parse-tests 绿。
