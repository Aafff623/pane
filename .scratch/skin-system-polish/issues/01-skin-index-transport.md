# 01 — 皮肤索引运输：Rust 拉取与 CORS 兜底

Type: task  
Status: done (2026-10-26) — Rust index + invoke-first + CORS worker; fresh-profile listing/download verified. Live deploy pending.  
Blocked by: none  
Spec: ../spec.md  
Issue: [#57](https://github.com/Aafff623/pane/issues/57)

## Scope

- 在 `src-tauri/src/skin_packs.rs` 增加远端索引类型和读取函数，复用现有 `send_with_direct_fallback`。
- 通过 `src-tauri/src/lib.rs` 暴露只读 command。
- `src/skins.ts::refreshSkinIndex()` 优先 `invoke`，浏览器 demo 才 `fetch`。
- localStorage last-good 缓存保留；HTTP、网络、JSON/结构错误分开报短原因。
- 给 Cloudflare 静态资源补 CORS 头，至少 `/skins/index.json`、`/skins/*.zip`。

## Why

线上资源 HTTP 200，但无 `Access-Control-Allow-Origin`；WebView 的 `fetch` 实测失败。Rust 请求不受浏览器 CORS 影响，且已有代理直连回退。

## Acceptance

- [ ] Rust 命令能解析当前线上索引并返回 3 个 pack。
- [ ] 本次会话环境实测 `invoke("skin_pack_index")` 成功。
- [ ] 浏览器 demo 在 command 不存在时仍走 fetch fallback。
- [ ] 全新 profile、空 `%APPDATA%\Pane\skins` 首次打开工坊可见三款远端皮肤。
- [ ] 断开网络时使用上一次成功索引；没有缓存时显示可理解的空态。
- [ ] `check-skin-packs.mjs --live` 仍通过。

## Comments

- 2026-10-25：由 #57 拆分。首选修法已由 owner 确认。
