# Spec: 花费按「价格观察时间线」分段计费

## 用户拍板的产品决策（2026-10-25，会话内确认）

1. 日级源日内调价：**按日末最后观察价**（调价日全天用 ≤日末的最后观察）。
2. 回扫估算：**按首条观察价计 + ≈ 标记**，计过即冻结。
3. override：**只向前生效**；修历史走显式重建（二次确认）。

## 用户确认的计费语义（设计宪法）

1. 系统自动拉取价格数据后，从拉取时间点起按新观察到的价格计费；直到下次拉取发现某 provider/model 计价方式变化，才从「发现变化的拉取时间点」起切换。
2. 任何历史时间段一旦按当时价格计过费，之后无论涨价还是降价，都不得追溯重算（**双向不追溯**）。
3. 每个时间段的计费 = 该时间段内消耗的 token × 该时间段最后一次被观察到的有效价格。

## 核心思路

把「现在查到什么价」改成「事件发生时点上已被观察到的价」。新增**只追加、不可改**的观察时间线 `schedules.json`；所有扫描器定价走 `resolve_pin_at(scope, 时段末, model)`；缓存探针从「按 now 回放」改为「按事件 ts 回放钉好的价」。时间线不可变 ⇒ 探针永久可复现 ⇒ catalog 刷新、涨跌价、override、corrections 都不再触发已计费文件重解析——双向不追溯由数据结构保证。

## ① 数据模型

文件：`%APPDATA%\Pane\pricing\schedules.json`（与 catalog 同目录，原子写）。只 append，禁改禁删历史条目；外部篡改检测（条目数减少/哈希链断）→ 隔离文件并要求用户显式重建，不自动修。

```json
{
  "version": 1,
  "observations": [
    {
      "scope": "codex",                 // 扫描器族 = provider card id
      "model": "gpt-5.5-codex",         // 规范化后的日志原样 key（trim+lowercase）
      "observed_at": 1750000000000,     // 发现该价格的拉取完成时刻（epoch ms）
      "pin": { "input": 1.25, "output": 10.0, "cache_read": 0.125, "cache_write": 1.5625,
               "input_200k": null, "output_200k": null, "cache_read_200k": null,
               "cache_write_200k": null, "cache_write_1h": null, "cache_write_1h_200k": null },
      "fast_mult": 2.0,                 // 当时 supplement 倍率；无则 1.0
      "priority_mult": null,            // 仅 codex/devin 类 scope 使用
      "long_context": "off",            // 或 { "threshold": 272000 }
      "basis": "catalog"                // catalog|builtin|override|correction|manual
    }
  ]
}
```

- **区间语义**：同 (scope, model) 按 observed_at 排序，第 i 条覆盖 `[obs_i, obs_{i+1})`，最后一条开区间；**首条额外覆盖 (-inf, obs_1) 但查询标记 estimated=true**。`pin: null` = 该时刻观察到「无价」，区间内事件走 note_unpriced。
- **粒度**：精确 (scope, model)，**不用前缀匹配**。现 `PriceWindow.model_prefix`（pricing.rs:558-566）保留仅供测试——生产前缀匹配会让同前缀新模型继承旧价，破坏复现确定性。模型改名/alias 变化天然落在新 key 上，各自成线。
- **空值/未知模型**：未知模型 → 实时解析；解析失败返回 None 且**不写条目**（避免日志里每个陌生 slug 污染时间线）；仅「有价→无价」跃迁才写 `pin:null`。
- **查价即定价** `resolve_pin_at(scope, seg_end_ms, model)`：取 observed_at ≤ seg_end 的最后一条；无则取第一条 + estimated。请求级源 seg_end = 事件 ts；日级源（cc-switch/workbuddy/codebuddy 的 add_day_event）seg_end = 当地日 23:59:59.999。日级源无法拆分日内调价，属已知近似（见风险 2）。

## ② 与 catalog_stamp 共存

- **探针改造**：`PriceProbe` 增加 `Pin { scope, key, ts_ms, pin: Option<PricePin> }`，替换 Lookup/LookupAt/FastMult 三 variant（`PERSIST_VERSION` 5→6，旧缓存整份弃用，一次性全重解析）。探针复放 = 用同一 ts 重查时间线；时间线只追加 ⇒ 已计费文件永久复放相同 ⇒ stamp 变化后 `probes_still_vouch` 恒真 ⇒ 不重解析。`catalog_stamp()` 降级为「完全相同则跳过复放」的快路径优化，不再承担正确性。
- **重解析变得无害**：即使因 overflow（>64 探针）、文件增长、checkpoint 异常而重解析，因每个事件按自己 ts 查时间线，历史段结果逐字节相同——重解析从「正确性风险」降级为「性能浪费」。
- **CORRECTIONS_REV 不再绕过**：现在 rev 不匹配 → 整份弃缓存 → 全量重解析 → 历史被现价重算（用户禁止）。新规：rev 变化不弃缓存；启动时检测到 → 对所有在用 (scope, model) 以 `basis:"correction"`、observed_at=升级时刻追新观察，**仅向前生效**。PERSIST_VERSION（解析器逻辑变更）维持「整份弃+全重解析」，但时间线保证其美元中性。另提供显式「重算全部历史」命令（用户确认后执行）作为修正历史 bug 的逃生门。

