# WorkBuddy / CodeBuddy v2 — 验收清单（owner 手动）

**状态：** 自动化全绿；以下项目需要真机 / 真账号由 owner 勾选。
**自动化结果（2026-10-19）：** `parse-tests` 898 通过 / 0 失败 / 13 忽略；`tsc --noEmit` exit 0；`vite build` 成功；`node --test tests/*.test.mjs` 51 通过 / 0 失败；`cargo check` 通过（18 条既有 warning）。

## A. 档位与登录

- [ ] 设置 → 授权中心能看到 **WorkBuddy AI** 分组（与 WorkBuddy、CodeBuddy CN 并列）
- [ ] 用国际版账号扫码，卡片出现，标题为 WorkBuddy AI
- [ ] 同时发起 WorkBuddy（国内）与 WorkBuddy AI 扫码：两边都能完成或各自超时，不互相报 `no pending sign-in`
- [ ] 同一账号重复扫码：仍只有一张卡，label 更新
- [ ] 把国内账号的凭据强行归到 AI 档（或反之）会被拒绝并给出可读错误（可用手工构造的 extra 验证；正常扫码不会触发）

## B. 账号来源与导入

- [ ] 扫码登录的账号行显示「扫码登录」
- [ ] 本机 WorkBuddy 桌面版已登录时，授权中心点「导入桌面版登录」→ 出现 `已导入 <label>`，账号行显示「已从桌面版导入」
- [ ] 再点一次同一导入 → 显示 `已更新 <label>`，不产生第二张卡
- [ ] 桌面版登录态为 `$wbEncrypted` 时，导入返回加密提示，账号库不新增
- [ ] 删除该账号后，官方 WorkBuddy 客户端仍保持登录（Pane 不碰官方文件）

## C. 额度卡面

- [ ] 桌面/网页渠道的卡片末尾出现 `Source` 行，内容为 `desktop app (read-only)` 或 `web session (explicit import)`
- [ ] 扫码账号的卡片没有多余的 `Source` 行
- [ ] 有多个有效 credits 包时只出现一行 `Packages` 汇总（数量 / 总剩余 / 最近到期），逐包行不刷屏
- [ ] 某包 7 天内到期时，该包单独一行并带 `expires in Nd`（最多三行）
- [ ] 只有一个包且不临期时，不出现额外包行
- [ ] 企业账号（个人接口空列表）仍显示企业额度；`limitNum = -1` 显示 Unlimited

## D. Token 用量

- [ ] 打开用量面板，出现 **WorkBuddy** 行（本机有 `~/.workbuddy/projects` 日志时），token 数不为 0
- [ ] 该行与「积分/计量用量」节分开，credits 没有被当成 token
- [ ] 模型无价格时该行仍计入 token，并出现未定价标记
- [ ] `fetch_spend_sources` 的来源盘点里 `workbuddy-jsonl` 为 `detected`；`workbuddy-ai-jsonl` / `codebuddy-cli-jsonl` 在本机为 `not_detected`（目录不存在）

## E. 失败态

- [ ] 断网时卡片显示网络错误，不是 0 消耗
- [ ] 会话被撤销后显示「session expired — sign in again from the auth center」，重新登录后恢复
- [ ] 多账号时账号 A 失效不影响账号 B 的卡片

## 已知限制（不是缺陷）

- 官方 `get-user-request-usage` 流水未接入（缺可验证账号，见 issues/09）。
- CodeBuddy IDE `history/index.json` 未接入（本轮只做 CLI JSONL）。
- 本地 JSONL 的跨文件整份副本可能重复计数（逐文件去重；见 issues/07 comments）。
- 国际版真实 OAuth / 账单路径来自上游已发布代码，Pane 侧尚未有真机账号验证。
