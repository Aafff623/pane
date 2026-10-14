# Provider Intel — 订阅档位 / 额度口径 / 峰谷折扣 / 优惠 / 新模型

> **抓取日期：2026-10-14；本轮官方来源复核日期同日。**
> 本轮重点回溯原稿中的待确认项，并纠正与官方页冲突的内容。各项复核边界见第 5 节；未列入复核范围的原稿信息均为待确认线索，不代表本轮已验证。
> 本文件是市场情报，不是 Pane 的适配清单。每条都标注来源 URL；抓不到官方来源的一律写「待确认」，**不编造**。
> 价格为页面原币种原样记录；同一厂商常有「目录价 / 限时优惠价 / 年付价」多个数字，已尽量区分。
> 维护方式见文末第 5 节。与 Pane 的差距见第 4 节。

---

## 一、编程 Agent / 订阅套餐

### ClinePass（Cline）
- **订阅档位与价格**：ClinePass $9.99/月（标准价，促销期后同为 $9.99）。
- **额度口径**：月度订阅，覆盖精选开源权重模型，速率限制为标准访问的 2–5×；无逐 token 账单。
- **峰谷时段**：未提及。
- **优惠福利**：官方的 2–5× 指相对标准 API 限流的用量，不是价格折扣；可能另收处理费。Cline 本体开源，支持 BYOK。
- **新模型与定价变化**：官方列 GLM-5.3 / 5.3 Flash / 5.2、Kimi K3 / K2.7 Code、DeepSeek V4 Pro / Flash、MiniMax M3、MiMo、Qwen3.8-Max 等。
- 来源：https://cline.bot/cline-pass （2026-10-14 官方页复核）

### OpenCode Go / Zen
- **订阅档位与价格**：Go **$10/月**；Go Plus **$40/月**。Zen 为独立的按量/模型目录（pay-as-you-go）。
- **额度口径**：额度以「月度美元额度」定义，每模型有月度上限；同一额度按 **5 小时 = 月度额度 20%、每周 = 50%、每月 = 100%** 刷新。
- **峰谷时段**：未提及。
- **优惠福利**：页面列出 Step 5 Preview 免费（限时）。
- **新模型与定价变化**：Go 覆盖多个开源模型，含 Step 系列等。
- 来源：https://opencode.ai/docs/go/ ；https://opencode.ai/go ；https://opencode.ai/docs/zen/ （2026-10-14）

### Command Code（GOAT）
- **订阅档位与价格**：Go $1/月；**GOAT $10/月**；Pro $20/月；Max 10× $100/月；Max 20× $200/月；Team Pro $40/月。
- **额度口径**：GOAT 官方正文列 **5h $14 / 周 $35 / 月 $70**，但页首宣传为 $80；各模型有不同月度 allowance，不能把它们相加成公共余额。Go / Pro / Max 的原稿额度尚待逐档复核。
- **峰谷时段**：套餐自身滚动窗口与时段无关；仅 DeepSeek 路由的模型继承 DeepSeek 峰谷（见 Pane `peakHours.ts` 注释）。
- **优惠福利**：MiniMax M3 的 GOAT allowance 为 $47；免费模型请求计 $0。DeepSeek V4.1 Flash 的 GOAT/Pro 均为 $100，至 **2026-10-16 07:00 PT**，之后分别 $80/$90；Mistral Large 4 的 Go $10 / GOAT $40 / Pro $50 至 **2026-10-20**，之后 $6/$20/$30。
- **新模型与定价变化**：DeepSeek V4.1 Flash（$100 额度）、GPT-5.6 Sol、GLM-5.2、Tencent Hy3、Qwen 3.8 27B、Claude Haiku 5.5 等。
- 来源：https://commandcode.ai/pricing ；https://commandcode.ai/docs/resources/pricing-limits ；https://commandcode.ai/docs/plans/goat （2026-10-14）

