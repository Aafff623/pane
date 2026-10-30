# Ticket 01 — 第一轮：清失败样本

Status: ready-for-human
Type: research
Date: 2026-10-29 · Build: 0.12.0（本机安装版）· Oracle: `GET /v1/providers` @ 2026-10-29T14:00:50Z

## 结论

12 张异常卡里，**只有 5 张需要换账号/重登**，4 张是传输层问题（与账号无关），1 张缺真实会话，1 张账号状态待确认，1 张正常。**没有一张是「数据解析写错」**——这轮不改代码的判断成立。

| 卡片 | 状态 | 归因 | 下一步 |
|---|---|---|---|
| claude | error | 凭据死亡：`~/.claude/.credentials.json` 停在 10-09，`expiresAt=1970`、`refreshTokenExpiresAt=2026-08-22`、无 refreshToken | 终端跑 `claude` 重登 |
| opencode | error | OpenCode Go key 401；本机 `auth.json` 的 key 存于 10-05 | 去 OpenCode 控制台换新 key |
| giphy | error | keyvault 有 token，上游 401 | 后台重新复制 dashboard `access_token` |
| sensenova | error | 本地只有 `access_token`，JWT 于 2026-10-26T16:36 过期，无 refresh token | Settings 里重新浏览器授权 |
| traecn | error | `storage.json` 停在 10-03，上游 401 | 打开 Trae CN 一次刷新会话 |
| stepfun-plan | error | 文件只剩 10-15 的 `.bak`；适配器从 vault 的 `web/stepfun-plan` 读会话，返回空用量 | 用账号密码重登后复测；空用量可能是无计划，也可能是接口变了 |
| codex ×3 | stale | chatgpt.com 连接超时；本机 curl 走 7892/7894 均可达（401），Pane 出站也确实在走 7892 | 先做手动代理复测（见下） |
| copilot | stale | 同上，api.github.com 同机可达 | 同上 |
| antigravity | stale | googleapis 云端不可达；本机 Antigravity 进程未运行（已安装） | 先开 IDE 复测，仍失败再归入代理问题 |
| zcode | needs_setup | 本地 JWT 可读，但没有 Start Plan 记录 | 去 ZCode 控制台确认账号是否有 Start Plan；有则是解析漏了 |

## 证据

**网络（codex / copilot / antigravity）**
`ProxyEnable=0`、`ProxyServer=http=127.0.0.1:7892;https=…`、`AutoConfigURL=…:7893/proxy.pac`；用户级 `HTTPS_PROXY` / `ALL_PROXY = 127.0.0.1:7894`，存活。curl：直连 chatgpt `000`，走 7892 / 7894 均 `401`；api.github.com 直连 `200`。75 秒采样 Pane 出站全部指向 `127.0.0.1:7892`，无直连 443。同期 tavily / firecrawl / brave（同为境外）正常出数，所以不是「境外全断」。

**凭据元数据**
claude：mtime 2026-10-09，无 refreshToken。opencode：`~/.local/share/opencode/auth.json` 10-05，含 `opencode-go`。sensenova：10-27，JWT exp 2026-10-26。traecn：`%APPDATA%\Trae CN\…\storage.json` 10-03。stepfun-plan：只剩 4 个 10-15 的 `.bak`。zcode：`~/.zcode/v2/credentials.json` 44.8K 存在。antigravity：已装于 `%LOCALAPPDATA%\Programs\Antigravity`，无 `language_server` 进程。

## 本轮发现的缺陷（记录，不在本轮修）

1. **P1｜`credentialState` 少报**：`source_probe::local_hint` 只有 21 家，`get_credential_status` 有 63 家，43 家只在后者。后果是 HTTP API 和首启扫描把已有本地会话说成「无凭据」。实测 7 张在卡与自身状态矛盾：qodercn（ok, 4 行）、orcaterm（ok, 4 行）、shandianshuo（ok, 2 行）、stepfun-plan、traecn、sensenova、zcode。这直接削弱 6736 作为实测 oracle 的可信度，应在第二轮开始前修。
2. **P2｜错误文案区分不了「凭据在 vault、上游无数据」**：stepfun-plan 报 “Step Plan returned no quota data”，同时探测说无凭据，两个信号互相打架。
3. **P3｜残留数据**：`accounts/linkso.json` 还有 1 条账号，但 `linkso` family 已从目录移除。
4. **P3｜antigravity 文案**：`Unavailable` 无法区分「IDE 未运行」和「云端不可达」，这轮只能靠进程列表人工判断。

## 需要 owner 做的（每项约 1 分钟）

1. claude：终端 `claude` 重登。
2. opencode：换 OpenCode Go key。
3. giphy：复制新的 dashboard `access_token`。
4. sensenova：Settings → SenseNova → 浏览器授权。
5. traecn：打开 Trae CN 一次。
6. stepfun-plan：Settings 用账号密码登录。
7. zcode：确认 ZCode 账号有没有 Start Plan。
8. ~~网络诊断~~ 2026-10-30 复测：0.12.2 上 codex×3、copilot、antigravity 的 stale 已全部清零，此项无需再做。

## 复测方法

每做完一项，等 6 分钟刷新周期或重启 Pane，再读 `/v1/providers` 对比该卡 status / lines / error；结果补到本文件的复测表，并同步到 `docs/provider-credentials.md` 对应行。

## Comments

- 2026-10-29：首轮取证完成，12 张卡全部归因完毕；原始快照在 `temp/provider-bench/2026-10-29/`。
- 2026-10-30 18:28（v0.12.2）：stale 5→0，codex×3 / copilot / antigravity 全部恢复，传输层问题不复现；6 个 error 与 zcode 的 needs_setup 不变（owner 尚未重登）；7 张卡的 `credentialState` 矛盾仍在。新快照 `temp/provider-bench/2026-10-30/`。

- 2026-10-30 19:20：P1 `credentialState` 已由 ticket 02 修复；7 张卡恢复 present，`/v1/providers` 中只剩 grok/qwen/qoder 的真实 `no_credentials`。owner 重登后的 7 项复测仍待完成；第 2 轮未开始。
