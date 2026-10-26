Status: done
Type: task
Blocked by: —

# 03 · 动态刷新间隔（临近重置/低余量加速）

## 做什么

1. spawn_auto_refresh（lib.rs:3681）睡眠计算改造：`interval = clamp(基线分钟 × f(余量, 临近重置), 1min, 基线)`——余量 <20% 或 5h 窗 30 分钟内重置 → 缩到 1min；成功后回用户配置值。
2. 纪律：失败退避优先于加速（退避期不允许插队）；加速不突破 usage_fetch_lock 全局节奏。
3. **不做** session 文件活动探测（强耦合 Codex CLI 私有 schema，多 provider 看板不值维护成本——决策记录在案）。

## 验收

- 单测：f 的边界（20%/30min 阈值）；退避优先级。
- owner 场景验收：低余量卡片明显比满余量刷得勤；重置后回常规节奏。