## ③ 双向不追溯：重算路径封死清单

| # | 现有路径 | 现行为 | 封死方案 |
|---|---|---|---|
| 1 | stamp 变→Lookup 探针按 now 复放失败→整文件重解析（spend.rs:527/972/1158） | 涨/跌价均追溯 | Pin 探针按 ts 复放，时间线不可变 |
| 2 | lookup_at 现无 window 时等于 now 查价（spend.rs:340-342；仅 codex:2249/copilot:205/gemini:3561 用时戳） | 追溯 | 时间线填充 + 全扫描器改走 pin |
| 3 | Overflow 探针（>64 模型或 key>128 字符，spend.rs:332/374）恒 false | 每次 catalog 变强制重算 | 重解析美元中性；顺带放宽上限 |
| 4 | CORRECTIONS_REV 不匹配整份弃缓存（spend.rs:509） | 追溯 | 改为向前追观察 |
| 5 | pricing_overrides.json 保存→清 memo→探针失败→重解析（pricing.rs:582-593；override 优先于 window，684） | 保存即追溯改历史 | override 保存=追一条 basis:"override" 观察；lookup_at 移除 override 优先分支（仅 lookup() 保留供新 pin 解析） |
| 6 | unpriced 无探针→probes_still_vouch(空)=真→后学价永不补（spend.rs:353-355） | 反向缺口（永久 $0） | unpriced 也记 Pin{pin:None}；后补价仅触发一次「填充」，填充后冻结 |
| 7 | spend_history::merge_daily 用本次扫描值覆写历史日格（spend_history.rs:140-148） | 美元变则历史变 | pin 下覆写值逐字节相同 ⇒ 美元中性；填充/显式重建除外（均一次性） |
| 8 | catalog 刷新 mid-scan（GENERATION bump） | 同文件前后段不同价 | 本就按事件 ts 定价，混合聚合即正确 |
| 9 | alias 规则/fast 倍率/baked 表随 catalog 变动（pricing.rs:346-400、778-838） | 探针失败→追溯 | 刷新时 diff 解析价即捕获 → 新观察 |

## ④ 首次安装/回扫规则

- **无观察的旧用量**：首次 collect 时 ensure_fresh 先拉 catalog → 对所有在用 (scope, model) 实时解析并落观察，observed_at = catalog 拉取时刻（state.json fetchedAt，非 now；无 catalog 文件时为 now，basis:"builtin"）→ ts 早于首条观察的事件按首条观察价计，estimated=true。一旦计过即冻结。
- **估算标记落两处**：`PersistEntry` 新增 `estimated: Vec<(day, model)>`；spend_history 的 `ModelDay` 新增 `estimated: bool`（旧记录默认 false），UI 在 History/Daily 视图显示「≈」。
- **一次性迁移**：PERSIST_VERSION=6 → 旧缓存弃用 → 全量重解析一次，历史统一按最早观察价估算。发布说明必须明示。
- **重解析触发条件（完备列举）**：a) mtime/size 变；b) zstd 恒全量；c) 探针复放失败（仅 None→Some 填充或时间线被手工改）；d) PERSIST_VERSION 变；e) 用户显式重建；f) checkpoint 边界/hash 异常。**catalog 变化、涨价、降价、override、corrections 均不在列。**
- **重建流程**：新 Tauri 命令 `rebuild_spend_pricing(scope: Option<String>)` → 清 spend_cache（可选清 spend_history 指定范围）→ 强制全重解析 → 时间线不可变使结果除目标变化外逐字节一致。UI 二次确认。

## ⑤ 自动拉取链路改造

现状（已验证）：`collect_daily`（spend.rs:6537，阻塞线程）→ `pricing::ensure_fresh()`：24h 拉取（有 unpriced 时 1h）、失败 30min 重试、ETag 304、10min 启动宽限、32MB 上限；前端每 refreshMinutes（默认 5）调 fetch_spend（main.ts:12725）。

1. ensure_fresh fetch 循环结束后，**若任一 source 发生 ingest（GENERATION bump）** → `record_observations()`：从 spend cache 枚举在用 (scope, model)（需给 FileEntry/PersistEntry 加 scope 字段，由各 scanner 注册时写入）→ 用新 catalog 逐 scope 解析 pin → 与时间线末条 diff → **仅变化者追加**，observed_at = 本次 ingest 时刻 → 原子写 schedules.json。
2. **扫描期首见**：resolve_pin_at 未命中时实时解析并记观察，observed_at = 当前 catalog epoch（最近一次成功 ingest 时刻），保证同一轮拉取内首见模型与刷新 diff 的时刻一致。
3. 失败/退避/宽限不变：拉取失败 ⇒ 无新观察 ⇒ 价格保持钉住 ⇒ 无追溯。304（内容未变）不产观察。
4. **与 ccusage 日更 CI 的分工**：Pane 是桌面端，运行时就地拉取是唯一价格入口，不需要 CI 快照；baked 表（builtin_price/kimi/tencent）走 basis:"builtin" 观察，属 CORRECTIONS_REV 语义（仅向前）。CI 嵌入快照作为断网红生方案列为后续可选项，不在本设计内。

