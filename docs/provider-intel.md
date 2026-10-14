# Provider Intel — 订阅档位 / 额度口径 / 峰谷折扣 / 优惠 / 新模型

> **抓取日期：2026-10-14（全部条目以该日抓取的公开页面为准）。**
> 本文件是市场情报，不是 Pane 的适配清单。每条都标注来源 URL；抓不到官方来源的一律写「待确认」，**不编造**。
> 价格为页面原币种原样记录；同一厂商常有「目录价 / 限时优惠价 / 年付价」多个数字，已尽量区分。
> 维护方式见文末第 5 节。与 Pane 的差距见第 4 节。

---

## 一、编程 Agent / 订阅套餐

### ClinePass（Cline）
- **订阅档位与价格**：ClinePass $9.99/月（标准价，促销期后同为 $9.99）。
- **额度口径**：月度订阅，覆盖精选开源权重模型，速率限制为标准访问的 2–5×；无逐 token 账单。
- **峰谷时段**：未提及。
- **优惠福利**：宣传为「GLM-5.2 等开源权重模型 2–5× 折扣」；Cline 本体开源免费，另有按 token 成本的 Cline credits 与 BYOK。
- **新模型与定价变化**：覆盖 GLM-5.2、DeepSeek、Kimi、MiniMax、MiMo、Qwen 等。
- 来源：https://cline.bot/cline-pass ；https://www.reddit.com/r/CLine/comments/1uiwvpf/ （2026-10-14）

### OpenCode Go / Zen
- **订阅档位与价格**：Go **$10/月**；Go Plus **$40/月**。Zen 为独立的按量/模型目录（pay-as-you-go）。
- **额度口径**：额度以「月度美元额度」定义，每模型有月度上限；同一额度按 **5 小时 = 月度额度 20%、每周 = 50%、每月 = 100%** 刷新。
- **峰谷时段**：未提及。
- **优惠福利**：页面列出 Step 5 Preview 免费（限时）。
- **新模型与定价变化**：Go 覆盖多个开源模型，含 Step 系列等。
- 来源：https://opencode.ai/docs/go/ ；https://opencode.ai/go ；https://opencode.ai/docs/zen/ （2026-10-14）

### Command Code（GOAT）
- **订阅档位与价格**：Go $1/月；**GOAT $10/月**；Pro $20/月；Max 10× $100/月；Max 20× $200/月；Team Pro $40/月。
- **额度口径**（月度 credits，按周期初重置）：Go $10 额度（5h $2 / 周 $5）；**GOAT $70–80 额度（5h $14 / 周 $35）**；Pro $80（5h $16 / 周 $40）；Max 10× $150（$45/$90）；Max 20× $300（$90/$180）。付费 credits 按模型成本计、可结转、不过期。
- **峰谷时段**：套餐自身滚动窗口与时段无关；仅 DeepSeek 路由的模型继承 DeepSeek 峰谷（见 Pane `peakHours.ts` 注释）。
- **优惠福利**：MiniMax M3 额度翻 2×（GOAT 达 $47、Pro $57）；免费模型请求计 $0、不占用额度；Mistral Large 4 额度阶段性提升（Go $10 / GOAT $40 / Pro $50）。
- **新模型与定价变化**：DeepSeek V4.1 Flash（$100 额度）、GPT-5.6 Sol、GLM-5.2、Tencent Hy3、Qwen 3.8 27B、Claude Haiku 5.5 等。
- 来源：https://commandcode.ai/pricing ；https://commandcode.ai/docs/resources/pricing-limits ；https://commandcode.ai/docs/plans/goat （2026-10-14）

### Cursor
- **订阅档位与价格**：官方文档列 Pro **$20/月**、Ultra **$200/月**；第三方（jetadmin、mem0，2026-09）列 Hobby 免费 / Pro $20 / Pro+ $60 / Ultra $200 / Teams $40 每用户。
- **额度口径**：Pro/Pro+/Ultra 含「Cursor Models」与「Other Models」两个用量池；Ultra ≈ 20× Pro。第三方模型按 API 目录价另计。
- **峰谷时段**：Super/Ultra 档标注「peak times 优先模型访问」（优先级而非折扣）。
- **优惠福利**：未抓到官方长期优惠。
- **新模型与定价变化**：页面出现「Grok Build」「Origin 代码托管」「Bugbot」等新功能；**档位命名待确认**（直接抓 cursor.com/pricing 返回的对照表混入了其它产品内容，出现 Plus $8 / Premium $30 / Super $100 等无法核实的分档）。
- 来源：https://cursor.com/docs/models-and-pricing ；https://www.jetadmin.io/blog/cursor-pricing-explained-plans-credit-system-and-real-costs-in-2026/ ；https://mem0.ai/library/llms-and-models/cursor-pricing （2026-10-14）

