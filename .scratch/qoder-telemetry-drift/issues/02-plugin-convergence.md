# 02: 插件侧收敛到同一个读数口径

**What to build:** 让专门看 Qoder 用量的两个能力与 Pane 用同一套事实，不再各存一份猜测。

**Blocked by:** 01（`/v1/usage` 的 `telemetry` 字段是它们的共同来源）

**Status:** needs-owner（要先拍「修还是弃」和能不能给上游提改动）

## 现状（2026-10-23 实测）

- `token-usage v0.4.0`：**已废且响亮失败**。它找 `%APPDATA%\Qoder\SharedClientCache\cache\db\local.db`
  与表 `chat_message.token_info`；本机 `Roaming\Qoder` 只剩 `qodercli\`，脚本打印「未找到 Qoder 本地数据库」
  后 exit 1。把 `--db` 手动指到 `com.qodercn.app.stable\main.sqlite` 直接 `no such table: chat_session`
  （新库是 `chat_sessions` / `chat_session_messages`，且**没有** `token_info` 列）。
  → 换源等于重写一份 Pane 已有的 ratio×窗口推导，违背 01 的单一口径。倾向**弃用**。
- `qoder-credits-inspector v2.7.4`：能跑，但 0.4.11 之后转录里窗口是 null，它**退回内置默认 200K**，
  头部数字变成「计费输入总量 7.67M 推导（Σ ratio × ≈200.0K 回退值）」，而 Pane 用的是 1,000,000 ——
  同一台机器上同一个问题两个答案，差 5 倍，压缩盈亏判定一起偏。另外它的启动器把安装位置写死成
  `%LOCALAPPDATA%\Programs\Qoder CN` 三个候选，本机实装在 `D:\dev_software\Qoder CN\0.4.11`。
- `qoder-credit-watch v1.0.0`：不读本机数据（Edge profile + CDP 打官网接口），桌面端升级打不到它；
  本机 `accounts.json` 不存在，属未配置而非损坏。

## 要点 / 待拍

1. 窗口未知时应显示「不可估」，而不是拿猜的默认窗口算出一个看起来很确定的总量（会牺牲头部数字）。
2. 两者都在 marketplace 插件缓存里，本地改动会被下次插件更新覆盖 → 要嘛给上游提，要嘛本地 wrapper。
3. 读数来源改成 `127.0.0.1:6736` 的 `/v1/usage`，与卡片同一结论。

## Comments

- 2026-10-23：owner 已认可「去掉 200K 静默回退」与「token-usage 弃用」的方向，但两项都涉及改
  第三方插件本体或给上游提改动，需单独授权后再动；本轮未改任何插件文件。