## ⑥ 边界（全部按「当时规则」）

- **cache write 5m/1h 两档**：Price 已含 cache_write(5m)/cache_write_1h 及各 200k 变体，pin 整卡钉住；厂商后补 1h 显式价 = 新观察，旧段保持「×2 input」约定（该约定是代码，改约定走 corrections 仅向前）。
- **200k 阈值/整请求升档**：阈值与 tier 费率都在 pin 内（long_context + *_200k 字段）；规则本身变化（codex 272k、聚合源 opt-out 用 "off"）表现为新 pin 的 threshold 不同 → diff 追加。request_cost_at 的纯数学不变，不钉。
- **fast/flex 倍率**：fast_mult 入 pin（来源 supplement fast_multipliers）；flex 倍率 Pane 现无实现，若引入从第一天起就是 pin 字段。
- **模型改名/alias**：按日志原样 key 各自成线；alias 规则变→同 key 解析价变→diff 追新观察，旧拼写历史不动。Cursor CSV 品牌拼写（cursor-grok-4.6）是独立 key，天然正确。

## ⑦ 验收断言（parse-tests 可测，15 条）

1. 时间线只追加：改历史条目被拒/隔离；区间边界 [obs_i, obs_{i+1}) 查价正确。
2. **涨价不追溯**：t1 扫出成本 X → 追加更高价观察 t2 → 再 collect：文件不重解析（探针复放真）、成本仍 X；t2 后新事件按新价。
3. **降价不追溯**：同 2 镜像（低价）——现有代码必然翻车、重点防的路径。
4. 日内调价：10:00/15:00 两事件 + 12:00 观察 ⇒ 日总 = 旧价×早段 + 新价×晚段（请求级）。
5. 日级源：观察落在日中 ⇒ 全天按「≤日末的最后观察」计。
6. 回扫估算：无观察首扫 ⇒ estimated 标记落 PersistEntry 与 ModelDay；后到观察不改旧日。
7. 填充一次：模型先 unpriced（Pin None）→ 补观察 → 仅该文件重解析且旧日变有价；再变价不再动它。
8. Overflow 文件（>64 模型）在 catalog 变化后重解析，美元逐字节相等。
9. CORRECTIONS_REV bump：缓存不弃、旧日不变、新事件用修正价。
10. override 保存：仅追加向前观察；历史不变；新事件用 override 价。
11. stamp 空转（同内容 touch mtime）：零重解析。
12. alias/倍率变化 → 新观察、旧文件美元不变。
13. checkpoint 续扫：追加行按新 pin，旧行美元保留。
14. merge_daily 幂等：连续两次 collect 的 spend_history 序列化字节相同。
15. 显式重建：无变化时重建前后美元一致。

## ⑧ 风险清单（按严重度）

| # | 风险 | 缓解 |
|---|---|---|
| 1（高） | 一次性迁移（v5→v6）把全部历史重估为「最早观察价」，老用户总额会跳变 | 发布说明 + UI「≈估算」标识，只此一次 |
| 2（高） | 日级源（cc-switch/workbuddy/codebuddy）无法拆分日内调价，整日按日末最后观察计——与「绝不在调价前按新价计」在调价日落有语义张力 | 需产品确认接受（备选：保守按日初观察、次日生效）；UI 标注 |
| 3（中） | schedules.json 被外部编辑/损坏 | 只追加 + 计数与摘要校验，违例隔离并走显式重建，禁止静默自动修复 |
| 4（中） | 系统时钟回拨导致 observed_at 乱序 | 每 key 单调性保护（拒绝早于末条的时刻，或钳制并告警） |
| 5（中） | scope 是新引入维度，scanner 注册表需与 provider card id 严格对齐，错 scope = 错价 | 注册时单测校验 scope 集合 |
| 6（中） | 扫描期首见模型写观察 → 并发写时间线 | ensure_fresh 的 diff 与扫描线程同在阻塞线程实际串行；仍加 Mutex + 原子写 |
| 7-10（低） | estimated 标记偏保守 / unpriced→priced 填充改一次历史美元（界定为「填充」非「重计价」）/ override 不再能修历史（Settings 文案需改）/ 探针复办成本 O(log n) 二分可忽略 | 文档写明 |

## 关键文件

`src-tauri/src/pricing.rs`（lookup_at:680、SCHEDULES:566、catalog_stamp:223、ensure_fresh:442、CORRECTIONS_REV:190）、`src-tauri/src/spend.rs`（PriceProbe:320-419、load_persisted_cache:504、file_days:954/1138、add_event:592、codex_emit:2228、collect_daily:6532）、`src-tauri/src/spend/sources/copilot.rs:205`、`src-tauri/src/spend_history.rs:126-163`、`src-tauri/src/lib.rs:3726-3754`。
