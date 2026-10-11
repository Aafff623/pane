# Pane TODO

<!-- next-task-id: 0035 -->

This file is the canonical board for cross-session task state.

Keep one line per task. Phrase every task as an observable outcome that can be demonstrated when complete. Completed tasks are removed after validation; this board is not a history log.

Handoff parcels in `temp/handoff/` cite task IDs from this board. The board does not depend on local-only handoff files.

## In progress

- [ ] T-0034 — 正式皮肤「云端邮局 / Cloud Post Office」（保留 signal 标识）：**2026-10-11 已随 v0.7.0 提交并发布**——昼夜原图、手绘图标裁切、纸张组件、正式工坊入口（不依赖实验开关）、详情圆环 44px/紧凑 36px、真实周期时长与倒计时、亮色辅助文字/按钮状态/额度轨道适配、分享素材内嵌与原生恢复；本轮 npm build / cargo check / parse-tests / 官网测试全过，时段高亮吞字、状态色饱和度、更新弹窗等用户截图纠偏项已修并复拍。范围与验收见 docs/skin-market-design.md。未知旧皮肤选择仍读取即清理；在线市场包下载/导入不在本期。
- [ ] T-0016 — 官网模块二交互演示右侧布局：将 Alt+2/Shift 交互卡片重构至面板右侧第 3 列空置区（**「模块三/四 GSAP 动效」范围已移交 T-0031，两任务不同时改同一批文件**）
- [ ] T-0019 — 花费面板时段完整性（通用 bug）：今天 / 昨天 / 7 天 / 30 天 / 全部 各时段在「美元」与「tokens」两种口径下都显示非零数据（官网演示数据已修；真机扫描口径待核对）

- [ ] T-0031 — 官网三区块重设计与视觉词汇收敛：**两层，均已实现（2026-10-10 实现，2026-10-11 随 v0.7.0 合并 main 并部署上线）**。**第一层=设计**——why-pane 行类型分道（tokens/quota/balance + 阈值刻度 + 余额端帽）与 Pane 卡视差、生态轨道领域色环 + 悬停名片 + 点击跳转、特性四联重画为「真实面板碎片」（含实时走秒倒计时）、动效开关合一（新 `site/public/motion.js` + localStorage）、编号纪律、网格调参 30→18、排版收敛。site 测试 12/12（干净镜像=CI 口径）；双主题×双端截图已出。4 个设计决策已拍板：特性=真实面板碎片 / 生态=分类着色+名片+跳转 / 汇集=保持 1.9s 自动 / 网格=只调参。T-0016 的模块三/四动效范围已并入本任务。**2026-10-11 已随 v0.7.0 合并 main（SITE_DEPLOY=true，推送即部署）**；交付说明与偏差见 temp/handoff/20261010-214500-T-0031-delivery.md。**第二层=按 HeroUI/React Aria 模型替换（用户指示"能替换的都替换"）**——新增 `site/public/ui.js`（Intl 格式化 / hover warm 协调 / roving 焦点 / 长按 / 开关绑定）：`.chip` 统一 6 处、9 处数字走 Intl、计量条得 meter/progressbar 语义、`.button` 两轴体系、悬停名片补齐键盘+触摸+safe area+delay、设置浮层成真 modal（焦点陷阱/Esc/焦点归还）、FAQ 方向键、demo-controls 成 toolbar、toast/alert 结构、`.frag-key` 成 `dl`、hero-stats 成 list、**跑马灯新增暂停按钮（WCAG 2.2.2 合规）** + 克隆 `aria-hidden`、GitHub 图标链接补可访问名称；并修复全站灰字对比度通病（`--faint` 单 token，双主题实测 0 处低于 WCAG AA，审计工具 temp/scripts/probe-contrast.js）。全量功能探针中英各一次全通过。交付说明（含 8 项未替换理由与 2 处自我修正）见 temp/handoff/20261010-223000-T-0031-heroui-model-pass.md
- [ ] T-0029 — 密钥保险箱 v3 重构（P0 安全，交接包驱动）：Phase 0–2 + 独立评审 + 评审修复 + Phase 5 核心已完成（2026-10-10 实现，2026-10-11 随 v0.7.0 提交发布；用户真机验收走一遍创建/解锁/恢复后本条可收敛）。**同日按用户拍板重做初始化**：两步初始化（① 解锁码输入两遍确认 ② 三道密保），完成后展示恢复码；创建标准=≥6 个字符、无强制组合（2026-10-11 交接及代码核对；解锁永不复查创建策略）；通过后 keyvault_v3_setup 一次性原子建库（码+三题+恢复码单次写盘）→ 恢复码展示+TXT 下载（应用内不做"再次查看"——库里不存码是安全根基，丢文件走"重新生成"）。另有：忘记解锁码重置三路径（答任意两题/手输恢复码×2/上传 TXT，复用 v2 冷却）、恢复码重新生成·停用、改码/换题、status 跨版本误报修复、AAD 合一、canonical JCS 与 serde_json Map 解耦（生产 preserve_order 实锤+手写排序+KAT+parse-tests 镜像）。测试 661:0:9 全绿，npm build/GNU debug exe 过。**本机 keyvault.json 已按用户授权备份（temp/backup-keyvault-20261010/）并删除=模拟新用户**。待做：用户 UI 验收；Phase 3 Windows 事务写入 + v2→v3 迁移；Phase 4 TPM 真机原型；Argon2 64MiB 真机延迟基准；v3 note 编辑（锁定拒绝文案待打磨）。方案：temp/keyvault-v3-handoff/