### Cursor
- **订阅档位与价格**：官方旧套餐文档列 Pro **$20/月**、Pro Plus **$60/月**、Ultra **$200/月**；当前 pricing 页为 X Subscription：Free / Plus **$8/月** / Premium **$30/月** / Super **$100/月** / Ultra **$200/月**，未含税。
- **额度口径**：旧套餐文档为按账期重置的 Cursor Models / Other Models 两池。新联合套餐为 Cursor / Grok 共享**周池**；Plus 仅 Grok 模型，Premium 以上第三方模型另按 API 价，不含在订阅池中。两种套餐不能混为一条规则。
- **峰谷时段**：新套餐 Super/Ultra 有高峰优先访问；是优先级，未提供时段折扣。
- **优惠福利**：未抓到官方长期优惠。
- **新模型与定价变化**：官方 Cursor 账号 2026-10-12 公告允许存量订阅保留旧套餐或切换 X Subscription；pricing 页包含 Grok / Origin / Bugbot 与该公告吻合，不能认定为抓取污染。新套餐实际 usage API 字段**待确认**。
- 来源：https://cursor.com/pricing ；https://cursor.com/docs/models-and-pricing ；https://x.com/cursor_ai/status/2109783879490191502 （2026-10-14 官方页面复核；公告由 AIGC Radar 检索定位，原帖未独立抓取）

### Claude（Claude Code）
- **订阅档位与价格**：Free；Pro **$20/月**或 **$200/年**（页面折合 $17/月）；Max 从 $100/月起（5×/20×）。Team Standard **$25/席/月**或年付折合 $20；Premium **$125/席/月**或年付折合 $100，2–150 席。
- **额度口径**：Claude Code 含在 Pro 与 Max；Max 有跨全部模型的**每周用量上限**，另有 5 小时 Session 窗口（Pane 读 `oauth/usage`）。
- **峰谷时段**：Max 有高峰优先访问，未列时段折扣。
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
- **订阅档位与价格**：Free $0 / **Build $20/月**（年付折合 $18）/ **Max $200/月**（年付折合 $180）/ Business $50 每席/月（年付 $45）/ Enterprise 定制。
- **额度口径**：当前页按 API 价提供月度 agent 美元用量：Build $20、Max $200、Business 每席 $20；原稿的 1,500/18,000 credits 与 Max $240 不用于当前套餐换算。追加用量支持 auto-reload 与团队消费上限；结转期限待官方补证。
- **峰谷时段**：未提及。
- **优惠福利**：年付价格如上；Free 支持自带推理，Business 明列 BYOK 与自定义 inference endpoint。
- **新模型与定价变化**：Warp Factories 为另一个产品的计费区，不能混入 Terminal 套餐。
- 来源：https://www.warp.dev/pricing （2026-10-14 官方页复核）

### Devin
- **订阅档位与价格**：Free $0 / **Pro $20/月** / **Max $200/月** / Team $80/月 + $40 每全职席位/月 / Enterprise 定制。
- **额度口径**：当前官方页明确为**每日 + 每周用量配额**，超额可按 API 价购买；原稿 Core/ACU 数字属于旧线索，不能作为当前套餐换算口径。
- **峰谷时段**：未提及。
- **优惠福利**：付费套餐在 Devin Desktop / CLI 可免费用 SWE-2 Free 至 **2026-10-16**；不推断后续继续免费。
- **新模型与定价变化**：新增 Devin Desktop；档位由「Core/Team」重构为 Free/Pro/Max/Team。
- 来源：https://devin.ai/pricing （2026-10-14 官方页复核）

