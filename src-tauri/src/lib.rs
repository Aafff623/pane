mod alerts;
mod httpapi;
mod pricing;
mod providers;
mod spend;
mod telemetry;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

// ---------------------------------------------------------------------------
// App settings, stored at %APPDATA%\Pane\config.json
// ---------------------------------------------------------------------------

fn config_path() -> PathBuf {
    providers::config_dir().join("config.json")
}

/// A parse failure here once silently reset all settings to defaults, so
/// failures are now logged durably and the last good copy is used instead.
fn note_config_error(context: &str) {
    let line = format!("{} {}\r\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), context);
    let path = providers::config_dir().join("config-error.log");
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
    eprintln!("[pane] {context}");
}

fn parse_config_file(path: &PathBuf) -> Result<Value, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
    // Tolerate a UTF-8 BOM (Notepad and PowerShell 5.1 both write one).
    serde_json::from_str(raw.trim_start_matches('\u{feff}')).map_err(|e| format!("parse: {e}"))
}

fn load_config() -> Value {
    let path = config_path();
    if !path.exists() {
        return json!({});
    }
    match parse_config_file(&path) {
        Ok(cfg) => cfg,
        Err(e) => {
            note_config_error(&format!("config.json unreadable ({e}) — trying backup"));
            let backup = providers::config_dir().join("config.json.bak");
            match parse_config_file(&backup) {
                Ok(cfg) => cfg,
                Err(e2) => {
                    note_config_error(&format!("config.json.bak also failed ({e2}) — defaults"));
                    json!({})
                }
            }
        }
    }
}

fn config_with_defaults(mut cfg: Value) -> Value {
    if !cfg.is_object() {
        cfg = json!({});
    }
    let obj = cfg.as_object_mut().unwrap();
    // Out-of-the-box experience: 1-min refresh, pacing always visible,
    // all three quota alerts on, dark + compact. (Autostart defaults on
    // in setup; tray icon defaults to Auto via pinned = null.)
    obj.entry("refreshMinutes").or_insert(json!(1));
    obj.entry("disabled").or_insert(json!([]));
    obj.entry("pinned").or_insert(Value::Null);
    obj.entry("trayProviders").or_insert(json!([]));
    obj.entry("pacingAlways").or_insert(json!(true));
    obj.entry("notifyAlmostOut").or_insert(json!(true));
    obj.entry("notifyCuttingClose").or_insert(json!(true));
    obj.entry("notifyWillRunOut").or_insert(json!(true));
    obj.entry("spendTab").or_insert(json!("today"));
    obj.entry("spendMetric").or_insert(json!("cost"));
    obj.entry("showUsed").or_insert(json!(false));
    obj.entry("resetExact").or_insert(json!(false));
    obj.entry("timeFormat").or_insert(json!("auto"));
    obj.entry("layout").or_insert(Value::Null);
    obj.entry("appearance").or_insert(json!("dark"));
    obj.entry("density").or_insert(json!("compact"));
    obj.entry("glassEffects").or_insert(json!(true));
    obj.entry("shortcut").or_insert(json!(""));
    obj.entry("proxy").or_insert(json!({ "enabled": false, "url": "" }));
    obj.entry("showTotalSpend").or_insert(json!(true));
    obj.entry("welcomeDismissed").or_insert(json!(false));
    // Empty = "never recorded": the frontend uses it to tell a fresh
    // install (no What's-new popup) from an update (popup with the notes).
    obj.entry("lastSeenVersion").or_insert(json!(""));
    // Telemetry defaults ON and must SAY so: without this default the
    // Settings toggle read `undefined` (rendered off) while the sender's
    // own default kept transmitting — a switch that displays off while
    // data flows is the one state a privacy control must never be in.
    obj.entry("telemetry").or_insert(json!(true));
    cfg
}

#[tauri::command]
fn get_config() -> Value {
    config_with_defaults(load_config())
}

/// Every key config.json may hold — the same set config_with_defaults seeds.
/// set_config drops anything else so a compromised frontend can't stash
/// arbitrary data in the config file.
const CONFIG_KEYS: &[&str] = &[
    // Not seeded by config_with_defaults (the autostart plugin is the
    // source of truth at runtime) but persisted here so setup() can apply
    // the user's choice on launch.
    "autostart",
    "refreshMinutes",
    "disabled",
    "pinned",
    "trayProviders",
    "pacingAlways",
    "notifyAlmostOut",
    "notifyCuttingClose",
    "notifyWillRunOut",
    "spendMetric",
    "spendTab",
    "showUsed",
    "resetExact",
    "timeFormat",
    "layout",
    "appearance",
    "density",
    "glassEffects",
    "shortcut",
    "proxy",
    "showTotalSpend",
    "welcomeDismissed",
    "lastSeenVersion",
    "telemetry",
];