## In progress

- [ ] T-0032 — Pane 的 agent 可调用面（Skill + **只读**本地 API）：**P0 + P1a 已实现并随 v0.7.0 提交发布（2026-10-11）**——①四个只读诊断端点上线（`httpapi.rs`：/v1/status 版本·uptime·刷新年龄·代理**来源**；/v1/providers 补 status/stale/error/warning + disabled/removed 列表 + counts；/v1/doctor REA 式 checks（失败必带 remediation、进程内不可判定者标 info 不猜）；/v1/sources 花费来源 detected 清单；provider 文本 URL 剥离为 `<endpoint>`（reqwest 错误带完整 URL 的泄漏面），安全姿态不变=loopback+Host 校验+无 CORS+无 token+永不出凭据），实测 21 卡/doctor 抓到真实 sensenova stale/11/15 sources，sensenova warning 已脱敏（`error sending request for url (<endpoint>)`）；②skill 落仓库 `skills/pane/`（SKILL.md 已改写为 API 优先 triage：doctor→providers→symptoms 树 + 4 references），分发三处：install.ps1 末尾 raw 下载（非致命失败）、设置→常规「AI Agent 排查 Skill」按钮（`agent_skill.rs` include_str! 编译进 exe，计划先行+首次 .bak+幂等，agent_skill_plan/install 两命令）、仓库即 `npx skills add Aafff623/pane` 源；③docs/local-http-api.md 补四端点契约。测试 672:0:9（+httpapi 9/agent_skill 2）。已知无害项：providers label 含打码 key 预览（`sk-829…7243`，与既有 /v1/usage 输出一致，非凭据）。待做：用户验收（真机 agent 装上 skill 跑一次 triage）；P1b 写动作+bearer token（后置，交接件第四节）；README/官网提及 agent 自助排查。调研 `temp/research/20261010-pane-agent-surface/`；交接件 `temp/handoff/20261010-223943-T-0032-agent-callable-surface.md`

## Next

