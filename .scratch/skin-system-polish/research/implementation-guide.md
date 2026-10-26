# 主对话实施指引（草案）

## 1. 市场读取：把索引 fetch 收到 Rust

当前：

```ts
const res = await fetch(SKINS_INDEX_URL, { cache: "no-store" });
```

建议改成一个 Rust 命令，复用现有直连回退：

```rust
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RemotePackEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub size: u64,
    pub sha256: String,
    pub url: String,
}

pub async fn skin_pack_index() -> Result<Vec<RemotePackEntry>, String> {
    let resp = super::providers::send_with_direct_fallback(|client| {
        client.get("https://pane.threetwoa.live/skins/index.json")
    })
    .await?;
    if !resp.status().is_success() {
        return Err(format!("skin index: HTTP {}", resp.status()));
    }
    let doc = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("skin index JSON: {e}"))?;
    serde_json::from_value(doc.get("packs").cloned().unwrap_or_default())
        .map_err(|e| format!("skin index shape: {e}"))
}
```

前端：

```ts
export async function refreshSkinIndex(): Promise<void> {
  let entries: RemotePackEntry[] | null = null;
  try {
    entries = await invoke<RemotePackEntry[]>("skin_pack_index");
  } catch {
    // Browser demo has no Tauri invoke. The installed app must use Rust.
    const res = await fetch(SKINS_INDEX_URL, { cache: "no-store" });
    if (res.ok) {
      const doc = await res.json();
      entries = doc.packs ?? [];
    }
  }
  // validate, dedupe by id, cache last good copy
}
```

同时给静态站补第二层头：

```text
/skins/*
  Access-Control-Allow-Origin: *
  Access-Control-Allow-Methods: GET, HEAD, OPTIONS
```

验收必须包含全新 profile 与空 `%APPDATA%\Pane\skins`。

## 2. Comic Pixel 颜色 token

先补固定前景 token：

```css
:is(:root[data-skin="comic-pixel"], .skin-sample[data-skin="comic-pixel"]) {
  --cp-yellow-ink: #080a0d;
  --cp-cyan-ink: #03231f;
  --cp-purple-ink: #ffffff;
}
```

所有黄色/青色底只允许使用对应 ink：

```css
.overview-cat-tabs .tab.active,
.spend-group-tabs .tab.active {
  background: var(--cp-yellow);
  color: var(--cp-yellow-ink);
}

.spend-metric-tabs .tab.active {
  background: var(--cp-cyan);
  color: var(--cp-cyan-ink);
}

.spend-col .col-total,
.spend-total-line strong {
  background: var(--cp-yellow);
  color: var(--cp-yellow-ink);
}
```

不要：

```css
color: var(--foreground);  /* 暗色白 */
color: var(--cp-frame);     /* 暗色白 */
```

## 3. 数据字体边界

建议 pack 自己持有可读字体栈，不修改用户全局 `uiFont`：

```css
:is(:root[data-skin="comic-pixel"], .skin-sample[data-skin="comic-pixel"]) {
  --cp-readable-font:
    "Segoe UI Variable Text",
    "Microsoft YaHei UI",
    "PingFang SC",
    sans-serif;
}

:is(...scope...) :is(
  .provider:not(.total-spend):not(.quota-overview) .provider-name,
  .overview-item-name,
  .left-val,
  .head-pct,
  .ovbar-pct,
  .ovbar-meta,
  .metric-label,
  .provider-meta,
  .setting-row,
  .settings-note,
  input,
  select,
  textarea,
  .mini-btn,
  .action-btn
) {
  font-family: var(--cp-readable-font);
  font-style: normal;
  letter-spacing: normal;
}
```

保留 display 字体的位置：

- `.total-spend > .provider-head .provider-name`
- `.quota-overview > .provider-head .provider-name`
- 设置分区 head
- 工坊 detail 标题

不要保留 display 字体的位置：

- Provider 卡名
- 额度百分比
- 重置时间
- 表单 label / value
- 告警正文

## 4. 组件材质分层

把现有“所有卡片同一粗框”拆成四类：

```css
/* hero：最强框体 */
.total-spend,
.quota-overview {
  border: 3px solid var(--cp-accent-frame);
  box-shadow: 5px 5px 0 var(--cp-shadow);
}

/* 数据卡：安静，只留状态侧条 */
.provider:not(.total-spend):not(.quota-overview) {
  border: 1px solid var(--cp-line-soft);
  box-shadow: none;
}

/* 控件：贴纸切角，只在 active/pressed 出现强轮廓 */
.mini-btn,
.action-btn,
.seg-btn,
.period-btn {
  border: 1px solid var(--cp-line);
  box-shadow: 2px 2px 0 var(--cp-shadow);
}

/* 状态：独立场景，不与正常卡片同构 */
.provider.muted,
.needs-setup,
.placeholder.loading,
.board-empty {
  border-left-width: 6px;
  box-shadow: none;
}
```