#[tauri::command]
fn set_config(patch: Value) -> Result<Value, String> {
    let mut cfg = config_with_defaults(load_config());
    if let (Some(target), Some(source)) = (cfg.as_object_mut(), patch.as_object()) {
        for (k, v) in source {
            if CONFIG_KEYS.contains(&k.as_str()) {
                target.insert(k.clone(), v.clone());
            } else {
                eprintln!("[pane] set_config: ignoring unknown key '{k}'");
            }
        }
    }
    let dir = providers::config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create config dir: {e}"))?;
    let path = config_path();
    // Keep the last good copy, then write atomically (temp file + rename) so
    // a crash or kill mid-write can never leave a truncated config behind.
    if path.exists() {
        let _ = std::fs::copy(&path, dir.join("config.json.bak"));
    }
    let tmp = dir.join("config.json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(&cfg).unwrap_or_default())
        .map_err(|e| format!("write config: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("replace config: {e}"))?;
    Ok(cfg)
}

// ---------------------------------------------------------------------------
// Start with Windows
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    // Remember the choice so startup knows whether to re-assert it.
    let _ = set_config(json!({ "autostart": enabled }));
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Tray icon with the pinned metric drawn onto it
// ---------------------------------------------------------------------------

// 4x6 pixel digit font, one nibble per row (bit 3 = leftmost pixel).
const DIGIT_FONT: [[u8; 6]; 10] = [
    [0x6, 0x9, 0x9, 0x9, 0x9, 0x6], // 0
    [0x2, 0x6, 0x2, 0x2, 0x2, 0x7], // 1
    [0x6, 0x9, 0x1, 0x2, 0x4, 0xF], // 2
    [0xE, 0x1, 0x6, 0x1, 0x9, 0x6], // 3
    [0x2, 0x6, 0xA, 0xF, 0x2, 0x2], // 4
    [0xF, 0x8, 0xE, 0x1, 0x9, 0x6], // 5
    [0x6, 0x8, 0xE, 0x9, 0x9, 0x6], // 6
    [0xF, 0x1, 0x2, 0x2, 0x4, 0x4], // 7
    [0x6, 0x9, 0x6, 0x9, 0x9, 0x6], // 8
    [0x6, 0x9, 0x9, 0x7, 0x1, 0x6], // 9
];

/// Renders one or two numbers (0-100) stacked on a 32x32 RGBA tray icon —
/// two rows mimic the Mac menu bar's "100% / 36%" pair. White digits with a
/// black outline so they read on both light and dark taskbars.
fn draw_tray_numbers(values: &[u32]) -> Vec<u8> {
    const SIZE: usize = 32;
    let scale = 2usize;
    let glyph_w = 4 * scale;
    let _glyph_h = 6 * scale;
    let gap = scale;

    let mut mask = [false; SIZE * SIZE];
    let rows: &[usize] = if values.len() >= 2 { &[3, 17] } else { &[10] };

    for (value, y0) in values.iter().zip(rows) {
        let digits: Vec<usize> = value
            .to_string()
            .chars()
            .filter_map(|c| c.to_digit(10).map(|d| d as usize))
            .collect();
        let text_w = digits.len() * glyph_w + digits.len().saturating_sub(1) * gap;
        let x0 = (SIZE.saturating_sub(text_w)) / 2;

        for (i, d) in digits.iter().enumerate() {
            let gx = x0 + i * (glyph_w + gap);
            for (row, bits) in DIGIT_FONT[*d].iter().enumerate() {
                for col in 0..4 {
                    if bits & (0x8 >> col) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let x = gx + col * scale + sx;
                                let y = y0 + row * scale + sy;
                                if x < SIZE && y < SIZE {
                                    mask[y * SIZE + x] = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    // Outline pass: black anywhere adjacent to a text pixel.
    for y in 0..SIZE {
        for x in 0..SIZE {
            if mask[y * SIZE + x] {
                continue;
            }
            let near = (-1i32..=1).any(|dy| {
                (-1i32..=1).any(|dx| {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    nx >= 0
                        && ny >= 0
                        && (nx as usize) < SIZE
                        && (ny as usize) < SIZE
                        && mask[ny as usize * SIZE + nx as usize]
                })
            });
            if near {
                let p = (y * SIZE + x) * 4;
                rgba[p..p + 4].copy_from_slice(&[0, 0, 0, 230]);
            }
        }
    }
    for y in 0..SIZE {
        for x in 0..SIZE {
            if mask[y * SIZE + x] {
                let p = (y * SIZE + x) * 4;
                rgba[p..p + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    rgba
}

/// Picks up to two metrics for the tray icon (like the Mac's stacked pair):
/// the pinned provider's pinned metric first, then its next progress metric.
fn pick_tray_metrics<'a>(
    snapshots: &'a [providers::Snapshot],
    pinned: &Value,
) -> Vec<&'a providers::Metric> {
    let pinned_provider = pinned.get("provider").and_then(Value::as_str);
    let pinned_label = pinned.get("label").and_then(Value::as_str);

    let provider = snapshots
        .iter()
        .find(|s| s.status == "ok" && Some(s.id.as_str()) == pinned_provider)
        .or_else(|| {
            snapshots
                .iter()
                .find(|s| s.status == "ok" && s.metrics.iter().any(|m| m.kind == "progress"))
        });
    let Some(provider) = provider else { return Vec::new() };

    let mut metrics: Vec<&providers::Metric> =
        provider.metrics.iter().filter(|m| m.kind == "progress").collect();
    if let Some(label) = pinned_label {
        if let Some(pos) = metrics.iter().position(|m| m.label == label) {
            metrics.rotate_left(pos);
        }
    }
    metrics.truncate(2);
    metrics
}

fn update_tray(app: &tauri::AppHandle, snapshots: &[providers::Snapshot], cfg: &Value) {
    let Some(tray) = app.tray_by_id("tray") else {
        return;
    };

    let mut tooltip = String::from("Pane");
    for s in snapshots.iter().filter(|s| s.status == "ok").take(6) {
        if let Some(m) = s.metrics.iter().find(|m| m.kind == "progress") {
            let left = (100.0 - m.used_percent.unwrap_or(0.0)).clamp(0.0, 100.0).round();
            tooltip.push_str(&format!("\n{} {}: {left:.0}% left", s.name, m.label));
        }
    }
    let _ = tray.set_tooltip(Some(&tooltip));

    // When the Mac-style tray strip is active it carries the numbers, so the
    // main icon stays the app logo (the strip icons are per-provider).
    let strip_active = cfg
        .get("trayProviders")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty());
    if strip_active {
        if let Some(default) = app.default_window_icon() {
            let _ = tray.set_icon(Some(default.clone()));
        }
        return;
    }

    let metrics = pick_tray_metrics(snapshots, cfg.get("pinned").unwrap_or(&Value::Null));
    if !metrics.is_empty() {
        let lefts: Vec<u32> = metrics
            .iter()
            .map(|m| (100.0 - m.used_percent.unwrap_or(0.0)).clamp(0.0, 100.0).round() as u32)
            .collect();
        let icon = tauri::image::Image::new_owned(draw_tray_numbers(&lefts), 32, 32);
        let _ = tray.set_icon(Some(icon));
    }
}

// ---------------------------------------------------------------------------
// Mac-style tray strip: a [provider logo][live numbers] icon pair per
// selected provider. The UI rasterizes each SVG logo to 32x32 RGBA (the
// webview already has the icons) and sends the pixels here.
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct StripEntry {
    id: String,
    logo: Vec<u8>, // 32x32 RGBA
    values: Vec<u32>,
    tooltip: String,
}

/// Every provider id that may appear in the tray strip. Doubles as the
/// allowlist for update_tray_strip: ids from the frontend are validated
/// against this before being spliced into tray icon ids, and stale strip
/// icons are removed for exactly this set.
const STRIP_PROVIDER_IDS: [&str; 19] = [
    "claude",
    "codex",
    "cursor",
    "opencode",
    "copilot",
    "grok",
    "devin",
    "minimax",
    "openrouter",
    "zai",
    "antigravity",
    "deepseek",
    "moonshot",
    "elevenlabs",
    "ollama",
    "codebuff",
    "kilo",
    "aihubmix",
    "qwen",
];

#[tauri::command]
fn update_tray_strip(app: tauri::AppHandle, entries: Vec<StripEntry>) -> Result<(), String> {
    let handle = app.clone();
    app.run_on_main_thread(move || {
        // Remove strip icons for providers no longer selected.
        for id in STRIP_PROVIDER_IDS {
            if !entries.iter().any(|e| e.id == id) {
                let _ = handle.remove_tray_by_id(&format!("strip-logo-{id}"));
                let _ = handle.remove_tray_by_id(&format!("strip-num-{id}"));
            }
        }

        for entry in &entries {
            // Only known provider ids may reach the tray icon namespace.
            if !STRIP_PROVIDER_IDS.contains(&entry.id.as_str()) {
                continue;
            }
            if entry.logo.len() != 32 * 32 * 4 {
                continue;
            }
            let logo_id = format!("strip-logo-{}", entry.id);
            let num_id = format!("strip-num-{}", entry.id);
            let logo_icon = tauri::image::Image::new_owned(entry.logo.clone(), 32, 32);
            let num_icon =
                tauri::image::Image::new_owned(draw_tray_numbers(&entry.values), 32, 32);

            if let Some(tray) = handle.tray_by_id(&num_id) {
                let _ = tray.set_icon(Some(num_icon));
                let _ = tray.set_tooltip(Some(&entry.tooltip));
                if let Some(logo_tray) = handle.tray_by_id(&logo_id) {
                    let _ = logo_tray.set_tooltip(Some(&entry.tooltip));
                }
                continue;
            }

            // New pair — numbers first: Windows inserts each new tray icon
            // to the LEFT of the previous one, so creating numbers → logo
            // lands as "logo | numbers" left-to-right, like the Mac strip.
            for (tray_id, icon) in [(num_id, num_icon), (logo_id, logo_icon)] {
                let _ = TrayIconBuilder::with_id(tray_id)
                    .icon(icon)
                    .tooltip(&entry.tooltip)
                    .show_menu_on_left_click(false)
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            position,
                            ..
                        } = event
                        {
                            toggle_popover(tray.app_handle(), position);
                        }
                    })
                    .build(&handle);
            }
        }
    })
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Usage fetching
// ---------------------------------------------------------------------------

/// A provider that just failed gets benched briefly instead of being
/// re-probed on every refresh: 60s for ordinary errors, 5 minutes for rate
/// limits (hammering a 429 makes it worse — learned that the hard way).
struct FailState {
    until_ms: i64,
    note: String,
}

fn fail_state() -> &'static std::sync::Mutex<std::collections::HashMap<String, FailState>> {
    static STATE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, FailState>>,
    > = std::sync::OnceLock::new();
    STATE.get_or_init(Default::default)
}

/// The provider family of a card id: "claude@ab12cd34" → "claude". The only
/// spelling allowed to leave the machine in telemetry.
fn family_of(id: &str) -> String {
    id.split('@').next().unwrap_or(id).to_string()
}

// Owned id/name so dynamically discovered account cards (claude@<hash>)
// can ride the same guard as the static providers under a 'static spawn.
async fn guarded<F>(id: String, name: String, fut: F) -> providers::Snapshot
where
    F: std::future::Future<Output = providers::Snapshot>,
{
    let id = id.as_str();
    let name = name.as_str();
    let now = now_ms() as i64;
    let benched = {
        let map = fail_state().lock().unwrap();
        map.get(id).filter(|f| now < f.until_ms).map(|f| f.note.clone())
    };
    if let Some(note) = benched {
        return providers::Snapshot::error(id, name, note);
    }
    let snap = fut.await;
    let mut map = fail_state().lock().unwrap();
    if snap.status == "error" {
        let err = snap.error.clone().unwrap_or_default();
        let rate_limited = err.contains("429");
        // A vendor-stated Retry-After wins over our fixed backoff — bench
        // for exactly that long (capped at an hour) instead of knocking on
        // a door the server said stays shut.
        let retry_after_ms = err
            .split("retry_after_s=")
            .nth(1)
            .and_then(|rest| {
                rest.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<i64>()
                    .ok()
            })
            .map(|s| (s * 1000).min(3_600_000));
        let bench_ms = retry_after_ms.unwrap_or(if rate_limited { 300_000 } else { 60_000 });
        map.insert(
            id.to_string(),
            FailState {
                until_ms: now + bench_ms,
                note: if let Some(ms) = retry_after_ms {
                    format!("rate limited — the vendor asked to wait ~{}m", (ms / 60_000).max(1))
                } else if rate_limited {
                    format!("rate limited — cooling down for a few minutes ({err})")
                } else {
                    err
                },
            },
        );
    } else {
        map.remove(id);
    }
    snap
}

/// Called by the UI. Refreshes every enabled provider at the same time and
/// returns whatever each one found — data, "not signed in", or an error.
#[tauri::command]
async fn fetch_usage(app: tauri::AppHandle) -> Vec<providers::Snapshot> {
    let cfg = config_with_defaults(load_config());
    let disabled: Vec<String> = cfg
        .get("disabled")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();

    // Each provider future is boxed onto the heap and spawned as its own
    // task. A single tokio::join! over 28 inlined futures builds one huge
    // combined state machine on the calling thread's stack — at 28 providers
    // that overflowed the main thread's 1 MB stack and killed the app.
    type BoxedSnap = std::pin::Pin<Box<dyn std::future::Future<Output = providers::Snapshot> + Send>>;
    // Disabled providers are skipped BEFORE anything is spawned — a merely
    // post-filtered provider still did all its work invisibly: network
    // calls, file reads, and in Kiro's case spawning a CLI whose own
    // auto-updater downloaded a fresh installer to %TEMP% on every refresh
    // (gigabytes within days). Futures are lazy, so building and dropping
    // a disabled entry here runs none of its code.
    let base: Vec<(&str, BoxedSnap)> = vec![
        ("claude", Box::pin(guarded("claude".into(), "Claude".into(), providers::claude::snapshot()))),
        ("codex", Box::pin(guarded("codex".into(), "Codex".into(), providers::codex::snapshot()))),
        ("cursor", Box::pin(guarded("cursor".into(), "Cursor".into(), providers::cursor::snapshot()))),
        ("opencode", Box::pin(guarded("opencode".into(), "OpenCode".into(), providers::opencode::snapshot()))),
        ("copilot", Box::pin(guarded("copilot".into(), "Copilot".into(), providers::copilot::snapshot()))),
        ("grok", Box::pin(guarded("grok".into(), "Grok".into(), providers::grok::snapshot()))),
        ("devin", Box::pin(guarded("devin".into(), "Devin".into(), providers::devin::snapshot()))),
        ("minimax", Box::pin(guarded("minimax".into(), "MiniMax".into(), providers::minimax::snapshot()))),
        ("openrouter", Box::pin(guarded("openrouter".into(), "OpenRouter".into(), providers::openrouter::snapshot()))),
        ("zai", Box::pin(guarded("zai".into(), "Z.ai".into(), providers::zai::snapshot()))),
        ("antigravity", Box::pin(guarded("antigravity".into(), "Antigravity".into(), providers::antigravity::snapshot()))),
        ("deepseek", Box::pin(guarded("deepseek".into(), "DeepSeek".into(), providers::deepseek::snapshot()))),
        ("moonshot", Box::pin(guarded("moonshot".into(), "Moonshot".into(), providers::moonshot::snapshot()))),
        ("elevenlabs", Box::pin(guarded("elevenlabs".into(), "ElevenLabs".into(), providers::elevenlabs::snapshot()))),
        ("ollama", Box::pin(guarded("ollama".into(), "Ollama".into(), providers::ollama::snapshot()))),
        ("codebuff", Box::pin(guarded("codebuff".into(), "Codebuff".into(), providers::codebuff::snapshot()))),
        ("kilo", Box::pin(guarded("kilo".into(), "Kilo".into(), providers::kilo::snapshot()))),
        ("aihubmix", Box::pin(guarded("aihubmix".into(), "AihubMix".into(), providers::aihubmix::snapshot()))),
        ("qwen", Box::pin(guarded("qwen".into(), "Qwen Code".into(), providers::qwen::snapshot()))),
    ];
    let mut futs: Vec<(String, BoxedSnap)> =
        base.into_iter().map(|(id, fut)| (id.to_string(), fut)).collect();
    // Extra Claude accounts (multi-login machines): each discovered config
    // dir renders its own card under a claude@<hash8> id, running the same
    // provider flow scoped to its dir. The default login keeps the bare id.
    for acct in providers::claude::discover_extra_accounts() {
        let (id, name, dir) = (acct.id, acct.name, acct.dir);
        futs.push((
            id.clone(),
            Box::pin(guarded(
                id.clone(),
                name.clone(),
                providers::claude::snapshot_at(dir, id, name),
            )),
        ));
    }
    for acct in providers::codex::discover_extra_accounts() {
        let (id, name, dir) = (acct.id, acct.name, acct.dir);
        futs.push((
            id.clone(),
            Box::pin(guarded(
                id.clone(),
                name.clone(),
                providers::codex::snapshot_at(dir, id, name),
            )),
        ));
    }
    let futs: Vec<(String, BoxedSnap)> = futs
        .into_iter()
        .filter(|(id, _)| !disabled.iter().any(|d| d == id))
        .collect();
    // Telemetry never learns account-scoped ids — a claude@<hash8> would
    // carry an account-derived hash off the machine. Report families,
    // deduplicated, so a multi-account install looks like "claude" once.
    // (family_of is applied at EVERY telemetry boundary: enabled ids here,
    // refresh outcomes, and starred-metric prefixes.)
    let enabled_ids: Vec<String> = {
        let mut fams: Vec<String> = Vec::new();
        for (id, _) in &futs {
            let fam = family_of(id);
            if !fams.contains(&fam) {
                fams.push(fam);
            }
        }
        fams
    };
    let handles: Vec<_> = futs
        .into_iter()
        .map(|(_, fut)| tauri::async_runtime::spawn(fut))
        .collect();
    let mut all = Vec::with_capacity(handles.len());
    for h in handles {
        if let Ok(snap) = h.await {
            all.push(snap);
        }
    }

    for s in &all {
        eprintln!(
            "[pane] {}: {} ({} metrics){}",
            s.id,
            s.status,
            s.metrics.len(),
            s.error.as_deref().map(|e| format!(" — {e}")).unwrap_or_default()
        );
    }

    // Transient server errors (a 503, a timeout) shouldn't blank a card the
    // user was just reading: fall back to the last good snapshot, marked
    // stale so the UI can say "Outdated" with the real error on hover. The
    // cache is persisted to disk so it survives app restarts; entries older
    // than a day are too misleading to show and get skipped.
    {
        use std::collections::HashMap;
        use std::sync::{Mutex, OnceLock};

        #[derive(serde::Serialize, serde::Deserialize, Clone)]
        struct CachedSnap {
            at: i64,
            snap: providers::Snapshot,
        }
        const MAX_STALE_MS: i64 = 24 * 60 * 60 * 1000;
        // One failed cycle isn't "Outdated": vendors hiccup routinely, and
        // at short refresh intervals the tag was flashing on every blip.
        // Data younger than the grace window serves silently; the tag (and
        // the error on hover) appears once staleness is real.
        const STALE_GRACE_MS: i64 = 3 * 60 * 1000;

        static LAST_OK: OnceLock<Mutex<HashMap<String, CachedSnap>>> = OnceLock::new();
        let cache_file = providers::config_dir().join("last_snapshots.json");
        let cache = LAST_OK.get_or_init(|| {
            let loaded: HashMap<String, CachedSnap> = std::fs::read_to_string(&cache_file)
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_default();
            Mutex::new(loaded)
        });
        // Cache identity stamp (upstream's Phase 1): if a DIFFERENT account
        // signed into a default home since the cache was written, that
        // family's cached last-good snapshot belongs to the old account —
        // drop it instead of painting the wrong account's numbers under the
        // bare id. Extra-account cards are immune: their ids are derived
        // from the account identity itself.
        {
            let stamp_file = providers::config_dir().join("cache_identities.json");
            let current = json!({
                "claude": providers::claude::default_identity(),
                "codex": providers::codex::default_identity(),
            });
            let stored: Value = std::fs::read_to_string(&stamp_file)
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_else(|| json!({}));
            if stored != current {
                let mut map = cache.lock().unwrap();
                let mut removed = false;
                for fam in ["claude", "codex"] {
                    // Only a KNOWN stored identity differing from a KNOWN
                    // current one is evidence of an account swap. A missing
                    // stamp (first launch after updating) or a momentarily
                    // unreadable identity file must not dump the last-good
                    // cache — that's the safety net, not a swap.
                    let swap = match (stored.get(fam), current.get(fam)) {
                        (Some(s), Some(c)) => !s.is_null() && !c.is_null() && s != c,
                        _ => false,
                    };
                    if swap && map.remove(fam).is_some() {
                        removed = true;
                    }
                }
                // Persist the PRUNED cache before the new stamp: if this
                // refresh finds nothing ok (offline launch) the on-disk
                // cache would otherwise keep the old account's entry while
                // the stamp already claims the new one, resurrecting the
                // wrong numbers next launch. Stamp last, so a failed write
                // just re-prunes next time.
                let _ = std::fs::create_dir_all(providers::config_dir());
                if removed {
                    if let Ok(serialized) = serde_json::to_string(&*map) {
                        let _ = std::fs::write(&cache_file, serialized);
                    }
                }
                drop(map);
                let _ = std::fs::write(
                    &stamp_file,
                    serde_json::to_string_pretty(&current).unwrap_or_default(),
                );
            }
        }
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Ok(mut map) = cache.lock() {
            let mut dirty = false;
            for s in all.iter_mut() {
                if s.status == "ok" {
                    map.insert(s.id.clone(), CachedSnap { at: now_ms, snap: s.clone() });
                    dirty = true;
                } else if s.status == "error" {
                    if let Some(previous) = map.get(&s.id) {
                        let age = now_ms - previous.at;
                        if age <= MAX_STALE_MS {
                            let warning = s.error.clone();
                            *s = previous.snap.clone();
                            if age > STALE_GRACE_MS {
                                s.stale = true;
                                s.warning = warning;
                            }
                        }
                    }
                }
            }
            if dirty {
                if let Ok(serialized) = serde_json::to_string(&*map) {
                    let _ = std::fs::create_dir_all(providers::config_dir());
                    let _ = std::fs::write(&cache_file, serialized);
                }
            }
        }
    }

    httpapi::publish(&all);
    update_tray(&app, &all, &cfg);

    // Anonymous daily-rollup telemetry (Settings → "Share anonymous usage
    // statistics"). Fire-and-forget: it must never delay or fail a refresh.
    {
        let enabled = cfg.get("telemetry").and_then(Value::as_bool).unwrap_or(true);
        let starred_metrics: Vec<String> = cfg
            .pointer("/layout/providers")
            .and_then(Value::as_object)
            .map(|provs| {
                provs
                    .iter()
                    .flat_map(|(pid, entry)| {
                        entry
                            .get("starred")
                            .and_then(Value::as_array)
                            .map(|a| {
                                a.iter()
                                    .filter_map(Value::as_str)
                                    // Family prefix only — an account-scoped
                                    // pid would ship an account-derived hash.
                                    .map(|m| format!("{}/{m}", family_of(pid)))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    })
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        // Two accounts starring the same metric collapse to one entry.
        let starred_metrics: Vec<String> = {
            let mut out: Vec<String> = Vec::new();
            for m in starred_metrics {
                if !out.contains(&m) {
                    out.push(m);
                }
            }
            out
        };
        let snap = telemetry::ConfigSnapshot {
            app_version: app.package_info().version.to_string(),
            enabled_providers: enabled_ids,
            starred_metrics,
            appearance: cfg
                .get("appearance")
                .and_then(Value::as_str)
                .unwrap_or("system")
                .to_string(),
            density: cfg
                .get("density")
                .and_then(Value::as_str)
                .unwrap_or("regular")
                .to_string(),
            refresh_minutes: cfg.get("refreshMinutes").and_then(Value::as_u64).unwrap_or(5),
        };
        let outcomes: Vec<telemetry::Outcome> = all
            .iter()
            .map(|s| telemetry::Outcome {
                // Family only: account-scoped ids never leave the machine.
                // Multiple accounts fold into one family row (accumulate
                // sums same-key counters).
                id: family_of(&s.id),
                status: s.status.clone(),
                stale: s.stale,
                error: s.error.clone().or_else(|| s.warning.clone()),
            })
            .collect();
        tauri::async_runtime::spawn(telemetry::record(enabled, snap, outcomes));
    }

    for alert in alerts::evaluate(&all, &cfg) {
        use tauri_plugin_notification::NotificationExt;
        let _ = app
            .notification()
            .builder()
            .title(&alert.title)
            .body(&alert.body)
            .show();
    }

    all
}

/// Computes local spend (Today / Yesterday / Last 30 Days) from the CLIs'
/// own session logs. Heavy file IO, so it runs on a blocking thread.
#[tauri::command]
async fn fetch_spend() -> Vec<spend::ProviderSpend> {
    eprintln!("[pane] spend: scan starting");
    let started = std::time::Instant::now();
    // Cursor's CSV export needs the async client; fetch it here and hand it
    // to the blocking scan.
    let cursor_csv = providers::cursor::fetch_usage_csv().await;
    let result = tauri::async_runtime::spawn_blocking(move || spend::collect(cursor_csv))
        .await
        .unwrap_or_default();
    eprintln!(
        "[pane] spend: {} providers in {:?}",
        result.len(),
        started.elapsed()
    );
    result
}

/// Saves (or clears, when `key` is empty) a user-pasted API key to
/// %APPDATA%\Pane\<provider>.json.
#[tauri::command]
fn set_api_key(provider: String, key: String) -> Result<(), String> {
    if !matches!(
        provider.as_str(),
        "openrouter"
            | "zai"
            | "minimax"
            | "deepseek"
            | "moonshot"
            | "elevenlabs"
            | "codebuff"
            | "kilo"
            | "aihubmix"
            | "qwen"
    ) {
        return Err(format!("unknown provider: {provider}"));
    }
    let dir = providers::config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("create config dir: {e}"))?;
    let path = dir.join(format!("{provider}.json"));
    let key = key.trim();
    if key.is_empty() {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    std::fs::write(&path, serde_json::json!({ "apiKey": key }).to_string())
        .map_err(|e| format!("write key file: {e}"))
}

/// Opens a provider quick link in the default browser. Only plain web URLs —
/// nothing that could launch a program.
#[tauri::command]
fn open_link(app: tauri::AppHandle, url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("only http(s) links allowed".into());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("open link: {e}"))
}

/// Puts a share-card PNG (rendered by the frontend on a canvas) onto the
/// Windows clipboard as a real image.
#[tauri::command]
fn copy_share_image(png_base64: String) -> Result<(), String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64.trim())
        .map_err(|e| format!("decode png: {e}"))?;
    let img = tauri::image::Image::from_bytes(&bytes).map_err(|e| format!("parse png: {e}"))?;
    let (w, h) = (img.width() as usize, img.height() as usize);
    let rgba = img.rgba().to_vec();
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("clipboard: {e}"))?;
    clipboard
        .set_image(arboard::ImageData { width: w, height: h, bytes: rgba.into() })
        .map_err(|e| format!("copy image: {e}"))
}

/// (Re-)registers the global toggle-popover shortcut. An empty string clears
/// it. The accelerator uses Tauri syntax, e.g. "Ctrl+Shift+U".
fn register_shortcut(app: &tauri::AppHandle, accel: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let accel = accel.trim();
    if accel.is_empty() {
        return Ok(());
    }
    let shortcut: Shortcut = accel
        .parse()
        .map_err(|_| format!("could not parse shortcut \"{accel}\""))?;
    gs.on_shortcut(shortcut, |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let pos = app
                .cursor_position()
                .unwrap_or(tauri::PhysicalPosition::new(1200.0, 700.0));
            toggle_popover(app, pos);
        }
    })
    .map_err(|e| format!("register shortcut: {e}"))
}

#[tauri::command]
fn set_shortcut(app: tauri::AppHandle, shortcut: String) -> Result<(), String> {
    register_shortcut(&app, &shortcut)
}

/// Spends one banked Codex rate-limit reset credit. Irreversible — the
/// frontend shows a confirm dialog before calling this.
#[tauri::command]
async fn codex_redeem_credit(
    credit_id: String,
    provider_id: Option<String>,
) -> Result<String, String> {
    // provider_id routes multi-account redeems; absent = the default card
    // (older frontend builds during an update overlap).
    let pid = provider_id.unwrap_or_else(|| "codex".into());
    providers::codex::redeem_credit(&pid, &credit_id).await
}

/// Updater with the app version stamped into the endpoint by us. Tauri's
/// `{{current_version}}` template arrives percent-encoded and never gets
/// substituted in query strings, so 0.4.17 installs literally reported
/// "?v={{current_version}}" — the version is now formatted in Rust.
/// GitHub stays as the automatic fallback; the pubkey comes from config.
fn build_updater(
    app: &tauri::AppHandle,
) -> Result<tauri_plugin_updater::Updater, String> {
    use tauri_plugin_updater::UpdaterExt;
    let version = app.package_info().version.to_string();
    let endpoints = vec![
        format!("https://pane.jazii.dev/api/update?v={version}")
            .parse()
            .map_err(|e| format!("endpoint parse: {e}"))?,
        "https://github.com/ItsJazii/pane/releases/latest/download/latest.json"
            .parse()
            .map_err(|e| format!("endpoint parse: {e}"))?,
    ];
    app.updater_builder()
        .endpoints(endpoints)
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())
}

/// Downloads and installs a pending update, then restarts the app. Only
/// called from the frontend banner after check_for_update announced one.
#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let updater = build_updater(&app)?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => {
            update
                .download_and_install(|_, _| {}, || {})
                .await
                .map_err(|e| e.to_string())?;
            app.restart();
        }
        // The update the button promised is gone (yanked release, CDN
        // hiccup). Succeeding silently would strand the frontend in its
        // "Installing…" state — fail so the button can recover.
        None => return Err("update no longer available — try again shortly".into()),
    }
    Ok(())
}