- [ ] T-0033 — 花费面板显示全部实测花费（含禁用/已删除 provider）：**已实现并随 v0.7.0 提交发布（2026-10-11，用户 UI 验收清单见本条末尾）**。①**方案 B**：花费面板 6 个视图入口（historyEntries/donutEntries/modelDonutEntries/modelHistoryEntries/spendDetailDayRows/热力图）全部去掉 `!isCardDisabled` 过滤——实测花费与卡片启停解耦。②**数据入口统一归一化**（`main.ts` normalizeProviderSpend/normalizeHistory/normalizeDaily，赋值三处各调一次）：产品归并 `claude-desktop`→**Claude Code**（不同账相加）、`minimax`→**MiniMax Code**（不同账相加）、`kimi`→**Kimi Code**（同账双名，天级取 max 防双计）。③**Kimi id 改名重复计数根治在 Rust 天级做**（`spend_history::range_from` 按天折叠别名，取 max 不相加；实测 history 42 天双 cell 值完全相同、9 天仅 kimi → 单测+真机锚点覆盖；前端行级 max 会丢那 9 天，故必须天级）。④**MiniMax v2 采集修复**（`providers::minimax::collect_v2_usage_events` 读 `~/.minimax/v2/sqlite/runtime-state.sqlite` 的 `local_runtime_token_usage`，snapshot+缓存同 v1；作 **mcode 的天级兜底**——cc-switch 覆盖的天归 cc，缺口天用 v2，防双计；真机锚点 910M ✓）。⑤**DeepSeek Harness 扫描器**（`spend::dsh`：多帧 zstd 流式解 `~/.dsh/sessions/**/session*.jsonl.zstd`，只认 `assistant/message.data.usage`，日归属顶层 `time`；模型名从同文件 `request/context.data.model` 兜底（本机 `deepseek-v4-flash`）→ 已定价不再 unpriced；真机锚点 **926.9M ✓**、定价 30d $0.86）。⑥「本地无计量」边界并入 `spend.info` 文案（三语：Trae CN/豆包无本地 token 台账无法计入）。**测试 674 通过（+dsh 可移植/别名折叠/两个真机锚点），npm build + exe build 过，dev 已重启**；`/v1/sources` 实测 dsh 与 minimax(v2) 均 detected、doctor healthy。**已知诚实项**：mcode 行的 model 在 cc-switch 里是 `"unknown"`（912.7M tokens 无价，cc-switch 面板同样 $0）→ 按 unpriced 呈现，不猜模型；Qoder credits 单读数折算 0 是差分法冷启动（已有 estimated 标记），第二读数后自然出数。**追加（用户 2026-10-11 拍板）**：⑦**Qoder CN / Trae CN credits 折中**——停止 credits→token 折算（删 `qoder_credit_estimate`/`build_qoder_days`/`QODER_CREDIT_TOKENS_ESTIMATE`/折算测试；qodercn 不再进 token/美元统计），花费面板新增「Credits」额外项（新命令 `fetch_credit_extras` + 前端 creditRow + i18n 三语）：**按天逐行罗列每天消耗的 credits**（带单位词），**不随时段 tab（今天/昨天/7天/30天/全部）变化**——用户 2026-10-11 纠正"每天消耗多少就罗列多少，不要放到每个列表下面"；**首日无基线时不列该天**（2026-10-11 用户实机纠正：2500 是卡片周期累计读数、被当成"10-10 当日消耗"是造假；无基线就不产出该天，等下一个读数才有差分）；⑧**发现并修复 credits 台账 version bug**——`CreditFile` 的 `derive(Default)` 让写入的 version=0，而读取 `filter(version == 1)` 静默丢弃整份台账 → **credits 历史从未真正累积**（这才是 Qoder「折算为 0」的真根因，不是交接件推测的"单读数冷启动"）；修为接受 `version <= CREDIT_VERSION` 并原地升级，真机锚点验证 qodercn 记录恢复可读；⑨**dsh 图标**——`src/assets/providers/dsh.svg`（DeepSeek 鲸鱼，官方 harness 同品牌）+ `providerVisuals` 注册。测试 673:0:13。**待做**：用户 UI 验收（面板里 Claude Code/Kimi Code/OpenCode/Grok/AihubMix/DeepSeek Harness 各出现且 Kimi 单行、金额不翻倍；底部「Credits」行显示 Qoder CN 已用 credits；dsh 显示鲸鱼图标）。调研 `temp/research/20261010-spend-hidden-providers-and-kimi-id-dup.md` + `20261010-trae-doubao-dsh-token-sources.md`；交接件 `temp/handoff/20261010-224500-T-0033-spend-hidden-providers.md`
- [ ] T-0003 — 注册表收口：`provider_runtime.rs` 单表驱动 refresh，docs 覆盖基线并入一致性门禁
- [ ] T-0004 — Provider 全量补全（5 批）：Batch 1-4 已提交（共 23 家：10 还原 + MiMo/Trae国际/Qoder国际/Zed + Droid/JetBrains/Groq/HuggingFace/LongCat/sub2api/Mistral/Perplexity + 火山方舟），测试 489→550；**未落地项**：千帆/腾讯TokenHub/华为云/讯飞/360 的额度端点在全部参考库中不存在（需装机抓包，不编造）；CodeBuddy/腾讯需 device-code 登录流（T-0009 型）；通义灵码无现成实现需自研抓包；Batch 5 长尾等用户点单。真实出数验证仍需用户提供各家的 key
- [ ] T-0005 — Analytics 用量面板：先出 `temp/preview/` 原型拍板，再接线（Rust 聚合命令 + 自绘 SVG）
- [ ] T-0007 — 把 4 个模块的 84 个「死测试」挂进 parse-tests harness（`lib.rs`/`tray_projection`/`telemetry`/`httpapi`）
- [ ] T-0008 — 收尾欠账：长墙镜像白名单逐家实测扩展、ru 语言 38 键、`docs/providers.md` 16 节
- [ ] T-0009 — Qoder 配额卡：用户完成一次 device-code 登录后按实测响应实现（端点情报已在 handoff）
- [ ] T-0010 — 发布资产与杂项：`ROADMAP`/`README` 计数、`stash@{0}` 处置、项目 skill 落位与入库、**装机目录自启 exe 换正式构建**（实测为依赖 Vite 的开发构建——无 Vite 时开机自启只显示报错页；换正式版后走应用内更新，不再卸载重装，并在换装后实测一次应用内更新（v0.5.0 的 latest.json 通道首次可验））。install.ps1 下载源已修正到本仓（2026-10-10）

