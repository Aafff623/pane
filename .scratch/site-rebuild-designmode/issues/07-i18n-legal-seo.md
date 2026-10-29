# 07 · 中英文与法务页面

Status: done 2026-10-29 | Priority: P1 | Depends: 03

## Scope

- 词典：`src/i18n/zh.ts` / `en.ts`，按区块分组（nav / hero / workflow / panel / faq / download / footer / meta），类型约束两种语言键完全一致。
- 路由：`/` 中文，`/en/` 英文；语言切换保留当前路径的对应路由（`/privacy` ↔ `/en/privacy`）。
- 法务：`/privacy`、`/terms`（中文）与 `/en/privacy`、`/en/terms`（英文摘要），内容以 `docs/privacy.md` 与旧站页面为源，事实不变；Worker 仍需返回 `content-language: en`。
- SEO：两语言各自的 title/description/canonical/OG；`sitemap.xml`、`robots.txt` 静态导出；`hreflang` 互指。
- 删除上游的多语言之外的多余内容源（blog/changelog 等已删）。

## Done when

- 中英页面事实、版本号、服务商数量一致。
- 法务链接在两种语言下都指向对应语言页面。
