Status: done (2026-10-26) — failed config writes reload backend config and reapply appearance/scale/skin/tray + toast.
Type: task
Blocked by: —

# 03 · 设置事务性保存（联动动作失败回滚）

## 做什么

1. 盘点有联动动作的设置项（托盘形态/更新频率/皮肤切换/语言等），列清单。
2. 保存路径改事务模式：diff 新旧 → 逐字段应用副作用 → 任一依赖动作失败 → 回滚内存态与已写副作用 → 整次保存报失败（前端 toast），杜绝「界面已改好、实际没生效」。
3. config.json 写入保持原子写。

## 验收

- 构造副作用失败（mock）→ 回滚且报错；正常路径无回归。
- owner 场景验收：切换托盘形态等项失败路径提示清晰。