### Kiro（AWS）
- **订阅档位与价格**：Free（50 credits）/ Pro **$20/月、1,000 credits** / Pro+ **$40、2,000** / Pro Max **$100、5,000** / Power **$200、10,000**；付费档均为每席位。
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
- **额度口径**：官方当前免费档为 **500 积分/月**；Lite / Pro / Pro+ / Ultra 额外分别 2,000 / 4,000 / 12,000 / 40,000 积分/月，页面另列会员签到 3,000 积分。
- **峰谷时段**：Pane 记录：每日 08:00–22:00 忙时；22:00–08:00 空闲按模型 0.08–0.36×。
- **优惠福利**：Seed-2.1 模型 **2.5 折**；首月优惠。企业版：团队版 ¥149/席/月、旗舰版 ¥259/席/月。
- **新模型与定价变化**：TraeCode 与 TraeWork 已融合为全新 TRAE。
- 来源：https://www.trae.cn/pricing ；https://docs.trae.cn/enterprise_coming-soon ；https://docs.trae.ai/ide/traework-to-traecode-data-migration （2026-10-14）

---

## 二、大模型 API / 中转

### DeepSeek
- **订阅档位与价格**：纯按量 API。**DeepSeek-V4.1-Flash** 低谷 输入 $0.15 / 输出 $0.6（每 1M，cache-miss）；**DeepSeek-V4-Pro-0813** 低谷 $0.66 / $1.98。
- **额度口径**：按 token；cache-hit 折扣极大。
- **峰谷时段**：**低谷价 = 峰值价的一半**；峰值为周一至周五 **01:00–04:00、06:00–10:00 UTC**，即北京时间 **09:00–12:00、14:00–18:00**；中国法定节假日除外，周末与节假日全天低谷。
- **优惠福利**：当前页未见可单独确认的长期促销；原稿「75% 折扣已转永久」待确认。
- **新模型与定价变化**：当前版本 V4.1-Flash / V4-Pro-0813；官方 Flash 请求名为 `deepseek-flash`，旧 `deepseek-v4-flash` 名仍接受但路由到新 Flash 并按新价计。
- 来源：https://api-docs.deepseek.com/quick_start/pricing/ （2026-10-14 官方页复核）

### Kimi（Moonshot）
- **订阅档位与价格**：国内新会员 **Go / Plus / Pro / Max**；国际 [kimi.ai/membership/pricing]：Moderato $19、Allegretto $39、Allegro $99、Vivace $199。
- **额度口径**：**Kimi Code** 为会员权益内的编程服务（5 小时 Session + 每周窗口，见 Pane 文档）；另有平台 wallet（Kimi API）余额。
- **峰谷时段**：未提及。
- **优惠福利**：未抓到。
- **新模型与定价变化**：K3、K2.7 Code 等（Pane 已按 K3 $3/$15/$0.30、K2.7 Code $0.95/$4/$0.19 计价）。
- 来源：https://www.kimi.com/membership/pricing ；https://www.kimi.com/help/membership/membership-pricing ；https://www.kimi.com/help/kimi-code/membership-guide （2026-10-14）

### Z.ai（GLM Coding Plan）
- **订阅档位与价格**：**2026-07-30 起新售 credits 制**，旧套餐不自动切换；Legacy V2 存量可续费/升级。官方 overview 写起价 **$18/月**；Pro/Max 价格、折扣与年付金额在公开订阅页未抓到，**待确认**，不以第三方价补全。
- **额度口径**：当前个人档 Lite **2,000/10,000**、Pro **12,000/60,000**、Max **28,000/140,000 credits（5h/周）**；5h 消耗后动态刷新，周窗从订阅激活后每 7 天重置。模型用量按 input / cached input / output tokens 分别乘系数，再除以 10,000；Team 与 Legacy 的旧数字不与新个人档混用，待逐版复核。
- **峰谷时段**：**峰值 = 周一至周五 14:00–18:00（SGT，UTC+8）**；当前个人版峰值为标准 credits 费率 **1×**，低谷 **0.5×**，周末全天低谷。GLM-5.3 的 input/cache/output 系数为 **6.9/1.7/24**，Flash 为 **2.3/0.56/8**；不是原稿所写的 3×/1× 时段倍率。
- **优惠福利**：2026-04-30 符合资格的迁移用户保留 50% Migration Support 折扣，**限原有效期内**。
- **新模型与定价变化**：GLM-5.3 / GLM-5.3-Flash 为当前旗舰；充分用低谷可较标准 API 省至 92%。
- 来源：https://docs.z.ai/devpack/notice/usage-revision ；https://docs.z.ai/devpack/overview ；https://www.aipricing.guru/z-ai-subscription-pricing/ （2026-10-14）

