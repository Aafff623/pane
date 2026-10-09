# Changelog

## 0.6.0 — 未发布

Provider 目录补全 · 前四批 23 家 / Kiro 免 spawn / 主密码 Q&A 恢复 / 「密钥与额度」机制说明

## 新增

- **主密码 Q&A 恢复（忘记密码兜底 + 二次验证）**：设置主密码后立即引导配置恢复问题——默认且推荐 2 个（内置 8 个问题可选：最喜欢的水果/最常用的编程工具/第一只宠物的名字等，也可自填），答案需输入两次确认；数量可在 1~5 个间调整，减到 1 个时弹窗说明"单个答案更易被身边人猜中"劝留 2 个；忘记主密码时在锁定状态点「忘记密码？」连续答对全部问题即可设置新密码（旧密码不出前端、库整体重封、答案不区分大小写与多余空格）；连续答错 5 次进入 5 分钟冷却防暴力猜测；更换主密码后恢复问题保持有效（密封自动轮转）。磁盘上问题为明文（锁定时显示挑战）、答案与主密码全部密封（答案各自 Argon2id 加盐 + AES-256-GCM，随机恢复密钥三路封存）
- **「密钥与额度」页管理升级**：主密钥入口移到页首右上角（添加/切换主密钥、锁定/解锁、恢复问题、忘记密码一处可达）；每行「查看」旁新增「复制」（同一主密码验证，明文不上屏）；添加/替换密钥改为「先测试、后保存」——粘贴 key 后必须通过「测试连通性」才解锁保存，改动输入即重新校验；本机登录类（Copilot / Antigravity / Claude / Kiro 等）行内显示绿色「已连接」并写明来源（读取哪些本地文件/配置）；按钮按语义配色（删除红、复制绿、问号灰）且字号整体 +1px
- **全量多账号**：37 家 API-key provider 全部支持子账号（添加账号、独立出卡、重命名/归档），不再限 8 家；已有单 key 下次刷新自动迁移为「账号 1」，无需手动操作；本地登录类仍读单一登录态
- **快捷键管理重做（对齐 VS Code / Raycast 惯例）**：统一行布局修掉「切换分类」错位；快捷键以键帽呈现，点击即录制、Esc 取消、Backspace 清除、↺ 恢复内置默认、× 清空后重录；冲突全行互查并红字提示
- **10 家 Provider 还原（第一批）**：`amp` · `bedrock` · `chutes` · `deepgram` · `openai-api` · `poe` · `venice` · `vertexai` · `warp` · `kiro` 恢复接入（Rust adapter、品牌图标、三语文案、环境变量与「测试连通性」全链路）；默认全部禁用，需在设置「密钥与额度」逐家启用；Bedrock / Vertex AI 读本机 AWS / gcloud 凭据（无凭据时显示未配置属正常），其余 7 家粘 key 即用
- **4 家新 Provider（第二批）**：`mimo`（小米 MiMo，粘贴平台 serviceToken 或 API key 二选一——token 报月度用量、key 报余额/Token Plan，自动识别）、`trae`（Trae 国际版，读本机 `%APPDATA%\Trae` 登录态，host 随账号区域）、`qoder`（Qoder 国际版，读本机 `com.qoder.app.stable` 登录态打 `openapi.qoder.sh`）、`zed`（纯本地：只读 Zed 编辑器 `threads.db`，统计 zed.dev 托管模型的全部/近 30 天 token 与近期模型 Top3，BYOK 线程不计）；四家同样默认禁用；`zstd` 依赖引入（解压 Zed 线程数据）
- **8 家新 Provider（第三批）**：`droid`（Factory：billing/limits 的 5h/周/月窗口 + 旧版订阅用量回退 + 加购余额行）、`jetbrains`（纯本地读 IDE 的 AIAssistantQuotaManager2.xml——月度积分进度、加购池、续期重置，多 IDE 取最新）、`groq`（公开 API 响应头里的每日请求/token 预算）、`huggingface`（whoami 身份 + 当月推理账单，计费 = max(0, 已用-包含)，403 给出权限指引）、`longcat`（粘贴网页 Cookie：token 包摘要优先、旧版 tokenUsage 回退、燃料包副行）、`sub2api`（自建网关：自填部署 URL + 分组 key，订阅窗口/配额/钱包三形态 + key 级请求与 token 统计）、`mistral`（admin Cookie：当月 token 合计 + Vibe 用量 + 信用钱包）、`perplexity`（会话令牌：信用额度按 经常→已购→赠品 顺序归因）
- **火山方舟（第四批）**：`volcengine`——粘贴 `AK:SK`（ark.volcengine.com 的 API Key 账号），V4 签名直译（HMAC-SHA256 签名链，与 Bedrock 同构共用 hmac/sha2）；Agent Plan 的 AFP 5小时/周/月窗口与 Coding Plan 的分层窗口双探测，任一返回即出卡
- **Kiro 直读本地库**：不再调用 `kiro-cli`（旧通道每次调用触发 CLI 自更新、向 %TEMP% 堆安装包）——改为只读打开 `%LOCALAPPDATA%\kiro-cli\data.sqlite3`（`KIRO_DATA_DIR` 可覆盖）取登录 token 与 profile ARN，直连 CodeWhisperer `GetUsageLimits`；额度卡新增 Overage（超额已用/上限）与超额计费行，bonus 花费并入计划用量时明确标注；未知 region 的 ARN 拒绝请求而非把 token 发往猜测端点
- **「密钥与额度」机制说明页**：覆盖全部 provider 家族——每行新增「?」，弹窗讲清该家取数机制（读取哪些本地文件/数据库、按优先级读哪些环境变量、查询哪些域名）、按认证形态给出「你要提供」步骤（粘贴 key / OAuth 登录 / 本地登录），底部附本次实测实时指标；本地登录类（Claude / Kiro / Trae / Qoder / Vertex AI 等）不再被要求粘 key
- **解析测试补齐**：第一批 24 条 + 第二批 10 条 + 第三批 16 条 + 第四批 5 条（MiMo 三形态、Zed threads.db zstd 往返、火山签名链与双计划解析、Perplexity 三段归因、Sub2API 三形态等），provider 解析测试 489 → 550
- **官网中英双语**：导航、Hero、下载、使用说明、快捷键、批注、服务商与 FAQ 全站切换，英文为重新撰写的专业文案；`/en` 路由由 Cloudflare Worker 指向同一文档、前端应用英文
- **官网主视觉与演示升级**：Hero 右侧三素材自动轮播（圆点切换、`prefers-reduced-motion` 降级）＋「看得见」艺术字重做（渐变/字距/响应式，消除断裂与下划线残影）；看板演示批注系统按面板模式（概览/花费/设置/刷新）切换、卡片碰撞避让、连接线避让后重算，第 06/07 条绑定真实的设置齿轮与刷新/状态栏；服务商双轨道相向循环滚动（完整复制避免接缝）；下载区渐变光晕与页脚收口

## 变更

- **`openai` 家族更名 `openai-api`**：按 Admin key 口径正名（显示名 "OpenAI API"，环境变量 `OPENAI_ADMIN_KEY`），避免与 Codex 订阅卡混淆；`hmac` 依赖引入主 crate（Bedrock SigV4 签名用，后续火山方舟 V4 签名共用）
- **Qoder CN 积分花费估算归位 `qodercn` 家族**：花费面板的「Qoder」积分估算卡更名为「Qoder CN」（数据源本就是 CN 版积分账本）；`qoder` 家族让位给国际版配额卡，两个版本不再共用一个 family
- **官网显示名统一 MiniMax Code**：官网服务商列表与演示面板中 `mcode` 显示名由 MaxCode 改为 MiniMax Code（Pane 应用侧显示名待同步）；官网亮暗主题与内嵌演示面板主题分离，切换官网主题不再连带切换演示面板

## 修复

- **自定义看板排序收严**：主页的自定义分组（用户自建的「鸡蛋/羊毛」等标识）只作用于主页看板——自定义看板恢复全程严格 A–Z 字典序、不再渲染分组标题，启停与配置入口的位置稳定可预期
- **打开设置先闪英文**：语言偏好本地缓存并在首帧同步恢复，设置窗口不再先显示英文再切中文
- **暗色模式图标可见性**：Factory / Qoder / Novita / CommandCode / ClawsGO 等近黑品牌标在暗色下渲染为白色剪影，Groq 白标在亮色下转黑——设置页、看板、托盘表现一致
- **官网视觉修复**：亮色模式图标发白、Claude 图标改回品牌橙、窄屏标题与描述溢出、批注箭头在卡片移动后指向旧位置

## 0.5.0 — 2026-10-07

设置与密钥管理 / 品牌换代与官网上线 / 多账号与长墙镜像 / 花费精度与更新推送

## 新增