- [ ] T-0012 — 官网模块二重做「使用流程」区块：快捷键（Alt+2/Shift/Ctrl+S/Esc）+ 基本功能（多账号/总花费/额度查询）+ 生态扩展声明（不止 Coding Agent，生产力工具皆可接入）
- [ ] T-0013 — 官网第三模块 provider 图标深浅主题可见性修复（claude/hermes 等暗色看不清；深浅主题下全部可辨，验收双主题逐图肉眼过）
- [ ] T-0014 — 复核官网第六轮成果（艺术字/默认配置卡/字号/keyvault 模拟），按审美与一致性问题重做或收敛
- [ ] T-0015 — 官网首屏深度美化：重新设计艺术字「让每一次调用 / 都看得见」的字形、层次、动效与首屏构图，完成桌面与移动端视觉验收
- [ ] T-0027 — winget 首发收录：manifest PR microsoft/winget-pkgs#449939 已提交（Pane.Pane 0.6.0，nullsoft/user/SHA256 与 latest.json 一致），待官方管道与审核合并；合并后 winget.yml 自动派发才生效（releaser 只能更新已有包）
- [ ] T-0028 — SignPath Foundation 免费签名申请（**用户指示暂缓**）：申请内容底稿已备（`temp/signpath/application-draft.md`，含 8 文本字段 + 4 下拉框值 + 英文长文）；内置浏览器无法可靠填写该跨域 HubSpot 表单（操作原生 select 两次触发渲染器崩溃、表单被清空），需用户在自己浏览器按底稿粘贴并提交；提交前需先办：Code signing policy 文档、下载页 SignPath 声明、遥测装期告知、install.ps1（已修）

- [ ] T-0030 — 更新通道可达性加固（可选评估）：国内备用更新源，候选 ① pane.threetwoa.live 的 Cloudflare Worker ② Gitee 镜像仓（匿名 raw 访问稳定性需先验证，账号需实名）。关键约束：latest.json 里写死的安装包下载地址必须与镜像自身一致（镜像只发清单不发安装包 = 只修 check 腿，下载腿仍卡 GitHub）；插件 endpoints 按序回退，签名校验保证镜像不可篡改。触发条件：再收到无代理用户的更新失败反馈

## Rules

- Task format: `- [ ] T-0001 — <demoable outcome>`
- Optional dependency format: `Blocked by: T-0002`
- Task IDs are immutable and MUST NEVER be reused. `next-task-id` is a monotonically increasing allocator; removing a completed task does not make its ID available again.
- Add or update board state only for changes that matter across sessions: a task is created, enters or leaves `In progress`, materially changes scope, becomes blocked or unblocked, or is validated complete and removed. Do not record agent micro-steps (reading files, planning an edit, partial implementation).
- Remove a completed task only after its acceptance condition is satisfied; if another live task's `Blocked by:` edge points at it, remove that edge in the same board mutation.
- `TODO.md` owns task state. Handoff parcels may explain a task but MUST NOT maintain a second task list or contradict the board.
- Read this file at session start and immediately before any board mutation. All writes follow the TODO optimistic concurrency protocol (see the `handoff-to-agent` Skill); a hash conflict is a reconciliation event, never permission to overwrite newer state.
