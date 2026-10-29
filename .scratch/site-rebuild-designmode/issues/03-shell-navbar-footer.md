# 03 · 外壳：Layout / Navbar / Footer / SEO

Status: done 2026-10-29 | Priority: P0 | Depends: 01, 02

## Scope

- `app/layout.tsx`：Pane metadata（title 模板、description、OG/Twitter、canonical、alternates hreflang zh-CN/en），`<html lang>` 由路由决定。
- Navbar：Pane 品牌（`pane-mark.svg` + 字标）、导航（功能 / 演示 / 常见问题 / GitHub）、语言切换（/ ↔ /en/）、主题切换、下载按钮（指向 `/download/windows`，保持 `data-latest-installer` 钩子或等价实现）。
- 移动端菜单保留上游交互（汉堡、inert、过渡）。
- Footer：源码 / 文档 / 反馈 / 隐私 / 条款 / MIT / © 2026 Pane + 独立工具免责声明；英文页链接到英文法务路由。
- 删除上游 analytics、browser banner、back-to-top 等站点专属组件（如保留 back-to-top 需与新设计一致）。

## Done when

- `/` 与 `/en/` 的 `<html lang>`、title、canonical、OG 全部正确。
- Navbar 在 1440/768/390 三个宽度无溢出，键盘可操作。
