# 02: 隐藏 webview 取数桥

**What to build:** `tokenrhythm_snapshot`：惰性创建隐藏 webview（同
origin 承载 cookie），`eval_js` 执行站内
`fetch('/api/wallet/summary',{credentials:'include'})`，JSON 经
`tokenrhythm:data` 事件回传 Rust；401 → 卡片转「需重新登录」态。

**Blocked by:** 01

**Status:** implemented（2026-10-19；cookie 从 Paine 自身 EBWebView 配置读取并缓存进 vault，reqwest 带 Cookie+X-CSRF-Token 调 /api/wallet/summary；401 → 需重新登录；响应结构待首登实测校准）

## 要点

- 隐藏 webview 惰性创建、常驻复用；失败/过期时可销毁重建
- fetch 全程在 webview 内，cookie 不经过 Paine、不落盘
- 事件回传带 path 标识（wallet / keys-usage 可分多次取）
- 防御式解析：字段名以首登实测为准，未命中字段记 `待确认` 不猜数
- 超时与取消：webview 加载失败、fetch 悬挂要有上限（参考既有
  httpapi/onenewapi 的超时常量级）

## 验收

- [ ] 已登录态取回真实余额 JSON 并解析成功
- [ ] 会话过期（401）时 Snapshot 明确表达「需重新登录」而非错误卡片
- [ ] 取数无 cookie/token 写入 Paine 文件或日志
