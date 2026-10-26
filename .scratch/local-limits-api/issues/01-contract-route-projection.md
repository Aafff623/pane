Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · limits 契约 + 路由 + 投影 + 描述符闸

## 做什么

1. src-tauri/src/limits.rs：信封 serde（四字段+无值省略）+ 投影（progress→consumption：used/limit=100/remaining/utilization/unit="percent"/resetsAt/windowSeconds；带 usage 读数取读数；数值过 valid()）。
2. httpapi.rs：LATEST_LIMITS+publish() 预计算；route() 接 GET /v1/limits 与 /v1/limits/:id（错误码复用；:id=精确卡 ID 或 family ID）。
3. provider_catalog.rs 描述符静态表 LimitsDescriptor{family_id,label,key,kind,unit}+limits_descriptor(family,label) 查询；action/expiry/UI-only 行无描述符不可进。
4. 脱敏：resources 只取结构化值；errors[]=status≠ok 卡（code 取 Snapshot.status）message 过 sanitize_text。
5. parse-tests 8 组；docs/local-http-api.md 更新。

## 验收

8 组全绿；`cargo check` 绿；手动 curl /v1/limits、/v1/limits/onenewapi@<id>、/v1/limits/onenewapi 符合信封；/v1/usage 及既有 :id 断言字节不变。
