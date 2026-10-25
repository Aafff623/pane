Status: implemented (2026-10-25) - automated green; owner acceptance pending
Type: task
Blocked by: —

# 01 · model 解析改 19 优先 / 21 兜底 + 账本行规则

## 改哪里

`src-tauri/src/spend.rs` L6208-6260 `decode_antigravity_call`、L6498 附近丢行处。

## 做什么

1. 解析顺序：field 19 trim 后非空、非 `"unknown"`、不以 `-default` 结尾 → 用之；否则 field 21（仅 wire type 2，坏线型忽略）；两者皆无 → `None`。
2. `AntigravityCall.model` 改 `Option<String>`，连带调用方（add_event / probe_lookup 分支）适配。
3. `-tiered` 后缀在 `probe_lookup` 前剥离（openusage 口径）；`-n` 不剥离（pricing.rs:907 既定决策）。
4. 丢行规则：无 model 且 usage field 2/3/5 全 0 → 丢（账本行）；无 model 但有生成 token → 保留 + unknown 占位走 `note_unpriced`。

## 验收

- parse-tests：仅 21 无 19 → 取 21；19+21 → 取 19；19=unknown/-default → 取 21；21 varint 线型 → 不隐藏 19；无 model 仅 field 1 → None；无 model 有 output → Some(unknown)。
- 本机实测回归：AG 总 token 不低于修复前（48 行账本行的行为不变）。

## 注意

21 的语义在两参考实现中矛盾（分档 ID vs 展示 label），本机 0 次出现无法仲裁——本实现只做前向兼容，取到值进 unpriced 也不算错钱。不要为 21 新增特殊计价逻辑。

## Comments
- 2026-10-25：已实现。19 优先（非空/非 unknown/非 -default）+ 21 兜底（仅 length-delimited 线型）+ 皆无 → None；-tiered 剥离、-n 保留。单测覆盖并存/仅 21/占位拼写/varint 不遮挡。