### MiniMax
- **订阅档位与价格**：M Plan；Free / Go / Explore / Build 的价格、年付优惠与 credit pack 数量**待确认**。当前 m-plan 页面抓取只返回应用框架，未复现原稿年付数字，因此不继续引用原稿的错误月均换算。
- **额度口径**：官方 FAQ 明确**非视频模型为 5 小时 + 周窗口，视频仅周窗口**；两窗独立、从首次使用开始计时，重置后下一次使用启动新窗。订阅用量不结转；credit packs 有效期 1 年。用量页 / `mmx quota` / `GET /v1/token_plan/remains` 可查。原稿「全部改为周额度」不成立。
- **峰谷时段**：Build 档标注「peak hours 优先访问」（优先级，非折扣）。
- **优惠福利**：Build 档含新功能抢先体验。
- **新模型与定价变化**：M3、M2.7、M2.5；M3.1 Flash Preview（文本，三档均含）；H3 视频（Explore/Build）；Speech 2.8、Music 3.0。
- 来源：https://www.minimax.io/m-plan ；https://platform.minimax.io/docs/m-plan/faq （2026-10-14：FAQ 正文已复核，定价页正文不可得）

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
- **订阅档位与价格**：官方个人版文档搜索摘要可确认有 **Lite / Pro**；正文需 JavaScript，未取得完整价目。原稿「¥8.9 + 半折券」活动价与有效期**待确认**，不用于当前价格展示。
- **额度口径**：Coding Plan 与模型 API 付费是不同产品；**TPM 是每分钟 token 吞吐/限额单位**，不是通用的每 token 单价。原稿 TPM 报价的具体产品和口径待确认。
- **峰谷时段**：未提及。
- **优惠福利**：新用户多重福利可叠加。
- **新模型与定价变化**：官方 Coding Plan 文档搜索摘要含 Doubao-Seed-2.1 系列等；模型价格表与可用范围待正文复核，不沿用原稿 Seed-1.8 的 TPM 价格作为 API token 价。
- 来源：https://www.volcengine.com/product/ark ；https://docs.volcengine.com/docs/ark/coding-plan-personal-plan-overview?lang=zh （2026-10-14；正文抓取受 JavaScript 限制，摘要只能作线索）

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
- **额度口径**：当前官方表列 Standard 平台费 **5.5%**、Business **8%**、Enterprise 可协商；公开抓取未展开 fee 的计算基数，不作「支付 $100 得 $94.50」换算。支持 BYOK。
- **峰谷时段**：未提及。
- **优惠福利**：Standard/Business BYOK 每月 **$25,000 目录价推理**免手续费，超出收 **5%**；不是 $25,000 赠送推理余额。
- **新模型与定价变化**：300+ 模型目录（动态）。
- 来源：https://openrouter.ai/pricing ；https://www.truefoundry.com/blog/openrouter-pricing ；https://openrouter.ai/docs/guides/overview/auth/byok （2026-10-14）