- **密钥保护（系统凭据库 + 查看验密）**：provider 主密钥与全部子账号密钥迁入系统凭据库（Windows 凭据管理器等，经 keyring 库）——JSON 文件不再明文落盘，旧明文在首次读取时自动迁移（写入成功才删源，失败保留文件回退）；设置「密钥与额度」页已配置的行新增「查看」按钮，需主密码验证后才显示明文；保险箱状态栏新增「更换密码」（先验旧密码）
- **设置中心独立窗口**：⚙ / `Ctrl+S` / 托盘右键→设置 打开独立设置面板，顶部胶囊目录 7 分类（常规/密钥与额度/快捷键/通知/隐私/网络/关于），静态卡片分区+发丝线行，配置跨窗口实时同步；悬浮窗内嵌设置页废弃
- **设置密钥页重构**：「密钥与额度」页从单一手填保险箱升级为两张卡——「Provider 密钥」列出全部可管密钥的 provider（品牌图标 + 状态 + 行内添加/替换/移除密钥，多账号家族含子账号重命名/删除/新增）；「MCP / 搜索密钥」为原保险箱并补齐品牌图标；面板顶部 7 个分类页签同时补齐图标
- **托盘菜单重做**：托盘右键直出各 provider 状态行（≤8 条，读本地快照零请求）+ 设置入口
- **开机品牌动画**：弹出主窗时播放 GSAP 时间轴（卡片落定→环描边→指针扫→字母渐入），同一开机只播一次（`boot_id` ±60 秒容差防校时误重播）；设置→常规可开关/重播；系统减动效时停格成片帧后正常退场
- **测试连通性**：卡片菜单「⚡ 测试连通性」对未启用的卡也可测（`test_provider` 命令），状态栏显示 `连通正常 · N ms` 或失败原因
- **额度查询说明**：卡片菜单「? 额度查询说明」按认证形态自动给出 key / OAuth / 本地 三种步骤模板与控制台直链，内嵌「立即测试」
- **子账号备注两级拆分**：「备注 Provider 名称」只改卡片标题，「备注子账号名称 · 账号名」只改该账号；≥4 账号的紧凑胶囊优先显示备注文本；默认主账号的子备注存合成键 `family@__default__`，不再串号
- **子账号管理菜单**：右键账号胶囊可 ★ 设为默认主账号（总览优先显示它）、⬇ 归档 / ⤴ 恢复账号（凭据随档，覆盖 API key、Antigravity、Cursor 三类账号存储；归档记录不再回传 `api_key`）
- **多账号总览折叠**：总览默认收起为 主账号环+用量条+N/M 徽章，点 caret 展开（`overviewExpanded` 记忆展开态）
- **长墙镜像**：周/月额度打满时，短周期环直接镜像墙状态（100% + 刷新倒计时），消除「5h 环还是绿的但请求全被拒」的假象；首期白名单 kimi，其余家族逐家实测后扩展（Codex 有 banked credits 明确排除）
- **Kimi 额度耗尽解释**：Session/Weekly 打满时详情显示 `Weekly limit reached · resets in Xd Yh`，不再是空详情的裸红环
- **Qoder/Trae 积分趋势**：卡片显示积分消耗趋势（本地账本 `credit_history.json`：日消耗差值、跨天空档整笔归下一观测日、重置读数下跌计 0、重置日按前后两段计费）
- **Qoder token 估算**：Qoder 按任务扣积分、官方无 token 公式，按 25,000 tokens/积分 的标注假设折算并带「估算」徽章，单点可调
- **Qoder CN 加购桶读取**：用量读取补齐顶层 `addOnQuota` 桶（加购/赠送积分包），并入聚合分母并出独立「Add-on credits」行——买了资源包或领了活动积分不再隐身于满额红环之后
- **Qoder CN 自动签到（默认关）**：每个领取窗口自动领取每日 100 积分福利（领取幂等，重复请求返回 `replayed:true` 不会多领）；设置→常规留开关，卡面一行轻量状态（已领/已领过/无活动/失败原因），失败 30 分钟后自动重试
- **花费卡 ⚡ 分档渐变**：按日 token 量分 100M/250M/500M 三档渐变（7d/30d 按天数等比放大），最高档呼吸光效；阈值可在设置自定义（`spendIconTiers`）
- **主页面滚动记忆**：`Alt+2` 唤回后停在上次滚动位置（`mainScrollTop` 防抖持久化）
- **总花费横栏更新推送指示器**：检测到新版本时横栏出现绿色圆形 ↑ 按钮（Cursor 式轻量提示，不弹窗打断）；点击即原地开始下载——圆环按真实字节进度（`update-progress` 事件）填充，完成后自动安装并重启到最新版；失败落在底部状态栏
- **抽屉搜索**：Customize 抽屉顶部搜索框按名称/family 子串就地过滤，分组头随空隐藏
- **Provider 删除**：provider 设置面板（齿轮）与卡片菜单新增「删除此 Provider」——从自定义看板与主页看板一并移除（凭据保留）；自定义看板底部「已删除的 Provider」一行随时可恢复
- **实验性功能开关**：皮肤市场默认隐藏，开启需二次确认，关闭即热切回原生皮肤（`experimentalFeatures`）
- **新装默认精简**：首次安装仅默认启用 8 个主流家族，其余 33 家偏门默认禁用（`DEFAULT_DISABLED_PROVIDERS`，老配置不受影响）
- **官网正式上线**：`https://pane.threetwoa.live`（Cloudflare 自定义域）；首屏交互演示由真实前端打包驱动——9 个快捷键演示（`Alt+2` / `Esc` / `Shift` / `Shift+1` / `Ctrl+S` / `Ctrl+E` / `Ctrl+L` / `Ctrl+R` / `T`，点击与按键均可触发）、深浅主题切换、移动端布局；「下载 Windows 版」点击直接取最新安装包（实时解析 GitHub 最新 Release，失败回退 Release 页面）；站点当前为中文单语，英文版后续再做

## 变更

- **品牌标统一**：以唯一矢量源 `pane-mark.svg` 重新生成全套图标（托盘/exe/安装器/官网 favicon/面板 logo），终结 exe 浅蓝圆环与官网深色仪表盘两套标识并存的漂移
- **详情卡头部重排**：名称按实测拟合自动缩字号（17px 起步、下限 11px），健康点+百分比与齿轮/折叠/星标/刷新按钮浮层化（悬停淡入不占宽）；容器宽 <330/290/250px 三档渐进降级，被收纳的信息全部进悬浮提示
- **置顶星标收敛**：账号 tab 行的星标全部移除，卡片头部常驻一颗，作用于当前展示账号
- **Custom Balance 更名 Custom Relay**：内置中转家族正名（Rust/TS 目录、界面、存储共 9 处同步）；抽屉行说明「自定义可增删改，内置只能启停」
- **设置页「额度」改名「密钥与额度」**（三语）：密钥保险箱在该页内，名实相符
- **齿轮按钮文案**：抽屉 provider 行的齿轮 tooltip「配置」→「设置」（三语）
- **品牌图标补齐**：新增 codebuff/elevenlabs/kilo/relaybalance/sensenova，traecn 换官方标，shandianshuo/clinepass/tavily/firecrawl/apigoto 补齐专属矢量标
- **分类行截断**：总览分类 chip 超宽省略号截断，设置可切两行展开（最多 6 个+…）
- **详情卡默认展开**：仅显式折叠的卡收起；折叠卡点空白区=展开+居中+高亮
- **平滑滚动兼容**：系统关闭「动画效果」时 WebView2 的 smooth 滚动整体空转，全库 8 处滚动点统一探测/降级
- **GLM V1 Pro 移除**：自接入的中转 provider（familyId `linkso`）从正式版全链路删除——Rust adapter 与目录、多账号分发、前端目录/图标/峰谷规则、官网 provider 列表与在线演示（`site/` 演示包已重新生成）；老配置里遗留的 `providerOrder`/`disabled` 条目由新增的「目录已不存在的 family 不渲染」规则兜住，不会留下无图标空白卡；历史 CHANGELOG 与旧凭据不动

## 修复

- **Antigravity 花费口径修正**：解码四字段正名（system / input / output / cacheRead——此前把输入按输出价计费、缓存重读整段漏算）+ 改按事件时间戳归日（不再把跨天对话压进一天）。修正后面板 30 天美元下降约一半、token 量级升至约 10 倍，均为读数修正结果而非数据丢失；价格体系版本 +1，历史美元全量重算
- **自定义看板面板布局错乱**：provider 设置面板的列布局误继承行内居中（按钮全部居中悬浮、字段收缩），长凭据胶囊跨行碎裂——已改为左对齐整列布局 + 胶囊整行换行
- **花费美元计价两处修正**：codex GPT-6 系（`gpt-6.1-sol`/`gpt-6-sol`/`gpt-6-luna`/`gpt-6-astra`）按官方价目表补齐 >272K 整单升档（输入/缓存×2、输出×1.5，此前 supplement 平价卡无档位字段导致长会话单向下偏低报）；`MiniMax-M3.1-Flash-Preview` 官方从未发布 per-token 价，按上代 M3 卡（$0.30/$1.20/$0.06，标注估算）兜底，不再 $0 记账（本机 33.7M tok 实测 +$2.78）。价格体系版本 +1，历史美元全量重算；Antigravity 的 `gemini-3.8-flash-n` 查无官方 SKU，维持 token-only 不编价
- **滚动死界面根因**：全局滚轮兜底把抽屉/皮肤市场/设置的滚动全部抢去滚主列表，收窄为仅主列表命中时接管
- **总览点击不跳转**：与平滑滚动空转同根因，此前系统关动画时总览跳转大面积失效
- **右键菜单飞远**：窄悬浮窗内菜单被推到窗口另一头，改为鼠标落点定位+放不下上翻+边缘钳制
- **头部假 0%**：纯文本指标的卡头部显示「● 0%」，无核心进度指标时改为隐藏（折叠徽章显示 `—`）
- **积分账本重置日计费**：日内重置会把当天消耗整段记丢，改为同时记录峰值与末值，按重置前后两段计费
- **账号存储原子写**：7 处裸 `fs::write`（含 OAuth/refresh token 存储）换 tmp+rename 原子写，杜绝进程被杀/断电时的截断窗口
- **趋势日期暗色不可见**：误用表面色变量，改回正文弱化色
- **详情卡头部溢出**：Copilot 等长名称卡的图标/按钮被挤出卡片，头部改 `min-width:0` + 控件 `flex:none`
- **隐藏设置层拦截点击**：隐藏面板不再吃点击事件；语言下拉关闭后不再残留展开态
- **浅色对比度补漏**：置顶头部/警示徽章/总览 meter 三档等 6 处补浅色主题变体
- **Command Code 卡片三处**：点击被「有额度优先」切回别的账号、合并卡沿用空账号布局、手动刷新丢失折叠态
- **暗色主题下「卡片分组」下拉发白**：展开的原生 `<select>` 菜单由 WebView2 宿主按 `PreferredColorScheme` 绘制，不理会页面的暗色主题与 `color-scheme`——系统浅色 + Pane 暗色时必然渲染成浅底浅字、几乎不可读（实测把宿主主题同步过去也无效）。改为自绘菜单，复用卡片分组菜单的玻璃样式与定位，两套主题下都跟随页面；同时保留宿主窗口的主题同步（跟随系统时传 null 释放回 Auto，避免污染 `prefers-color-scheme`），滚动条等其它原生 UI 一并对齐
- **「5 小时窗口即将重置」开关拨了不生效**：该键从未进入 `set_config` 的白名单，前端每次保存都被静默丢弃（只留一行 `ignoring unknown key`），界面显示与后端行为长期不一致——补入白名单后可正常保存
- **卡片名与折叠箭头间距过窄**：仅给箭头加 5px 左间距（原 7px 是名称/图标/箭头共用的 flex gap，直接调大会连图标一起撑开），视觉间距约 12px
- **Bocha 品牌图标补齐**：此前是「博」字占位圆，换为按官方标复刻的矢量图——等粗单线条「b」（笔画宽 = 字高 20%，外圆 R 与内孔 r 同心）+ 两道浅蓝羽翼（`#A5CCFF`/`#C5DEFF`）+ 气泡尾点，主色 `#006EFF`
- **官网演示花费面板全零**：演示数据在今天/昨天/7 天/30 天各时段的美元口径恒为 0，补齐后各时段 × 美元/tokens 两种口径都是非零数据（示例：今天 $19.98 / 209.5M，30 天 $359.64 / 3.8B）
- **官网移动端留白与两处演示崩溃**：演示面板与快捷键卡两侧留白 20px（此前贴边导致无法滑动），手机上浮动批注改为面板下方 6 张静态介绍卡；演示桥接层补 `setTheme`（此前演示卡在 Starting… 不动）、演示配置补 `removedProviders`/`qoderCheckin`（此前自定义抽屉一点就崩）

## 0.4.69 — 2026-10-05

大面板与授权中心 / StepFun·SenseNova 登录根治 / 全机花费账本 / 发布流程规范

## 新增

