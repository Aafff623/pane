Status: done (2026-10-26)
Type: task
Blocked by: —

# 01 · project_month 纯函数 + 命令 + spend 卡片一行

## 做什么

1. spend_history.rs：`project_month_from_series`+`project_month_from(file,today)`+`project_month()`（Local::now 包装）；常量与规则见 spec；别名 fold 复用 fold_day_aliases。
2. lib.rs 新命令 fetch_spend_projection（注册表 :6429 一带）或经 fetch_spend_history 扩展。
3. main.ts：rangeCol()（:5520-5539）插投影行；Promise.all 三元（:13642-13649）；title tooltip 构成；i18n 3 key；仅 !rtkMode && cost 指标时渲染。
4. parse-tests 9 条。

## 验收

9 条全绿；`cargo check`+`pnpm build` 绿；本机出现「本月预计 ≈$X」且可手算复核；月末当天与有效日<3 不显示；owner UI 验收。
