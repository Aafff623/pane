# Ticket 12 · 讯飞校准步骤（owner 操作）

代码侧已就位：`PANE_CONSOLE_PROBE=1` 时，两个讯飞 provider 会在本机写
**shape-only** 日志（字段名 + 类型 + 白名单枚举/日期字符串；不含 Cookie、
不含姓名/手机号/身份证，最多 40 条，原子写入）。

- `%APPDATA%\PaneAafff623\xfyun_probe_log.json`（星火 Spark-X2.6）
- `%APPDATA%\PaneAafff623\xfyun_speech_probe_log.json`（开放平台语音）

## 1. 打开捕获

用**用户级**环境变量，不要用 `$env:` 临时变量——开发启动器
`temp/scripts/restart-pane-dev.ps1` 走 WMI 创建进程，进程级变量传不进去。

```powershell
[Environment]::SetEnvironmentVariable('PANE_CONSOLE_PROBE','1','User')
```

然后完全退出 Pane（托盘右键退出）再启动，让新进程读到该变量。

## 2. 星火 Spark-X2.6（#38）

1. Alt+2 → 授权中心 → 讯飞星火 → 登录，在弹出的独立窗口里登完
   `maas.xfyun.cn`。
2. 回卡片点刷新，记下卡片显示的数字。
3. 在浏览器里打开讯飞 MaaS 控制台的 Spark-X2.6 免费额度页，抄下：
   总额度 / 已用 / 剩余 / 到期时间 / 状态文案（是否显示“已领取”）。
4. 如果卡片显示“尚未领取免费额度”，也告诉我——那说明 `claim` 真的是 null，
   需要先领取再刷新一次。

## 3. 开放平台语音（#47）

1. 授权中心 → 讯飞开放平台语音 → 登录 `console.xfyun.cn`。
2. 刷新卡片，记下每个资源包的剩余量。
3. 控制台里对照同一批资源包（语音听写/合成/长文本等）的“剩余量”和
   “赠送量”，把两边数字发我。
4. 顺手确认一下：退出登录后卡片是否变成“请重新登录”，而不是 0%。

## 4. 完成后

回一句“登录好了”，我直接读上面两个本地日志做校准（改 fixture、解析器、
三语文案与文档），不需要你把 Cookie 或响应体贴给我。

校准完请关掉捕获：

```powershell
[Environment]::SetEnvironmentVariable('PANE_CONSOLE_PROBE',$null,'User')
```

日志文件随时可删，删掉不影响任何功能。