- **运行时大面板窗口**：同一 bundle 上新增 cc-switch 形态的大面板（双形态 PoC），设置中心与授权中心迁入面板左侧导航，跨窗口配置实时同步
- **cc-switch 式设置中心 + 完整托盘菜单**：大面板内的分节设置视图；托盘菜单补齐全量入口
- **授权中心视图**：按家族分组的账号行（头像+名称+额度列+⋯菜单），聚合 OAuth/捕获/导入的多账号管理入口
- **StepFun Step Plan 重写**：凭据升级为三元组（`token/账号/密码`，兼容旧 `{apiKey}`）；L0-L3 授权阶梯（JWT 预检→RefreshToken 单飞轮换→密码三步登录），HTTP 429 永不上梯；解析器同时支持 5h/周窗口与 Credit 月池双形态，30 分钟死局破解
- **SenseNova 浏览器一键授权（PKCE）**：设置内两步式授权块（S256 + state），`oauth_finish` 接受回跳 URL/令牌 JSON/裸 JWT/纯 code 四种输入；`refresh_token` 持久化+轮换链路实测跑通，3 小时过期自动续期，永不再粘
- **Z.ai 双域名回退**：`api.z.ai`（Bearer）凭据被拒或无指标时自动落 `open.bigmodel.cn`（裸 key）；凭据错误按 HTTP 200 + `success:false` 响应体判定（实测智谱网关行为），网络错误/5xx/429 不降级
- **每日花费热力图**：总花费条详情入口，ZCode 风格 26 周日对齐方格 + 月份标签 + Less/More 图例，悬浮显示 $/token/工具数，单日下钻按工具/模型展开
- **ZCode / MaxCode 品牌图标**：花费列表与图例使用官方品牌图标
- **密钥保险箱主密码加密**：Argon2id 派生 AES-256-GCM 封存每个 key（服务名/标签/备注保持明文可见）；设一次密码，解锁查看/复制（逐行显隐），手动或重启上锁；key 支持用户备注
- **自动更新根治**：release 以本项目自有 minisign 私钥签名（公钥内嵌），更新器读本仓库 GitHub Releases `latest.json`，新版本弹窗一键安装+重启；上游更新端点弃用
- **官网一期**：`site/` 子项目，Magpie 式零依赖静态站 + Cloudflare Worker + 测试与 CI

## 变更

- **Claude 花费以 cc-switch 本地账本为主源**（`~/.cc-switch/cc-switch.db` 只读）：中转清零 CLI 日志的场景按真实 token 记账（自扫描曾低估 7~24 倍）；Pane 自扫描仅补 cc-switch 未覆盖的天，按天粒度合流零重复
- **新增 cc-switch 独有花费卡**：MaxCode（mcode）、Gemini、Claude Desktop；pi 并入 Claude 卡；成本始终走 Pane 自有价格表
- **总花费头部精简**：删除与下方重复的「今日/本周/每月」胶囊组，⚡ 数字直接跟随「今天/昨天/7天/30天/全部」当前选择
- **版本号移位**：从独立设置行改为设置页底部的弱化页脚
- **总览横栏单行化**：分类 chips 与「5h/7d/月」胶囊同行（分类区窄窗内横滚），「MCP / 搜索」文案精简为「MCP 搜索」
- **ZCode 计量修正**：`input` 已含 cache 读写，总量= input+output（不再虚增），成本按非缓存部分计费；spend_history v2→v3 迁移

## 修复

- **Command Code 家族状态聚合**：全部账号不可用时，主卡不再误报「需重新登录」——改为显示家族内重置时间最早的健康账号（红环+重置倒计时=待刷新）；账号彻底失效时直白话报错 `subscription inactive or expired`（不再抛形状解析错误）
- **SenseNova 设置粘贴保存链路**：通用 `{apiKey}` 粘贴格式此前写入后 provider 读不回，已加 serde 别名兼容
- **总览分类行窄窗截断**：chip 内边距与行距压缩，最窄窗口下「MCP 搜索」完整显示

## 0.4.68 — 2026-10-04
- Fixed provider-card context menus so right-click actions stay anchored to the active card and account tab.

## 0.4.67 — 2026-10-04

- Add explicit add-account/API-key and remove-current-account actions to the
  provider card menu.
- Strengthen skin detail contrast in light and dark themes.

## 0.4.66 — 2026-10-04

- Show one active period at a time in spend details so today, yesterday, and
  range totals cannot appear as duplicate provider columns.
- Add configurable local shortcuts for settings, refresh, Customize, theme,
  expiring view, and quota period, with duplicate-binding feedback.
- Keep pooled-account badges informational blue rather than using an alarm red.

## 0.4.65 — 2026-10-04

- Prevent horizontal overflow when scrolling the overview on narrow windows.
- Separate maxed quota (red) from fetch/auth errors (orange) and show weekly
  fallback feedback when a provider has no monthly quota.
- Keep account-card removal scoped to that account; add native skin reset,
  conflict feedback for global shortcuts, and a Phosphor ClinePass mark.

## 0.4.64 — 2026-10-04

- Add the skin market with built-in wallpaper and mascot combinations.
- Add pooled ClinePass API-key accounts, quota cards, and provider account badges.
- Show rejected pooled accounts as explicit error states.
- Refresh the README around the current product workflow and local-first boundaries.

## 0.4.63 — 2026-10-04

### Added
- Token usage source inventory for local JSONL, SQLite, Antigravity protobuf,
  and Cursor CSV collectors, with detection status and documented coverage limits.
- ZCode SQLite and Antigravity conversation usage collectors; measured tokens
  remain counted when a model has no known price.
- Key vault and multi-key MCP quota aggregation for Bocha, Tavily, Firecrawl,
  and Brave Search; additional subscription providers and account routing.

### Changed
- Quota overview groups cards by labels without available/unavailable sections.
- Status counts share one baseline; folded spend and overview spacing is tighter.
- Qoder CN aggregates credit packages; Step Plan authentication includes its device JWT.


## 0.4.62 — 2026-09-17

### Added
- **Trae CN per-pack credit breakdown.** The card no longer collapses
  all credit packs into one summed bar: each pack gets its own row —
  the pack currently being drained as a progress bar with a countdown,
  untouched ones as "N credits · expires YYYY-MM-DD" rows, soonest
  expiry first. Packs split by scope like the official dashboard
  (general vs TraeWork-only, suffixed "(Work)"); same-named packs
  merge; feature-flag-only packs are skipped. The summed Credits row
  carries no reset date (packs expire on different days).
- **"5-hour window resetting soon" alert.** A rolling window with an
  hour or less remaining announces itself once per period, also right
  after launch. Default on; toggle in Settings.
- **Customize drawer brand icons.** Every provider row head shows the
  same mark as its dashboard card; providers without an icon show a
  "?" placeholder. The drawer also mirrors the dashboard's card-group
  sections (group name + count headers, A–Z within).
- **Card rows never fold.** Metric feet, text, action and spend rows
  are single-line: the right side ellipsizes instead of wrapping,
  countdowns lead the detail (truncation can only drop the usage
  figure, never the time), and hovering shows the full text.
- **Real Trae CN logo** (green frame + diamond eyes) replaces the
  placeholder glyph everywhere.

## 0.4.61 — 2026-09-16

### Added
- **Custom UI font picker.** Settings → General → Font lists every font
  family installed on the system (DirectWrite enumeration, localized
  names included — 微软雅黑 and Microsoft YaHei UI both resolve). The
  dropdown renders each option in its own face, filters as you type,
  supports arrow/Enter/Esc, marks the active family, and follows the
  light/dark theme. A picked family replaces only the head of the stock
  stack (missing glyphs still fall back); clearing the field restores
  the default. Persisted as `uiFont` in config.json and re-applied
  before first paint on launch.
- **Kimi monthly quota anchor.** Kimi's membership-wide monthly limit
  never appears in the usage API — it surfaces only as failed
  inference requests. The refresh day (the 27th) is now measured from
  the member dashboard instead of being probed for: while walled, the
  panel classifies Kimi as unavailable with a "refreshes on the 27th"
  countdown, the waiting probe sleeps entirely (zero requests until
  the anchor passes, then the 30-min rhythm confirms recovery), and
  healthy rounds still probe once per refresh to catch a new wall
  early. Restarting pane forces a fresh probe (e.g. after buying
  Extra Usage ahead of the anchor).

## 0.4.60 — 2026-09-15

### Added
- **Peak-hours status indicators.** The five providers with time-of-day
  billing (Z.ai, GLM V1 Pro relays, Command Code GOAT, Qoder CN, Trae
  CN) show a yellow status dot while their peak window is active —
  but only when they are otherwise available; red (maxed/error) and
  gray keep their meaning. Windows are Beijing time, rules verified
  against official docs: Z.ai/GLM Mon–Fri 14:00–18:00, GOAT
  Mon–Fri 09:00–12:00 & 14:00–18:00 (DeepSeek-routed models only),
  Qoder CN & Trae CN daily 08:00–22:00 (weekend daytime included).
- **Overview peak chip + section.** The Quota Overview badge row gains
  an always-rendered yellow `N peak` chip (three disjoint counts:
  available = off-peak, peak = in-peak available, maxed), and the
  sectioned board splits the available set into 可用 / 高峰 / 不可用 —
  the peak section header renders even when empty, so the dimension
  stays visible off-peak. Hovering a peak tile appends the family's
  multiplier rule (what standard rate costs vs the off-peak discount).
- **Soonest-reset keyboard toggle.** With the panel open, bare `T`
  flips the Quota Overview to the soonest-reset list and back — same
  guards as the bare-Shift 5h/Weekly board toggle (no typing targets,
  no Customize, no chords); a collapsed overview unfolds and scrolls
  into view first.

### Changed
- **Dual auto-hiding sidebars.** The provider icon trail moved from
  the left rail to a mirrored right rail (4 px sliver, slides out from
  the right edge, same glass/feather/fisheye treatment). The left rail
  keeps only the app controls: logo, theme, refresh, Customize,
  Settings — and the trail now uses the full rail height, so a dozen
  providers no longer shrink or scroll as early.

### Fixed
- Right-rail cascade polish: light-theme stray border, focus shadow
  direction, glass feather specificity, duplicate light rules; trail
  badges follow the peak tint; folded cards no longer show two peak
  markers.

## 0.4.50 — 2026-09-10

### Added
- **Quota Overview 5h / Weekly capsule.** Header switch next to the
  availability chips. Ordinary provider rings follow the selected window;
  Z.ai, One/New API, and Copilot keep their original binding. Hover still
  shows the other window; click still jumps to the card.
- **Overview one-click refresh.** A ⟳ button in the Quota Overview header
  (next to the fold chevron) forces a full refresh with the same spin
  feedback as the per-card buttons.

### Fixed
- **Launch no longer sits on "Outdated" until every card is refreshed by
  hand.** The background loop now fetches first and sleeps after, so a
  live pass runs at launch instead of one interval in; when nothing comes
  back live (boot with the network still coming up) it clears the
  ordinary-error benches and retries every 15 s, up to five times. Manual
  refreshes clear ordinary-error benches too — a failed pass must not
  turn the Refresh button into a no-op; 429 cooldowns still hold.

## 0.4.49 — 2026-09-09

### Added
- **Cross-platform OS seam.** Host-only work (credential store, UI
  language, process/port discovery, screen-share detection, WebView memory
  hint, config/data paths) now lives in `src-tauri/src/platform/`. Windows
  keeps Credential Manager / Win32; macOS and Linux use Keychain and
  Secret Service. The lib crate-type is `rlib` only — `cdylib` pulled
  Windows `advapi32`/`bcrypt` into the macOS/Linux link. Tagged releases
  build Windows NSIS, macOS dmg, and Linux AppImage/deb. Windows remains
  the supported tray experience; the other two are the first published
  binaries, not a finished desktop port.

### Fixed
- **Cursor Team Total usage no longer clamps to 100%.** Spend past the
  API dollar floor is shown as text, not a maxed bar. The Quota Overview
  ring follows the Auto pool (`Cursor Models`), not the API pool.