### Claude（Claude Code）
- **订阅档位与价格**：Free / Pro；**Max 5× $100/月**、**Max 20× $200/月**；Team（价格待确认，官方页抓到 "Team $100"，疑为年付/席位口径）。
- **额度口径**：Claude Code 含在 Pro 与 Max；Max 有跨全部模型的**每周用量上限**，另有 5 小时 Session 窗口（Pane 读 `oauth/usage`）。
- **峰谷时段**：未提及。
- **优惠福利**：未抓到。
- **新模型与定价变化**：页面出现 Opus 4.6/4.7 等；Max 档说明「$100–$200/月 API 额度用于 Claude Platform」。
- 来源：https://claude.com/pricing ；https://support.claude.com/en/articles/11049741-what-is-the-max-plan （2026-10-14）

### Codex（OpenAI / ChatGPT）
- **订阅档位与价格**：Plus（含扩展 Codex 用量）；**Pro $100/月**（最多 Codex 任务、无限 GPT-5.6 与文件上传，受防滥用限制）。
- **额度口径**：本地消息与云端 chat **共享同一额度**，可能叠加**每周上限**；Enterprise/Edu 弹性计费按 **credits** 缩放。
- **峰谷时段**：未提及。
- **优惠福利**：未抓到。
- **新模型与定价变化**：GPT-5.6（Plus 含 Medium/High，Extra High 不含）、GPT-6 相关页面存在。
- 来源：https://chatgpt.com/codex/pricing/ ；https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan （2026-10-14）

### GitHub Copilot
- **订阅档位与价格**：Pro **$10/月**、Pro+ **$39/月**、Max **$100/月**。
- **额度口径**：**2026-06-01 起改为 GitHub AI Credits 计费**——Pro 含 $10/月 AI Credits、Pro+ 含 $39/月；老年度套餐保留「premium requests」（Pro 300/月、Pro+ 1500/月，超出 $0.04/请求）。
- **峰谷时段**：未提及。
- **优惠福利**：部分用户 Pro 免费。
- **新模型与定价变化**：Pro+ 起可用 premium 模型（含 Opus）；年度老套餐模型倍率 2026-06-01 上调。
- 来源：https://github.com/features/copilot/plans ；https://docs.github.com/en/copilot/get-started/plans ；https://github.blog/news-insights/company-news/github-copilot-is-moving-to-usage-based-billing/ （2026-10-14）

### Windsurf
- **订阅档位与价格**：Free $0 / **Pro $20/月** / **Max $200/月** / Teams $40 每席位/月 / Enterprise 定制。年付省约 17–20%。
- **额度口径**：**2026-03 起取消 credits，改为每日 + 每周配额**。按模型分档的每日消息数：Premium Plus（Opus 4.6、GPT-5.4、GPT-5.3-Codex）Pro/Teams 7–27、Max 42–170；Premium 8–101 / 47–631；Lightweight 47–190 / 291–1,190。Tab 无限。
- **峰谷时段**：未提及。
- **优惠福利**：学生约 50% off Pro（≈$10/月）；推荐返 $10 额外用量；超额按 API 价。
- **新模型与定价变化**：Pro 由 $15 涨到 $20；新增 SWE-1.5、Fast Context 等；2026-03 更新期间官方 docs 一度滞后。
- 来源：https://devin.ai/blog/windsurf-pricing-plans ；https://www.cloudzero.com/blog/windsurf-pricing/ ；https://www.verdent.ai/guides/windsurf-pricing-2026 （2026-10-14）

### Warp
- **订阅档位与价格**：Free $0 / **Build 起 $20/月**（含 $20 = 1,500 credits agent 用量）/ **Max 起 $200/月**（含 $240 = 18,000 credits）/ Business $50 每用户/月 / Enterprise 定制。
- **额度口径**：credit 制；Build/Business 含月度 credits，Reload credits 可结转、12 个月有效；旧 Pro/Turbo/Lightspeed 续订时迁移到 Build。
- **峰谷时段**：未提及。
- **优惠福利**：Reload credits 比旧 overage 便宜约 50%；支持 BYOK。
- **新模型与定价变化**：迁移时旧套餐未用价值转入 Stripe 余额。
- 来源：https://www.warp.dev/pricing ；https://www.warp.dev/blog/warp-new-pricing-flexibility-byok （2026-10-14）