/// Popover-open update check: the footer asks on every tray click and
/// shows an Update button when this returns a newer version.
#[tauri::command]
async fn check_update(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let updater = build_updater(&app)?;
    updater
        .check()
        .await
        .map(|u| u.map(|u| u.version.clone()))
        .map_err(|e| e.to_string())
}

/// Startup + every 4 h: quiet update check; a hit emits "update-available"
/// with the new version so the frontend can show its banner. 404 (no
/// releases yet) and offline are non-events.
fn spawn_update_checker(app: &tauri::AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            if let Ok(updater) = build_updater(&handle) {
                match updater.check().await {
                    Ok(Some(update)) => {
                        let _ = handle.emit("update-available", update.version.clone());
                    }
                    Ok(None) => {}
                    Err(e) => eprintln!("[pane] update check: {e}"),
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(4 * 3600)).await;
        }
    });
}

// ---------------------------------------------------------------------------
// Tray + popover window plumbing
// ---------------------------------------------------------------------------

// Clicking the tray icon while the popover is open first steals focus
// (which hides the window) and then delivers the click event. Without a
// guard, that click would instantly re-open the window the user just
// closed. We remember when the last auto-hide happened and ignore tray
// clicks that arrive right after it.
static LAST_AUTO_HIDE_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Tells WebView2 to release memory while the popover is hidden and return
/// to normal when it shows. Tauri doesn't expose wry's setter for this, so
/// we make the same COM calls wry does (SetMemoryUsageTargetLevel).
fn set_webview_memory_level(window: &tauri::WebviewWindow, low: bool) {
    let _ = window.with_webview(move |webview| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL,
        };
        use windows_core::Interface;
        if let Ok(core) = webview.controller().CoreWebView2() {
            if let Ok(wv19) = core.cast::<ICoreWebView2_19>() {
                let level = COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL(if low { 1 } else { 0 });
                let _ = wv19.SetMemoryUsageTargetLevel(level);
            }
        }
    });
}

