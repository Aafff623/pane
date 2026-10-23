# 02: 三套余量接口解析与单位归一

**What to build:** 给 `xfyun-speech` 实现取数与解析：枚举账号下所有应用，对每个应用拉三类余量（次数类、录音文件转写、实时语音转写），归一成 Pane 的 `Metric` / `UsageReading`，并把「登录失效」「无此计量」「取到但缺字段」三种情况严格区分开。

**Blocked by:** 01

**Status:** ready-for-agent

## 要点

- 请求：`GET https://console.xfyun.cn/dashboard/…`，**只带 `Cookie` 即可**（实测 `User-Agent` / `Referer` / `Origin` 不影响结果）。沿用 `xfyun.rs` 的 `call_console` 形态（8s 超时、`MAX_BODY_BYTES`），但**新增信封判定**：
  - `flag == true && code == 0` → 正常；
  - `flag == false && code == 80000` → `CallErr::Auth`（不能只看 HTTP 状态码，实测恒 200）；
  - `flag == false && code == 20003` → 我们参数拼错了，按 `Other` 报并带上下文；
  - `flag == true && data == null` → 该渠道无此计量，是正常空态。
- 应用枚举：`/dashboard/app/list?currPage=1&keyword=&type=`（参数名必须是 `currPage/keyword/type`；带 `pageSize` 实测 `20003`），按 `totalCount` 分页。`/dashboard/app/default` 只代表默认应用，不能当全部。`currentPageData[].serviceList` 返回 null，所以**某个 app 开了哪些服务看不出来**，只能逐渠道探测。
- 次数类：`/dashboard/meter/api-use-count/get?businessId=<ch>&appId=<appid>` → `{usedCount, threshold, remainCount, left, expireDate, historyUsedCount, conc}`。实测有效渠道仅 `tts` / `iat` / `ise`；`lfasr`、`rta`、`rtasr`、`spark` 等返回 `data:null`。语义：`left` 是套餐包余量，`threshold/remainCount` 是 500 次试用池（`remainCount = threshold - usedCount`），两者不是一个池子。渠道清单不要外推，未实测的标 `待确认`。
- 录音文件转写：`/dashboard/lfasr/getlfasrInfo?appId=` → **数组**，每项 `{count, leftCount}`（小时）、`effectiveDate/dueDate`（`YYYY-MM-DD HH:MM:SS` 字符串）、`status/convertStatus`（含义待确认）。多张包需按 `dueDate` 排序后再决定合并还是分行。
- 实时语音转写：`/dashboard/lfasr/get-rtasr-app-auth?appId=` → `leftCount`（**秒**）、`usedCount`、`maxConnection`（路数，免费档 1 路）、`expireTime`（**unix 秒**）、`connectType/timeType/isDel`（含义待确认）。
- 单位映射到现有 `UsageUnit`（Tokens/Credits/Minutes/Requests/Characters/Pages，不新增枚举除非实现者认为更清楚）：次数 → `Requests`；lfasr 小时 → `Minutes` ×60；rta 秒 → `Minutes` ÷60。换算不得产出负数或 NaN。
- 三种到期编码（无时区字符串、带秒字符串、unix 秒）分别解析；解析不出来显示「未知」，不伪造日期。
- **禁止调用** `/dashboard/webapi/getInfo/v1` 与 `/dashboard/app/getHttpSign`：实测它们返回明文 APIKey/APISecret，本需求不需要。
- 测试用合成 JSON，不打真实网络；fixture 脱敏（不写真实 cookie/uid/APPID/订单号）。必测：信封四种组合、`data:null`、lfasr 空数组/单张/多张乱序到期、秒与小时换算、非法 `expireTime`、`code=80000` 不产生「0 额度」。

## 验收

- [ ] 五个资源包各自取到剩余量、单位和到期，数字与网页控制台一致（owner 实机对照）。
- [ ] 会话过期时是「重新登录」，不是 0；未开通渠道是空态，不是错误。
- [ ] 多应用账号能枚举出全部 app，不漏第二个应用。
- [ ] 试用池与套餐包在数据层就是两条事实，没有被相加成一个数。
- [ ] `parse-tests`（`cd parse-tests; cargo +stable-x8_64-pc-windows-gnu test`）与 `node --test tests/*.test.mjs` 全绿；解析器对未知结构诚实失败。

## Comments

- 2026-10-23：全部响应样本与字段语义推断来自本轮真实登录态实测，见 `temp/xfyun-console-quota-analysis-20261023.md`；其中列了 7 条待确认（试用池划分、时区、`status/convertStatus/connectType/timeType/isDel` 含义、其余渠道 id、多张包消费顺序）。限流方面：并发 12 次无异常，但官方无承诺。
