# 04 — dsh 来源改用主遍历，删掉第二份手写递归

**What to build:** 让 DeepSeek Harness 的会话扫描复用 `recent_log_files` 那套护栏（深度上限、目录数
上限、canonical 去环），修掉「`~/.dsh/sessions` 内出现软链成环就把整个 Pane 进程挂死」的缺陷。
**Blocked by:** 无
**Status:** ready-for-agent
**GitHub:** [#48](https://github.com/Aafff623/pane/issues/48) §4 · **Priority:** P1 · **量:** 小

## 要点

1. **现状（`src-tauri/src/spend.rs:1764-1777`）**：

   ```rust
   fn scan(dir: &Path, out: &mut Vec<PathBuf>) {
       let Ok(entries) = std::fs::read_dir(dir) else { return };
       for entry in entries.flatten() {
           let path = entry.path();
           if path.is_dir() { scan(&path, out); }        // ← 跟随链接、无深度、无环检测
           else if ... name.ends_with(".jsonl.zstd") { out.push(path); }
       }
   }
   ```

   `is_dir()` 跟随符号链接与目录 junction；无深度上限、无目录数上限、无 canonical 去环。
   环 = 无限递归 = 栈溢出或挂死。

2. **护栏本仓已经有一份完整的，直接用**：`recent_log_files`
   （深度 16 `spend.rs:826`、目录数 20000 `:830`、canonical 去环 `:895-937`，且 `.jsonl.zst` /
   `.jsonl.zstd` 后缀已在 `:1036` 处理）。**不要给 dsh 单开一份参数化护栏**——那还是第三份遍历。

3. **dsh 的 root 支持环境变量覆盖**（`spend.rs:1782-1785`：`DSH_HOME`，默认 `~/.dsh`，再拼
   `sessions`），所以本票可以在临时目录里安全复现与回归，**不需要也不允许碰用户真实的 `~/.dsh`**。

4. 顺带确认：dsh 走 `.jsonl.zstd`，与 05 票的 zstd 容错口径是同一条数据通路；两票若同批做，
   测试夹具可以共用。

## 取证材料 / 复现步骤

**全部在临时目录做，用完删掉。**

```bash
# 1) 造一个带自指 junction 的假 DSH 目录（junction 不需要管理员权限，符号链接 /D 需要）
mkdir -p /tmp/dsh-loop/sessions
printf '{"type":"assistant/message"}\n' > /tmp/dsh-loop/sessions/session-real.jsonl.zstd
cmd //c mklink //J "C:\\Users\\Lenovo\\AppData\\Local\\Temp\\dsh-loop\\sessions\\loop" \
                  "C:\\Users\\Lenovo\\AppData\\Local\\Temp\\dsh-loop\\sessions"

# 2) 用 DSH_HOME 指向它启动 dev Pane（或跑 parse-tests 里的 dsh 用例）
DSH_HOME=/tmp/dsh-loop  # 对应 spend.rs:1782
```

修复前：扫描挂死 / 栈溢出（进程没响应）。修复后：正常返回，且 `session-real.jsonl.zstd` 仍被计入。

**清理注意**：删 junction 只能删链接本身，用 `cmd //c rmdir "<link path>"`；
`rm -rf` 在某些工具下会跟进去把目标目录内容一起删掉。

## 验收

- [ ] dsh 不再有自己的递归实现（`grep` 确认 `dsh_data` 内无 `fn scan`）
- [ ] 环复现场景下进程正常返回（本票的复现步骤即回归用例，落成测试）
- [ ] 深度与目录数上限生效，且**跳过时留日志**（避免「用量静默缺失」查不出来）
- [ ] 同一份真实 `~/.dsh` 数据修复前后记录数一致（金额与 token 不回归）
- [ ] `spend::dsh` 相关测试 + `parse-tests` 全量绿；`pnpm build` 通过
- [ ] owner 目检：不适用（本票无可视变化），只需确认面板各家数值无变化

## Comments

- 2026-10-23：立项。这条与上游无关，是 #48 对照过程中顺手翻出的 Pane 自有缺陷；
  之所以排 P1，是因为 Pane 是常驻看板，挂死等于全盲。
