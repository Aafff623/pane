Status: done
Type: task
Blocked by: —

# 02 · 快捷键内核：命令表 + 纯函数 + 单点分发

## 做什么

1. `shortcutCommands` 命令表：盘点 Pane 应用内命令（导航/搜索/翻页/发送/设置/主题…），每条 id/channel/scope/defaultBindings[]（结构照 ZCode shortcutCommands.ts:62-103）。
2. 纯函数内核（Apache-2.0 可直接抄 ZCode bindings.ts/conflicts.ts，保留版权头 + LICENSE 署名）：IME/长按噪声过滤（keyCode 229）、修饰键精确匹配+平台隔离、event.code 兜底、录制器（裸键防呆）、生效表=默认+覆盖整组替换（显式空数组=用户清除不回退）、保留键黑名单、物理等价归一（Ctrl+m ≡ CmdOrCtrl+m）、同作用域占用检测、抢绑二次确认、多绑定行级增删换。
3. 分发收口：现有 ~15 处独立 keydown 监听（核心 main.ts:20102-20186）收拢为单 capture 监听+命令表路由；handlers 走 ref；录制态短路；无 handler 不 preventDefault。
4. 持久化升级：config 快捷键结构改为 Record<id, bindings[]>（旧值迁移）。

## 验收

- 内核纯函数单测（node --test）：匹配/归一/冲突/录制各分支。
- 手动验证：现有 6 条 LOCAL_SHORTCUTS 全部经新分发生效、行为无回归；Pane 系统热键（Alt+2/Shift+1）不受影响。
