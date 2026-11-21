# Spec: BibiGPT 会员积分额度卡片

**Status:** implemented 2026-11-20 — 待真实响应核对字段名 · **Owner:** Aafff623

## 现象

BibiGPT 按会员档位发放月度积分（当前账号是 Plus，6000/月），界面上能看到
`5,824 / 6,000`、可叠加的充值余额、以及恢复日期。这些数字散在一个网页控制台里，
每天要主动点开才知道还剩多少。

## 接口（2026-11-20 无会话验证）

- API host 是 **`bibigpt.cn`**，`bibigpt.co` 只是 302 壳。
- `GET /api/v1/me` 未认证返回 **401**（不是 404），响应体里 `path: "agent.me"`。
- 统一信封：`{ message, code, data: { code, httpStatus, zodError, creditsShortfall } }`。
- 认证不是 NextAuth（`/api/auth/session` 是 404），登录页在 `/user/login`。

## 决定

1. 新建 `bibigpt` family，复用 Pane 自己的 incognito 登录窗口
   （`read_webview_cookies` → `secretstore`，键 `Pane:bibigpt:session`），
   与 `xfyun-speech` 同一套 seam，不碰浏览器 profile。
2. **登录成功的判据是能读到一次真实 `/api/v1/me` 响应**，而不是"有 cookie"。
3. 该响应会被写到 `%APPDATA%\PaneAafff623\bibigpt-response.json`（仅本机），
   用于核对字段名。日志只记路径，不记内容。
4. 卡片三行：
   - 月度额度 → `Metric::progress`（`used / total` + 恢复日期）
   - 充值余额 → 计数，**不画百分比**（它本来没有分母）
   - 缺失任一半边 → 文本行说明缺什么，**不显示 0%**
5. 解析器按候选键名读取（`creditsUsed` / `creditsTotal` / `topupBalance` /
   `currentPeriodEnd` 等），拿到真实响应后收敛成确定路径。

## 边界

- 只做余额/额度读取；不触发任何消耗积分的操作。
- `.cn` 与 `.co` 是否两套账号体系未验证，只接 `.cn`。
- Pro 档位的额度差异未验证；卡片按接口返回的数字显示，不硬编码档位配额。

## 验证

- 解析器单测 5 条：进度行、缺总量不造假、充值余额无进度条、401 报错、
  RFC3339 映射到重置倒计时。
- 前端 258 通过；catalog / dispatch / 前端目录 / 图标全部注册。
- 实机待验：登录一次 → 核对 `bibigpt-response.json` → 把候选键名换成实测路径。

## 顺带修掉

三条 Copilot 用量测试把夹具日期写死成 `2026-10-20`，而扫描器只读 31 天窗口，
日期滚过之后整批数据被过滤，测试从此恒失败。改为按当前时间生成相对日期。
