Status: ready
Issue: https://github.com/Aafff623/pane/issues/59
Priority: P2
Area: Insight

# magpie 思想借鉴批次：反事实重放 / 百分比反推绝对用量 / 扫描缓存代码版本哈希

## 背景

magpie（yetone/magpie，MIT）功能盘点中发现三个「社区没注意但价值高」的机制，经用户确认全部立项借鉴。与 Pane 的契合点：Pane 是只读看板，恰好拥有 magpie 做这三件事所需的全部本地数据。证据与移植级细节：`temp/research/20261025-magpie-deep-dive.md`（本地）。

## 01 反事实重放（tune.go 思想）

用用户自己的历史调用做"如果当初"推演："你若在 14:02 压缩过上下文，本会话能省 38 万 token"。magpie 用它做参数寻优；Pane 只读版先做单一场景 MVP（压缩时机），数据源=zcode turn_usage（每轮聚合 token）+ Claude Code JSONL（每请求 token+时间戳）。这是看板从"记账"升级为"帮你省钱"的分水岭功能。

## 02 百分比反推绝对用量（window_holds.go 思想）

厂商只给"剩余 42%"，用本地日志反推绝对值。**用户确认的关键设计：必须分模型、分两种口径**——
- **token 计价套餐**（zai/kimi/minimax 类）：精确算。剩余 = 总额度 − Σ 各模型消耗（spend.rs 已有每模型每日消耗）。
- **黑盒额度窗口**（Claude 5h / Codex 周）：拿"你自己已烧掉的量"当标尺：从窗口起点消耗 X、厂商说已用 p%，则剩余 ≈ X × (1−p)/p，单位标注「按你目前的用法组合估算」；模型混用不影响（标尺即自身混合用法），但用法结构变化会偏差，文案必须带"按目前用法"。
- **护栏（magpie 同款，不编数）**：本地日志必须追溯到窗口起点；p=100%（无消耗）或 p 无效不推算；resets.rs 窗口检测已就绪。

## 03 扫描缓存代码版本哈希失效（request_disk.go 思想）

magpie 缓存条目除 size+mtime 外绑定「生成它的代码」哈希——升级后不脏读、也不全量重扫 1.5 万文件。Pane 的 spend 光标已有 byte-offset + boundary_hash（spend.rs:295），缺"解析器版本"维度：升级改解析逻辑后，旧 cursor 会带着旧口径继续跑。做法：cursor/persist 条目加 parser_version salt（或 PERSIST_VERSION 细分到 per-source），版本不匹配才重扫该源。

## 许可与红线

MIT 可自由借鉴（抄代码带声明）；三个功能全部只读本地数据，无凭据上传；tune 思想的"参数寻优"部分不抄（涉及改用户配置），只做"洞察展示"。

## 验收

见各票；02 需 owner 确认两种口径的 UI 文案；03 需在本机实测升级后重扫范围仅限受影响源。
