Status: done
Type: task
Blocked by: —

# 02 · 额度响应身份校验 + 凭据指纹日志

## 做什么

1. 逐 provider 盘点额度响应中的账号标识字段（user/org/account id）；有则 snapshot 写入前与账号库绑定标识比对，不符 → alert（IDENTITY_MISMATCH 语义）而非静默入库；双方都有值才比。
2. 刷新失败日志记凭据指纹：sha256 前 8 位 + 长度 + 过期时间（ISO），不含原文；生产默认不落盘。

## 验收

- parse-tests：标识比对命中/不命中/缺字段三分支。
- 手动验证：换绑后旧 token 拉到的响应触发 alert 而非静默错账。

## 证据

temp/research/20261025-cockpit-deep-dive.md §二-5（validate_usage_identity / token_summary 语义）。
