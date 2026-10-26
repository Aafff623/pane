Status: done
Type: task
Blocked by: —

# 04 · OAuth 三纪律（双栈 loopback / 手动兜底 / pending quarantine）

## 做什么

为 Pane 的 OAuth 场景（Cursor PKCE 优先，其余渐进）补三条通用纪律（原创实现）：
1. loopback 回调服务器同时绑 [::1] 与 127.0.0.1 同端口，两者都成功才广告 localhost，单栈广告字面 IP。
2. 手动粘贴回调兜底：接受完整 URL / 纯 path / 纯 query 三种输入；deep-link 失败时 UI 给出粘贴入口。
3. pending 态落盘：0700/0600 原子写 + 文件名遍历校验；解析失败 quarantine（rename 保留）不删除；device-auth 流永不从盘恢复。
另：全局单 flow——已有进行中 flow 时返回既有 auth_url，防重复开浏览器。

## 验收

- 双栈绑定行为单测（只绑 IPv4 时广告字面 IP）；三种粘贴输入都能解析；损坏 pending 文件被隔离且原文件保留。
- owner 场景验收：断 deep-link 后手动兜底走通。