fn toggle_popover(app: &tauri::AppHandle, click: tauri::PhysicalPosition<f64>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        set_webview_memory_level(&window, true);
        return;
    }

    if now_ms().saturating_sub(LAST_AUTO_HIDE_MS.load(Ordering::Relaxed)) < 300 {
        return;
    }

    set_webview_memory_level(&window, false);

    // Anchor the popover's bottom-right corner near the tray click,
    // which sits next to the clock on a standard bottom taskbar.
    let size = window
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(380, 600));
    let x = (click.x - f64::from(size.width)).max(0.0);
    let y = (click.y - f64::from(size.height) - 8.0).max(0.0);
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("popover-shown", ());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Second launches just poke the existing instance's popover open
        // instead of spawning a duplicate tray icon (Mac parity).
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let pos = app
                .cursor_position()
                .unwrap_or(tauri::PhysicalPosition::new(1200.0, 700.0));
            toggle_popover(app, pos);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            fetch_usage,
            fetch_spend,
            set_api_key,
            get_config,
            set_config,
            get_autostart,
            set_autostart,
            update_tray_strip,
            open_link,
            copy_share_image,
            set_shortcut,
            codex_redeem_credit,
            install_update,
            check_update
        ])
        .setup(|app| {
            spawn_update_checker(app.handle());
            let quit = MenuItem::with_id(app, "quit", "Quit Pane", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Pane")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        toggle_popover(tray.app_handle(), position);
                    }
                })
                .build(app)?;

            // The popover starts hidden, so start the webview in low-memory
            // mode too; it flips to normal the first time it is shown.
            if let Some(wv) = app.get_webview_window("main") {
                set_webview_memory_level(&wv, true);
            }

            httpapi::start();

            let saved_shortcut = load_config()
                .get("shortcut")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if let Err(e) = register_shortcut(app.handle(), &saved_shortcut) {
                eprintln!("[pane] shortcut: {e}");
            }

            // Start with Windows is on by default (like the Mac app's
            // launch-at-login) and re-asserted each launch so the registry
            // entry follows the exe if it moves — e.g. loose exe → installed.
            // Only an explicit "off" in Settings is respected. Skipped in dev
            // builds so the debug exe never registers itself.
            if !cfg!(debug_assertions) {
                let wants_autostart = load_config()
                    .get("autostart")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                if wants_autostart {
                    use tauri_plugin_autostart::ManagerExt;
                    let _ = app.autolaunch().enable();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::Focused(false) = event {
                    if window.hide().is_ok() {
                        LAST_AUTO_HIDE_MS.store(now_ms(), Ordering::Relaxed);
                        if let Some(wv) = window.app_handle().get_webview_window("main") {
                            set_webview_memory_level(&wv, true);
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