不建议继续用同一条规则一次覆盖 `.provider,.st-card,.acc-group,...` 的所有 3px 边框。

## 5. 像素图标接入

把 26 个 SVG 放到 pack：

```text
skins/comic-pixel/assets/icons/pixel/
```

用 CSS mask，不改 `uiIcon()` 的全局 API：

```css
:root[data-skin="comic-pixel"] :is(
  #theme-btn,
  #refresh,
  #skin-btn,
  #settings-btn
) .ui-icon {
  display: none;
}

:root[data-skin="comic-pixel"] #theme-btn::before {
  content: "";
  width: 16px;
  height: 16px;
  background: currentColor;
  -webkit-mask: url("assets/icons/pixel/sun.svg") center/contain no-repeat;
  mask: url("assets/icons/pixel/sun.svg") center/contain no-repeat;
}

/* data-settings-tab / data-overview-* 同理，按语义逐个映射 */
```

注意：

- 原生 SVG 仍保留给 Signal/Ink，不能全局删除。
- 图标按钮的可点击尺寸不能缩到 16px；SVG 16px，按钮至少 30–40px。
- hover/active/disabled 仍走父按钮颜色；mask 会自动继承 `currentColor`。
- 分享 PNG 前需验证 mask 是否被 foreignObject 快照保留；若不稳定，改成 inline SVG。

## 6. GSAP 主题转场

现有 `activatePanelPage()` 只隐藏块。建议抽出：

```ts
playSkinTransition(skinId, source, {
  reduced,
  apply: () => {
    setPanelView("settings");
    activatePanelPage("general");
  },
});
```

三条时间线：

| 主题 | 覆盖元素 | 关键点 |
|---|---|---|
| Comic | 22px 像素格 | stagger + `steps()`；从中点/点击点铺开 |
| Signal | 圆形章 | 放大、抖动、换页时“落章” |
| Ink | 6 个不规则 blob | 墨面铺开时换页，再淡出 |

硬约束：

- 动画总长 350–650ms。
- `source` 只在原位置缩放/抖动，不复制 DOM 作为幽灵。
- `apply()` 只执行一次。
- 动画期间 `pointer-events:none`；结束清理 overlay。
- `reduceAnimations || prefers-reduced-motion` 直接 `apply()`。
- 键盘触发与鼠标触发走同一条路径。
- 不能依赖隐藏页的 `offsetWidth`；覆盖层只量可见的点击源和设备矩形。

## 7. 发布前必须重打皮肤包

当前：

```text
repo skin.css      57,407 B
installed/zip CSS  55,096 B
```

源码改了不等于用户能看到。必须：

1. `node site/scripts/build-skin-packs.mjs <version>`
2. `node site/scripts/check-skin-packs.mjs --live`
3. 重新安装/覆盖本机 pack
4. 清除 WebView2 cache 后再验收

不要只改 `site/public/skins` 中的 CSS 而不重打 zip；Rust 安装的是 zip 内文件。

## 8. 最小测试矩阵

```text
comic-pixel / signal / ink / native
× dark / light
× 380px / 450px
```

页面：

- 主看板：黄色 active、紫色 period、金额块、provider 名、bar、低额度。
- 设置：三个 nav 卡片、焦点、ticks、select、input、toggle、保存按钮。
- 密钥额度：provider 行与空/错误态。
- 工坊：列表、选中、下载、已安装、空索引、详情。
- 弹窗：更新完成、确认框、长内容滚动。
- 动效：三主题、打断、重播、减少动画。
- 分享 PNG：字体、mask 图标、硬阴影、背景。

自动化至少：

```powershell
corepack pnpm build
corepack pnpm test
node --test site/site.test.mjs
node site/scripts/check-skin-packs.mjs --live
cd parse-tests
$env:PATH = "D:\Tools\mingw64\bin;$env:PATH"
Remove-Item Env:HTTP_PROXY,Env:HTTPS_PROXY,Env:ALL_PROXY -ErrorAction SilentlyContinue
cargo +stable-x86_64-pc-windows-gnu test -- --test-threads=1
```