### Devin
- **订阅档位与价格**：Free $0 / **Pro $20/月** / **Max $200/月** / Team $80/月 + $40 每全职席位/月 / Enterprise 定制。
- **额度口径**：历史资料以 **ACU**（Agent Compute Unit）计：Core ~$20 起 PAYG、约 $2.25/ACU；Team ~$500/月含 250 ACU（$2.00/ACU）。**与当前定价页档位不一致，ACU 口径待确认。**
- **峰谷时段**：未提及。
- **优惠福利**：未抓到。
- **新模型与定价变化**：新增 Devin Desktop；档位由「Core/Team」重构为 Free/Pro/Max/Team。
- 来源：https://devin.ai/pricing ；https://pensero.ai/blog/devin-pricing ；https://www.usecarly.com/blog/devin-pricing/ （2026-10-14）

### Kiro（AWS）
- **订阅档位与价格**：Free（永久，含 50 credits）/ Pro $20/月 / Pro+ / Pro Max / Power（档位价待确认）。
- **额度口径**：credits 制，按请求分数扣减；可买 add-on credits **$0.04/个**（$5=125 → $100），未用 add-on 12 个月有效；团队版超额 $0.04/credit。
- **峰谷时段**：未提及。
- **优惠福利**：**首次升级任意付费档返 $20**（按剩余天数折算）；Kiro Startup Credits Program 合资格初创可免费获最长 1 年 Pro+。
- **新模型与定价变化**：已 GA。
- 来源：https://kiro.dev/pricing/ ；https://kiro.dev/docs/billing/ ；https://aws.amazon.com/aws-startups/learn/power-your-startup-with-1-year-of-kiro-pro-plus-now-available-through-aws-startups/ （2026-10-14）

### Qoder CN
- **订阅档位与价格**：免费 / **Pro 20 USD/月** / **Pro+ 60** / **Ultra 200**；**限时优惠**：Pro 10、Pro+ 30、Ultra 100 USD/月。
- **额度口径**：个人资源包 **20 USD / 1,500 Credits**；订阅为 credit 池。
- **峰谷时段**：Pane 记录：每日 08:00–22:00 标准价，22:00–08:00 低谷（含周末/节假日）按模型 0.04–0.2×。
- **优惠福利**：上列限时半价优惠。
- **新模型与定价变化**：未抓到官方模型变更。
- 来源：https://docs.qoder.com/zh/account/pricing ；https://qoder.com/zh/blog/qoder-subscriptions-are-here （2026-10-14）

### Trae CN
- **订阅档位与价格**：**¥49/月（首月 ¥39）** / **¥99/月（首月 ¥69）** / **¥239/月（优惠至 ¥219）** / **Ultra ¥699/月（优惠至 ¥629）**。
- **额度口径**：含免费 2,000 积分；credit/积分制。
- **峰谷时段**：Pane 记录：每日 08:00–22:00 忙时；22:00–08:00 空闲按模型 0.08–0.36×。
- **优惠福利**：Seed-2.1 模型 **2.5 折**；首月优惠。企业版：团队版 ¥149/席/月、旗舰版 ¥259/席/月。
- **新模型与定价变化**：TraeCode 与 TraeWork 已融合为全新 TRAE。
- 来源：https://www.trae.cn/pricing ；https://docs.trae.cn/enterprise_coming-soon ；https://docs.trae.ai/ide/traework-to-traecode-data-migration （2026-10-14）

---

## 二、大模型 API / 中转

### DeepSeek
- **订阅档位与价格**：纯按量 API。**DeepSeek-V4.1-Flash** 低谷 输入 $0.15 / 输出 $0.6（每 1M，cache-miss）；**DeepSeek-V4-Pro-0813** 低谷 $0.66 / $1.98。
- **额度口径**：按 token；cache-hit 折扣极大。
- **峰谷时段**：**低谷价 = 峰值价的一半**；「其余所有时段均为低谷，含周末与中国法定节假日全天」→ 即**峰值 = 低谷 ×2**。
- **优惠福利**：V4-Pro 促销 75% 折扣已转永久。
- **新模型与定价变化**：V4.1-Flash / V4-Pro-0813 为当前在售；第三方称 2026-07 一度取消峰谷、随后恢复 2× 峰值（以官方文档为准）。
- 来源：https://api-docs.deepseek.com/quick_start/pricing/ ；https://coworker.ai/blog/deepseek-api-pricing （2026-10-14）