### Groq
- **订阅档位与价格**：**Free → Developer → Enterprise**。
- **额度口径**：官方限速表**按模型区分** RPM/RPD/TPM/TPD，作用域为组织，不能用统一 30/6,000/14,400 套用所有模型。Developer 提供更高限额及 Batch/Flex；具体以账户 limits 页与响应头为准。
- **峰谷时段**：未提及；Developer 档含 Batch 与 Flex 处理。
- **优惠福利**：原稿「Developer 全部 token 25% off、限速统一 ×10」未在官方限速页找到，**待确认**。
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
- **订阅档位与价格**：Max 官方帮助页确认为 **$200/月或 $2,000/年**（年付仅网页）；不能把 $167 当月付价。Pro 的原稿 $17/月尚未在可读取官方正文确认账期，**待确认**；Education Pro **$10/月**需 SheerID 认证。
- **额度口径**：官方帮助列按功能的周/月限制；原稿「Pro $40 / Max $450 Computer credits」未在本轮可读帮助页确认，**待确认**，不与独立 API 余额混用。
- **峰谷时段**：未提及。
- **优惠福利**：教育版 $10/月。
- **新模型与定价变化**：以官网为准。
- 来源：https://www.perplexity.ai/help-center/en/articles/11680686-perplexity-max ；https://www.perplexity.ai/help-center/en/articles/11187416-which-perplexity-subscription-plan-is-right-for-you ；https://www.perplexity.ai/pro （2026-10-14：帮助正文已读，定价页未返回正文）

### Poe
- **订阅档位与价格**：Free / 付费订阅；官方 subscribe 页未返回各档价目，原稿 $4.99–$249.99 档位及 660,000 points 等数量**待确认**，不以第三方变更传闻当当前价目。
- **额度口径**：官方 FAQ 确认为共享 points 池，按 bot 费率扣点；免费用户每天领点，24 小时结束重置且不结转，未在正文给统一点数。预购 points 参考价 **$30/1M**；订阅可购 add-on。
- **峰谷时段**：未提及。
- **优惠福利**：年付折扣与适用档位待确认。
- **新模型与定价变化**：原稿 2026-07-28 从 1M 降至 660k 的说法未找到官方证据，**待确认**。
- 来源：https://poe.com/subscribe ；https://help.poe.com/hc/en-us/articles/19944206309524-Poe-FAQs ；https://help.poe.com/hc/en-us/articles/19945140063636-Poe-Purchases-FAQs （2026-10-14）

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
| 2 | **GLM 2026-07-30 新售 credits 制**，input/cache/output 分别扣费，低谷仍半价 | `peakHours.ts` 的 `zai` 时段与 1×/0.5× 提示和当前个人版官方页一致；尚需核验新套餐接口的单位与解析 | 高。需核实 quota API 的 credits 语义；**不应按原稿错误的 3×/1× 改现有峰谷提示**。 |
| 3 | **MiniMax 非视频双窗、视频仅周窗** | `providers/minimax.rs::parse_remains` 选择 coding 模型行，有 interval 字段才建 Session 行，另解析 Weekly | 原稿「取消 5h」风险已排除。现有 coding 窗口与 FAQ 一致；视频专属展示需求另行确认，未做带凭据请求。 |
| 4 | **新旗舰模型定价**：GLM-5.3/5.3-Flash、DeepSeek V4.1 Flash、MiniMax M3、Qwen3.8、Grok 4.7 等 | `pricing.rs::resolve` 优先查 supplement / LiteLLM / models.dev，再到 built-in；已有 DeepSeek V4、coding-glm-5.3 和 MiniMax M3.1 Flash Preview 的补充及测试 | 高，但**缺少硬编码不等于实际未定价**。先检查真实模型 ID 和运行时目录命中，再决定补价；转售渠道价不可直接当原厂价。 |
| 5 | **GitHub Copilot 2026-06-01 转 AI Credits 计费**（Pro $10 / Pro+ $39 / Max $100） | Pane 读 `copilot_internal/user` 的 credits/quota 与 plan | 中高。需确认新字段是否仍能解析出档位与 credits 余额。 |
| 6 | **Cursor 新联合套餐周池与存量月双池并存** | Pane 文档显示 Cursor Models / Other Models 用量条 | 中高。新套餐实际 usage API 形状待确认，需样本后按真实订阅适配；不能只替换档位名，不能将所有存量账号改成周池。 |
| 7 | **Trae CN 新套餐 + Seed-2.1 2.5 折**（¥49/¥99/¥239/Ultra ¥699） | Pane 只按 credit pack 展示，无折扣信息 | 中。折扣是重要成本信号，建议作为文本行展示。 |
| 8 | **Qoder CN 限时半价**（Pro 10 / Pro+ 30 / Ultra 100 USD/月） | Pane 展示 plan tier + credit 池，无折扣 | 中。 |
| 9 | **OpenCode Go Plus $40 档** | Pane 的 OpenCode Go 卡主要按 Go 档 + `zen/go/v1/usage` | 中。需确认 Plus 档用量是否同口径返回。 |
| 10 | **Kiro 档位与 add-on credits**（Pro $20 起、$0.04/credit、首次升级返 $20） | Pane 有 kiro `nativeCodingPlan` 卡 | 低中。可补充 add-on 余额/折扣提示。 |

