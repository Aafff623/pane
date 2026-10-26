Status: ready
Priority: P2
Area: Architecture

# cockpit-tools 语义借鉴：Retry-After 解析 / 额度身份校验 / 动态间隔 / OAuth 纪律 / token 锁 / 窗口语义补强

## 背景

cockpit-tools（jlcodes99，**CC BY-NC-SA 4.0**）第三轮深挖：单飞调度本体已立项 #54（refresh-single-flight），本轮补 #54 未覆盖的调度与 OAuth 语义，外加 20 平台端点情报库。**全部为 Pane 原创重写，零代码移植**（许可证红线）。证据：`temp/research/20261025-cockpit-deep-dive.md`（含 20 平台额度获取情报表：端点/鉴权/窗口语义/刷新节奏/风险分级）。

## 范围（6 票）

- **01 统一 Retry-After 头解析**（P1，半天）：现在只有 claude 写 retry_after_s，全部 provider 429 都该吃到；退避=max(指数封顶 300s, 服务器提示封顶 3600s)。
- **02 额度响应身份校验+凭据指纹**（P1/P2）：带账号标识的响应在 snapshot 写入前比对，不符 alert；失败日志记指纹不含原文。
- **03 动态刷新间隔**（P2）：余量<20% 或临近重置临时加速到 1min，退避优先于加速；**不做活动探测**。
- **04 OAuth 三纪律**（P2）：双栈 loopback / 手动粘贴回调兜底 / pending quarantine（device 流永不恢复）。
- **05 账号级 token 刷新锁**（P3，依赖 #54）：双检代际复用防双消费 refresh_token。
- **06 provider 窗口语义补强**（P3）：按情报表逐家核对（codex 双窗/copilot 档位/cursor 双 reset/Argon 周窗/grok 单池）；kiro 403→BANNED 教训。

## 明确不做

session 活动探测全量实现（强耦合 Codex 私有 sqlite schema）；「错误字符串即通道」的字段集扩展（走结构化字段）；跨进程文件锁；20 平台 plan 分类 UI；OAuth client 内置凭据（法律红线，任何情况不搬运）；解密他人凭据存储（claude cookie/zcode enc:v1:/workbuddy safeStorage——保持拒绝）。

## 验收

见 issues/ 各票；01/02/06 有 parse-tests 断言；03/05 owner 场景验收。