### Kimi（Moonshot）
- **订阅档位与价格**：国内新会员 **Go / Plus / Pro / Max**；国际 [kimi.ai/membership/pricing]：Moderato $19、Allegretto $39、Allegro $99、Vivace $199。
- **额度口径**：**Kimi Code** 为会员权益内的编程服务（5 小时 Session + 每周窗口，见 Pane 文档）；另有平台 wallet（Kimi API）余额。
- **峰谷时段**：未提及。
- **优惠福利**：未抓到。
- **新模型与定价变化**：K3、K2.7 Code 等（Pane 已按 K3 $3/$15/$0.30、K2.7 Code $0.95/$4/$0.19 计价）。
- 来源：https://www.kimi.com/membership/pricing ；https://www.kimi.com/help/membership/membership-pricing ；https://www.kimi.com/help/kimi-code/membership-guide （2026-10-14）

### Z.ai（GLM Coding Plan）
- **订阅档位与价格**：**2026-07-30 起改为 credits 制**（旧套餐停售，存量保留）。第三方列 Lite $18 / Pro $72 / Max $160，另有限时 30% off（$12.60 / $50.40 / $112）与年付 $151.20 / $604.80 / $1,344——**官方新档位价与 credits 数量待确认**（见 z.ai/subscribe）。
- **额度口径**：旧 Legacy V2 按 **5 小时 + 每周** 提示数：Lite ~80/400、Pro ~400/2,000、Max ~1,600/8,000。Team Plan 按席位：Standard 5h GLM-5.3 60M tokens / 周 300M；Premium 5h 160M / 周 800M（Flash 更高）。
- **峰谷时段**：**峰值 = 周一至周五 14:00–18:00（SGT，UTC+8）**。**GLM-5.3：低谷 1×、峰值 3×**；**GLM-5.3-Flash：低谷 0.4×、峰值 1.2×**。**周末全天按低谷计**。
- **优惠福利**：2026-04-30 迁移用户保留 50% Migration Support 折扣。
- **新模型与定价变化**：GLM-5.3 / GLM-5.3-Flash 为当前旗舰；充分用低谷可较标准 API 省至 92%。
- 来源：https://docs.z.ai/devpack/notice/usage-revision ；https://docs.z.ai/devpack/overview ；https://www.aipricing.guru/z-ai-subscription-pricing/ （2026-10-14）

### MiniMax
- **订阅档位与价格**：M Plan 分 Free / Go / Explore / Build。年付（「2 个月免费」）：**Go $264→$220/年**、**Explore $648→$550/年**、**Build $1,560→$1,320/年**（折合月约 $22 / $55 / $132）。
- **额度口径**：订阅额度 + credit packs（$5=5,000、$25=25,000、$100=100,000，有效期 1 年）；用量页 / `mmx quota` / `GET /v1/token_plan/remains` 可查。**社区反馈 MiniMax 已从 5 小时刷新改为周额度（待官方确认）**。
- **峰谷时段**：Build 档标注「peak hours 优先访问」（优先级，非折扣）。
- **优惠福利**：Build 档含新功能抢先体验。
- **新模型与定价变化**：M3、M2.7、M2.5；M3.1 Flash Preview（文本，三档均含）；H3 视频（Explore/Build）；Speech 2.8、Music 3.0。
- 来源：https://www.minimax.io/m-plan ；https://platform.minimax.io/docs/m-plan/faq ；https://www.reddit.com/r/vibecoding/comments/1rxwua3/ （2026-10-14）

### StepFun（阶跃星辰）
- **订阅档位与价格**：**Step Plan 订阅制**：**¥49 / ¥99 / ¥199 / ¥699** 四档。
- **额度口径**：某档含 **40,000 MCredits/月**；Studio 可另用相当于套餐 40% 的创作额度。适配 OpenClaw、Claude Code、Trae、Cursor 等。
- **峰谷时段**：未提及。
- **优惠福利**：未抓到。
- **新模型与定价变化**：Step Code 面向编程。
- 来源：https://platform.stepfun.com/step-plan ；https://platform.stepfun.com/docs/zh/step-plan/overview ；https://platform.stepfun.com/step-code （2026-10-14）