> 补充：Warp（Build $20 agent 用量；不再套 1,500 credits 换算）、Devin（每日/周配额）、ClinePass（$9.99）、Command Code（GOAT 5h/week 额度）在 Pane 中均有对应卡片，后续应按真实返回值核对。上述为源码静态比对与适配建议，未实际调用用户账户 API，也未修改 provider 实现。

---

## 五、维护方式

### 2026-10-14 本轮复核记录

- **检索通道**：先通过项目可发现的 `aigc-news.search_documents` 检索 Cursor / MiniMax 套餐线索，再回到官方定价页、官方 FAQ 和官方公告。索引归属：[AIGC Radar · aigcnews.cn](https://aigcnews.cn)。索引摘要用于发现来源，不自动当作官方正文已读。
- **已回溯可读官方正文**：ClinePass；OpenCode Go / Zen（现重定向 Console）；Command Code GOAT / Pricing Limits；Cursor pricing 与 Models & Pricing；Claude 定价页（Team 席位口径）；Warp 定价页；Devin 定价页；Kiro 定价页；Trae CN 定价页；DeepSeek 价格页；Z.ai 变更公告、个人版 overview 与 FAQ；MiniMax FAQ；OpenRouter 定价表；Groq 限速页；Perplexity 帮助页（Max 价格、Education Pro、功能配额）；Poe FAQ；Firecrawl Free/Hobby 与 Brave Search 的免费额度。本轮未复核这些页所链接的每一项促销细则。
- **仍待确认**：MiniMax M Plan 价格及年付优惠（仅抓到应用框架）；Z.ai Pro/Max 价目（subscribe 页未提供可读价目）；火山方舟 Lite/Pro 完整档位/活动价（正文要求 JavaScript）；Poe 各档价格与点数（订阅页未提供价目）；Perplexity Pro 账期价与 Computer credits；Command Code 页首 $80 与正文 $70 的统一口径；Groq 25% 折扣；Qoder 半价活动当前有效期。已明确标注，不能据此改计费逻辑。
- **原稿保留、未完成本轮逐字段复核**：Codex、Copilot、Windsurf、Kimi、StepFun、Qwen、SiliconFlow、Novita、Mistral、xAI、Hugging Face、Tavily、Bocha，以及上列仅局部复核条目的其余字段。这些内容和二手来源均作**待确认线索**，不是本轮确认的当前价格。保留原有章节便于后续增量核实。
- **代码比对边界**：读取 `providerCatalog.ts`、`peakHours.ts`、`docs/providers.md`、`providers/minimax.rs`、`providers/copilot.rs` 和 `pricing.rs`；纠正第 4 节三项错误差距判断。未读取凭据、未调用带用户凭据的厂商接口；接口返回样本与 UI 验收不在本次情报扫描的验证范围。

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
