Status: ready
Type: task
Blocked by: 04
Priority: P3

# 07 · 候选 family 扩展（109 家清点中挑 5-10 家）

## 做什么

对照 temp/research/20261025-codexbar-deep-dive.md §二③ 全清点表（109 家=typed 25+16、插件 ~70；**Pane 缺 B 组 typed 16 家 + D 组插件 55 家**）：
1. 与 owner 共同挑 5-10 家候选（按用户量/需求频率；typed 家优先——Alibaba/QwenCloud/Gemini/HuggingFace/Wayfinder 等可 MIT 署名直译 strategy）。
2. 每家一票：端点情报（从 CodexBar Sources/CodexBarCore/Providers/<Name>/ 提取）→ spend/配额接入 → fixture 测试。
3. 插件型（D 组）先过「插件式 web provider」评估（spec 备查节），评估前不接。

## 验收

- 每家：接入后本机/fixture 出数；parse-tests 断言。
- 与工具落点扩展（用户自实施清单）去重——同一工具不重复立项。