### Qwen（阿里云百炼 / Qwen Code）
- **订阅档位与价格**：**Coding Plan 首月 ¥39.90**（官网目录价 **¥200/月**，后续按 ¥200 续费）；**Token Plan ¥139/月**（超量 88 折），个人用量包 ¥100/月、团队 ¥5,000/月。
- **额度口径**：Coding Plan 按请求数计（滚动 5 小时 + 周 + 月，Pane 已读）；Lite 套餐 **2026-04-13 起停止续费与升级**。
- **峰谷时段**：未提及。
- **优惠福利**：首月 ¥39.90；Token Plan 超量 88 折。
- **新模型与定价变化**：Qwen Coder 分档（32K–128K ¥1.5/¥6 等）；开源 Qwen3.6（35B-A3B ¥1.8/¥10.8）、Qwen3.5 系列。
- 来源：https://help.aliyun.com/zh/model-studio/coding-plan ；https://www.aliyun.com/benefit/scene/tokenplan ；https://help.aliyun.com/zh/model-studio/model-pricing （2026-10-14）

### 豆包 / 火山方舟（Volcengine Ark）
- **订阅档位与价格**：**Coding Plan** 含 Lite 等档，新用户叠加福利后**最低 ¥8.9 购 Lite 并送半折代金券**（活动价，**档位明细待确认**）。
- **额度口径**：Coding Plan 按套餐；模型按 TPM 计价。
- **峰谷时段**：未提及。
- **优惠福利**：新用户多重福利可叠加。
- **新模型与定价变化**：**Doubao-Seed-1.8**（23.04 元/10k 输入 TPM、5.76 元/1k 输出 TPM）、Seed-1.6、Seed-1.6-vision。
- 来源：https://www.volcengine.com/product/ark ；https://zhuanlan.zhihu.com/p/2006651736853586198 （2026-10-14）

### SiliconFlow（硅基流动）
- **订阅档位与价格**：**按量付费，无订阅档**（价格按每百万 token 展示）。
- **额度口径**：余额 / 充值余额（Pane 读 `totalBalance` / `chargeBalance`）。
- **峰谷时段**：未提及。
- **优惠福利**：未抓到官方长期优惠。
- **新模型与定价变化**：以模型页为准（动态）。
- 来源：https://www.siliconflow.cn/pricing ；https://www.siliconflow.com/zh/pricing （2026-10-14）

### Novita AI
- **订阅档位与价格**：按量付费。
- **额度口径**：注册送 ~$0.50 credit；推荐双方各得 $10；部分开源模型 $0/token。
- **峰谷时段**：未提及。
- **优惠福利**：**Batch inference 输入/输出 token 立减 50%**（introductory）。
- **新模型与定价变化**：Kimi K3 $3/$15、K2.7 Code $0.95/$4、Qwen3.8 Flash $0.15/$0.47、Qwen3.8 Max $2/$6、GLM-5.2 $1.40/$4.40、MiniMax M2.7 $0.3/$1.20、DeepSeek V4 Flash $0.14/$0.28。
- 来源：https://novita.ai/pricing ；https://www.aipricing.guru/blog/novita-ai-api-pricing-guide-2026/ ；https://pricepertoken.com/endpoints/novita/free （2026-10-14）

### OpenRouter
- **订阅档位与价格**：Pay-as-you-go；Free 档（50 请求/天）。
- **额度口径**：预付 credits；**购买时收 5.5% 平台费**（模型 token 价透传，无二次加价）；$100 充值约得 $94.50 推理额度；支持 BYOK。
- **峰谷时段**：未提及。
- **优惠福利**：BYOK 有免费额度阈值。
- **新模型与定价变化**：300+ 模型目录（动态）。
- 来源：https://openrouter.ai/pricing ；https://www.truefoundry.com/blog/openrouter-pricing ；https://openrouter.ai/docs/guides/overview/auth/byok （2026-10-14）

### Groq
- **订阅档位与价格**：**Free → Developer → Enterprise**。
- **额度口径**：Free 无卡即可用全部模型，限 30 req/min、6,000 token/min、14,400 req/天（按组织计，多 key 不叠加）；**Developer 档加卡即免费升级，限速 ×10 且全部 token 价 25% off**（无最低消费）。
- **峰谷时段**：未提及；Developer 档含 Batch 与 Flex 处理。
- **优惠福利**：Developer 档 25% token 折扣。
- **新模型与定价变化**：以 console 为准。
- 来源：https://console.groq.com/docs/rate-limits ；https://www.cloudzero.com/blog/groq-pricing/ ；https://pricepertoken.com/endpoints/groq/free （2026-10-14）

