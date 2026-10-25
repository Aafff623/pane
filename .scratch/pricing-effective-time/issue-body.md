Status: ready
Issue: https://github.com/Aafff623/pane/issues/56
Priority: P1
Area: Spend

# 花费按「价格观察时间线」分段计费——调价不追溯改历史

## 问题

用户确认的计费语义（必须严格遵守）：

- 系统自动拉取价格数据后，从拉取时间点起按新观察到的价格计费；直到下次拉取发现某 provider/model 计价方式变化，才从「发现变化的拉取时间点」起切换。
- 任何历史时间段一旦按当时价格计过费，之后无论上游**涨价还是降价**，都不得追溯重算（双向不追溯）。
- 每个时间段的计费 = 该时间段内消耗的 token × 该时间段最后一次被观察到的有效价格。

现状恰恰是用户禁止的行为：成本虽在扫描时算、美元固化落盘（spend.rs:592-599 → spend_cache.json），但 `catalog_stamp()`（pricing.rs:223-244，= catalog mtime+size+CORRECTIONS_REV）一变 → `load_persisted_cache`（spend.rs:504-535）对相关文件**整文件重解析、按新价重算**；`CORRECTIONS_REV` 变更更直接整份弃缓存全量重算（pricing.rs:190-210 注释明言此设计）。`PriceWindow`/`lookup_at`（pricing.rs:558-706）是现成的当时定价机制，但 `SCHEDULES` 默认空、只有 3 个 scanner 走时戳定价，处于空转。

## 方案（详见 spec.md）

**核心思路：把「现在查到什么价」改成「事件发生时点上已被观察到的价」。** 新增只追加、不可改的观察时间线 `%APPDATA%\Pane\pricing\schedules.json`（精确 (scope, model) 粒度，不用前缀匹配）；所有扫描器定价走 `resolve_pin_at(scope, 时段末, model)`；缓存探针从「按 now 回放」改为「按事件 ts 回放钉好的价」。时间线不可变 ⇒ 探针永久可复现 ⇒ catalog 刷新、涨价、降价、override、corrections 都不再触发已计费文件重解析——双向不追溯由数据结构保证，不靠 if 判断。

关键机制：
- 区间语义：同 (scope, model) 条目按 observed_at 排序，第 i 条覆盖 `[obs_i, obs_{i+1})`；**首条额外覆盖 (-inf, obs_1) 但查询标记 estimated=true**（回扫估算）。
- 请求级源 seg_end = 事件 ts；日级源（cc-switch/workbuddy/codebuddy）seg_end = 当地日 23:59:59.999（日粒度近似，见风险 2）。
- `PriceProbe` 增加 `Pin` variant 替换 Lookup/LookupAt/FastMult（PERSIST_VERSION 5→6，旧缓存一次性整弃全重解析，发布说明必须明示）。
- `catalog_stamp` 降级为快路径优化；`CORRECTIONS_REV` 变化不再弃缓存，改为向前追一条 `basis:"correction"` 观察；override 保存 = 追一条 `basis:"override"` 观察（仅向前）；unpriced 也记 `Pin{pin:None}`，后补价只触发一次「填充」而非重计价。
- 自动拉取链路：`ensure_fresh` 的 fetch 循环结束后若发生 ingest → `record_observations()` 枚举在用 (scope, model) 与新 catalog 逐 scope diff，**仅变化者追加**，observed_at = 本次 ingest 时刻。
- 重解析触发条件收敛为：mtime/size 变、zstd 恒全量、探针复放失败（仅 None→Some 填充）、PERSIST_VERSION 变、显式重建、checkpoint 异常。**catalog 变化/涨跌价/override/corrections 均不在列。**

## 验收

- parse-tests 15 条断言（spec.md §7）：重点「降价不追溯」（现有代码必然翻车的路径）、涨价不追溯、日内调价分段、日级源日末规则、回扫 estimated、CORRECTIONS_REV bump 不弃缓存、override 保存仅向前、merge_daily 幂等（连续两次 collect 序列化字节相同）。
- 迁移影响：v5→v6 一次性全重解析，老用户历史总额会按「最早观察价」重估并标 ≈——发布说明明示，只此一次。

## 需要用户拍板的产品决策（已与用户确认语义，落地前需最终确认）

1. **日级源的日内调价近似**：调价当天，cc-switch/workbuddy/codebuddy 类日粒度源无法拆分日内，整日按「≤日末的最后观察」计——与「绝不在调价前按新价计」在调价日落存在语义张力。默认方案即上述；备选：保守按日初观察、次日生效。
2. **首次安装的回扫估算**：早于首条观察的历史按首条观察价计并标 estimated（UI 显示 ≈）。
3. **override 不再能修历史**：Settings 里改价只向前生效；修历史走显式重建命令（二次确认）。

## 不做

- 不做 ccusage 式日更 CI + build.rs 嵌入快照：Pane 是桌面端，运行时就地拉取是唯一价格入口（设计 §5-4）；断网嵌入快照列为后续可选项。
- 不做前缀匹配的 PriceWindow 生产化（保留仅供测试）——会让同前缀新模型继承旧价，破坏复现确定性。
