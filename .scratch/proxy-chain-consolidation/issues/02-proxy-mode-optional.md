Status: deferred (optional; #74-01 shipped)
Type: task
Blocked by: 01
Priority: P3

# 02 ·（可选）proxy.mode 三档（system/manual/direct）

## 做什么

1. config schema `proxy.mode`（读时回退旧 enabled：true→manual，否则 system；direct=强制直连）；lib.rs:195 播种补 "mode":"system"。
2. providers/mod.rs:314 proxy_url() 加 direct 短路返 None。
3. main.ts:18533-18545 开关改三选一；i18n 三语（settings.proxyMode+三选项 label+desc）。

## 验收

三档切换本机实测；旧 config（仅 enabled）无缝回退语义正确；`pnpm build` 绿。
