Status: ready
Type: task
Blocked by: —

# 03 · 授权中心头像与空态双型

## 做什么

1. auth_center.rs：AuthAccountRow 加 `avatar_url: Option<String>`，在 codex/copilot/kimi 装配点透出（对照 cc-switch 各 OAuth user-info 调用：copilot_auth.rs:463-494 GitHub /user、kimi_oauth/mod.rs:222 identity.avatar）。
2. 前端 authRowHtml（main.ts:19175）：avatar_url 存在 → `<img loading=lazy referrerPolicy=no-referrer>` + onError 降级字母；无 → 现有字母头像。
3. 空态双型：i18n 拆 `auth.empty`（手动添加型「还没有登录 X 账号」）/ `auth.emptyAuto`（自动发现型「还没有 X 账号。X 里已登录的号会自动出现在这里」，用于 agy/kimi 类）；main.ts:19417 按家族选择。
4. 「需要重新登录」红胶囊若 Pane 缺则补（cc-switch :581-590）。

## 验收

- 登录 GitHub 系账号显示真实头像；断网/失效 URL 降级字母不破图。
- 空态两种文案三语正确；owner 视觉验收（隐私口径：外链头像已确认可接受，URL 无凭据+no-referrer）。