### Mistral
- **订阅档位与价格**：Free / **Pro $14.99/月** / Team $24.99 每用户/月（$50 月最低）/ Enterprise 定制；教育档 $50。
- **额度口径**：Pro 含约 6× Free 消息、5× 搜索、40× 图像；Team 含 30GB/用户。**Pro 不含 API 调用**。
- **峰谷时段**：未提及。
- **优惠福利**：**认证学生 Pro $5.99/月**。
- **新模型与定价变化**：Mistral Vibe（CLI/IDE/远程编码）、Mistral Medium 3.5、Codestral（API $0.30/$0.90）。
- 来源：https://mistral.ai/pricing/ ；https://www.cloudzero.com/blog/mistral-api-pricing/ （2026-10-14）

### xAI / Grok
- **订阅档位与价格**：Free $0 / X Premium $8 / **SuperGrok Lite $10** / **SuperGrok $30**（年 $300）/ X Premium+ $40 / **SuperGrok Plus $100** / **SuperGrok Heavy $300**。
- **额度口径**：付费档含**每周用量池**，跨 Chat/Imagine/Voice/Build 共享；Extra Usage Credits 起 $5（仅网页、1 年有效、按标准价计）。API 独立：Grok 4.7 $2/$6 每 1M（500k 上下文），Grok 4.3 $1.25/$2.50。
- **峰谷时段**：Plus/Heavy 档标注「peak load 优先访问」。
- **优惠福利**：SuperGrok 年付 $300（省 2 个月）；Heavy 含 X Premium+。
- **新模型与定价变化**：Grok 4.7（2026-09-21）；Grok Build 功能；Imagine 视频 480p 低价、1080p $15/分钟。
- 来源：https://x.ai/pricing ；https://felloai.com/grok-pricing/ （2026-10-14）

### Perplexity
- **订阅档位与价格**：Pro（页面列 **$17/月**，疑为年付口径）、Max（页面列 **$167/月**）；Education Pro $10/月（SheerID 认证）。
- **额度口径**：Pro 送 $40 Computer credits、Max 送 $450 Computer credits。
- **峰谷时段**：未提及。
- **优惠福利**：教育版 $10/月。
- **新模型与定价变化**：以官网为准。
- 来源：https://www.perplexity.ai/hub/pricing ；https://www.perplexity.ai/pro ；https://www.perplexity.ai/help-center/en/articles/11187416-which-perplexity-subscription-plan-is-right-for-you （2026-10-14）

