Status: claimed (implementer subagent, worktree)
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

## 验收

- 单测：标识命中/不命中/缺字段；两并发刷新只一次；锁后代际复用。
- 换绑旧 token 的手动场景有可复现步骤；隐私眼睛可键盘操作。