- **Independent quota pools.** Cursor Auto/API and Antigravity
  Gemini/Claude only mark a card maxed when every present pool is
  exhausted. Missing `remaining` no longer becomes a fake 0.
- **Overview hover.** Ordinary 5-hour rings tip the weekly window;
  Copilot/Z.ai keep the ring window; Cursor/Antigravity list each pool.
  The "click to jump" line is gone.

### Changed
- HTTP clients are reused (`OnceLock`). Cursor `state.vscdb` prefers an
  immutable open over copying the whole DB. The 30-second tick updates
  countdown text only. A failed provider is benched for at least one
  refresh interval.

## 0.4.48 — 2026-09-07

### Added
- **Quota Overview with per-window rings.** Generalized from the initial
  5-hour overview to pick the most binding quota per provider (5h session
  metrics first, otherwise shortest-period usage percent such as
  daily/weekly/monthly), bringing One/New API daily plans and Cursor/Copilot
  monthly plans into the overview rings with reset countdowns.
- **Independent availability symbols.** Quota Overview header displays two
  independent status symbols (`● N available` green, `● N maxed` red),
  shown only when non-zero.
- **Independent project detachment (ADR 0001).** Detached from the upstream
  fork network to evolve autonomously under Aafff623/pane; updater fallback
  repointed to Aafff623/pane releases.

### Changed
- **Dev loop pinned to IPv4 loopback.** Vite host and Tauri `devUrl` locked
  to `127.0.0.1:1420` to eliminate Windows machine-wide IPv6 loopback
  access-denied issues. Documented canonical WMI/CreateProcess startup flow.

## 0.4.47 — 2026-09-05

### Added
- **One/New API sites as accounts.** Relay sites become first-class accounts:
  the dashboard renders a single merged card with a tab per configured key.
  Site management moved from Settings into the Customize drawer under the
  family row. Supports token-only sites reading `/api/subscription/self`.
- **Multi-account expansion.** Antigravity Google OAuth slots captured from
  Windows Credential Manager; Cursor PKCE multi-account tokens.
- **Governance assets.** Tracked canonical `docs/dev-startup.md`, `CONTEXT.md`,
  `AGENTS.md`, and `CLAUDE.md` to prevent agent configuration drift.

### Changed
- **Pane's website moved to trypane.xyz.** Public links and install
  commands use the new domain. Existing `pane.jazii.dev` updater clients
  and dashboard links remain supported through a permanent domain redirect.
- **UI polish and breathing room.** Folded cards surface nearest reset
  countdown as a status-toned pill; frosted glass sidebar with feathered edge;
  authentic brand SVGs.

## 0.4.46 — 2026-09-02

