Status: done (2026-10-26) — Rust 侧已接线并全绿；owner UI 验收待做（见下"手动场景"）
Type: task
Blocked by: 01

# 06 · 刷新与身份防护（#63）

## 做什么

1. **最小多账号刷新接线**：沿现有 Codex 账号刷新路径引入 per-account
   `account_lock` 与代际双检；不做通用 worker。
2. **#63-02 身份校验**：能在额度响应里拿到 account/user/org id 的 provider，
   写 snapshot 前与账号库绑定标识比对；不符 → alert，不静默入库；双方都有值
   才比。失败日志只写凭据指纹。
3. **#63-05 token 锁**：`decide_after_lock` 接入刷新路径；两并发只真实刷新
   一次，后者复用。
4. **#63-06**：cursor hourly 双窗解析（有官方样本才做；没有则记录待确认）。
5. **#63-08 隐私眼睛**：授权中心身份掩码按钮，记住偏好，三语 aria。

## 结果

- **#63-05 已接线**：`providers/codex.rs::login_access` 重构为
  `resolve_login_under_lock`——等锁前记 `login.generation`，锁内用共享注册表
  `account_token_lock::account_lock("codex", account_id)` + 重载账号 +
  `decide_after_lock`：代际已推进且 token 有效 → 复用（不再消费已轮换的
  refresh_token）；否则真实刷新一次（`refresh_login_token` →
  `save_refreshed_from`，写回时代际 +1；re-login 经 `upsert` 同样 +1）。
  `CodexLogin` 新增 `generation: u64`（serde default，旧 JSON 兼容）。
  `account_lock` 注册表从 std Mutex 换成 `tokio::sync::Mutex`：生产调用点
  跨 HTTP await 持锁，std guard 会让 future 非 Send（lib.rs `guarded()` 的
  Send 界编不过）。本地弱引用锁表 `login_refresh_lock` 已删，锁统一进模块。
- **#63-02 已接线（Codex 路径）**：`providers/codex.rs::verify_login_identity`
  在 snapshot 发布前比对"凭据自带身份"与"账号库存储身份"：
  id_token（缺失时 access_token）JWT 的
  `https://api.openai.com/auth.chatgpt_account_id` 对存储 `account_id`；
  account 声明缺失时才用 email 声明兜底（同账号改邮箱不误报）。
  双方都有值才比，缺字段 ≠ 不匹配（`identity_guard::check_identity` 三值）。
  不符 → `Snapshot::error` 告警拦截，不入库；`eprintln` 日志与刷新失败日志
  只写 `credential_fingerprint`（sha256 前缀+长度+过期时间），永不含原文。
- **provider 覆盖清单（#63-02）**：
  - 已覆盖：**codex**（Pane 托管登录卡：存储身份独立于凭据，可比对；
    `wham/usage` 响应本身不带身份字段——无样本证明，不虚构——身份取自
    凭据 JWT 声明）。
  - 未覆盖：其余 provider。逐类原因：claude/cursor/copilot/grok/kiro 等的
    额度响应解析中无已验证的 account/user/org id 字段（cursor 的 `sub` 是
    会话凭据侧，xfyun 的 `uid` 只在登录捕获响应出现，额度响应无）；
    CLI-home 型 codex 默认卡的 account_id 与凭据同源，无独立存储身份可比。
    后续拿到真实响应样本再按 `identity_guard` 三值语义逐家接入。
- **#63-06 待确认**：cursor hourly 双窗**未实现**。仓库内（cursor.rs 解析
  与全部 fixture）只有月度 `billingCycleStart/End` 样本，无任何 hourly
  窗口真实样本/官方 fixture；不猜字段。待 owner 提供 hourly 窗口真实响应
  后再做。
- **#63-08 已由 cockpit-borrow 08 小件包落地**（本票核验，无重复改动）：
  `auth-toolbar` 的眼睛按钮（`data-auth-privacy`，原生 `<button>` 可键盘
  操作，`aria-pressed` + 三语 `auth.privacyHide/Show` title+aria-label，
  复用 uiIcons `eye`/`eyeSlash`），偏好存 `localStorage["pane.auth.mask"]`，
  `authRowName` 掩码身份、不动额度数字。

## 测试

- parse-tests 串行全绿：1173 passed / 0 failed / 18 ignored（基线 1170 + 新增 3）。
- 新增：`codex_accounts::credential_rotation_advances_the_generation`
  （刷新写回 +1、CAS 拒绝的过期写回不 +1、re-login +1）；
  `providers::codex::identity_guard_hit_miss_and_missing_field`
  （命中/不命中/缺字段/仅 email 不符/无声明五分支，错误文案不含密钥）；
  `providers::codex::two_racing_login_refreshes_do_exactly_one_refresh`
  （multi_thread tokio，两并发同账号 observed gen 相同 → 计数器 == 1，
  存储代际 == 1，双方都拿到新 token）。
- `account_token_lock` 既有三测保持绿（tokio 化后线程测试改 `blocking_lock`）。
- src-tauri `cargo +stable-x86_64-pc-windows-gnu check` 通过（Send 界验证）。

## 手动场景（owner 验收用，可复现步骤）

1. **换绑旧 token → alert 而非静默错账**：授权中心给账号 A 完成 Codex 登录；
   退出后用 `%APPDATA%\PaneAafff623\codex-accounts.json` 把 A 行的
   `account_id` 改成另一个真实账号 B 的 id（或把 B 的 vault 凭据换进 A 的
   slot）；触发刷新 → A 卡必须显示 identity mismatch 错误（红色错误态），
   且 B 的额度数字不出现在 A 卡；日志（pane stderr）只有
   `[pane] codex identity mismatch on codex@…: credential <8位hex>:len=…:exp=…`，
   无 token 原文。
2. **并发刷新只刷一次**：把 A 卡 token 置为过期（改 `expires_at` 为过去），
   同时触发两次刷新（面板刷新 + 授权中心刷新）；网络面板/日志中
   `auth.openai.com/oauth/token` 只出现一次请求，第二路直接复用。
3. **隐私眼睛**：授权中心工具栏眼睛按钮可 Tab 聚焦、Enter/Space 切换；
   掩码只影响姓名/邮箱，额度数字不变；重开面板偏好保持；EN/中文/РУ
   三语 title 与 aria-label 正确。
