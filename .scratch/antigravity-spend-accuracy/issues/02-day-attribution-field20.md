Status: implemented (2026-10-25) - automated green; owner acceptance pending
Type: task
Blocked by: —

# 02 · 归日修掉同 idx 假设（field-20 跳转 + 邻居插值）

## 改哪里

`src-tauri/src/spend.rs` L6472-6489（建 map 处）、L6505（归日查询处）。

## 做什么

1. 建 map 改为：遍历 steps，解析 metadata 的 field 20 → field 3 = gen idx，配该 step 的 ts，建 `gen_idx → secs`。
2. L6505 归日改查新 map。
3. map 未覆盖的 gen（实测 94/8865）：邻居插值（前一个已引用 gen 的 ts，无则后一个）；仍无才走 summaries last_modified → db mtime → now。
4. 保留内嵌 ts（field 9→4→1）为第一优先（实测 0% 存在，前向兼容）。

## 验收

- parse-tests：3 step（ts=T、f20=1）+ gen idx=1，且 `steps[1]` ts 明显不同 → 归日取 T；gen 无 f20 → 邻居插值。
- 本机实测脚本重跑：field-20 覆盖率 ≥98.9%；UTC 日错配率从 ~11% 降到接近 0。
- 降级路径：field 20 不存在时（未来版本）map 建空 → 全库退化插值/粗锚，不崩。

## 参考

`temp/cc-switch/src-tauri/src/services/session_usage_agy/native_models.rs`（field 20 跳转的活实现；MIT 可借鉴，抄了带声明）。

## Comments
- 2026-10-25：已实现。steps 扫描改读 field 20→3 建 gen→ts 跳表（BTreeMap），归日查表 + 邻居插值 + 粗锚兜底；本机 103 库 8909 行实测：精确覆盖 98.9%、含插值 99.9%；旧同 idx 关联 100% 时间戳不符、11.0% UTC 日错配随之消除。