### Added
- **One/New API sites.** Settings has a new **One/New API sites**
  section: add as many One API / New API hosts as you want, each with
  its own keys. Each key gets its own quota card (PR #172).
- **Kimi Code without the CLI.** Settings → API keys has a new **Kimi
  Code** field for your Kimi For Coding plan key. With no `kimi login` on
  the PC, Pane sends that key to the same usage endpoint the CLI uses and
  shows Session / Weekly bars and reset times. A CLI login still wins
  when both exist. The Z.ai field is now labeled **Z.ai / GLM** — a GLM
  Coding Plan key already worked there (issue #173).

### Fixed
- **Cursor card no longer blanks to "Requests this cycle 0".** When
  Cursor's usage API hiccupped for one refresh, Pane fell back to the
  old request-count endpoint, which on current plans answers with an
  empty count, and that empty card replaced your bars until the next
  good refresh. That fallback now only counts when it carries a real
  request quota; otherwise Pane keeps the last good card (Outdated only
  after three minutes of failed refreshes).
- **Cursor bars stay live when `api2.cursor.sh` is unreachable.** On
  some networks that host's TLS handshake times out for minutes while
  `cursor.com` keeps answering. Pane now reads the dashboard's
  `usage-summary` report in that case — same Cursor Models / Other
  Models / Total usage / On-demand figures, from the host that works —
  instead of showing a stale card. Also covers Enterprise/team accounts
  that hide plan usage from the RPC (what OpenUsage on Mac does),
  including the live shape that has percentages and on-demand but no
  pooled dollar cap. Credits and Bonus rows still need the primary API.
- **Hermes prices Qwen3.8-Max-0902 through AihubMix.** The snapshot
  logs as `qwen3.8-max-0902` / `qwen3.8-max-2026-09-02`, which missed
  the rate table, so Today showed tokens at $0.00. Those AihubMix rows
  now use the gateway card ($1.69 / $5.07, cache $0.169 / $2.1125).

## 0.4.45 — 2026-08-29

### Added
- **Russian in Settings → Language.** Same surfaces as Chinese: cards,
  Settings, Customize, tray tooltips, and quota toasts. Auto follows the
  PC display language (Chinese or Russian). Provider names, plan names,
  and the changelog stay English.

### Fixed
- **Hermes recognizes HY4 Preview and Qwen3.8 Flash spend.** New-model
  sessions routed through AihubMix now use that gateway's live prices,
  so their tokens no longer collapse into a misleading `$0.00` Other row.
  The card lists the two most recent user models and leaves Hermes's small
  title-generation and approval calls out of that summary.

## 0.4.44 — 2026-08-27

### Fixed
- **Customize keeps progress bars where you put them.** Dragging a
  meter like Codex Extra credits into Show more used to look like it
  saved, then the next usage refresh yanked it back above Usage Trend.
  That "bars stay visible" rule now only places a row the first time
  it appears. A later refresh no longer undoes the drag (issue #166).

## 0.4.43 — 2026-08-26

### Added
- **Grok reset credits** show on the card as read-only rows (no Use
  button). A **Status** link opens [status.x.ai](https://status.x.ai)
  next to Usage.

### Fixed
- **Tray follows Customize.** Hide, disable, star, pin, and card order
  now drive the tray icon and tooltip. A newly enabled provider stays
  off the tray until a refresh finishes. If a save or tray update fails,
  the footer says so instead of staying quiet.

## 0.4.42 — 2026-08-23

### Added
- **Chinese in Settings → Language.** Auto follows the PC language;
  English and 中文 are explicit. Cards, Settings, Customize, tray
  tooltips, and quota toasts switch as soon as you pick one. Provider
  names, plan names, and the changelog stay English. Vendor "connect me"
  error hints stay English for now.

### Fixed
- **Refresh responds while usage is updating.** The button used to stay
  locked until the slow spend scan finished (tens of seconds on a cold
  start), so clicks looked dead and a just-saved **Kimi API** key left
  "moonshot key saved" stuck on the footer. Usage unlocks Refresh as soon
  as the cards update; a click (or key save) during a fetch shows
  Refreshing… immediately; and the sidebar sits above Settings so Refresh
  still works with the panel open.

## 0.4.41 — 2026-08-20

### Fixed
- **Kimi Code plan name matches the membership page** — current paid
  names on [kimi.ai](https://www.kimi.ai/membership/pricing) are
  Moderato, Allegretto, Allegro, and Vivace. Pane still printed the old
  three-name table, so `LEVEL_INTERMEDIATE` showed as Moderato
  (issue #156; it is Allegretto) and `LEVEL_ADVANCED` showed as
  Allegretto (it is Allegro). `LEVEL_PREMIUM` is Vivace.

## 0.4.40 — 2026-08-20

### Fixed
- **Refreshing catalogs no longer re-reads every session log.** The spend
  cache used to throw itself away whenever LiteLLM / models.dev /
  the supplement file changed on disk (that happens daily, and hourly
  while any model is unpriced), then re-parse hundreds of MB of jsonl.
  Each cached file now records the pricing questions it asked; a catalog
  refresh only re-reads files whose prices actually moved. A baked-pricing
  code change still discards the cache. One last full scan migrates to
  the new format. Probe keys stored in that cache are length-capped so a
  huge model string cannot inflate the on-disk file.
- **Pasting an API key actually shows that provider.** Saving a key now
  turns the provider on if Customize had it off (first-run parks
  keyless providers there), and a save no longer races an in-flight
  refresh into a stuck "key saved" status with no fetch. A key pasted
  during that first refresh is also kept out of the auto-disable list,
  so the just-saved provider is not parked because the in-flight fetch
  still said it had no credentials. A failed save or a cleared key drops
  that exemption immediately, so Customize can turn the provider back
  off.
- **Kimi usage routed through Codex (or Claude) lands on the Kimi card.**
  Sessions that log `kimi-oauth/k3` or `moonshot-ai/…` via a router used
  to keep those dollars on Codex. They move to Kimi Code (or Moonshot
  when there's no Kimi login), with the vendor prefix peeled so they
  merge with the CLI's own logs.
- **Cursor promo credits and bonus usage show up.** Credit grants
  (`GetCreditGrantsBalance`) are a used/total progress bar like Codex
  Extra credits, and Cursor's sponsored bonus usage (`bonusSpend`) is a
  text row behind Show more — free usage model providers cover, shown
  as context rather than a meter. Cents-as-strings parse; a failed
  grants call logs instead of going silent. An expired grant
  (`hasCreditGrants: false`) stays hidden even if leftover totals are
  still on the payload. Tucking Bonus behind Show more runs once so a
  later Customize drag is not undone every refresh.
- **Kimi Code plan usage now has dollar amounts** — the official CLI logs
  plan K3 as `kimi-code/k3` (and K2.7 Code as `kimi-for-coding`), which
  missed the rate table, so Today could show a million tokens at $0.00.
  Every current Moonshot model now uses the published API card
  (K3 $3 / $15 / $0.30 cache; K2.7 Code $0.95 / $4 / $0.19; HighSpeed
  is 2×; K2.6 $0.95 / $4 / $0.16; K2.5 $0.60 / $3 / $0.10; V1 8k/32k/128k
  at $0.20/$2, $1/$3, $2/$5). Reseller catalog rows no longer override
  those first-party rates. Codex OAuth (`kimi-oauth/k3`) and prefixed
  API spellings share the same table. Discontinued K2 / kimi-latest
  names still come from the public catalogs.

### Changed
- Customize toggle, Settings key field, leftover dashboard card, and
  spend card labeled **Kimi API** (was Moonshot). The internal id is
  still `moonshot` so existing layouts and telemetry keep working. The
  toggle still gates the wallet fetch and the API bar on the Kimi Code
  card.

## 0.4.39 — 2026-08-19

### Added
- **Kimi Code plan card** — one card like OpenCode: Session and Weekly
  request bars from the official Kimi Code CLI login. If a Moonshot
  wallet key is saved, an **API** bar (same high-water credits meter)
  joins them; plan-only installs stay at two bars. Local session spend
  that used to sit on the Moonshot card lives here. The leftover
  Moonshot card hides while Kimi Code is connected (and comes back if
  that login goes away); it still appears for API-only installs. If the
  Kimi card loads without a wallet bar, a successful Moonshot fetch is
  kept so the balance doesn't vanish for one cycle.
  Switching Moonshot off in Customize still stops the wallet fetch — no
  API bar and no Moonshot network calls. If the Kimi card itself is
  off, session spend stays on Moonshot so the dollars don't vanish. A
  starred Moonshot "Credits used" pin moves to the Kimi API bar only
  when that bar is actually on the card.

## 0.4.38 — 2026-08-15

### Fixed
- **Refreshes are fast again — sorry 0.4.37 made Pane laggy and slow.**
  That release's bounded log walk resolved every directory it visited to
  its canonical path, and on Windows that opens a real handle per
  directory (with an antivirus round-trip on each), so opening Pane
  crawled and "Scanning session logs…" could sit for over a minute on
  every refresh. Sorry for the inconvenience. Only symlinks and
  junctions pay that cost now — they're the only way a scan cycle can
  form — and regular files and folders are typed straight from the
  directory listing with no extra system calls. The 0.4.37 protections
  (depth cap, directory budget, cycle detection, link-target
  timestamps) all still hold.

## 0.4.37 — 2026-08-15

### Security
- **Local log scan can't be turned into a crash or a hang** — the
  recursive `.jsonl` sweep every spend scanner shares followed symlinks
  and junctions with no depth limit and no visited set, so a link cycle
  under a scanned log root (roots come from the CLIs' own `*_CONFIG_DIR`
  / `*_HOME` variables) grew the call stack until the process aborted,
  and a link pointing at a huge tree turned a project folder into a
  drive-wide crawl. The walk is now iterative and bounded: 16 levels
  deep, 20,000 directories per root (logged when it stops there), and
  already-seen canonical directories are skipped. Relocated logs behind
  a link are still found.
- **Share-card decode is bounded** — copying a share card now caps the
  base64 payload and the PNG's declared pixel count before decoding, so
  a decompression-bomb image can't force a multi-gigabyte allocation.
- **OpenCode's database copy is private** — the scratch copy Pane makes
  of `opencode.db` now lands in a per-user directory with restricted
  permissions, partial copies are discarded, and scratch files left by a
  crashed run are swept.

## 0.4.36 — 2026-08-14

### Added
- **Hermes desktop card** — Nous Research's Hermes app now has a provider
  card like the others: last model used, which backend billed it, and
  session count on the face; Today / Yesterday / Last 30 Days spend
  (per-model breakdown on hover) sit behind Show more. MiniMax- and
  OpenRouter-routed sessions still join those slices; AihubMix
  (including a custom URL pointed at aihubmix.com) stays on the Hermes
  card; a custom URL pointed at MiniMax or OpenRouter joins those slices.
  No network calls — Pane reads the local ledger at
  `%LOCALAPPDATA%\hermes\state.db`.
- **GLM-5.3 spend prices from day one** — AihubMix's preview rate card
  for `coding-glm-5.3` ($0.060 in / $0.220 out per MTok; unpublished
  cache bills at the input rate) is baked in until public catalogs learn
  the model. Hermes AihubMix rows logged as `glm-5.3` use that SKU;
  the generic name stays unpriced for other vendors. Gateway-prefixed
  DeepSeek V4 slugs (`accounts/fireworks/models/deepseek-v4-pro-0813`)
  also resolve.

## 0.4.35 — 2026-08-13

### Added
- **Daybreak Blue and Cursor Router spend names price correctly** — Pane
  now loads the full OpenUsage pricing-alias list (it used to keep only
  the first 64 rules, which dropped Daybreak, GPT-5.3–5.6 effort slugs,
  and Cursor Router labels like "Opus 5 (Auto Balanced)").
- **Codex auto-review stays its own row** — reviewer traffic still shows
  as `codex-auto-review` in the model breakdown; the dollars use that
  day's GPT rate (gpt-5.5 from April 2026 onward), matching the Mac app.
- **Reduce animations** — Settings toggle skips card entrance motion and
  the day/night wipe. Windows' own Animation effects setting is still
  respected either way.
- **Hide tray numbers while screen sharing** — optional Privacy setting
  (off by default). During Presentation Settings, exclusive fullscreen,
  or remote control, the main tray percentage and starred strip numbers
  hide; the Pane icon and starred provider logos stay.
- **Reset all settings** — Settings → Advanced restores every preference
  to its default and re-detects installed tools. API keys and usage
  history stay.

### Fixed
- **The Cursor card mirrors Cursor's Plan & Usage page** — Cursor's new
  bucket-era plans show two bars ("Cursor Models" and "Other Models")
  and no total bar, but Pane's Total usage meter trusted the API's
  `totalPercentUsed`, which now measures against included *plus free
  bonus* pools (~$345 on live accounts) — so the bar sat at 0% while
  the caption said "$2.37 of $20.00 included". Bucket-era cards now
  show the same two bars as the dashboard, with the exact percentages
  Cursor shows, plus dollars spent this cycle as a text row (no percent
  is honest there: Cursor itself reports three contradictory total
  numbers). Pre-bucket accounts keep the classic included bar, computed
  as spend ÷ plan limit when Cursor reports spend (the API's own percent
  otherwise — never a fabricated one). Team accounts keep their bucket
  bars alongside the dollar Total meter. Saved layouts migrate
  automatically: stars, pins, hidden flags, and order carry over from
  "Auto usage" / "API usage" to the new labels, and a star or tray pin
  on a Total row that became text repoints to Cursor Models instead of
  silently vanishing from the tray.

## 0.4.34 — 2026-08-13

### Added
- **OpenCode meters are account-wide now** — OpenCode shipped the
  official usage API we'd been waiting on
  ([anomalyco/opencode#16513](https://github.com/anomalyco/opencode/pull/16513)),
  and the card now reads Session / Weekly / Monthly percentages and
  resets straight from OpenCode's servers: the same numbers as the Zen
  dashboard, including your other devices and anyone sharing the
  subscription. The old this-PC-only computation from `opencode.db`
  remains as the offline fallback, and dollar spend stays local.

### Fixed
- **Grok 4.6 prices from day one** — xAI's launch-day rates
  ($2 in / $0.50 cached / $6 out per MTok, doubling for ≥200k-token
  prompts; the fast variant at 2x) are baked in until the public
  catalogs learn the model, so spend from Grok 4.6 sessions shows
  dollars instead of the unpriced ⚠ — including Cursor sessions, whose
  usage export brands the model `cursor-grok-4.6-xhigh`.
- **DeepSeek V4 spend prices from day one** — AihubMix's rate cards for
  `deepseek-v4-pro` ($0.464 in / $0.928 out / $0.004 cache read per
  MTok) and `deepseek-v4-flash` ($0.154 / $0.308 / $0.003) are baked in
  until public catalogs learn the family, dated snapshots like
  `-0813` included, so Hermes sessions on these models show dollars
  instead of the unpriced ⚠.

## 0.4.33 — 2026-08-12

### Fixed
- **Launch shows your numbers instantly** — the window now paints the
  previous run's snapshots from disk (marked Outdated) the moment it
  opens, then swaps in live data as it arrives. Before, the first paint
  waited for the slowest provider's full network round — at boot, with
  Wi-Fi still connecting, that was 30-40 seconds of "Refreshing…".
- **The daily pricing refresh no longer lands at login** — a catalog
  update discards the spend cache and re-reads every session log
  (gigabytes on long-lived installs), and with autostart that landed
  exactly when Windows was busiest. Catalog downloads now wait out the
  first 10 minutes after launch; a first run with no catalogs on disk
  still fetches immediately.
- **Dead networks fail fast** — provider requests now give up on
  connecting after 5 seconds instead of riding the full 20-second
  request timeout, so a boot-time refresh with no network settles in
  seconds.

### Security
- **Credential provenance hardening** (from an external security
  report): credential discovery now proves a token belongs
  to the provider it will be sent to, instead of accepting the first
  field-shape match. Extra Codex accounts found by the multi-account
  scan must carry OpenAI's own claim namespace in their `id_token`
  before any request or refresh (a foreign `auth.json` that merely has
  a `tokens.account_id` no longer qualifies); Copilot token lookup is
  scoped to the `github.com` entry in `apps.json`/`hosts.json` and to
  the `github.com` section of the GitHub CLI's `hosts.yml`, so a
  GitHub Enterprise token sharing the file is never sent to
  github.com; the MiniMax CLI fallback reads exactly
  `provider.minimax.options.apiKey` from `~/.minimax/config.yaml`
  instead of the first `apiKey:` line anywhere in the file.
- **Disabled Cursor makes no requests** — the spend scan's Cursor CSV
  export (an authenticated network call) now honors the provider
  toggle, matching how usage refresh already skips disabled providers.
- **Local API rejects non-loopback Host headers** — the read-only
  usage API on 127.0.0.1:6736 now refuses requests whose Host header
  isn't a loopback spelling, closing the DNS-rebinding read that CORS
  alone can't prevent.
- **Release workflows pin actions to commit SHAs** — third-party
  GitHub Actions in the signed release and winget pipelines are pinned
  to exact commits instead of movable tags.
- **Pricing catalogs download under a hard size cap** — the three
  third-party pricing feeds are read in bounded chunks (32 MB cap), so
  a compromised feed can no longer exhaust memory or disk. This also
  makes SECURITY.md's existing "inputs are size-capped" claim true.
- **README offers a look-before-you-run install path** — alongside the
  one-liner, a two-step download-inspect-run variant and a pointer to
  winget for anyone who prefers not to pipe scripts into PowerShell.

## 0.4.32 — 2026-08-11

### Added
- **Multi-account Codex** — same treatment for Codex: every discovered
  `CODEX_HOME`-style login (identified by its ChatGPT account id, named
  by the account email) gets its own card with its own limits, credits,
  and spend from its own session logs. Reset-credit redemption routes
  to the account whose card offered the credit, and each card's Extra
  credits meter keeps its own high-water baseline. A cache identity
  stamp also guards both families' default cards: if a different
  account signs into the default folder between launches, the old
  account's cached numbers are dropped instead of shown under the new
  login.
- **Multi-account Claude** — machines with more than one Claude login
  (say a personal plan in `~\.claude` and an enterprise seat kept in a
  second config dir via `CLAUDE_CONFIG_DIR`) now get one card per
  account, each with its own limits, plan, and spend scoped to its own
  logs. Discovery scans dot-folders in your home directory and
  `~\.config` for Claude-shaped config dirs and only makes a card when
  the dir can name its account (from its `.claude.json`); the same
  account signed in twice stays one card. Extra cards are named from
  the account's organization or email ("Claude — Acme"); the default
  login keeps the plain `claude` identity, so existing layouts, stars,
  and API consumers are untouched. Design follows upstream OpenUsage's
  account-first model. Telemetry never learns account identities —
  a multi-account install reports plain "claude" once.

## 0.4.31 — 2026-08-08

### Changed
- **Share cards copy exactly what you see, collapsed too** — a collapsed
  card's ⧉ copy used to drop the Usage Trend and pace hints for a
  "compact composition"; since both are visible on the card, the copy
  read as missing data. Every share now includes everything the card
  currently renders; only buttons, links, and carets stay out. Light
  mode shares also get a real surface hierarchy — grey frame, white
  card with a soft shadow — instead of the washed grey-on-white blob.

### Fixed
- **Devin no longer shows a fresh bar when the weekly quota is spent** —
  Devin's API drops zero-valued fields from its JSON (proto3), so an
  exhausted week loses its remaining-percent field entirely and Pane
  misread that as "no weekly quota", falling back to the untouched
  hidden daily meter: a 0%-used bar at the exact moment you were rate
  limited. A missing percent next to a present reset timestamp now
  correctly reads as 0% left — "Limit reached", real countdown — the
  same class of fix Grok needed for its omitted usage field.

## 0.4.30 — 2026-08-07

### Fixed
- **Bought Codex credits show up now** — the ChatGPT API sends the
  credit balance as a quoted string in some responses, which the parser
  didn't accept, so buying Extra credits made the row vanish from the
  card entirely (an empty balance still rendered via a fallback). A
  funded balance now shows as an **Extra credits** plan-style meter —
  percent bar against the highest balance Pane has seen (top-ups raise
  it), feeding pace colors and the Almost Out notification like every
  other window — with the dollar and credit count in the caption.
  Devin's Extra balance gets the same meter treatment when funded.
  Layouts learned a matching rule: progress bars never hide behind
  Show more and always sit above the Usage Trend, so an upgraded row
  (or one saved as a text row in an older layout) surfaces properly.

## 0.4.29 — 2026-08-05

### Added
- **Qwen Code provider (#19)** — Alibaba Model Studio's Coding Plan gets
  a card: the plan's request-counted 5-hour / weekly / monthly quotas
  with resets and plan name, read via the Model Studio console's own
  quota call (key from Settings, `BAILIAN_TOKEN_PLAN_API_KEY`, or
  `DASHSCOPE_API_KEY`); falls back to local request/token counts when
  the console won't take the key. Today / Yesterday / 30-day spend, the
  per-model breakdown, and a donut slice come from the Qwen Code CLI's
  own per-request ledger.
- **Claude Code × AihubMix spend lands on the AihubMix card** — sessions
  run against AihubMix's Anthropic-compatible endpoint log qwen-family
  models into Claude Code's local history; those rows now re-route to
  the AihubMix slice (same mechanism as MiniMax-routed sessions)
  instead of inflating the Claude card.

- **Pi coding agent usage folds into Claude and Codex** — pi is a
  bring-your-own-account agent, so a Claude or Codex subscription driven
  through pi never appears in those CLIs' own logs and spend
  under-reported. Pane now reads pi's session logs
  (`~\.pi\agent\sessions`) and folds that usage into the account's card:
  pi's own recorded cost when it carries one, catalog pricing otherwise
  (ported from OpenUsage 0.7.6).

### Changed
- **Grok's provider mark updated** — ported upstream's refreshed logo.

### Fixed
- **Zero-rate catalog placeholders no longer price requests at $0.00** —
  public catalogs often list brand-new models (e.g. qwen3.8-max) with
  0/0 placeholder rates, which silently billed every request at zero
  and could flip a model's spend to $0 after a catalog refresh. Those
  rows are now skipped: a source with real rates wins, while a model
  listed 0/0 in every source still prices as genuinely free ($0.00, no
  warning — ":free" variants and local models keep working).
  qwen3.8-max (GA'd 2026-08-03, still 0/0 in every public catalog) gets
  Alibaba's documented rates baked in — $2/$6 per MTok, $0.25 cache
  reads — consulted only until the live catalogs learn them.
- **"-priority" model slugs are priced** — Devin logs OpenAI's priority
  service tier inside the model name (`gpt-5.6-luna-xhigh-priority`),
  which no catalog carries, so those requests counted tokens but no
  dollars. They now bill at the base model's rates times the priority
  multiplier (2× standard, 2.5× for gpt-5.5), matching how Codex
  priority turns are already priced.

## 0.4.28 — 2026-07-31

### Added
- **"What's new" after every update** — the first time you open Pane on a
  new version, a card-styled popup shows that version's changelog (click
  anywhere outside it to dismiss). Settings also gains a **What's new ·
  Changelog** button that lists every released version. Fresh installs
  keep the welcome card only — no double popup.
- **AihubMix provider (#18)** — the OpenAI-compatible multi-model
  gateway gets a full card: usage metered against your account's
  spending limit (key auto-detected from OpenCode, or pasted in
  Settings), plus Today / Yesterday / 30-day spend, per-model breakdown,
  and the Usage Trend from requests routed through OpenCode. AihubMix
  dollars get their own donut slice, separate from the OpenCode plan.

### Fixed
- **GPT-5.6 Terra and Luna price at OpenAI's new rates** — OpenAI cut
  both models' prices (Luna is 5× cheaper) and the public price catalogs
  still carry launch pricing, so spend was overstated. Pane corrects the
  stale catalog values (a self-retiring override: the moment the
  catalogs publish updated numbers, live data wins again) and updates
  the long-context rate table to the new tiers.

### Changed
- **OpenCode meters drop the "this PC only" tag** — the caveat crowded
  every row of the card (and its share cards); the local-counting
  caveat now lives in the docs instead. Also fixed a cosmetic quirk
  where an idle session could read "$-0.00 of $12".
- **Confirmations look like Pane now** — using a Codex reset credit or
  resetting all layouts used to pop the browser's bare "localhost says"
  dialog; both now use an in-app card-styled dialog matching the app
  (with a red button where the action is destructive). The reset-credit
  message also notes that refreshed windows can take a couple of
  minutes to appear.

## 0.4.27 — 2026-07-30

### Added
- **Grok card shows your subscription plan** — the header now reads
  "X Premium", "SuperGrok", and friends, resolved from xAI's settings
  display name with a tier-code fallback (free tiers stay unbadged; a
  plan lookup failure can never hide usage data). Contributed by
  @JaminYe — their second Pane contribution. 🎉

## 0.4.26 — 2026-07-29

### Changed
- **Share cards copy what you see** — the ⧉ copy of an expanded card now
  includes everything visible: the usage trend, spend rows, and pace
  hints. Collapsed cards keep the clean compact composition. Buttons,
  links, and carets stay out of the image either way.

### Fixed
- **Daily statistics count on the day they belong to** — the once-a-day
  telemetry gate used the machine's local date while every consumer
  buckets by UTC day, so an install east of UTC whose app runs at local
  midnight always landed in the previous UTC day (Pakistan showed "0
  today" while actively using Pane). The gate is UTC now, matching the
  dashboard.

## 0.4.25 — 2026-07-28

### Fixed
- **The telemetry toggle shows its real state** — the "Share anonymous
  usage statistics" switch rendered as off for installs that had never
  touched it, while the default-on sender kept reporting. The default is
  now stated in the config itself, so the switch reads ON unless you
  actually turned it off (opting out worked correctly all along; only
  the display was wrong).
- **OpenCode meters use OpenCode's real windows and show resets** — the
  card summed rolling 7-day and 30-day windows, but OpenCode actually
  meters a UTC Monday-start week and a monthly cycle anchored to your
  first-ever Go usage (ported from the Mac app's window math, which
  ported the official opencode-go plugin). All three meters now carry
  reset countdowns — the session shows when the oldest in-window spend
  ages out — so they pace and count down like every other card.

## 0.4.24 — 2026-07-27

### Added
- **Anonymous usage statistics (opt-out)** — Pane now sends at most two
  events per day: an "alive today" ping (version, enabled providers,
  starred metrics, appearance settings) and per-provider refresh
  success/failure counts with error *categories* only — under a random
  ID derived from nothing, with PostHog person profiles disabled and
  IP addresses discarded at ingestion. Never sent: usage amounts,
  spend, model names, keys, paths, or error text. Settings → Privacy →
  "Share anonymous usage statistics" is a hard stop: turning it off
  counts nothing, writes nothing, and deletes the stored ID. The whole
  implementation is one auditable file (src-tauri/src/telemetry.rs, no
  SDK); docs/privacy.md documents every field.

## 0.4.23 — 2026-07-22

### Changed
- **Instant startup for the spend engine** — per-file parse summaries now
  persist across launches (`%APPDATA%\Pane\spend_cache.json`, a few MB at
  most), so a fresh start re-parses only logs that changed instead of
  re-reading every session file ever written. The cache is discarded
  whenever the pricing catalogs update, so costs are never served stale.

### Added
- **Hermes desktop spend** — Pane now reads the local usage ledger of
  Nous Research's Hermes app (`%LOCALAPPDATA%\hermes\state.db`, this PC
  only, nothing sent anywhere). Each session is filed under the backend
  that actually billed it: MiniMax-routed chats merge into the MiniMax
  spend slice, OpenRouter-routed into OpenRouter, and anything else shows
  as a Hermes slice.

### Fixed
- **Symlinked session logs count their real age** — the spend scanner
  follows symlinks/junctions when walking log directories, but judged a
  linked file's recency by the link's own timestamp; logs relocated to
  another drive could silently vanish from the 31-day window. The
  target file's timestamp decides now.
- **Kimi K3 cache reads were overpriced ~10×** — models.dev lists the
  same model under many resellers and Pane took the alphabetically first
  entry, which for kimi-k3 was a stub with no cache pricing; cache hits
  then defaulted to the full $3.00 input rate instead of $0.30. The
  catalog entry with the most complete pricing wins now.

## 0.4.22 — 2026-07-20

### Changed
- **"Others" wedge folds more aggressively** — providers now fold into
  the Others slice under $5 on Today/Yesterday (was $1) and under $10 on
  Last 30 Days (was $5), keeping the ring focused on the big spenders.
  A lone under-threshold provider folds too (it used to keep its own
  legend row); the ring only stays unfolded when everyone is small.

## 0.4.21 — 2026-07-19

### Fixed
- **Grok no longer shows "Outdated" right after its weekly reset** —
  xAI's billing API omits the usage-percent field entirely while usage
  is 0%, which Pane misread as a broken response and kept showing last
  week's 100%-used bar with an ⚠ Outdated tag. A fresh window with no
  usage now correctly renders as a 0% bar with the new reset countdown.

## 0.4.20 — 2026-07-18

### Fixed
- **Disabled providers now do nothing at all** — disabling a provider
  only hid its card; the work still ran invisibly on every refresh
  (network calls, file reads, and for Kiro: spawning kiro-cli, whose
  own auto-updater downloaded a fresh ~25 MB installer to %TEMP% each
  time — gigabytes within days once Amazon shipped an update). Disabled
  providers are now skipped before anything runs.

### Removed
- **Kiro support** — the experimental Kiro provider worked by invoking
  kiro-cli, and a CLI that self-updates on every invocation is a side
  effect Pane cannot control. Rather than throttle it, the provider is
  gone entirely; the card disappears and saved layouts clean themselves
  up.

### Added
- **Grok usage bar shows its reset countdown** — the aggregate credit
  meter now carries the billing period's real end time and duration,
  so it paces and counts down like the other cards. Contributed by
  @JaminYe — Pane's first outside contribution. 🎉

### Changed
- **OpenCode Go meters say "this PC only"** — Go quotas are counted
  account-wide on OpenCode's servers, and there is no API to read
  them; the local meters can't see other devices or other participants
  on a shared subscription, and now say so instead of "local estimate".

## 0.4.18 — 2026-07-17

### Fixed
- **Update checks report the real app version** — Tauri's
  `{{current_version}}` URL template arrives percent-encoded and never
  substitutes in query strings, so 0.4.17 installs literally sent
  "currentversion". The endpoint URL is now built in Rust with the
  actual version stamped in.

## 0.4.17 — 2026-07-17

### Changed
- **Update checks go through pane.jazii.dev first** (GitHub stays as
  the automatic fallback, and every update remains signature-verified).
  The endpoint serves the same manifest and counts anonymous daily
  installs — distinct-install estimate, country code, app version, and
  nothing else; no IP addresses are stored. Full mechanics in
  docs/privacy.md ("The update check").

## 0.4.16 — 2026-07-16

### Added
- **Kimi K3 prices everywhere** — K3 usage through Devin, Cursor, the
  Kimi Code CLI, or the API now bills at Moonshot's published rates
  ($3 in / $15 out / $0.30 cache hit per MTok) via a built-in fallback,
  in every slug spelling the tools use. The live catalogs win
  automatically once they learn the model.

### Changed
- **"Others" thresholds scale with the period** — Today and Yesterday
  fold providers under $1 into the grey Others wedge; 30 Days folds
  under $5 (was a flat $10 everywhere, which swallowed most of a quiet
  day's ring). The hover breakdown states the active bar.

## 0.4.15 — 2026-07-16

### Added
- **Moonshot/Kimi spend** — the Moonshot card gains Today / Yesterday /
  30-day dollars and tokens, a per-model breakdown, and the usage trend,
  scanned from the Kimi Code CLI's local session logs (one usage record
  per turn). Models the price catalogs don't know yet count their
  tokens with the usual ⚠ until pricing lands.
- **Credit meters for pay-as-you-go cards** — Moonshot and DeepSeek get
  a "Credits used" percent bar metered against the highest balance Pane
  has seen (a top-up raises it automatically), so balance cards read
  like every other card — and the low-credit case now fires the same
  "Almost Out" notification the quota providers get.

### Fixed
- **Devin usage counts under the model that actually ran** — Devin
  rewrites a session's model label whenever you switch models, which
  retroactively relabeled (and mispriced) everything the session ran
  before. Each message's own recorded model now wins, so switching to
  Fable (or anything else) mid-session shows up correctly, priced at
  the right rates. Fable's Max mode slug also normalizes now.
- **Grok no longer shows the same meter twice** — the billing payload
  repeats one percentage under several keys; the card now keeps one
  row per label, and layouts saved while the duplicate existed repair
  themselves on the next refresh.

## 0.4.14 — 2026-07-16

### Changed
- **Small spenders fold into "Others"** — Total Spend providers under
  $10 in the visible period group into one grey wedge and legend row;
  hovering it lists exactly who spent what. Providers at $10 or more
  keep their own name and color. (A lone small spender stays named —
  a group of one is just a rename — and if everyone is under $10 the
  ring stays fully named rather than turning into one grey blob.)

### Fixed
- **Balance-only cards show their rows** — providers with no usage
  meters (Moonshot, DeepSeek, and other pay-as-you-go cards) tucked
  every row behind the "show more" caret, leaving an empty panel with
  a floating arrow. Their Balance/Voucher rows now stay visible, and
  already-saved layouts repair themselves on the next refresh.

## 0.4.13 — 2026-07-16

### Fixed
- **Cursor spend updates within minutes, not an hour** — the dashboard
  usage export was cached for a full hour, so a live Cursor session
  could work invisibly for up to 60 minutes while every other provider
  updated in minutes. The cache is now 5 minutes, and a failed refetch
  serves the last good export instead of blanking the Cursor spend
  rows entirely.

## 0.4.12 — 2026-07-16

### Added
- **MiniMax spend** — the MiniMax card now shows Today / Yesterday /
  30-day dollars and tokens like the other CLIs, fed by two local
  sources: the MiniMax Agent CLI's own per-turn usage store (its
  recorded cost is used as-is), and any Claude Code sessions run
  against MiniMax's Anthropic-compatible endpoint — that usage used to
  be counted (mislabeled) under the Claude card and now moves to the
  MiniMax card where it belongs.

## 0.4.11 — 2026-07-16

### Fixed
- **⚠ Outdated tooltips explain the problem and the fix** — hovering
  the warning now classifies what went wrong (sign-in expired, vendor
  rate limit, vendor outage, no connection) and says exactly what to
  do about it — including the right re-login command per provider —
  instead of showing a bare error code.
- **Total Spend always draws its ring** — a period with no usage now
  shows a quiet zeroed track with $0.00 in the center instead of
  collapsing to a bare "No spend in this period" line.
- **Dead Claude sign-in says what to do** — when another app rotates
  the Claude Code refresh token (leaving Pane's copy invalid), the card
  now says "run `claude` in a terminal once and Pane recovers
  automatically" instead of "token refresh failed: HTTP 400".

## 0.4.10 — 2026-07-14

### Fixed
- **Claude card recovers faster from rate-limit cooldowns** — when
  Anthropic's usage endpoint returns 429 (a plan change can trigger a
  ~25-minute cooldown), Pane now honors the vendor's own Retry-After
  timing instead of knocking every 5 minutes, and the card's note says
  how long the wait is.
- **Codex subagent replays no longer inflate spend** — when Codex spawns
  a subagent (or forks a session), the child's rollout file replays the
  parent's entire token history with fresh timestamps. Pane counted
  those replayed lines as real usage; they're now skipped via the log's
  own markers (the Mac app shipped the same fix after a ~20x inflation
  report). Re-emitted stale snapshots are skipped too, and turns that
  only report cumulative totals are recovered as deltas.
- **Codex fast/priority turns price at fast rates** — the service tier
  is read per session from the rollout's `thread_settings_applied`
  lines (never from `config.toml`, which would retroactively reprice
  history when toggled) and applies each model's Codex priority
  multiplier (GPT-5.5 ×2.5; GPT-5.4 and the GPT-5.6 family ×2).
  Supported Codex models switch to OpenAI's long-context rates above
  272k prompt tokens — the OpenAI boundary, not Anthropic's 200k.
- **Claude advisor usage counts under the advisor's model** — Fable-era
  logs nest advisor work in `usage.iterations`; advisor-message entries
  now count once under their own model without double-counting the
  parent totals. `<synthetic>` placeholder turns are never priced,
  sidechain logs replaying a parent message under a fresh request id
  are deduplicated, and persisted `claude -p` runs count like
  interactive usage.
- **OpenCode free-model usage shows up** — messages on free models
  record a real cost of $0 with real token counts; those tokens now
  appear in the token totals and Usage Trend instead of vanishing.

## 0.4.9 — 2026-07-11

### Added
- **Cost/MTok** — the Total Spend ring's metric now cycles Cost →
  Cost/MTok → Tokens on click (right-click cycles backward). The center
  shows your true blended rate — total dollars over total megatokens —
  and each legend row shows that provider's own $/MTok.
- Cursor's spend rows say **"estimated"** — its usage export aggregates
  requests, so per-request exactness isn't possible.
- Reset countdowns inside a minute read **"Resets soon"** instead of a
  dying timer.

### Fixed
- **Long-context requests price correctly** — models with 1M-token
  context bill the *whole* request at a higher tier once the prompt
  crosses 200k tokens; Pane now applies those tiers. Claude's 1-hour
  cache writes bill at twice the input rate (they were priced as
  ordinary writes before), and Claude fast-mode requests apply the
  published fast multiplier. Spend histories reprice automatically —
  expect Claude's 30-day figure to correct upward.
- **Codex percentages show as reported** — the old "fresh window
  reads 1%, call it 0" normalization masked real early usage and is
  gone (the Mac dropped it too). "Not started" now keys on the window
  still being full-length, and windows under 5% used never flash a
  red pace projection off a floored reading.

## 0.4.8 — 2026-07-11

### Fixed
- **The whole model-family surface prices now** — reasoning-effort
  tiers (light/low/medium/high/xhigh), Max **and Ultra** modes, the
  fast tier, and any composition of them ("gpt-5.6-sol-max-fast",
  "…-ultra-high", "…-max-fast-xhigh") resolve to the base model's
  rates, with fast keeping its real per-family multiplier in every
  composition. Previously composed slugs fell into the unpriced ⚠
  bucket — on the test machine that was ~$19 of one day's Cursor
  Max-fast usage hiding from the totals. Verified by a 51-slug
  regression matrix across GPT-5.6 Luna/Terra/Sol; the handling is
  generic, so other families (Grok fast tiers etc.) get the same
  guarantees.
- **"Outdated" stopped crying wolf** — a single failed refresh no
  longer tags every card; data under three minutes old serves
  silently, and the amber tag (with the real error on hover) appears
  only when staleness is real. Persistent failures surface exactly as
  before.

## 0.4.7 — 2026-07-10

### Added
- **Devin spend** — the Devin CLI's local session store now feeds the
  Total Spend donut, spend rows, per-model breakdown, and usage trend,
  priced with the live catalog like the other CLIs. Windsurf-style
  model names ("gpt-5-6-sol-max") are normalized so they price, and
  the store is read through SQLite's backup API so numbers stay
  correct while the Devin app is actively writing. Cloud Devin
  sessions bill in ACUs and keep no local logs, so only CLI usage
  appears.
- **Dollars ⇄ tokens** — click the Total Spend ring (or right-click)
  to flip the donut, legend, and center total between money and raw
  token counts; the choice persists.
- **Reorder without leaving the popover** — every card grows a drag
  grip in its header; drop it where you want and the order saves to
  the same layout Customize edits.

### Changed
- **The popover looks like the Mac's now** — provider cards are a
  clean header over an inset panel, the usage trend sits in a labeled
  row, and the Total Spend ring is rebuilt from true wedge segments:
  radial-cut ends with soft corners, hairline gaps, and tiny spenders
  that stay thin slivers instead of swelling into dots. Hovering a
  wedge (or its legend row) slides it outward and dims the rest.
- **Spend colors** — Codex blue, Grok green, Devin sky blue, and
  Cursor its brand black (flipped to white in dark mode so it stays
  visible).
- **Share cards** — the copied image is a curated composition: buttons,
  links, and spend chrome stripped, the canvas hugs the content, the
  header aligns with the panel's text column, and the footer carries
  the app icon with the full tagline.
- **Unpriced usage keeps its tokens** — requests on models with no
  public pricing now count their measured tokens in token totals and
  the trend; dollars still refuse to guess, and the ⚠ (on the
  provider's spend row only) explains it in plain words.

### Fixed
- **Grok spend works again** — the Grok CLI changed its log format and
  the old scanner silently matched nothing; the new one reads token
  counts from the CLI's turn events and attributes models per process,
  like the Mac app.
- **Cursor Max-mode models price correctly** — "-max" slugs bill
  token-based at the base model's rates, so they now resolve through
  the full pricing chain instead of landing in the unpriced bucket.
- **Kilo fresh accounts** — a just-created account shows a friendly
  "no credits yet" card instead of an error.

## 0.4.6 — 2026-07-09

### Fixed
- **Cursor spend tiles work again** — Cursor's usage-events export now
  requires an explicit date range and token strategy; Pane sends both
  (last 31 days), so Today / Yesterday / 30 Days dollars populate.
- **New models price within the hour, not within the day** — when spend
  events reference models the price catalog doesn't know yet (new Cursor
  slugs ship often), Pane now rechecks the catalog hourly instead of
  daily, and a catalog update re-prices already-scanned logs instead of
  waiting for them to change on disk.

## 0.4.5 — 2026-07-09

### Fixed
- **Cursor Pro/Pro+/Ultra/Teams accounts now show real usage** — percent
  of the plan's included usage, Auto/API usage, on-demand spend, and
  credits, with the actual billing-cycle reset date, via the same API
  Cursor's own dashboard uses. Previously modern accounts showed only a
  meaningless "Requests this cycle: 0" from the legacy request-counter
  era (old request-based plans still fall back to it). Session tokens
  are auto-refreshed in memory when stale, and reading Cursor's login
  no longer fails when Cursor briefly holds a lock on its database.

## 0.4.4 — 2026-07-08

### Performance
- Background refreshes no longer rebuild the popover interface while
  it's hidden in the tray (~99% of the time) — rendering now happens
  once, at the moment you open Pane. Same for the 30-second countdown
  ticks. Less idle CPU, all day.
- New **Settings → General → "Liquid glass effects"** toggle (on by
  default). Turn it off on slower PCs: the glass refraction and blurs
  become clean flat surfaces, and the expensive lens machinery never
  even initializes — from the very first frame of a cold start.

## 0.4.3 — 2026-07-08

### New
- **Pane's new logo** 🎯 — the ring, everywhere: installer, app and
  taskbar icons, tray, popover sidebar, and share-card footers.
- **Update checks on every open** — the footer version stamp re-checks
  each time you open Pane and becomes a blue **⬆ Update** button when a
  new release is out; one click installs and restarts. (Replaces the
  floating update banner.)
- Party mode is now a triple-click on the sidebar logo away. 🎉

### Fixed
- Tray strip pairs now render logo-then-numbers, left to right, like
  the macOS original (Windows inserts new tray icons leftward).
- The update flow can no longer freeze at "Installing…" if a release
  disappears mid-install — it fails visibly with a retry button.

## 0.4.2 — 2026-07-08

**⚠ Installs of 0.4.1 and earlier: this release is signed with a new
key, so the in-app updater will decline it. Reinstall once via
`irm https://pane.jazii.dev/install.ps1 | iex` — updates then resume
normally.**

### Changed (security)
- **Breaking for browser-page consumers:** the local HTTP API no longer
  sends CORS headers, so web pages can no longer read
  `127.0.0.1:6736` through a browser — previously any website you visited
  could silently read your usage data. PowerShell, curl, Rainmeter, and
  native apps are unaffected (CORS only constrains browsers). If a
  legitimate browser integration needs access, open an issue — the plan
  is an opt-in origin allowlist, not a permissive default.
- Release binaries are now built and published by GitHub Actions from the
  pushed tag (public build logs), instead of on the maintainer's machine.
- The updater signing key was rotated to a passphrase-protected key
  (2026-07-08). Installs of 0.4.1 and earlier trust the old public key, so
  their auto-updater will decline the first release signed with the new
  key — reinstall once via `irm https://pane.jazii.dev/install.ps1 | iex`
  (or the release installer) and updates resume normally.
- **Webview hardening pass** (from a community security review): a strict
  Content-Security-Policy replaces the previous `csp: null`; the Tauri
  capability set is trimmed to core IPC only (the UI never used the
  opener/updater/process plugin APIs); `withGlobalTauri` is off; the
  pinned-metric dropdown is built via DOM instead of HTML strings.
- **Rust-side input validation:** `set_config` only accepts known config
  keys; tray-strip updates only accept known provider ids (also fixes
  unstarred tray icons of newer providers not being removed); pricing
  supplement alias rules are size- and count-capped.
- **Credential safety:** CLI credential files are copied to `*.pane-bak`
  before Pane writes a refreshed token back.

### Added
- SECURITY.md (private vulnerability reporting), docs/privacy.md (every
  network call), docs/providers.md (per-provider: files read + endpoints
  called), docs/local-http-api.md, CONTRIBUTING.md.

### Fixed
- Share cards (⧉) now copy the card exactly as it looks on screen —
  donut, tabs, theme and all — framed with a Pane logo footer, instead
  of a simplified redrawn version that didn't match the UI.

## 0.4.1 — 2026-07-08

First-run and Customize fixes from fresh-install testing.

### Fixed
- Fresh installs now start with just Claude + Codex enabled (their
  "connect me" cards are the onboarding); a PC with zero detected AI
  tools no longer enables all 18 providers.
- Rapidly toggling several providers off kept only the last change —
  toggles now apply instantly and save through a serial delta queue.
- Disabled providers disappear immediately from the dashboard, the
  Total Spend donut, and the tray strip (previously they lingered until
  the next refresh).
- Total Spend shows a quiet "No spend data yet" card on machines whose
  CLIs haven't logged usage, instead of no card at all.

## 0.4.0 — 2026-07-07

### New
- **Three CLI-detected providers** (research credit: steipete/CodexBar, MIT):
  Codebuff (credits + weekly limit, `codebuff login` file or key), Kilo
  (credit blocks + Kilo Pass, CLI login file or key), and Kiro
  (experimental — reads `kiro-cli /usage`). **18 providers total.**
- **Auto-updater** — Pane checks GitHub releases on launch and every 4
  hours; a banner offers one-click download + restart. Updates are
  cryptographically signed and verified before install.
- **Deeper metric rows** (Mac-parity polish): Claude per-model weeklies
  from Anthropic's new `limits` API (Fable era) + Extra Usage overage
  dollars; Codex Spark / Spark Weekly windows + Extra Usage credit
  balance; Z.ai monthly Web Searches quota; Grok pay-as-you-go cap badge.
- **"Not started"** — untouched 5-hour session windows say so (with an
  explainer) instead of showing a countdown that hasn't begun; Codex's
  floored 1%-on-fresh-window quirk is normalized to a true zero.
- **Keyboard** — Esc backs out of Customize/Settings, Ctrl+R refreshes.

### Removed
- Deepgram, OpenAI, Venice, Poe, Chutes, Warp, Crof, Amp, Vertex AI, and
  AWS Bedrock providers — cut to keep the lineup focused on the AI coding
  tools people actually track. Saved layouts self-clean any retired ids.

### Fixed
- Terminal windows no longer flash during refreshes (provider CLI checks
  now run windowless or scan the filesystem instead of spawning `cmd`).
- Retired providers no longer linger as ghost rows in Customize.
- Startup crash at higher provider counts (fetch futures now heap-boxed).
- Clicking the tray icon always reopens on the main page, even if the app
  was left on Settings or Customize.

## 0.3.0

### Renamed to Pane
- The app is now **Pane** (formerly OpenUsage for Windows). Installs to
  `%LOCALAPPDATA%\Pane`; settings move automatically from
  `%APPDATA%\OpenUsage` to `%APPDATA%\Pane` on first launch — keys, layout,
  and caches all carry over.

### Accuracy (Wave 9)
- **Live model pricing** — per-model rates now come from LiteLLM, models.dev,
  and the OpenUsage pricing supplement (daily refresh, ETag caching, offline
  fallback) instead of a hardcoded table. Claude spend was overstated ~2.6×
  at old Opus rates; Codex fast-tier requests now get their real multiplier.
- **Unpriced events are excluded, not guessed** — models no catalog prices
  are left out of totals and flagged with ⚠ (count + model names on hover).
- **Cursor spend** — computed from the dashboard's usage-events CSV export,
  priced locally.
- **Codex dedupe** — archived session copies no longer double-count.
- **Backoff & cooldown** — failing providers are benched 60s (5 min for
  rate limits) while cached data is served with the reason on hover.
- **Reset all layouts** also re-detects installed AI tools.
- **Codex reset credits** — each banked credit shows its exact expiry and a
  Use button that redeems it (confirm-guarded, idempotent).
- Single-instance guard; popover reopens scrolled to the top.

### UI
- Auto-hiding liquid-glass sidebar (prasen.dev lens: SDF rim refraction,
  chromatic fringe) with magnetic minimap trail; glass footer with build
  stamp; full-window Customize and accordion Settings panels; ☀/☾ theme
  toggle with circular wipe; per-day trend tooltips; skeleton loading,
  staggered card entrances, and a full light-mode audit.

### New
- **Wave 11 provider pack** — seven providers that authenticate with a pasted
  API key (Settings → API keys) or nothing at all:
  DeepSeek (balance), Moonshot/Kimi (balance, .ai + .cn), ElevenLabs
  (character quota with reset pacing), Deepgram (project balances),
  OpenAI (org costs — needs an Admin key), Venice (USD/DIEM/VCU balances),
  and Ollama (local server: installed + loaded models, no key needed).
  Providers added by an update that have no credentials on this PC start
  disabled — enable them in Customize.
- **MiniMax provider** — Coding/Token Plan quota (5-hour Session + Weekly
  windows) via the same endpoint the official `mmx quota` command uses. Key
  auto-detected from the MiniMax CLI's config, `MINIMAX_API_KEY`, or the new
  Settings field.
- **Copilot CLI / modern gh detection** — GitHub tokens are now also read
  from Windows Credential Manager (`gh:github.com:<user>`), which is where
  current gh versions (and the Copilot CLI) keep them. hosts.yml-only setups
  keep working.
- **Motion pass** — cards slide in with a stagger and bars fill when the
  popover opens, skeleton shimmer while first data loads, hover elevation on
  cards, smooth caret/tab/button transitions. All entrance animations play
  only on open (never on background refreshes) and respect the system's
  reduced-motion setting.

### Fixed
- config.json parsing now tolerates a UTF-8 BOM and logs parse failures
  instead of silently resetting settings to defaults.

## 0.2.0 — 2026-07-07

Full feature parity with the macOS original, plus Windows-specific polish.

### New
- **Antigravity support** — reads quota from the IDE's local language server
  when it's running, and falls back to Google's Cloud Code API (token from
  Windows Credential Manager) when it isn't. Session / Weekly / Claude /
  Claude Weekly metrics plus plan name.
- **Provider quick links** — Status / Dashboard links at the bottom of each
  card, same targets as the Mac app.
- **Share cards** — hover a card and click ⧉ to copy it as a PNG image to the
  clipboard (works for Total Spend too).
- **Local HTTP API** — `GET http://127.0.0.1:6736/v1/usage` (and
  `/v1/usage/:providerId`) serves the latest snapshots in the Mac app's
  documented wire format. Scripts written for the Mac app work unchanged.
- **Appearance** — System / Light / Dark theme setting.
- **Compact layout** — tighter density option.
- **Global shortcut** — e.g. `Ctrl+Shift+U` to toggle the popover from
  anywhere.
- **Proxy** — optional `socks5://` / `http(s)://` outbound proxy.
- **First-launch detection** — a fresh install starts with only the providers
  that have credentials on the PC; the rest wait in Customize.

### Fixed
- The popover no longer sits on "Loading usage data…" while the spend engine
  scans session logs on a cold start — usage cards paint immediately and the
  Total Spend card fills in when the scan finishes.
- Last-good snapshots are now cached **on disk**, so a transient provider
  outage (or rate limit) right after an app restart shows amber "⚠ Outdated"
  data instead of an error card. Entries expire after 24 hours.
- Drag-and-drop in Customize works (Tauri's native drag interceptor disabled).

## 0.1.0 — 2026-07-06

First Windows release: 10 providers (Claude, Codex, Cursor, OpenCode,
Copilot, Grok, Devin, OpenRouter, Z.ai, Antigravity detection), local spend
engine with model breakdown and 30-day trend, pace projections, toast
notifications, tray strip with per-provider icons, Customize screen, NSIS
installer, autostart.
