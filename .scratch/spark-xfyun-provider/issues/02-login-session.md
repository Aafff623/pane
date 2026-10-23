# 02: 讯飞星火 family + 网页登录 + 会话持久化 + 多账号

**What to build:** 新增 `xfyun` family（显示「讯飞星火」），在 Pane 自己的隔离登录窗口登录 `maas.xfyun.cn`；网站接口确认登录成功后保存会话，重启仍可用；Settings 和认证中心支持添加、重新登录、清除、归档多个账号；卡片先显示会话状态（已连接 / 未登录 / 已过期），额度取数留给 03。

**Blocked by:** 01

**Status:** ready-for-agent

## 要点

- family 双端注册：`supports_api_key: false`、`supports_extra_accounts: true`、web-session 能力；不提供 API Key 输入槽。
- 登录窗口用隔离/incognito WebView2；Pane 不碰验证码和账号密码，用户自己完成讯飞登录。
- “已登录”必须由网站接口确认，不能只看 cookie 是否存在；`code:0` 为已登录，`code:4001` 为未登录/过期。
- 实现顺序：先验证 cookie-only 是否可通过认证；不行再加**只读、最小面**的 localStorage token 读取通道，只取必要 token/cookie，不开放任意脚本执行。
- 会话 scope 用 `web/xfyun`，OS vault 优先；vault 失败按现有明文例外约定回退并提示。多账号走现有账号存储，按网站稳定用户 id 去重；重登只更新该账号，清除只清当前账号。
- 会话过期返回 `no_credentials` 语义并提示重新登录；网络/5xx/解析失败才返回 `error`。
- 登录窗口关闭、超时、用户取消都不能留下僵尸窗口或半条会话。
- 不读浏览器 profile 的 SQLite/Cookies；不要求用户手动粘贴 token 作为正式交互。

## 验收

- [ ] owner 能在 Pane 登录窗口完成讯飞登录，窗口自动关闭，设置行显示已登录。
- [ ] 重启 Pane 后会话仍在；清除会话只清当前账号。
- [ ] 同一账号重复登录不产生重复卡片；两个账号各自独立显示、独立过期。
- [ ] 会话过期时卡片/设置明确提示重新登录，不影响其他 provider 或账号。
- [ ] token/cookie 不写日志或 Git 跟踪文件；公开状态接口不泄露。
- [ ] `tokenrhythm` 与现有 provider 无回归；parse-tests、tsc、vite、i18n 门禁全绿；owner UI 验收。
