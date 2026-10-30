# Review — 旧自启项退役（2026-10-29）

独立 code review 子代理审查 `legacy_autostart.rs`、`setup()` 调用、parse-tests 挂载与文档。

## 已修复

| 级别 | 发现 | 处理 |
|---|---|---|
| P1 | 旧版在自身 setup 末尾会重新写入 `Run\Pane`，早期一次删除会被撤销，问题在每个开机循环复发 | 早期删除后增加 12s / 42s / 132s（累计）复检，成功一次即停；真机用“启动后 2 秒再写入”验证通过 |
| P3 | `%LOCALAPPDATA%`、短路径、junction 可能指向当前 exe 却被按文本误删 | 含 `%` 的值保守跳过；两个文件都存在时用 `canonicalize` 比较 |
| P3 | 打开/读取/删除失败与“本来就没有”无法区分 | 非 NotFound 错误写入 stderr，成功才返回旧值 |
| P3 | 缺失闭合引号的 Run 值永远识别不到 | 无闭合引号时回退扫描 `.exe` |
| P3 | 旧项在任务管理器可能残留 | 删除成功后顺带清理 `StartupApproved\Run\Pane` |

## 保留边界

- 只处理 `HKCU`；HKLM 需要管理员权限，启动文件夹快捷方式未处理。
- 只删除文件名 `pane.exe` 且目标不是当前 exe 的值；指向当前 exe 或含环境变量的值不碰。
- 复检窗口 132 秒；极慢开机超过窗口的旧版写入会留到下一次启动再清。

## 证据

- `cargo check` 通过；parse-tests `legacy_autostart` 3/3。
- 真机：早期删除通过；延迟写回场景通过；`Pane-Aafff623` 全程未被误删（测试期间被身份迁移指到 debug，已恢复安装版路径）。
