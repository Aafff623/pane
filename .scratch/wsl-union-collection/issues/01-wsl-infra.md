Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · WSL 基建：枚举 / home 解析 / 纯函数 / 配置

## 做什么

1. spend::wsl 新模块+漏斗 `scan_roots_for(rel)`（native home+每启用发行版 UNC 根）。
2. 枚举：`wsl.exe -l -v` UTF-16LE+BOM 解码+`*` 默认标记；非零退出用 `-l -q` 二次确认；仅 ENOENT=未装（负缓存当天不再试）。
3. home 解析：每发行版一次 `wsl -d <d> -- pwd`，进程内 OnceLock+持久化 TTL 1h。
4. 三纯函数+parse-tests（wsl_consistency.rs）：prefix_for_version（WSL1→\\wsl.localhost\、WSL2→\\wsl$\）、parse_distro_list、posix_to_win（/mnt/c→C:\、/home→None）。
5. platform::run_hidden_bytes（UTF-16LE；必须走 platform/mod.rs:196/215 的 run_hidden 守卫模式）。
6. config `spend.wsl_mode`（native-only/both/wsl-first，默认 native-only）；UNC 独立目录预算（每 root 2k）。

## 验收

纯函数单测全绿；手测列出发行版与 UNC 根；采集器不动、默认行为不变（native-only）。