### Poe
- **订阅档位与价格**：Free（~300 compute points/天，2026-03 由 ~3,000 下调约 90%）；$4.99/月（10k points/**天**）；**$19.99/月（660,000 points/月**，年付 $199.99）；$49.99（~1.65M/月）；$99.99（~3.3M/月）；$249.99（~8.25M/月）。
- **额度口径**：订阅 + 共享 compute-points 计量，按模型费率扣点；add-on points 起 $30/1M。付费档年付约省 17%。
- **峰谷时段**：未提及。
- **优惠福利**：年付折扣。
- **新模型与定价变化**：Premium 档 2026-07-28 由（二手来源）1M 降至 660k points/月。
- 来源：https://www.usagepricing.com/blueprint/poe ；https://www.therundown.ai/tools/poe （2026-10-14）

### Hugging Face
- **订阅档位与价格**：Free / **PRO $9/月** / Enterprise Hub 起 $50 每用户/月。
- **额度口径**：Team/Enterprise 组织含 **$2.00/席位/月** inference credits（可超出 PAYG）；Dedicated Inference 起 $0.033/小时。
- **峰谷时段**：未提及。
- **优惠福利**：未抓到。
- **新模型与定价变化**：以 Hub 为准。
- 来源：https://huggingface.co/pricing ；https://huggingface.co/docs/inference-providers/en/pricing ；https://www.metacto.com/blogs/the-true-cost-of-hugging-face-a-guide-to-pricing-and-integration （2026-10-14）

---

## 三、搜索 / MCP

### Tavily
- **订阅档位与价格**：**Free（Researcher）1,000 credits/月**（无需信用卡）；PAYG **$0.008/credit**；Project 月度滑块 **$30(4,000) / $100(15,000) / $220(38,000) / $500(100,000)**；Enterprise 定制。
- **额度口径**：credit 制（1,000 credits ≈ 1,000 基础搜索或 500 高级搜索）。
- **峰谷时段**：未提及。
- **优惠福利**：学生免费（页面横幅，未验证资格流程）。
- **新模型与定价变化**：无订阅「档位」，为用量制。
- 来源：https://www.tavily.com/pricing ；https://docs.tavily.com/documentation/api-credits ；https://katteb.com/blog/tavily-ai-agent/ （2026-10-14）

### Firecrawl
- **订阅档位与价格**：Free（1,000 credits/月）/ Hobby $19（5,000）/ Standard $99（100,000）/ Growth $399（500,000）/ Scale $749（1,000,000）；年付分别约 $16 / $83 / $333 / $599；Enterprise 定制。
- **额度口径**：credit 制；Scrape/Crawl/Map/Monitor 各 1 credit/页，Search 2 credits/10 结果，Interact 2 credits/浏览器分钟；JSON 格式 +4 credits。**Scale/Enterprise credits 可结转 1 个月**，Hobby/Standard/Growth 不结转。
- **峰谷时段**：未提及。
- **优惠福利**：Free 档 2026 年内由 500 翻倍至 1,000。
- **新模型与定价变化**：新增 Monitor、changeTracking 等格式/能力。
- 来源：https://www.firecrawl.dev/pricing ；https://scrapegraphai.com/blog/firecrawl-pricing （2026-10-14）

### Brave Search API
- **订阅档位与价格**：Free 档取消（现按量）；各档**每月自动赠送 $5 credits**。
- **额度口径**：$5 free credits/月，之后约 **$5/1,000 次搜索**。
- **峰谷时段**：未提及。
- **优惠福利**：$5 月度免费额度（与支付方式无关，自动到账）。
- **新模型与定价变化**：由「免费」转为「赠 $5 + 按量」。
- 来源：https://brave.com/search/api/ ；https://api-dashboard.search.brave.com/documentation/pricing ；https://www.reddit.com/r/openclaw/comments/1rf235r/ （2026-10-14）

### BochaAI（博查）
- **订阅档位与价格**：按量付费，按每月平均日调用量阶梯定价。
- **额度口径**：**Web Search API ¥0.036/次调用**；Semantic Reranker API **限时免费**。
- **峰谷时段**：未提及。
- **优惠福利**：Reranker 限时免费；定价称「为国外同类产品的 1/3」。
- **新模型与定价变化**：以开放平台为准。
- 来源：https://open.bochaai.com/ ；https://market.aliyun.com/detail/cmapi00069848 ；https://github.com/voidmuse-dev/voidmuse/blob/main/doc/zh/bochaSearchConfig.md （2026-10-14）

---

## 四、与 Pane 的比对：「情报有、Pane 未适配」（按价值排序，Top 10）

对照 Pane 的 `src/providerCatalog.ts`、`src/peakHours.ts`、`docs/providers.md`。

| # | 情报项 | Pane 现状 | 价值 / 建议 |
|---|---|---|---|
| 1 | **DeepSeek V4 峰谷定价**（峰值 = 低谷 ×2，其余全时段低谷，含周末/节假日） | `peakHours.ts` 的 `PEAK_RULES` **只有 zai / commandcode / qodercn / traecn**，无 deepseek | 高。Pane 已接 DeepSeek 余额；若用户在峰时跑批，成本提示缺失。建议加 `deepseek` 峰谷规则。 |
| 2 | **GLM 2026-07-30 改 credits 制 + 新倍率**（GLM-5.3 低谷 1×/峰值 3×、Flash 0.4×/1.2×，周末全天低谷） | `peakHours.ts` 的 `zai` 规则仍是「14:00–18:00 记 1×、其余 0.5×」 | 高。与官方新规不符，峰谷提示会误导。建议按新版重写 zai 规则并区分模型。 |
| 3 | **MiniMax 计费窗口疑由 5 小时改为周额度** | `docs/providers.md` 的 MiniMax 卡仍渲染「5 小时 Session + Weekly」 | 高（待确认）。若属实，Session 行会长期停摆。建议核实 `token_plan/remains` 字段。 |
| 4 | **新旗舰模型定价**：GLM-5.3/5.3-Flash、DeepSeek-V4.1-Flash/V4-Pro-0813、MiniMax M3/M3.1 Flash、Qwen3.8、Grok 4.7、Mistral Large 4、Tencent Hy3 | `src-tauri/src/pricing.rs` 的模型表未覆盖这些新模型 | 高。本地 spend 折算会走 fallback，成本不准。建议补定价条目。 |
| 5 | **GitHub Copilot 2026-06-01 转 AI Credits 计费**（Pro $10 / Pro+ $39 / Max $100） | Pane 读 `copilot_internal/user` 的 credits/quota 与 plan | 中高。需确认新字段是否仍能解析出档位与 credits 余额。 |
| 6 | **Cursor 档位变化**（Plus/Premium/Super 等新命名；官方 docs 仍列 Pro $20 / Ultra $200） | Pane 显示 Cursor Models / Other Models 用量条 | 中。档位名与「Priority model access at peak times」等新属性未映射。 |
| 7 | **Trae CN 新套餐 + Seed-2.1 2.5 折**（¥49/¥99/¥239/Ultra ¥699） | Pane 只按 credit pack 展示，无折扣信息 | 中。折扣是重要成本信号，建议作为文本行展示。 |
| 8 | **Qoder CN 限时半价**（Pro 10 / Pro+ 30 / Ultra 100 USD/月） | Pane 展示 plan tier + credit 池，无折扣 | 中。 |
| 9 | **OpenCode Go Plus $40 档** | Pane 的 OpenCode Go 卡主要按 Go 档 + `zen/go/v1/usage` | 中。需确认 Plus 档用量是否同口径返回。 |
| 10 | **Kiro 档位与 add-on credits**（Pro $20 起、$0.04/credit、首次升级返 $20） | Pane 有 kiro `nativeCodingPlan` 卡 | 低中。可补充 add-on 余额/折扣提示。 |

> 补充：Warp（Build $20/1,500 credits）、Devin（Free/Pro/Max/Team 重构）、ClinePass（$9.99）、Command Code（GOAT 5h/week 额度）在 Pane 中均有对应卡片，但档位口径随厂商频繁变动，建议纳入定期核对。

---

## 五、维护方式

### 下次扫描要更新的字段（逐厂商）
1. **价格与档位**：目录价、限时优惠价、年付价三者区分记录。
2. **额度口径**：属于 5 小时 / 周 / 月 / credits / 余额 中的哪几种；刷新锚点（订阅日 vs 首用）。
3. **峰谷时段与倍率**：起止时间、时区（UTC+8）、周末/节假日是否低谷、按模型的倍率。
4. **优惠福利**：限时折扣、学生/初创/迁移折扣、返利、活动截止日。
5. **新模型与定价变化**：新旗舰模型 ID 与 input/output/cache 单价、上线日期、旧模型退役日期。
6. **失效标记**：过期条目加日期戳并在下次扫描删除，避免堆积。

### 稳定信息源清单（优先官方定价页 / 文档）
- 编程 Agent：cline.bot/cline-pass、opencode.ai/docs/go、commandcode.ai/pricing、cursor.com/docs/models-and-pricing、claude.com/pricing、chatgpt.com/codex/pricing、github.com/features/copilot/plans、windsurf.com（+ devin.ai/blog）、warp.dev/pricing、devin.ai/pricing、kiro.dev/pricing、docs.qoder.com/zh/account/pricing、trae.cn/pricing
- 大模型/中转：api-docs.deepseek.com/quick_start/pricing、kimi.com/membership/pricing、docs.z.ai/devpack/（overview + notice/usage-revision）、minimax.io/m-plan、platform.stepfun.com/step-plan、help.aliyun.com/zh/model-studio/coding-plan、volcengine.com/product/ark、siliconflow.cn/pricing、novita.ai/pricing、openrouter.ai/pricing、console.groq.com/docs/rate-limits、mistral.ai/pricing、x.ai/pricing、perplexity.ai/hub/pricing、poe 订阅页、huggingface.co/pricing
- 搜索/MCP：tavily.com/pricing、firecrawl.dev/pricing、brave.com/search/api、open.bochaai.com
- 交叉校验（二手，仅作线索）：aipricing.guru、usagepricing.com、pricepertoken.com、cloudzero.com、felloai.com —— 只用于发现变化，最终以官方页为准。

### 执行约定
- 每次扫描在文件顶部更新「抓取日期」，逐条保留来源 URL。
- 抓不到的字段写「待确认」，**不推测填充**。
- 厂商档位/倍率变动频繁（本文件已记录 DeepSeek、GLM、MiniMax、Windsurf、Copilot、Cursor 的近期变化），建议按季度重扫，或在这些厂商发版/调价公告出现时增量更新。
