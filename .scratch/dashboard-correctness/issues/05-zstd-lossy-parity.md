# 05 — zstd 文件容错口径与普通路径统一

**What to build:** 让压缩态的会话日志不再因为**一个坏字节**就整文件算 0，并让「读不出来」在界面上
有别于「这一天真的是 0」。
**Blocked by:** 无（与 04 共用夹具更省事）
**Status:** ready-for-agent
**GitHub:** [#48](https://github.com/Aafff623/pane/issues/48) §5 · **Priority:** P2 · **量:** 小

## 要点

1. **同一份数据两种容错口径。**

   | 路径 | 实现 | 后果 |
   |---|---|---|
   | 普通文件 | `String::from_utf8_lossy`（`spend.rs:1063`）+ 单行解析失败跳过 + 游标只在完整行推进（`:1060-1062`） | 坏一行丢一行 |
   | zstd 文件 | `zstd::stream::decode_all(&raw[..]).ok()?` + 严格 `String::from_utf8(bytes).ok()?`（`:1076-1078`） | **任一失败整文件 `None` = 0 记录** |

   受影响的是 Codex 的 `.jsonl.zst` rollout（`spend.rs:849`/`:1036` 认这个后缀，`:1197` 调用）和
   dsh 的 `.jsonl.zstd`（`:1772`）。

2. **要求**：解码失败按帧/按段尽力解（能解出多少算多少），文本侧统一 `from_utf8_lossy` + 坏行跳过，
   与普通路径一致；确实整文件读不出来时，让该来源落到 `source_statuses()` 已有的
   `unreadable` / `parse_error` 态（`spend/sources/mod.rs:325-382`），**而不是静默 0**。
   这条正是 #42 建状态盘点层时留的口子，不要另造机制。

3. **一个必须显式决策的点**：`parse_zstd_file` 返回 `Some(0)`（`:1082`），意味着压缩文件**不参与**
   增量续读游标。本票要么一并让它进增量（要处理游标语义与多帧边界），要么明确写「本票不动增量」，
   不许含糊带过。现成参照：`spend.rs:5706` 已有 `write_multiframe_zstd` 的多帧测试夹具，
   `:4705` 已有 `rollout-test.jsonl.zst` 用例。

4. 与 04 同批做可以共用夹具（dsh 走的就是 `.jsonl.zstd`）。

## 取证材料 / 复现步骤

**在临时目录造，不碰真实 `~/.codex`。** `CODEX_HOME` 覆盖已被支持（`spend.rs:2351`）。

```bash
# 正常压缩一个两行的 rollout，然后把中间一个字节改坏
printf '{"a":1}\n{"b":2}\n' | zstd -q -o /tmp/zst-probe/rollout-x.jsonl.zst
printf '\xff' | dd of=/tmp/zst-probe/rollout-x.jsonl.zst bs=1 seek=<中间偏移> count=1 conv=notrunc
```

修复前：该文件贡献 0 条记录，界面上那天像真的没用过。
修复后：能解出的行照常入账；若整帧不可解，来源状态显示 `unreadable`/`parse_error` 而不是 0。

最省事的落点其实是直接在 `parse-tests` 里加用例（夹具基础设施现成），不必手搓 shell。

## 验收

- [ ] 单个坏字节不再让整文件归零（有测试断言其余行仍入账）
- [ ] 整文件确实不可读时，来源状态是 `unreadable`/`parse_error`，不是静默 0
- [ ] zstd 与普通路径的容错行为一致（同一份内容、同样一处损坏 → 同样的记录数）
- [ ] 「是否让压缩文件进增量续读」有明确结论并写进 Comments（做 / 不做 + 理由）
- [ ] 多帧 zstd 不回归（`:5706` 那组用例仍通过）
- [ ] `spend::` 定向测试 + `parse-tests` 全量绿；`pnpm build` 通过
- [ ] owner 目检：不适用；只需在设置页的来源诊断里确认损坏文件会显示成异常态而非 0

## Comments

- 2026-10-23：立项。Pane 自有缺陷，#48 对照时翻出。排 P2 是因为它需要文件损坏才触发，
  但触发后表现为「某家某天静默归零」，属于看板瞎一格。
- 2026-10-23 实现：两处严格解码统一成与明文路径同口径 —— `parse_zstd_file` 用 `String::from_utf8_lossy` 后逐行交给各家 parser（坏行照旧被认不出的行挡掉），`dsh_files_data` 同样改 lossy。一个坏字节不再让整文件返回 `None`。
- **明确不做**（票面选择「只统一容错」）：让压缩文件参与增量续读。现在调用处是 `checkpoint = if is_zstd { None }`，即压缩日志每次全量重读、不进增量；改成进增量要一并定义 `Some(0)` 的返回值语义与多帧截断时的续读点，属于另一件事，留在此处不跟进。
- 回归：`spend::tests::corrupt_byte_in_zstd_log_keeps_the_good_lines` —— 一个好行 + 一行非 UTF-8 垃圾 + 一个好行压成 `.jsonl.zst`，修复前整文件归零（tokens 0），修复后仍拿到 1100。既有 `parse_lines_from` 的明文容错测试未动。
