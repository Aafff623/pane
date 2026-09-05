mod accounts;
mod antigravity_accounts;
mod cursor_accounts;
mod cursor_oauth;
mod alerts;
mod httpapi;
mod i18n;
mod oauth;
mod pricing;
mod providers;
pub(crate) mod provider_catalog;
mod spend;
mod telemetry;
mod tray_projection;
mod usage_history;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use provider_catalog::family_of;

/// Last-good snapshots older than this are too misleading to show or to
/// use as a stand-in for a live Moonshot card.
const SNAPSHOT_CACHE_MS: i64 = 24 * 60 * 60 * 1000;
/// One failed cycle isn't "Outdated": vendors hiccup routinely.
const STALE_GRACE_MS: i64 = 3 * 60 * 1000;

// ---------------------------------------------------------------------------
// App settings, stored at %APPDATA%\Pane\config.json
// ---------------------------------------------------------------------------

fn config_path_in(dir: &Path) -> PathBuf {
    dir.join("config.json")
}

/// A parse failure here once silently reset all settings to defaults, so
/// failures are now logged durably and the last good copy is used instead.
fn note_config_error(context: &str) {
    let line = format!(
        "{} {}\r\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        context
    );
    let path = providers::config_dir().join("config-error.log");
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
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
    load_config_from(&providers::config_dir())
}

fn load_config_from(dir: &Path) -> Value {
    let path = config_path_in(dir);
    if !path.exists() {
        return json!({});
    }
    match parse_config_file(&path) {
        Ok(cfg) => cfg,
        Err(e) => {
            note_config_error(&format!("config.json unreadable ({e}) — trying backup"));
            let backup = dir.join("config.json.bak");
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
    // Out-of-the-box experience: 1-min refresh, all three quota alerts on,
    // dark + compact. (Autostart defaults on in setup; tray icon defaults
    // to Auto via pinned = null.)
    obj.entry("refreshMinutes").or_insert(json!(1));
    obj.entry("disabled").or_insert(json!([]));
    obj.entry("pinned").or_insert(Value::Null);
    obj.entry("trayProviders").or_insert(json!([]));
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
    obj.entry("shortcut").or_insert(json!("Alt+2"));
    obj.entry("proxy")
        .or_insert(json!({ "enabled": false, "url": "" }));
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
    obj.entry("reduceAnimations").or_insert(json!(false));
    obj.entry("hideUsageWhileSharing").or_insert(json!(false));
    obj.entry("showTrend").or_insert(json!(false));
    obj.entry("locale").or_insert(json!("auto"));
    cfg
}

#[tauri::command]
fn system_ui_locale() -> &'static str {
    i18n::system_ui_locale()
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
    "reduceAnimations",
    "hideUsageWhileSharing",
    "showTrend",
    "locale",
];

static CONFIG_WRITE: Mutex<()> = Mutex::new(());
static CONFIG_TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn apply_config_patch(cfg: &mut Value, patch: &Value) {
    if let (Some(target), Some(source)) = (cfg.as_object_mut(), patch.as_object()) {
        for (k, v) in source {
            if CONFIG_KEYS.contains(&k.as_str()) {
                if k == "locale" {
                    let ok = matches!(v.as_str(), Some("auto" | "en" | "zh" | "ru"));
                    target.insert(k.clone(), if ok { v.clone() } else { json!("auto") });
                } else {
                    target.insert(k.clone(), v.clone());
                }
            } else {
                eprintln!("[pane] set_config: ignoring unknown key '{k}'");
            }
        }
    }
}

fn persist_config_in(dir: &Path, cfg: &Value) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("create config dir: {e}"))?;
    let path = config_path_in(dir);
    // Keep the last good copy, then write atomically (temp file + rename) so
    // a crash or kill mid-write can never leave a truncated config behind.
    if path.exists() {
        let _ = std::fs::copy(&path, dir.join("config.json.bak"));
    }
    let tmp = dir.join(format!(
        "config.{}.{}.tmp",
        std::process::id(),
        CONFIG_TMP_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let raw = serde_json::to_string_pretty(cfg).unwrap_or_default();
    if let Err(e) = std::fs::write(&tmp, raw) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("write config: {e}"));
    }
    if let Err(e) = std::fs::rename(&tmp, &path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("replace config: {e}"));
    }
    Ok(())
}

fn set_config_in(dir: &Path, patch: Value) -> Result<Value, String> {
    let _guard = CONFIG_WRITE.lock().unwrap_or_else(|e| e.into_inner());
    let mut cfg = config_with_defaults(load_config_from(dir));
    apply_config_patch(&mut cfg, &patch);
    persist_config_in(dir, &cfg)?;
    Ok(cfg)
}

fn set_config_inner(patch: Value) -> Result<Value, String> {
    let cfg = set_config_in(&providers::config_dir(), patch)?;
    HIDE_WANT.store(hide_usage_flag(&cfg), Ordering::Relaxed);
    Ok(cfg)
}

#[tauri::command]
fn set_config(app: tauri::AppHandle, patch: Value) -> Result<Value, String> {
    let cfg = set_config_inner(patch)?;
    apply_tray_locale(&app, &cfg);
    Ok(cfg)
}

fn apply_tray_locale(app: &tauri::AppHandle, cfg: &Value) {
    let next = i18n::resolved_locale(cfg);
    static LAST: Mutex<Option<&'static str>> = Mutex::new(None);
    let Ok(mut last) = LAST.lock() else {
        return;
    };
    if *last == Some(next) {
        return;
    }
    *last = Some(next);
    drop(last);
    let Ok(quit) = MenuItem::with_id(app, "quit", i18n::quit_label(cfg), true, None::<&str>) else {
        return;
    };
    let Ok(menu) = Menu::with_items(app, &[&quit]) else {
        return;
    };
    if let Some(tray) = app.tray_by_id("tray") {
        let _ = tray.set_menu(Some(menu));
    }
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
    let _ = set_config_inner(json!({ "autostart": enabled }));
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

fn apply_main_tray_projection(
    app: &tauri::AppHandle,
    projection: &tray_projection::MainTrayProjection,
) -> Result<(), String> {
    let tray = app
        .tray_by_id("tray")
        .ok_or_else(|| "main tray icon is unavailable".to_string())?;
    if HIDE_STRIP.load(Ordering::Relaxed) {
        let default = app
            .default_window_icon()
            .ok_or_else(|| "default Pane icon is unavailable".to_string())?;
        tray.set_icon(Some(default.clone()))
            .map_err(|error| format!("set hidden main tray icon: {error}"))?;
        tray.set_tooltip(Some("Pane"))
            .map_err(|error| format!("set hidden main tray tooltip: {error}"))?;
    } else {
        if let Err(_) = tray.set_tooltip(Some(&projection.tooltip)) {
            let _ = tray.set_tooltip(Some("Pane"));
        }
        match projection.icon_mode {
            tray_projection::MainTrayIconMode::Logo => {
                let default = app
                    .default_window_icon()
                    .ok_or_else(|| "default Pane icon is unavailable".to_string())?;
                tray.set_icon(Some(default.clone()))
                    .map_err(|error| format!("set main tray logo: {error}"))?;
            }
            tray_projection::MainTrayIconMode::Numbers => {
                let icon = tauri::image::Image::new_owned(
                    draw_tray_numbers(&projection.remaining_percentages),
                    32,
                    32,
                );
                tray.set_icon(Some(icon))
                    .map_err(|error| format!("set main tray numbers: {error}"))?;
            }
        }
    }
    if let Ok(mut slot) = last_main_tray().lock() {
        slot.lefts = projection.remaining_percentages.clone();
        slot.tooltip = projection.tooltip.clone();
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Mac-style tray strip: a [provider logo][live numbers] icon pair per
// selected provider. The UI rasterizes each SVG logo to 32x32 RGBA (the
// webview already has the icons) and sends the pixels here.
// ---------------------------------------------------------------------------

/// Hide starred tray numbers while a screen share / presentation is on
/// (Settings → Privacy, off by default — Mac parity with OpenUsage #1013).
static HIDE_WANT: AtomicBool = AtomicBool::new(false);
static HIDE_STRIP: AtomicBool = AtomicBool::new(false);

struct LastMainTray {
    lefts: Vec<u32>,
    tooltip: String,
}

fn last_main_tray() -> &'static Mutex<LastMainTray> {
    static S: OnceLock<Mutex<LastMainTray>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(LastMainTray {
            lefts: Vec::new(),
            tooltip: String::from("Pane"),
        })
    })
}

fn last_strip() -> &'static Mutex<Vec<StripEntry>> {
    static S: OnceLock<Mutex<Vec<StripEntry>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(Vec::new()))
}

fn tray_strip_apply_lock() -> &'static tauri::async_runtime::Mutex<()> {
    static LOCK: OnceLock<tauri::async_runtime::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tauri::async_runtime::Mutex::new(()))
}

fn hide_usage_flag(cfg: &Value) -> bool {
    cfg.get("hideUsageWhileSharing").and_then(Value::as_bool) == Some(true)
}

fn set_main_tray_logo(app: &tauri::AppHandle) {
    let Some(tray) = app.tray_by_id("tray") else {
        return;
    };
    if let Some(default) = app.default_window_icon() {
        let _ = tray.set_icon(Some(default.clone()));
    }
    let _ = tray.set_tooltip(Some("Pane"));
}

fn paint_cached_main_tray(app: &tauri::AppHandle) {
    if HIDE_STRIP.load(Ordering::Relaxed) {
        set_main_tray_logo(app);
        return;
    }
    let cached = last_main_tray()
        .lock()
        .map(|g| (g.lefts.clone(), g.tooltip.clone()))
        .unwrap_or_else(|_| (Vec::new(), String::from("Pane")));
    let Some(tray) = app.tray_by_id("tray") else {
        return;
    };
    let _ = tray.set_tooltip(Some(&cached.1));
    if cached.0.is_empty() {
        if let Some(default) = app.default_window_icon() {
            let _ = tray.set_icon(Some(default.clone()));
        }
        return;
    }
    let icon = tauri::image::Image::new_owned(draw_tray_numbers(&cached.0), 32, 32);
    let _ = tray.set_icon(Some(icon));
}

fn screen_is_being_shared() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Shell::{
            SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
        };
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_REMOTECONTROL};

        // Someone is remotely controlling this session (Quick Assist, etc.).
        if unsafe { GetSystemMetrics(SM_REMOTECONTROL) } != 0 {
            return true;
        }
        if let Ok(state) = unsafe { SHQueryUserNotificationState() } {
            // Presentation Settings / exclusive fullscreen — the closest
            // public Windows equivalent of macOS's screen-watcher flag.
            // QUNS_BUSY is skipped: a fullscreen YouTube tab would hide
            // numbers all evening.
            if state == QUNS_PRESENTATION_MODE || state == QUNS_RUNNING_D3D_FULL_SCREEN {
                return true;
            }
        }
        false
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn spawn_share_watcher(app: tauri::AppHandle) {
    HIDE_WANT.store(
        hide_usage_flag(&config_with_defaults(load_config())),
        Ordering::Relaxed,
    );
    tauri::async_runtime::spawn(async move {
        let mut was_hidden = false;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let hide = HIDE_WANT.load(Ordering::Relaxed) && screen_is_being_shared();
            HIDE_STRIP.store(hide, Ordering::Relaxed);
            if hide == was_hidden {
                continue;
            }
            was_hidden = hide;
            let _guard = tray_strip_apply_lock().lock().await;
            let cached = last_strip().lock().map(|g| g.clone()).unwrap_or_default();
            if let Err(error) = apply_tray_strip(app.clone(), cached, hide, Vec::new(), false).await
            {
                let action = if hide { "hide" } else { "restore" };
                eprintln!("[pane] {action} tray strip: {error}");
            }
            if hide {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || set_main_tray_logo(&handle));
            } else {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || paint_cached_main_tray(&handle));
                let _ = app.emit("tray-strip-restore", ());
            }
        }
    });
}

#[derive(Clone, serde::Deserialize)]
struct StripEntry {
    id: String,
    logo: Vec<u8>, // 32x32 RGBA
    values: Vec<u32>,
    tooltip: String,
}

/// Every provider family that may appear in the tray strip. Frontend
/// strip ids are validated against this before becoming tray icon ids,
/// including `family@account` cards. Stale family-level strip icons are
/// removed for exactly this set.
const STRIP_PROVIDER_IDS: [&str; 26] = [
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
    "hermes",
    "kimi",
    "onenewapi",
    "stepfun",
    "siliconflow",
    "novita",
    "relaybalance",
];

async fn update_tray_strip(app: tauri::AppHandle, entries: Vec<StripEntry>) -> Result<(), String> {
    validate_strip_entries(&entries)?;
    let _guard = tray_strip_apply_lock().lock().await;
    let previous = last_strip()
        .lock()
        .map(|slot| slot.clone())
        .unwrap_or_default();
    let reset_ids = strip_reset_ids(&previous, &entries);
    let rebuild_order = !reset_ids.is_empty();
    let result = apply_tray_strip(
        app.clone(),
        entries.clone(),
        HIDE_STRIP.load(Ordering::Relaxed),
        reset_ids,
        rebuild_order,
    )
    .await;
    if result.is_err() {
        if clear_tray_strip_icons(app, &previous, &entries)
            .await
            .is_ok()
        {
            if let Ok(mut slot) = last_strip().lock() {
                slot.clear();
            }
        }
        return result;
    }
    let Ok(mut slot) = last_strip().lock() else {
        return result;
    };
    commit_strip_state_after_apply(&mut slot, &entries, result)
}

fn commit_strip_state_after_apply(
    current: &mut Vec<StripEntry>,
    next: &[StripEntry],
    result: Result<(), String>,
) -> Result<(), String> {
    result?;
    *current = next.to_vec();
    Ok(())
}

fn strip_is_active(strip_ok: bool, entries: &[StripEntry]) -> bool {
    strip_ok && !entries.is_empty()
}

fn strip_icon_ids_to_clear(known: &[StripEntry], attempted: &[StripEntry]) -> Vec<String> {
    let mut ids: Vec<String> = STRIP_PROVIDER_IDS
        .iter()
        .map(|id| (*id).to_string())
        .collect();
    for entry in known.iter().chain(attempted) {
        if !ids.iter().any(|seen| seen == &entry.id) {
            ids.push(entry.id.clone());
        }
    }
    ids
}

#[tauri::command]
async fn sync_tray_surfaces(
    app: tauri::AppHandle,
    snapshots: Vec<providers::Snapshot>,
    projection: tray_projection::TrayProjectionConfig,
    entries: Vec<StripEntry>,
) -> Result<(), String> {
    let strip_result = update_tray_strip(app.clone(), entries.clone()).await;
    let main = tray_projection::project_main_tray(
        &snapshots,
        &projection,
        strip_is_active(strip_result.is_ok(), &entries),
    );
    let main_result = apply_main_tray_projection(&app, &main);
    match (main_result, strip_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(main_error), Err(strip_error)) => Err(format!("{main_error}; {strip_error}")),
    }
}

fn validate_strip_entries(entries: &[StripEntry]) -> Result<(), String> {
    if entries.len() > 4 {
        return Err("tray strip accepts at most 4 providers".into());
    }
    for (index, entry) in entries.iter().enumerate() {
        if !strip_provider_id_is_allowed(&entry.id) {
            return Err(format!("invalid tray strip provider id: {}", entry.id));
        }
        if entries[..index].iter().any(|seen| seen.id == entry.id) {
            return Err(format!("duplicate tray strip provider id: {}", entry.id));
        }
        if entry.logo.len() != 32 * 32 * 4 {
            return Err(format!("invalid tray strip logo for {}", entry.id));
        }
        if entry.values.is_empty() || entry.values.len() > 2 {
            return Err(format!("invalid tray strip values for {}", entry.id));
        }
    }
    Ok(())
}

fn strip_provider_id_is_allowed(id: &str) -> bool {
    match id.split_once('@') {
        None => STRIP_PROVIDER_IDS.contains(&id),
        Some((family, account)) => {
            STRIP_PROVIDER_IDS.contains(&family)
                && !account.is_empty()
                && account
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        }
    }
}

fn strip_tray_key(id: &str) -> String {
    id.replace('@', "--")
}

fn strip_reset_ids(previous: &[StripEntry], next: &[StripEntry]) -> Vec<String> {
    let same_order = previous.len() == next.len()
        && previous.iter().zip(next).all(|(old, new)| old.id == new.id);
    if same_order {
        return Vec::new();
    }

    let mut ids = Vec::new();
    for entry in previous.iter().chain(next) {
        if !ids.contains(&entry.id) {
            ids.push(entry.id.clone());
        }
    }
    ids
}

fn strip_entry_application_order(entries: &[StripEntry], rebuild_order: bool) -> Vec<&StripEntry> {
    let mut ordered: Vec<&StripEntry> = entries.iter().collect();
    if rebuild_order {
        // Windows inserts each new tray icon to the left. Rebuild Provider
        // pairs from right to left so their visible order matches providerOrder.
        ordered.reverse();
    }
    ordered
}

async fn clear_tray_strip_icons(
    app: tauri::AppHandle,
    known: &[StripEntry],
    attempted: &[StripEntry],
) -> Result<(), String> {
    let ids = strip_icon_ids_to_clear(known, attempted);
    let handle = app.clone();
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    app.run_on_main_thread(move || {
        for id in &ids {
            let key = strip_tray_key(id);
            handle.remove_tray_by_id(&format!("strip-logo-{key}"));
            handle.remove_tray_by_id(&format!("strip-num-{key}"));
        }
        let _ = sender.blocking_send(());
    })
    .map_err(|error| error.to_string())?;
    receiver
        .recv()
        .await
        .ok_or_else(|| "tray strip clear ended before reporting a result".to_string())
}

async fn apply_tray_strip(
    app: tauri::AppHandle,
    entries: Vec<StripEntry>,
    hide_numbers: bool,
    reset_ids: Vec<String>,
    rebuild_order: bool,
) -> Result<(), String> {
    let handle = app.clone();
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    app.run_on_main_thread(move || {
        let result = (|| -> Result<(), String> {
            // Removal returns None when an icon is already absent; that is
            // the desired end state rather than an update failure.
            for id in STRIP_PROVIDER_IDS {
                if !entries.iter().any(|entry| entry.id == id) {
                    handle.remove_tray_by_id(&format!("strip-logo-{id}"));
                    handle.remove_tray_by_id(&format!("strip-num-{id}"));
                }
            }
            for id in &reset_ids {
                let key = strip_tray_key(id);
                handle.remove_tray_by_id(&format!("strip-logo-{key}"));
                handle.remove_tray_by_id(&format!("strip-num-{key}"));
            }

            for entry in strip_entry_application_order(&entries, rebuild_order) {
                let tray_key = strip_tray_key(&entry.id);
                let logo_id = format!("strip-logo-{tray_key}");
                let num_id = format!("strip-num-{tray_key}");
                let logo_icon = tauri::image::Image::new_owned(entry.logo.clone(), 32, 32);
                let num_icon = tauri::image::Image::new_owned(
                    if hide_numbers {
                        vec![0u8; 32 * 32 * 4]
                    } else {
                        draw_tray_numbers(&entry.values)
                    },
                    32,
                    32,
                );
                let tooltip = if hide_numbers {
                    entry
                        .tooltip
                        .split('\n')
                        .next()
                        .unwrap_or("Pane")
                        .to_string()
                } else {
                    entry.tooltip.clone()
                };

                let new_trays = if let Some(tray) = handle.tray_by_id(&num_id) {
                    tray.set_icon(Some(num_icon))
                        .map_err(|error| format!("set {} strip numbers: {error}", entry.id))?;
                    tray.set_tooltip(Some(&tooltip))
                        .map_err(|error| format!("set {} strip tooltip: {error}", entry.id))?;
                    if let Some(logo_tray) = handle.tray_by_id(&logo_id) {
                        logo_tray.set_tooltip(Some(&tooltip)).map_err(|error| {
                            format!("set {} strip logo tooltip: {error}", entry.id)
                        })?;
                        Vec::new()
                    } else {
                        vec![(logo_id, logo_icon)]
                    }
                } else {
                    vec![(num_id, num_icon), (logo_id, logo_icon)]
                };

                // New pairs are numbers first: Windows inserts each new tray
                // icon to the left, yielding "logo | numbers" on screen.
                for (tray_id, icon) in new_trays {
                    TrayIconBuilder::with_id(tray_id)
                        .icon(icon)
                        .tooltip(&tooltip)
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
                        .build(&handle)
                        .map_err(|error| format!("build {} strip icon: {error}", entry.id))?;
                }
            }
            Ok(())
        })();
        let _ = sender.blocking_send(result);
    })
    .map_err(|error| error.to_string())?;
    receiver
        .recv()
        .await
        .ok_or_else(|| "tray strip update ended before reporting a result".to_string())?
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

fn fail_state() -> &'static Mutex<HashMap<String, FailState>> {
    static STATE: OnceLock<Mutex<HashMap<String, FailState>>> = OnceLock::new();
    STATE.get_or_init(Default::default)
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct CachedSnap {
    at: i64,
    snap: providers::Snapshot,
}

fn last_ok() -> &'static Mutex<HashMap<String, CachedSnap>> {
    static LAST_OK: OnceLock<Mutex<HashMap<String, CachedSnap>>> = OnceLock::new();
    LAST_OK.get_or_init(|| {
        let cache_file = providers::config_dir().join("last_snapshots.json");
        let loaded = std::fs::read_to_string(&cache_file)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        Mutex::new(loaded)
    })
}

/// The plan (subscription tier) of the last good snapshot for a family —
/// Codex reports team, Copilot reports Pro (Student), Antigravity reports
/// Google AI Pro, Grok reports X Premium/SuperGrok. Reads the in-memory
/// cache when warm, else the persisted last_snapshots.json.
fn cached_plan_for(family: &str) -> Option<String> {
    if let Ok(map) = last_ok().lock() {
        if let Some(entry) = map.get(family) {
            if entry.snap.status == "ok" {
                if let Some(plan) = &entry.snap.plan {
                    return Some(plan.clone());
                }
            }
        }
    }
    let path = providers::config_dir().join("last_snapshots.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let doc: serde_json::Value = serde_json::from_str(&raw).ok()?;
    doc.get(family)?
        .get("snap")?
        .get("plan")?
        .as_str()
        .map(str::to_string)
}

fn persist_last_ok_at(
    path: &std::path::Path,
    map: &HashMap<String, CachedSnap>,
) -> Result<(), String> {
    let serialized =
        serde_json::to_string(map).map_err(|e| format!("serialize snapshot cache: {e}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "snapshot cache path has no parent".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| format!("create snapshot cache dir: {e}"))?;
    std::fs::write(path, serialized).map_err(|e| format!("write snapshot cache: {e}"))
}

fn persist_last_ok(map: &HashMap<String, CachedSnap>) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    let cache_file = providers::config_dir().join("last_snapshots.json");
    persist_last_ok_at(&cache_file, map)
}

fn forget_provider_snapshots(ids: &[String]) -> Result<(), String> {
    let mut map = last_ok().lock().unwrap();
    let mut next = map.clone();
    let mut changed = false;
    for id in ids {
        changed |= next.remove(id).is_some();
    }
    if changed {
        persist_last_ok(&next)?;
        *map = next;
    }
    drop(map);
    let mut failures = fail_state().lock().unwrap();
    for id in ids {
        failures.remove(id);
        alerts::forget_snapshot(id);
    }
    Ok(())
}

fn forget_provider_snapshot(id: &str) -> Result<(), String> {
    forget_provider_snapshots(&[id.to_string()])
}

fn forget_onenewapi_key_ids(key_ids: impl IntoIterator<Item = String>) -> Result<(), String> {
    let snapshot_ids: Vec<String> = key_ids
        .into_iter()
        .map(|key_id| format!("onenewapi@{key_id}"))
        .collect();
    forget_provider_snapshots(&snapshot_ids)
}

fn onenewapi_snapshot_ids(key_ids: &[String]) -> Vec<String> {
    key_ids.iter().map(|id| format!("onenewapi@{id}")).collect()
}

fn cached_onenewapi_id_is_configured(id: &str, configured: &HashSet<String>) -> bool {
    family_of(id) != "onenewapi" || configured.contains(id)
}

fn is_extra_account_card_id(id: &str) -> bool {
    id.split_once('@')
        .is_some_and(|(family, fingerprint)| {
            !fingerprint.is_empty() && provider_catalog::supports_extra_accounts(family)
        })
}

fn configured_extra_account_ids() -> HashSet<String> {
    let mut ids: HashSet<String> = provider_catalog::provider_definitions()
        .iter()
        .filter(|definition| definition.supports_extra_accounts)
        .flat_map(|definition| {
            accounts::load_accounts(definition.family_id)
                .into_iter()
                .map(move |account| {
                    accounts::card_id_for_account(definition.family_id, &account)
                })
        })
        .collect();
    // Antigravity slots and Cursor imported accounts use their own
    // fingerprint domains — without these the cached first paint would
    // filter their cards out until the live fetch lands.
    ids.extend(
        antigravity_accounts::load_slots()
            .iter()
            .map(antigravity_accounts::card_id_for_slot),
    );
    ids.extend(
        cursor_accounts::load_accounts()
            .iter()
            .map(cursor_accounts::card_id_for_account),
    );
    // One/New API relay sites are accounts too — their card ids come from
    // the site store (one per key, plus token-only sites), not accounts.rs.
    ids.extend(providers::onenewapi::key_card_ids());
    ids
}

fn cached_extra_account_id_is_configured(id: &str, configured: &HashSet<String>) -> bool {
    !is_extra_account_card_id(id) || configured.contains(id)
}

fn retain_current_onenewapi_results(
    all: &mut Vec<providers::Snapshot>,
    expected: &HashMap<String, u64>,
    current: &HashMap<String, u64>,
) -> Vec<String> {
    let stale: Vec<String> = all
        .iter()
        .filter(|snapshot| {
            family_of(&snapshot.id) == "onenewapi"
                && expected.get(&snapshot.id) != current.get(&snapshot.id)
        })
        .map(|snapshot| snapshot.id.clone())
        .collect();
    let stale_set: HashSet<&str> = stale.iter().map(String::as_str).collect();
    all.retain(|snapshot| !stale_set.contains(snapshot.id.as_str()));
    stale
}

static ONENEWAPI_MUTATION_GENERATION: AtomicU64 = AtomicU64::new(0);
static ONENEWAPI_ACTIVE_MUTATIONS: AtomicU64 = AtomicU64::new(0);
static ONENEWAPI_SNAPSHOT_GENERATIONS: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();

fn onenewapi_mutation_generation() -> u64 {
    ONENEWAPI_MUTATION_GENERATION.load(Ordering::Acquire)
}

fn onenewapi_snapshot_generations(ids: impl IntoIterator<Item = String>) -> HashMap<String, u64> {
    let generations = ONENEWAPI_SNAPSHOT_GENERATIONS
        .get_or_init(Default::default)
        .lock()
        .unwrap();
    ids.into_iter()
        .map(|id| {
            let generation = generations.get(&id).copied().unwrap_or(0);
            (id, generation)
        })
        .collect()
}

fn bump_onenewapi_snapshot_generations(ids: &[String]) {
    let mut generations = ONENEWAPI_SNAPSHOT_GENERATIONS
        .get_or_init(Default::default)
        .lock()
        .unwrap();
    for id in ids {
        *generations.entry(id.clone()).or_default() += 1;
    }
}

struct OneNewApiMutationGuard {
    snapshot_ids: Vec<String>,
}

impl OneNewApiMutationGuard {
    fn begin(snapshot_ids: Vec<String>) -> Self {
        ONENEWAPI_ACTIVE_MUTATIONS.fetch_add(1, Ordering::AcqRel);
        bump_onenewapi_snapshot_generations(&snapshot_ids);
        ONENEWAPI_MUTATION_GENERATION.fetch_add(1, Ordering::AcqRel);
        Self { snapshot_ids }
    }
}

impl Drop for OneNewApiMutationGuard {
    fn drop(&mut self) {
        bump_onenewapi_snapshot_generations(&self.snapshot_ids);
        ONENEWAPI_MUTATION_GENERATION.fetch_add(1, Ordering::AcqRel);
        ONENEWAPI_ACTIVE_MUTATIONS.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Strip deleted One/New API *key cards* from config. Never removes family
/// id `onenewapi` just because keys went away. Returns only changed keys.
fn purge_onenewapi_from_config(cfg: &mut Value, snapshot_ids: &[String]) -> Value {
    if snapshot_ids.is_empty() {
        return json!({});
    }
    let drop: HashSet<&str> = snapshot_ids.iter().map(String::as_str).collect();
    let mut patch = serde_json::Map::new();

    if let Some(arr) = cfg.get_mut("disabled").and_then(Value::as_array_mut) {
        let before = arr.len();
        arr.retain(|v| v.as_str().map(|s| !drop.contains(s)).unwrap_or(true));
        if arr.len() != before {
            patch.insert("disabled".into(), Value::Array(arr.clone()));
        }
    }

    let mut layout_changed = false;
    if let Some(layout) = cfg.get_mut("layout").and_then(Value::as_object_mut) {
        if let Some(order) = layout
            .get_mut("providerOrder")
            .and_then(Value::as_array_mut)
        {
            let before = order.len();
            order.retain(|v| v.as_str().map(|s| !drop.contains(s)).unwrap_or(true));
            layout_changed |= order.len() != before;
        }
        if let Some(providers) = layout.get_mut("providers").and_then(Value::as_object_mut) {
            for id in snapshot_ids {
                layout_changed |= providers.remove(id).is_some();
            }
        }
    }
    if layout_changed {
        if let Some(layout) = cfg.get("layout") {
            patch.insert("layout".into(), layout.clone());
        }
    }

    let pinned_hit = cfg
        .get("pinned")
        .and_then(|p| p.get("provider"))
        .and_then(Value::as_str)
        .is_some_and(|p| drop.contains(p));
    if pinned_hit {
        cfg["pinned"] = Value::Null;
        patch.insert("pinned".into(), Value::Null);
    }

    if let Some(arr) = cfg.get_mut("trayProviders").and_then(Value::as_array_mut) {
        let before = arr.len();
        arr.retain(|v| v.as_str().map(|s| !drop.contains(s)).unwrap_or(true));
        if arr.len() != before {
            patch.insert("trayProviders".into(), Value::Array(arr.clone()));
        }
    }

    Value::Object(patch)
}

fn onenewapi_purge_restore_patch(original: &Value, purge_patch: &Value) -> Value {
    let mut restore = serde_json::Map::new();
    if let Some(obj) = purge_patch.as_object() {
        for key in obj.keys() {
            restore.insert(key.clone(), original.get(key).cloned().unwrap_or(Value::Null));
        }
    }
    Value::Object(restore)
}

fn persist_onenewapi_config_purge(snapshot_ids: &[String]) -> Result<Value, String> {
    // Tests must not rewrite the developer's real config.json.
    if cfg!(test) {
        return Ok(json!({}));
    }
    let mut cfg = config_with_defaults(load_config());
    let original = cfg.clone();
    let patch = purge_onenewapi_from_config(&mut cfg, snapshot_ids);
    let restore = onenewapi_purge_restore_patch(&original, &patch);
    if patch.as_object().is_some_and(|o| !o.is_empty()) {
        set_config_inner(patch)?;
    }
    Ok(restore)
}

fn restore_onenewapi_config_purge(restore: Value) -> Result<(), String> {
    if restore.as_object().map(|o| o.is_empty()).unwrap_or(true) {
        return Ok(());
    }
    if cfg!(test) {
        return Ok(());
    }
    set_config_inner(restore).map(|_| ())
}

fn purge_onenewapi_cards(key_ids: &[String]) -> Result<(), String> {
    purge_onenewapi_cards_coordinated(
        key_ids,
        persist_onenewapi_config_purge,
        |ids| forget_onenewapi_key_ids(ids.iter().cloned()),
        restore_onenewapi_config_purge,
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn purge_onenewapi_cards_with(
    key_ids: &[String],
    persist_config: impl FnOnce(&[String]) -> Result<(), String>,
) -> Result<(), String> {
    purge_onenewapi_cards_coordinated(
        key_ids,
        |ids| persist_config(ids).map(|()| json!({})),
        |ids| forget_onenewapi_key_ids(ids.iter().cloned()),
        |_| Ok(()),
    )
}

fn purge_onenewapi_cards_coordinated(
    key_ids: &[String],
    persist_config: impl FnOnce(&[String]) -> Result<Value, String>,
    forget: impl FnOnce(&[String]) -> Result<(), String>,
    restore_config: impl FnOnce(Value) -> Result<(), String>,
) -> Result<(), String> {
    if key_ids.is_empty() {
        return Ok(());
    }
    let restore = persist_config(&onenewapi_snapshot_ids(key_ids))?;
    if let Err(error) = forget(key_ids) {
        return match restore_config(restore) {
            Ok(()) => Err(error),
            Err(restore_error) => Err(format!(
                "{error}; restore card settings failed: {restore_error}"
            )),
        };
    }
    Ok(())
}

fn onenewapi_after_site_save(
    previous: &providers::onenewapi::SiteDto,
    site: &providers::onenewapi::SiteDto,
) -> Result<(), String> {
    if site.base_url != previous.base_url {
        return Ok(());
    }
    if site.name != previous.name {
        let renames: Vec<(String, String)> = site
            .keys
            .iter()
            .map(|key| {
                (
                    format!("onenewapi@{}", key.id),
                    format!("{} · {}", site.name, key.label),
                )
            })
            .collect();
        rename_cached_snapshots(&renames)?;
    }
    Ok(())
}

fn rename_cached_snapshot(id: &str, new_name: String) -> Result<(), String> {
    rename_cached_snapshots(&[(id.to_string(), new_name)])
}

fn rename_cached_snapshots(renames: &[(String, String)]) -> Result<(), String> {
    let mut map = last_ok().lock().unwrap();
    rename_cached_snapshots_in(&mut map, renames, persist_last_ok)
}

#[cfg_attr(not(test), allow(dead_code))]
fn rename_cached_snapshot_in<Persist>(
    map: &mut HashMap<String, CachedSnap>,
    id: &str,
    new_name: String,
    persist: Persist,
) -> Result<(), String>
where
    Persist: FnOnce(&HashMap<String, CachedSnap>) -> Result<(), String>,
{
    rename_cached_snapshots_in(map, &[(id.to_string(), new_name)], persist)
}

fn rename_cached_snapshots_in<Persist>(
    map: &mut HashMap<String, CachedSnap>,
    renames: &[(String, String)],
    persist: Persist,
) -> Result<(), String>
where
    Persist: FnOnce(&HashMap<String, CachedSnap>) -> Result<(), String>,
{
    let mut next = map.clone();
    let mut changed = false;
    for (id, new_name) in renames {
        if let Some(entry) = next.get_mut(id) {
            if entry.snap.name != *new_name {
                entry.snap.name = new_name.clone();
                changed = true;
            }
        }
    }
    if changed {
        persist(&next)?;
        *map = next;
    }
    Ok(())
}

/// One/New API is two-level: family id `onenewapi` disables every key card.
/// Claude/Codex extra accounts stay independent of the bare family id.
fn card_is_disabled(id: &str, disabled: &[String]) -> bool {
    if disabled.iter().any(|d| d == id) {
        return true;
    }
    family_of(id) == "onenewapi" && disabled.iter().any(|d| d == "onenewapi")
}

// Owned id/name so dynamically discovered account cards (claude@<hash>)
// can ride the same guard as the static providers under a 'static spawn.
async fn guarded<F>(id: String, name: String, fut: F) -> providers::Snapshot
where
    F: std::future::Future<Output = providers::Snapshot> + Send + 'static,
{
    let id = id.as_str();
    let name = name.as_str();
    let now = now_ms() as i64;
    let benched = {
        let map = fail_state().lock().unwrap();
        map.get(id)
            .filter(|f| now < f.until_ms)
            .map(|f| f.note.clone())
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
                    format!(
                        "rate limited — the vendor asked to wait ~{}m",
                        (ms / 60_000).max(1)
                    )
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

/// Last-good Kimi snapshot on disk. Used to skip the leftover Moonshot
/// fetch only when that card has actually painted *recently* — a
/// credentials file, or a day-old cache entry, must not hide the wallet.
fn cached_kimi_ok() -> bool {
    let path = providers::config_dir().join("last_snapshots.json");
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(doc) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    cached_kimi_ok_from(&doc, now_ms)
}

fn cached_kimi_ok_from(doc: &Value, now_ms: i64) -> bool {
    if doc.pointer("/kimi/snap/status").and_then(Value::as_str) != Some("ok") {
        return false;
    }
    let at = doc.pointer("/kimi/at").and_then(Value::as_i64).unwrap_or(0);
    at > 0 && now_ms.saturating_sub(at) <= SNAPSHOT_CACHE_MS
}

fn fold_moonshot_into_kimi(all: &mut Vec<providers::Snapshot>) {
    let Some(kimi) = all.iter().find(|s| s.id == "kimi" && s.status == "ok") else {
        return;
    };
    // Don't throw away a freshly fetched wallet just because the plan
    // card loaded. Fold only when Kimi already carries those rows, or
    // when Moonshot has nothing to show (plan-only / no_credentials).
    let kimi_has_wallet = kimi.metrics.iter().any(|m| is_kimi_wallet_label(&m.label));
    let moonshot_has_rows = all
        .iter()
        .any(|s| s.id == "moonshot" && !s.metrics.is_empty());
    if kimi_has_wallet || !moonshot_has_rows {
        all.retain(|s| s.id != "moonshot");
    }
}

fn is_kimi_wallet_label(label: &str) -> bool {
    matches!(
        label,
        "API" | "Credits used" | "Balance" | "Vouchers" | "Cash"
    )
}

fn restore_kimi_wallet_rows(current: &mut providers::Snapshot, previous: &providers::Snapshot) {
    if current.metrics.iter().any(|m| m.label == "API") {
        return;
    }
    for m in &previous.metrics {
        if is_kimi_wallet_label(&m.label) && !current.metrics.iter().any(|x| x.label == m.label) {
            current.metrics.push(m.clone());
        }
    }
}

fn restore_last_success_after_error(
    current: &mut providers::Snapshot,
    previous: &providers::Snapshot,
    age_ms: i64,
) -> bool {
    if current.status != "error" || age_ms > SNAPSHOT_CACHE_MS {
        return false;
    }
    let warning = current.error.clone();
    *current = previous.clone();
    if age_ms > STALE_GRACE_MS {
        current.stale = true;
        current.warning = warning;
    }
    true
}

/// One extra account's snapshot, fetched through the same snapshot_with_key
/// flow a pasted key uses (what test_api_key probes), with the account card's
/// own id and display name passed into the provider adapter. Lives here
/// rather than in accounts.rs because it dispatches into provider modules the
/// parse-tests harness (which compiles accounts.rs) doesn't mirror.
async fn account_snapshot(
    family: String,
    id: String,
    name: String,
    key: String,
    base_url: Option<String>,
) -> providers::Snapshot {
    match family.as_str() {
        "deepseek" => providers::deepseek::snapshot_with_key_as(&key, &id, &name).await,
        "kimi" => providers::kimi::snapshot_with_key_as(&key, &id, &name).await,
        "stepfun" => providers::stepfun::snapshot_with_key_as(&key, &id, &name).await,
        "siliconflow" => providers::siliconflow::snapshot_with_key_as(&key, &id, &name).await,
        "novita" => providers::novita::snapshot_with_key_as(&key, &id, &name).await,
        "relaybalance" => match base_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
            Some(url) => {
                providers::relaybalance::snapshot_with_key_at(&key, url, &id, &name).await
            }
            None => providers::Snapshot::error(
                &id,
                &name,
                "this account has no base URL — remove and re-add it".into(),
            ),
        },
        other => providers::Snapshot::error(
            &id,
            &name,
            format!("unknown multi-account provider: {other}"),
        ),
    }
}

/// Default-account model migration, run once per family at fetch. A main
/// key saved through the old gear field imports as the FIRST (default)
/// account; after the import the old `<provider>.json` file is REMOVED so
/// deleting the account actually deletes the key (no zombie that re-imports
/// on the next fetch). When that same key was already added as an account —
/// identical fingerprint — nothing is written, and the stored file is
/// removed too: the account card IS that key now.
fn accounts_with_imported_main_key(family: &str) -> Vec<accounts::AccountEntry> {
    let mut list = accounts::load_accounts(family);
    let base_url = if family == "relaybalance" {
        providers::stored_base_url(family)
    } else {
        None
    };
    if let Some(key) = providers::stored_key_file(family) {
        let entry = accounts::AccountEntry {
            label: String::new(),
            api_key: key,
            base_url,
        };
        let id = accounts::card_id_for_account(family, &entry);
        let already_present = list.iter().any(|a| accounts::card_id_for_account(family, a) == id);
        if !already_present {
            list.insert(0, entry);
        }
        // One-shot: drop the migrated source. If the import errored, keep
        // the file so the next fetch retries.
        let saved = accounts::save_accounts(family, &list).is_ok();
        if saved {
            providers::remove_stored_key_file(family);
        }
    }
    list
}

/// Called by the UI. Refreshes every enabled provider at the same time and
/// returns whatever each one found — data, "not signed in", or an error.
/// The disabled argument is accepted for compatibility but ignored:
/// config.json is the single source of truth, so the background refresh
/// loop and the UI can never disagree about who is enabled.
#[tauri::command]
async fn fetch_usage(
    app: tauri::AppHandle,
    disabled: Option<Vec<String>>,
) -> Vec<providers::Snapshot> {
    let _ = disabled;
    run_usage_fetch(&app).await
}

/// Refreshes ONE provider card on demand (the ⟳ button in the card head).
/// Dispatches to the same snapshot function the full refresh cycle uses,
/// but skips alert checking, telemetry, and the snapshot cache — those
/// belong to the global cycle. The returned snapshot is merged into the
/// frontend's `lastSnapshots` by the caller.
#[tauri::command]
async fn refresh_provider(provider_id: String) -> Result<providers::Snapshot, String> {
    let family = family_of(&provider_id);
    let cfg = config_with_defaults(load_config());
    if cfg
        .get("disabled")
        .and_then(Value::as_array)
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(family.as_str())))
        && family_of(&provider_id) != "onenewapi"
    {
        return Err(format!("{family} is disabled"));
    }

    // Bare family id → the native provider path.
    if provider_id == family {
        return Ok(match family.as_str() {
            "claude" => providers::claude::snapshot().await,
            "codex" => providers::codex::snapshot().await,
            "cursor" => providers::cursor::snapshot().await,
            "opencode" => providers::opencode::snapshot().await,
            "copilot" => providers::copilot::snapshot().await,
            "grok" => providers::grok::snapshot().await,
            "devin" => providers::devin::snapshot().await,
            "minimax" => providers::minimax::snapshot().await,
            "openrouter" => providers::openrouter::snapshot().await,
            "zai" => providers::zai::snapshot().await,
            "antigravity" => providers::antigravity::snapshot().await,
            "deepseek" => providers::deepseek::snapshot().await,
            "moonshot" => providers::moonshot::snapshot().await,
            "elevenlabs" => providers::elevenlabs::snapshot().await,
            "ollama" => providers::ollama::snapshot().await,
            "codebuff" => providers::codebuff::snapshot().await,
            "kilo" => providers::kilo::snapshot().await,
            "aihubmix" => providers::aihubmix::snapshot().await,
            "qwen" => providers::qwen::snapshot().await,
            "hermes" => providers::hermes::snapshot().await,
            "kimi" => providers::kimi::snapshot().await,
            "stepfun" => providers::stepfun::snapshot().await,
            "siliconflow" => providers::siliconflow::snapshot().await,
            "novita" => providers::novita::snapshot().await,
            "relaybalance" => providers::relaybalance::snapshot().await,
            _ => return Err(format!("no single-provider refresh for {family}")),
        });
    }

    // Antigravity credential slots.
    if family == "antigravity" {
        let slot = antigravity_accounts::load_slots()
            .into_iter()
            .find(|s| antigravity_accounts::card_id_for_slot(s) == provider_id)
            .ok_or_else(|| format!("no antigravity slot {provider_id}"))?;
        let name = if slot.label.trim().is_empty() {
            "Antigravity — captured".to_string()
        } else {
            format!("Antigravity — {}", slot.label.trim())
        };
        return Ok(
            providers::antigravity::snapshot_for_slot(&slot.refresh_token, &provider_id, &name)
                .await,
        );
    }

    // Cursor imported accounts.
    if family == "cursor" {
        let account = cursor_accounts::load_accounts()
            .into_iter()
            .find(|a| cursor_accounts::card_id_for_account(a) == provider_id)
            .ok_or_else(|| format!("no cursor account {provider_id}"))?;
        let name = if account.label.trim().is_empty() {
            format!("Cursor — {}", account.email)
        } else {
            format!("Cursor — {}", account.label.trim())
        };
        return Ok(
            providers::cursor::snapshot_with_token_as(
                &account.access_token,
                account.refresh_token.as_deref(),
                &provider_id,
                &name,
                account.membership.as_deref(),
            )
            .await,
        );
    }

    // Claude / Codex extra accounts.
    if family == "claude" {
        let account = providers::claude::discover_extra_accounts()
            .into_iter()
            .find(|a| a.id == provider_id)
            .ok_or_else(|| format!("no claude account {provider_id}"))?;
        return Ok(
            providers::claude::snapshot_at(account.dir, provider_id.clone(), account.name).await,
        );
    }
    if family == "codex" {
        let account = providers::codex::discover_extra_accounts()
            .into_iter()
            .find(|a| a.id == provider_id)
            .ok_or_else(|| format!("no codex account {provider_id}"))?;
        return Ok(
            providers::codex::snapshot_at(account.dir, provider_id.clone(), account.name).await,
        );
    }

    // One/New API relay accounts: cards come from the site store, keyed by
    // relay key id (or site id for token-only sites). The bare family id
    // resolves to the first card — the merged card's default tab.
    if family == "onenewapi" {
        let cards = providers::onenewapi::key_cards()?;
        let card = cards
            .iter()
            .find(|card| card.id == provider_id)
            .or_else(|| cards.first())
            .ok_or_else(|| "no One/New API sites configured".to_string())?;
        let client = providers::http_no_redirect();
        return Ok(providers::onenewapi::snapshot_key_with_client(
            client,
            card.clone(),
        )
        .await);
    }

    // API-key multi-account families (kimi@fp, deepseek@fp, …).
    if accounts::provider_takes_accounts(&family) {
        let locale = i18n::resolved_locale(&config_with_defaults(load_config()));
        let accounts = accounts::load_accounts(&family);
        let index = accounts
            .iter()
            .position(|a| accounts::card_id_for_account(&family, a) == provider_id)
            .ok_or_else(|| format!("no {family} account {provider_id}"))?;
        let acct = &accounts[index];
        let name = format!(
            "{} — {}",
            accounts::family_display_name(&family),
            accounts::display_label(&acct.label, index + 1, locale)
        );
        return Ok(account_snapshot(
            family.clone(),
            provider_id.clone(),
            name,
            acct.api_key.clone(),
            acct.base_url.clone(),
        )
        .await);
    }

    Err(format!("no single-provider refresh for {provider_id}"))
}

/// Only one usage refresh runs at a time: the background loop and a manual
/// Refresh press would otherwise race the same provider endpoints and the
/// snapshot cache.
fn usage_fetch_lock() -> &'static tauri::async_runtime::Mutex<()> {
    static LOCK: OnceLock<tauri::async_runtime::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tauri::async_runtime::Mutex::new(()))
}

/// The full refresh with no webview involved. The tray popover spends most
/// of its life hidden, where WebView2 throttles the frontend's setInterval
/// to a halt — so the auto-refresh loop in setup() drives this directly.
async fn run_usage_fetch(app: &tauri::AppHandle) -> Vec<providers::Snapshot> {
    let _guard = usage_fetch_lock().lock().await;
    let cfg = config_with_defaults(load_config());
    let disabled: Vec<String> = cfg
        .get("disabled")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    // Each provider future is boxed onto the heap and spawned as its own
    // task. A single tokio::join! over 28 inlined futures builds one huge
    // combined state machine on the calling thread's stack — at 28 providers
    // that overflowed the main thread's 1 MB stack and killed the app.
    type BoxedSnap =
        std::pin::Pin<Box<dyn std::future::Future<Output = providers::Snapshot> + Send>>;
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
        ("moonshot", Box::pin(guarded("moonshot".into(), "Kimi API".into(), providers::moonshot::snapshot()))),
        ("elevenlabs", Box::pin(guarded("elevenlabs".into(), "ElevenLabs".into(), providers::elevenlabs::snapshot()))),
        ("ollama", Box::pin(guarded("ollama".into(), "Ollama".into(), providers::ollama::snapshot()))),
        ("codebuff", Box::pin(guarded("codebuff".into(), "Codebuff".into(), providers::codebuff::snapshot()))),
        ("kilo", Box::pin(guarded("kilo".into(), "Kilo".into(), providers::kilo::snapshot()))),
        ("aihubmix", Box::pin(guarded("aihubmix".into(), "AihubMix".into(), providers::aihubmix::snapshot()))),
        ("qwen", Box::pin(guarded("qwen".into(), "Qwen Code".into(), providers::qwen::snapshot()))),
        ("hermes", Box::pin(guarded("hermes".into(), "Hermes".into(), providers::hermes::snapshot()))),
        ("kimi", Box::pin(guarded("kimi".into(), "Kimi Code".into(), providers::kimi::snapshot()))),
        ("stepfun", Box::pin(guarded("stepfun".into(), "StepFun".into(), providers::stepfun::snapshot()))),
        ("siliconflow", Box::pin(guarded("siliconflow".into(), "SiliconFlow".into(), providers::siliconflow::snapshot()))),
        ("novita", Box::pin(guarded("novita".into(), "Novita AI".into(), providers::novita::snapshot()))),
        ("relaybalance", Box::pin(guarded("relaybalance".into(), "Custom Balance".into(), providers::relaybalance::snapshot()))),
    ];
    // Skip the leftover Moonshot fetch only when the last Kimi card
    // actually painted — a credentials file alone is not enough (expired
    // login / network blip would otherwise hide the wallet with nothing
    // to fall back to). The post-fetch retain still drops it whenever
    // this cycle's Kimi snapshot is ok.
    let kimi_card_live = cached_kimi_ok();
    // Default-account model (ccSwitch's accounts + default_account_id):
    // for multi-account families the accounts list owns every key. When it
    // is non-empty the family's own fetch (stored main key, CLI OAuth
    // fallback) is skipped and accounts[0] publishes under the bare family
    // id — the main card IS the default account, so one key can never show
    // twice.
    let mut extra_accounts: Vec<(&str, Vec<accounts::AccountEntry>)> = Vec::new();
    for definition in provider_catalog::provider_definitions()
        .iter()
        .filter(|definition| definition.supports_extra_accounts)
    {
        let list = accounts_with_imported_main_key(definition.family_id);
        if !list.is_empty() {
            extra_accounts.push((definition.family_id, list));
        }
    }
    let multi_account_families: HashSet<&str> =
        extra_accounts.iter().map(|(family, _)| *family).collect();
    let mut futs: Vec<(String, BoxedSnap)> = base
        .into_iter()
        .filter(|(id, _)| {
            (*id != "moonshot"
                || !providers::kimi::has_credentials()
                || disabled.iter().any(|d| d == "kimi")
                || !kimi_card_live)
                && !multi_account_families.contains(*id)
        })
        .map(|(id, fut)| (id.to_string(), fut))
        .collect();
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
    let mut expected_onenewapi_generations = HashMap::new();
    let onenewapi_generation_before = onenewapi_mutation_generation();
    let onenewapi_active_before = ONENEWAPI_ACTIVE_MUTATIONS.load(Ordering::Acquire);
    if !disabled.iter().any(|d| d == "onenewapi") {
        if let Ok(cards) = providers::onenewapi::prepare_key_cards().await {
            expected_onenewapi_generations =
                onenewapi_snapshot_generations(cards.iter().map(|card| card.id.clone()));
            let onenewapi_generation_after = onenewapi_mutation_generation();
            let onenewapi_active_after = ONENEWAPI_ACTIVE_MUTATIONS.load(Ordering::Acquire);
            let stable = onenewapi_active_before == 0
                && onenewapi_active_after == 0
                && onenewapi_generation_before == onenewapi_generation_after;
            if stable {
                let clients = providers::onenewapi::refresh_clients(&cards);
                for card in cards {
                    let client = clients
                        .get(&card.origin)
                        .cloned()
                        .unwrap_or_else(providers::http_no_redirect);
                    let (id, name) = (card.id.clone(), card.name.clone());
                    futs.push((
                        id.clone(),
                        Box::pin(guarded(
                            id,
                            name,
                            providers::onenewapi::snapshot_key_with_client(client, card),
                        )),
                    ));
                }
            } else {
                expected_onenewapi_generations.clear();
            }
        }
    }
    // Extra API-key accounts: index 0 (the default account) publishes
    // under the bare family id — it IS the main card. The rest render
    // their own stable <provider>@<fingerprint> cards running the same
    // snapshot_with_key flow a pasted key uses.
    let locale = i18n::resolved_locale(&cfg);
    for (family, list) in &extra_accounts {
        for (i, acct) in list.iter().enumerate() {
            let (id, name) = if i == 0 {
                (
                    family.to_string(),
                    accounts::family_display_name(family),
                )
            } else {
                (
                    accounts::card_id_for_account(family, acct),
                    format!(
                        "{} — {}",
                        accounts::family_display_name(family),
                        accounts::display_label(&acct.label, i + 1, locale)
                    ),
                )
            };
            futs.push((
                id.clone(),
                Box::pin(guarded(
                    id.clone(),
                    name.clone(),
                    account_snapshot(
                        family.to_string(),
                        id.clone(),
                        name.clone(),
                        acct.api_key.clone(),
                        acct.base_url.clone(),
                    ),
                )),
            ));
        }
    }
    // Antigravity credential slots: captured Google accounts that are NOT
    // logged into the IDE. Each refreshes with its own refresh token and
    // queries Cloud Code directly, so slots work while the IDE is closed.
    // The bare antigravity card above stays the logged-in account — a slot
    // whose refresh token matches the current login IS that account, so it
    // spawns no card (same layout as Kimi: one card per account, never a
    // duplicate of the main card).
    let current_ag_refresh = providers::antigravity::current_refresh_token();
    let ag_slots = antigravity_accounts::load_slots();
    for slot in &ag_slots {
        if current_ag_refresh
            .as_deref()
            .is_some_and(|current| current == slot.refresh_token.trim())
        {
            continue;
        }
        let refresh = slot.refresh_token.clone();
        let id = antigravity_accounts::card_id_for_slot(slot);
        let name = if slot.label.trim().is_empty() {
            "Antigravity — captured".to_string()
        } else {
            format!("Antigravity — {}", slot.label.trim())
        };
        // The future must own its inputs: refresh/id/name are local here.
        let future = {
            let refresh = refresh.clone();
            let id = id.clone();
            let name = name.clone();
            async move { providers::antigravity::snapshot_for_slot(&refresh, &id, &name).await }
        };
        futs.push((
            id.clone(),
            Box::pin(guarded(id.clone(), name.clone(), future)),
        ));
    }
    // Cursor imported accounts: each queries with its own token pair.
    // The bare cursor card above stays the locally logged-in account.
    for acct in cursor_accounts::load_accounts() {
        let id = cursor_accounts::card_id_for_account(&acct);
        let label = if acct.label.trim().is_empty() {
            acct.email.clone()
        } else {
            acct.label.trim().to_string()
        };
        let name = if label.is_empty() {
            "Cursor — imported".to_string()
        } else {
            format!("Cursor — {label}")
        };
        let future = {
            let access = acct.access_token.clone();
            let refresh = acct.refresh_token.clone();
            let membership = acct.membership.clone();
            let id = id.clone();
            let name = name.clone();
            async move {
                providers::cursor::snapshot_with_token_as(
                    &access,
                    refresh.as_deref(),
                    &id,
                    &name,
                    membership.as_deref(),
                )
                .await
            }
        };
        futs.push((
            id.clone(),
            Box::pin(guarded(id.clone(), name.clone(), future)),
        ));
    }
    let futs: Vec<(String, BoxedSnap)> = futs
        .into_iter()
        .filter(|(id, _)| !card_is_disabled(id, &disabled))
        .collect();
    // Telemetry never learns account-scoped ids — an API-key fingerprint
    // must never leave the machine. Report families,
    // deduplicated, so a multi-account install looks like "claude" once.
    // (family_of is applied at EVERY telemetry boundary: enabled ids here,
    // refresh outcomes, and starred-metric prefixes.)
    let mut enabled_ids: Vec<String> = {
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
    let current_onenewapi_generations = onenewapi_snapshot_generations(
        all.iter()
            .filter(|snapshot| family_of(&snapshot.id) == "onenewapi")
            .map(|snapshot| snapshot.id.clone()),
    );
    let stale_onenewapi_ids = retain_current_onenewapi_results(
        &mut all,
        &expected_onenewapi_generations,
        &current_onenewapi_generations,
    );
    if !stale_onenewapi_ids.is_empty() {
        if !all
            .iter()
            .any(|snapshot| family_of(&snapshot.id) == "onenewapi")
        {
            enabled_ids.retain(|id| id != "onenewapi");
        }
        let mut failures = fail_state().lock().unwrap();
        for id in stale_onenewapi_ids {
            failures.remove(&id);
        }
    }

    for s in &all {
        let log_id = if family_of(&s.id) == "onenewapi" {
            "onenewapi"
        } else {
            s.id.as_str()
        };
        eprintln!(
            "[pane] {}: {} ({} metrics){}",
            log_id,
            s.status,
            s.metrics.len(),
            s.error
                .as_deref()
                .map(|e| format!(" — {e}"))
                .unwrap_or_default()
        );
    }

    // Transient server errors (a 503, a timeout) shouldn't blank a card the
    // user was just reading: fall back to the last good snapshot, marked
    // stale so the UI can say "Outdated" with the real error on hover. The
    // cache is persisted to disk so it survives app restarts; entries older
    // than a day are too misleading to show and get skipped.
    {
        let cache = last_ok();
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
            let mut map = cache.lock().unwrap();
            let mut removed = false;
            let mut to_store = serde_json::Map::new();
            for fam in ["claude", "codex"] {
                let cur = current.get(fam).cloned().unwrap_or(Value::Null);
                let old = stored.get(fam).cloned().unwrap_or(Value::Null);
                // Only a KNOWN stored identity differing from a KNOWN
                // current one is evidence of an account swap. A missing
                // stamp (first launch after updating) or a momentarily
                // unreadable identity file must not dump the last-good
                // cache — that's the safety net, not a swap.
                if !old.is_null() && !cur.is_null() && old != cur && map.remove(fam).is_some() {
                    removed = true;
                }
                // And a transient null never OVERWRITES a known identity:
                // erasing it would make a swap that happens before the next
                // launch undetectable.
                to_store.insert(
                    fam.to_string(),
                    if cur.is_null() && !old.is_null() {
                        old
                    } else {
                        cur
                    },
                );
            }
            // Persist the PRUNED cache before the new stamp: if this
            // refresh finds nothing ok (offline launch) the on-disk cache
            // would otherwise keep the old account's entry while the stamp
            // already claims the new one, resurrecting the wrong numbers
            // next launch. Stamp last, so a failed write just re-prunes.
            let cache_persisted = !removed || persist_last_ok(&map).is_ok();
            drop(map);
            let to_store = Value::Object(to_store);
            if cache_persisted && to_store != stored {
                let _ = std::fs::write(
                    &stamp_file,
                    serde_json::to_string_pretty(&to_store).unwrap_or_default(),
                );
            }
        }
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Ok(mut map) = cache.lock() {
            let mut dirty = false;
            let mut history_samples: Vec<(String, f64)> = Vec::new();
            for s in all.iter_mut() {
                if family_of(&s.id) == "onenewapi" {
                    let current = onenewapi_snapshot_generations([s.id.clone()]);
                    if expected_onenewapi_generations.get(&s.id) != current.get(&s.id) {
                        continue;
                    }
                }
                // Plan bars can succeed while the folded Moonshot wallet
                // call fails; keep last-known API/Balance rows so Almost
                // Out and the tray pin don't blink off for one timeout.
                // Do not re-cache the patched snapshot — that would reset
                // `at` and keep serving the same balance forever.
                let mut skip_cache = false;
                if s.id == "kimi" && s.status == "ok" && s.warning.is_some() {
                    if let Some(previous) = map.get("kimi") {
                        let age = now_ms - previous.at;
                        if age <= SNAPSHOT_CACHE_MS {
                            let n = s.metrics.len();
                            restore_kimi_wallet_rows(s, &previous.snap);
                            if s.metrics.len() > n {
                                skip_cache = true;
                                if age > STALE_GRACE_MS {
                                    s.stale = true;
                                }
                            }
                        }
                    }
                }
                if s.status == "ok" && !skip_cache {
                    map.insert(
                        s.id.clone(),
                        CachedSnap {
                            at: now_ms,
                            snap: s.clone(),
                        },
                    );
                    if let Some(used) = usage_history::worst_used_percent(&s.metrics) {
                        history_samples.push((s.id.clone(), used));
                    }
                    dirty = true;
                } else if s.status == "error" {
                    if let Some(previous) = map.get(&s.id) {
                        let age = now_ms - previous.at;
                        restore_last_success_after_error(s, &previous.snap, age);
                    }
                }
            }
            if dirty {
                if let Err(error) = persist_last_ok(&map) {
                    eprintln!("[pane] snapshot cache refresh: {error}");
                }
                usage_history::record_samples(&history_samples);
            }
        }
    }

    // A mutation can begin after the first post-request check. Recheck after
    // the cache critical section so an old result is neither published nor
    // included in alerts/telemetry while its replacement is being saved.
    let current_onenewapi_generations = onenewapi_snapshot_generations(
        all.iter()
            .filter(|snapshot| family_of(&snapshot.id) == "onenewapi")
            .map(|snapshot| snapshot.id.clone()),
    );
    let stale_onenewapi_ids = retain_current_onenewapi_results(
        &mut all,
        &expected_onenewapi_generations,
        &current_onenewapi_generations,
    );
    if !stale_onenewapi_ids.is_empty() {
        if !all
            .iter()
            .any(|snapshot| family_of(&snapshot.id) == "onenewapi")
        {
            enabled_ids.retain(|id| id != "onenewapi");
        }
        let mut failures = fail_state().lock().unwrap();
        for id in stale_onenewapi_ids {
            failures.remove(&id);
        }
    }

    // One Kimi card: Session / Weekly / API. Hide the leftover Moonshot
    // wallet card whenever the plan card is actually showing.
    fold_moonshot_into_kimi(&mut all);

    httpapi::publish(&all);
    // Anonymous daily-rollup telemetry (Settings → "Share anonymous usage
    // statistics"). Fire-and-forget: it must never delay or fail a refresh.
    {
        let enabled = cfg
            .get("telemetry")
            .and_then(Value::as_bool)
            .unwrap_or(true);
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
            refresh_minutes: cfg
                .get("refreshMinutes")
                .and_then(Value::as_u64)
                .unwrap_or(5),
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
        let outcomes = telemetry::collapse_onenewapi_outcomes(outcomes);
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

/// Builds the tray projection config from config.json — the same data the
/// frontend passes to sync_tray_surfaces, so the background loop can keep
/// the main tray numbers fresh while the window is hidden.
fn tray_projection_config_from(cfg: &Value) -> Option<tray_projection::TrayProjectionConfig> {
    let layout = cfg.get("layout").cloned().unwrap_or(Value::Null);
    serde_json::from_value(json!({
        "disabled": cfg.get("disabled").cloned().unwrap_or_else(|| json!([])),
        "providerOrder": layout.get("providerOrder").cloned().unwrap_or_else(|| json!([])),
        "providers": layout.get("providers").cloned().unwrap_or_else(|| json!({})),
        "pinned": cfg.get("pinned").cloned().unwrap_or(Value::Null),
        "locale": i18n::resolved_locale(cfg),
    }))
    .ok()
}

fn refresh_minutes_from(cfg: &Value) -> u64 {
    cfg.get("refreshMinutes")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .max(1)
}

/// Auto-refresh that does not depend on the webview: sleeps refreshMinutes
/// (re-read every cycle, so a Settings change applies without a restart),
/// fetches, updates the main tray numbers, then emits "usage-updated" so
/// an open window can adopt the fresh snapshots.
fn spawn_auto_refresh(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let minutes = refresh_minutes_from(&config_with_defaults(load_config()));
            tokio::time::sleep(std::time::Duration::from_secs(minutes * 60)).await;
            let snapshots = run_usage_fetch(&app).await;
            // The tray strip's provider logos are rasterized by the webview,
            // so the background loop only refreshes the main tray icon and
            // treats the last applied strip as the strip state.
            let cfg = config_with_defaults(load_config());
            if let Some(projection_cfg) = tray_projection_config_from(&cfg) {
                let strip_active = last_strip().lock().map(|s| !s.is_empty()).unwrap_or(false);
                let projection =
                    tray_projection::project_main_tray(&snapshots, &projection_cfg, strip_active);
                if let Err(error) = apply_main_tray_projection(&app, &projection) {
                    eprintln!("[pane] background tray sync: {error}");
                }
            }
            let _ = app.emit("usage-updated", snapshots);
        }
    });
}

/// The previous run's last-good snapshots, straight from the disk cache —
/// the instant first paint at launch. Cards show numbers in milliseconds
/// instead of a blank "Refreshing…" while the slowest provider answers
/// (at boot, with the network still coming up, that wait ran 30-40 s).
/// Everything is marked stale; the first live fetch replaces it.
#[tauri::command]
fn cached_usage() -> Vec<providers::Snapshot> {
    #[derive(serde::Deserialize)]
    struct CachedSnap {
        at: i64,
        snap: providers::Snapshot,
    }
    const MAX_STALE_MS: i64 = SNAPSHOT_CACHE_MS;
    let Ok(raw) = std::fs::read_to_string(providers::config_dir().join("last_snapshots.json"))
    else {
        return Vec::new();
    };
    let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, CachedSnap>>(&raw)
    else {
        return Vec::new();
    };

    let cfg = config_with_defaults(load_config());
    let disabled: Vec<String> = cfg
        .get("disabled")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let configured_onenewapi: HashSet<String> = providers::onenewapi::key_cards()
        .map(|cards| cards.into_iter().map(|card| card.id).collect())
        .unwrap_or_default();
    let configured_extra_accounts = configured_extra_account_ids();

    // Same account-swap rule as the live path: if a different account
    // signed into a default home since the cache was written, that
    // family's bare-id entry belongs to the old account — never paint it,
    // not even for the seconds until the live fetch lands.
    let stored: Value =
        std::fs::read_to_string(providers::config_dir().join("cache_identities.json"))
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_else(|| json!({}));
    let swapped: Vec<&str> = [
        ("claude", providers::claude::default_identity()),
        ("codex", providers::codex::default_identity()),
    ]
    .into_iter()
    .filter(|(fam, current)| {
        let old = stored.get(fam).cloned().unwrap_or(Value::Null);
        matches!((current, &old), (Some(cur), Value::String(o)) if cur != o)
    })
    .map(|(fam, _)| fam)
    .collect();

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let mut out: Vec<providers::Snapshot> = map
        .into_iter()
        .filter(|(id, c)| {
            now_ms - c.at <= MAX_STALE_MS
                && !card_is_disabled(id, &disabled)
                && cached_onenewapi_id_is_configured(id, &configured_onenewapi)
                && cached_extra_account_id_is_configured(id, &configured_extra_accounts)
                && !swapped.iter().any(|f| f == id)
        })
        .map(|(_, c)| {
            let mut s = c.snap;
            s.stale = true;
            s
        })
        .collect();
    fold_moonshot_into_kimi(&mut out);
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Computes local spend (Today / Yesterday / Last 30 Days) from the CLIs'
/// own session logs. Heavy file IO, so it runs on a blocking thread.
#[tauri::command]
async fn fetch_spend() -> Vec<spend::ProviderSpend> {
    eprintln!("[pane] spend: scan starting");
    let started = std::time::Instant::now();
    // Cursor's CSV export needs the async client; fetch it here and hand it
    // to the blocking scan. Unlike every other spend source it's an
    // authenticated NETWORK call, so it honors the disabled toggle the same
    // way fetch_usage does — a switched-off Cursor makes no requests.
    let cursor_disabled = config_with_defaults(load_config())
        .get("disabled")
        .and_then(Value::as_array)
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some("cursor")));
    let cursor_csv = if cursor_disabled {
        None
    } else {
        providers::cursor::fetch_usage_csv().await
    };
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

/// Sampled quota history per card id — the trend fallback for cards with no
/// local CLI logs (API-key accounts, relay keys). Cheap: one small file.
#[tauri::command]
fn fetch_usage_history() -> std::collections::BTreeMap<String, Vec<f64>> {
    usage_history::trend_map()
}

/// Saves (or clears, when `key` is empty) a user-pasted API key to
/// %APPDATA%\Pane\<provider>.json. Providers with a user-chosen endpoint
/// (relaybalance) pass `base_url` too, stored alongside as `baseUrl`.
#[tauri::command]
fn set_api_key(provider: String, key: String, base_url: Option<String>) -> Result<(), String> {
    if !provider_catalog::supports_api_key(&provider) {
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
    let mut doc = serde_json::json!({ "apiKey": key });
    if let Some(url) = base_url.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        if provider != "relaybalance" {
            return Err("base URL is only supported for Custom Balance".into());
        }
        providers::relaybalance::validate_base_url(url)?;
        doc["baseUrl"] = serde_json::Value::from(url);
    }
    std::fs::write(&path, doc.to_string()).map_err(|e| format!("write key file: {e}"))
}

/// The base URL saved alongside a provider's API key (relaybalance's
/// user-chosen relay host), so Settings can pre-fill its input.
#[tauri::command]
fn get_base_url(provider: String) -> Option<String> {
    match provider.as_str() {
        "relaybalance" => providers::stored_base_url("relaybalance"),
        _ => None,
    }
}

/// Outcome of a live test_api_key probe, shown in the Customize ⚙ panel:
/// the metric count on success, the backend's error verbatim on failure.
#[derive(serde::Serialize)]
struct TestResult {
    ok: bool,
    metrics: usize,
    message: String,
}

/// Live test of a pasted API key against its provider (Customize "Test
/// connection"). Pure probe — nothing is written, the key never touches
/// disk. Custom Balance additionally needs the relay's base URL; testing
/// always uses the pasted values, never the stored ones.
#[tauri::command]
async fn test_api_key(
    provider: String,
    key: String,
    base_url: Option<String>,
) -> Result<TestResult, String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("API key is empty".into());
    }
    let snap = match provider.as_str() {
        "openrouter" => providers::openrouter::snapshot_with_key(key).await,
        "zai" => providers::zai::snapshot_with_key(key).await,
        "minimax" => providers::minimax::snapshot_with_key(key).await,
        "deepseek" => providers::deepseek::snapshot_with_key(key).await,
        "moonshot" => providers::moonshot::snapshot_with_key(key).await,
        "elevenlabs" => providers::elevenlabs::snapshot_with_key(key).await,
        "codebuff" => providers::codebuff::snapshot_with_key(key).await,
        "kilo" => providers::kilo::snapshot_with_key(key).await,
        "aihubmix" => providers::aihubmix::snapshot_with_key(key).await,
        "qwen" => providers::qwen::snapshot_with_key(key).await,
        "kimi" => providers::kimi::snapshot_with_key(key).await,
        "opencode" => providers::opencode::snapshot_with_key(key).await,
        "stepfun" => providers::stepfun::snapshot_with_key(key).await,
        "siliconflow" => providers::siliconflow::snapshot_with_key(key).await,
        "novita" => providers::novita::snapshot_with_key(key).await,
        "relaybalance" => {
            let url = base_url
                .as_deref()
                .map(str::trim)
                .filter(|u| !u.is_empty())
                .ok_or_else(|| "a base URL is required for Custom Balance".to_string())?;
            providers::relaybalance::snapshot_with_key(key, url).await
        }
        _ => return Err(format!("unknown provider: {provider}")),
    };
    Ok(TestResult {
        ok: snap.status == "ok",
        metrics: snap.metrics.len(),
        message: snap.error.unwrap_or_default(),
    })
}

/// Captures the IDE's current Google OAuth bundle (Windows Credential
/// Manager `gemini:antigravity`) into a named Antigravity slot so its
/// quota keeps being monitored after the user logs into another account.
#[tauri::command]
fn antigravity_capture_account(label: String) -> Result<(), String> {
    // Snapshot the IDE's current Google OAuth bundle into a slot.
    let token = providers::antigravity::load_stored_token_pub()
        .ok_or_else(|| "Antigravity 未登录 — 先在 IDE 里登录要捕获的账号".to_string())?;
    if token.refresh_token.as_deref().unwrap_or_default().trim().is_empty() {
        return Err("凭据管理器里的 token 没有 refresh_token，无法捕获".into());
    }
    let mut slots = antigravity_accounts::load_slots();
    let candidate = antigravity_accounts::AgSlot {
        label: label.trim().to_string(),
        refresh_token: token.refresh_token.unwrap_or_default(),
        access_token: token.access_token,
        expires_at: token
            .expires_at_ms
            .map(|ms| chrono::DateTime::from_timestamp_millis(ms).map(|d| d.to_rfc3339()))
            .flatten(),
        captured_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    };
    let candidate_id = antigravity_accounts::card_id_for_slot(&candidate);
    if slots
        .iter()
        .any(|s| antigravity_accounts::card_id_for_slot(s) == candidate_id)
    {
        return Err("该 Google 账号已捕获过".into());
    }
    slots.push(candidate);
    antigravity_accounts::save_slots(&slots)
}

/// Appends one extra API-key account (Customize ⚙ → "Add account") to
/// %APPDATA%\Pane\accounts\<provider>.json. The key has already passed a
/// test_api_key probe on the frontend; nothing here touches the network.
#[tauri::command]
fn account_add(
    provider: String,
    label: String,
    api_key: String,
    base_url: Option<String>,
) -> Result<(), String> {
    if !accounts::provider_takes_accounts(&provider) {
        return Err(format!("unknown multi-account provider: {provider}"));
    }
    let key = api_key.trim();
    if key.is_empty() {
        return Err("API key is empty".into());
    }
    let base_url = base_url
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(str::to_string);
    if let Some(url) = &base_url {
        if provider == "relaybalance" {
            providers::relaybalance::validate_base_url(url)?;
        } else {
            return Err("base URL is only supported for Custom Balance".into());
        }
    }
    if provider == "relaybalance" && base_url.is_none() {
        return Err("a base URL is required for Custom Balance".into());
    }
    let mut entries = accounts::load_accounts(&provider);
    let candidate = accounts::AccountEntry {
        label: label.trim().to_string(),
        api_key: key.to_string(),
        base_url,
    };
    let candidate_id = accounts::card_id_for_account(&provider, &candidate);
    if entries
        .iter()
        .any(|entry| accounts::card_id_for_account(&provider, entry) == candidate_id)
    {
        return Err("this API key and relay URL are already added".into());
    }
    entries.push(candidate);
    accounts::save_accounts(&provider, &entries)
}

/// Removes the account at `index` — its position in the accounts file,
/// 0-based, the same order account_list reports. The account's
/// <provider>@<fingerprint> card disappears on the next fetch.
#[tauri::command]
fn account_remove(provider: String, index: usize) -> Result<(), String> {
    if provider == "antigravity" {
        let mut slots = antigravity_accounts::load_slots();
        if index >= slots.len() {
            return Err(format!("no antigravity slot #{index}"));
        }
        slots.remove(index);
        return antigravity_accounts::save_slots(&slots);
    }
    if provider == "cursor" {
        let mut accounts = cursor_accounts::load_accounts();
        if index >= accounts.len() {
            return Err(format!("no cursor account #{index}"));
        }
        accounts.remove(index);
        return cursor_accounts::save_accounts(&accounts);
    }
    if !accounts::provider_takes_accounts(&provider) {
        return Err(format!("unknown multi-account provider: {provider}"));
    }
    let mut entries = accounts::load_accounts(&provider);
    if index >= entries.len() {
        return Err(format!("no account #{index} for {provider}"));
    }
    // Removing index 0 promotes the next account to the bare family id —
    // clear that id's cache so the promoted account doesn't inherit the
    // deleted one's numbers on a failed fetch.
    let removed_default = index == 0;
    entries.remove(index);
    accounts::save_accounts(&provider, &entries)?;
    if removed_default {
        let _ = forget_provider_snapshot(&provider);
    }
    Ok(())
}

/// Makes the account at `index` the default: it moves to position 0 and
/// publishes under the bare family id on the next fetch (its old
/// <provider>@<fingerprint> card folds away into the main card).
#[tauri::command]
fn account_set_default(provider: String, index: usize) -> Result<(), String> {
    if provider == "antigravity" || provider == "cursor" {
        return Err("该服务的账号是平行卡片，没有置顶概念".into());
    }
    if !accounts::provider_takes_accounts(&provider) {
        return Err(format!("unknown multi-account provider: {provider}"));
    }
    let mut entries = accounts::load_accounts(&provider);
    if index >= entries.len() {
        return Err(format!("no account #{index} for {provider}"));
    }
    let entry = entries.remove(index);
    entries.insert(0, entry);
    accounts::save_accounts(&provider, &entries)?;
    // The bare id's meaning changes with the default account; the old
    // account's last-good cache must not bleed onto the new one's card
    // if the next fetch fails.
    let _ = forget_provider_snapshot(&provider);
    Ok(())
}

/// Renames the account at `index`. The label is display-only (the stable
/// fingerprint id never changes), so caches and layouts survive a rename;
/// the account card's title picks the new label up on the next fetch.
#[tauri::command]
fn account_rename(provider: String, index: usize, label: String) -> Result<(), String> {
    if provider == "antigravity" {
        let mut slots = antigravity_accounts::load_slots();
        if index >= slots.len() {
            return Err(format!("no antigravity slot #{index}"));
        }
        slots[index].label = label.trim().to_string();
        return antigravity_accounts::save_slots(&slots);
    }
    if provider == "cursor" {
        let mut accounts = cursor_accounts::load_accounts();
        if index >= accounts.len() {
            return Err(format!("no cursor account #{index}"));
        }
        accounts[index].label = label.trim().to_string();
        return cursor_accounts::save_accounts(&accounts);
    }
    if !accounts::provider_takes_accounts(&provider) {
        return Err(format!("unknown multi-account provider: {provider}"));
    }
    let mut entries = accounts::load_accounts(&provider);
    if index >= entries.len() {
        return Err(format!("no account #{index} for {provider}"));
    }
    entries[index].label = label.trim().to_string();
    accounts::save_accounts(&provider, &entries)
}

/// The saved extra accounts of a provider, for the gear panel's account
/// list. The key never leaves whole — only mask_key's "sk-…abcd" tail.
#[tauri::command]
fn account_list(provider: String) -> Result<Vec<Value>, String> {
    if provider == "onenewapi" {
        // Relay sites are the accounts: one entry per site key, or a single
        // token-only entry when the site carries just a dashboard access
        // token. Ids mirror onenewapi::key_cards_at exactly, and store
        // order is the merged card's tab order.
        let sites = providers::onenewapi::list_sites().unwrap_or_default();
        return Ok(sites
            .iter()
            .flat_map(|site| {
                let live_keys: Vec<_> = site.keys.iter().filter(|k| k.has_api_key).collect();
                if !live_keys.is_empty() {
                    let single = live_keys.len() == 1;
                    return live_keys
                        .into_iter()
                        .map(move |key| {
                            json!({
                                "id": format!("onenewapi@{}", key.id),
                                "label": if single {
                                    site.name.clone()
                                } else {
                                    format!("{} · {}", site.name, key.label)
                                },
                                "maskedKey": "sk-…",
                                "baseUrl": site.base_url,
                            })
                        })
                        .collect::<Vec<Value>>()
                }
                if site.has_access_token && !site.user_id.is_empty() {
                    return vec![json!({
                        "id": format!("onenewapi@{}", site.id),
                        "label": site.name,
                        "maskedKey": "access token",
                        "baseUrl": site.base_url,
                    })];
                }
                Vec::new()
            })
            .collect());
    }
    if provider == "antigravity" {
        // Antigravity has no API-key accounts file; its "accounts" are the
        // captured Google credential slots.
        return Ok(antigravity_accounts::load_slots()
            .iter()
            .map(|slot| {
                json!({
                    "id": antigravity_accounts::card_id_for_slot(slot),
                    "label": slot.label,
                    "maskedKey": antigravity_accounts::mask_token(&slot.refresh_token),
                    "baseUrl": null,
                })
            })
            .collect());
    }
    if provider == "cursor" {
        // Cursor accounts are imported token pairs / OAuth logins.
        return Ok(cursor_accounts::load_accounts()
            .iter()
            .map(|acct| {
                json!({
                    "id": cursor_accounts::card_id_for_account(acct),
                    "label": acct.label,
                    "email": acct.email,
                    "maskedKey": cursor_accounts::mask_token(&acct.access_token),
                    "baseUrl": null,
                })
            })
            .collect());
    }
    if !accounts::provider_takes_accounts(&provider) {
        return Err(format!("unknown multi-account provider: {provider}"));
    }
    Ok(accounts::load_accounts(&provider)
        .into_iter()
        .map(|a| {
            json!({
                "id": accounts::card_id_for_account(&provider, &a),
                "label": a.label,
                "maskedKey": accounts::mask_key(&a.api_key),
                "baseUrl": a.base_url,
            })
        })
        .collect())
}

/// Known env-var fallbacks per provider, for get_credential_status. The
/// saved-file probe is providers::stored_key_file; local-CLI detection is
/// each provider's own local_credential_hint (dispatched just below).
fn provider_env_vars(provider: &str) -> &'static [&'static str] {
    match provider {
        "openrouter" => &["OPENROUTER_API_KEY"],
        "zai" => &["ZAI_API_KEY", "GLM_API_KEY"],
        "minimax" => &["MINIMAX_API_KEY"],
        "deepseek" => &["DEEPSEEK_API_KEY"],
        "moonshot" => &["MOONSHOT_API_KEY", "KIMI_API_KEY"],
        "elevenlabs" => &["ELEVENLABS_API_KEY", "XI_API_KEY"],
        "codebuff" => &["CODEBUFF_API_KEY"],
        "kilo" => &["KILO_API_KEY"],
        "aihubmix" => &["AIHUBMIX_API_KEY"],
        "qwen" => &["BAILIAN_TOKEN_PLAN_API_KEY", "DASHSCOPE_API_KEY"],
        "kimi" => &["KIMI_CODING_API_KEY"],
        "opencode" => &["OPENCODE_GO_API_KEY"],
        "stepfun" => &["STEPFUN_API_KEY"],
        "siliconflow" => &["SILICONFLOW_API_KEY"],
        "novita" => &["NOVITA_API_KEY"],
        _ => &[],
    }
}

/// Credential probe for the Customize "?" and gear panels: is a key saved
/// in %APPDATA%\Pane\<provider>.json, is one of the provider's env vars
/// set, and which local CLI/desktop sign-in exists (a human-readable
/// description from the provider's local_credential_hint, null when none).
/// Account cards ("claude@ab12cd34") report their family's status — the
/// family owns every credential source.
#[tauri::command]
fn get_credential_status(provider: String) -> Value {
    let family = family_of(&provider);
    // Multi-account families: the account list is the key store now (the
    // old <provider>.json main key file was imported and removed), so
    // "stored key" means "the accounts list has at least one entry".
    let stored_key = if accounts::provider_takes_accounts(&family) {
        !accounts::load_accounts(&family).is_empty()
    } else {
        providers::stored_key_file(&family).is_some()
    };
    let env_key = provider_env_vars(&family)
        .iter()
        .any(|var| std::env::var(var).is_ok_and(|v| !v.trim().is_empty()));
    let mut local_cli = match family.as_str() {
        "claude" => providers::claude::local_credential_hint(),
        "codex" => providers::codex::local_credential_hint(),
        "cursor" => providers::cursor::local_credential_hint(),
        "opencode" => providers::opencode::local_credential_hint(),
        "copilot" => providers::copilot::local_credential_hint(),
        "grok" => providers::grok::local_credential_hint(),
        "devin" => providers::devin::local_credential_hint(),
        "minimax" => providers::minimax::local_credential_hint(),
        "openrouter" => providers::openrouter::local_credential_hint(),
        "zai" => providers::zai::local_credential_hint(),
        "antigravity" => providers::antigravity::local_credential_hint(),
        "deepseek" => providers::deepseek::local_credential_hint(),
        "moonshot" => providers::moonshot::local_credential_hint(),
        "elevenlabs" => providers::elevenlabs::local_credential_hint(),
        "ollama" => providers::ollama::local_credential_hint(),
        "codebuff" => providers::codebuff::local_credential_hint(),
        "kilo" => providers::kilo::local_credential_hint(),
        "aihubmix" => providers::aihubmix::local_credential_hint(),
        "qwen" => providers::qwen::local_credential_hint(),
        "hermes" => providers::hermes::local_credential_hint(),
        "kimi" => providers::kimi::local_credential_hint(),
        "stepfun" => providers::stepfun::local_credential_hint(),
        "siliconflow" => providers::siliconflow::local_credential_hint(),
        "novita" => providers::novita::local_credential_hint(),
        "relaybalance" => providers::relaybalance::local_credential_hint(),
        _ => None,
    };
    // Subscription/membership badge: Cursor reads it straight from the
    // editor's local state DB (same source cockpit badges accounts with);
    // other providers surface their plan on the quota card itself.
    let membership = match family.as_str() {
        "cursor" => providers::cursor::local_membership(),
        // Every other provider: the plan of the last good snapshot — Codex
        // reports team, Copilot reports Pro (Student), Antigravity reports
        // Google AI Pro, Grok reports X Premium/SuperGrok — so the tier is
        // visible even before the quota card paints.
        _ => cached_plan_for(&family),
    };
    // Kimi can expose an old CLI OAuth file even when the card is actually
    // using the API key supplied to Pane. Report only the source selected by
    // the same precedence used by the refresh path.
    let active_source = if family == "kimi" {
        providers::kimi::preferred_source(stored_key || env_key, local_cli.is_some())
    } else {
        None
    };
    if family == "kimi" && active_source == Some("api_key") {
        local_cli = None;
    }
    // Pane's own OAuth login (the gear panel's "Sign in with browser") is
    // a credential source separate from the CLI's sign-in; report its
    // account label so the status chips can show it.
    let oauth_label = match family.as_str() {
        "codex" | "grok" | "copilot" => oauth::label(&family),
        _ => None,
    };
    json!({ "storedKey": stored_key, "envKey": env_key, "localCli": local_cli, "oauth": oauth_label, "activeSource": active_source, "membership": membership })
}

/// Starts Pane's own OAuth device-code login (Codex / Grok). Returns the
/// user code + verification URL for the panel to show and open; the
/// frontend then polls oauth_poll.
#[tauri::command]
async fn oauth_start(provider: String) -> Result<oauth::StartResponse, String> {
    oauth::start(&provider).await
}

/// Starts a Cursor PKCE browser login (cockpit-style deep link + poll).
#[tauri::command]
fn cursor_oauth_start() -> Result<cursor_oauth::CursorOAuthStart, String> {
    cursor_oauth::start_login()
}

/// One poll tick of a pending Cursor login. `done` carries the imported
/// account once the browser login completes; error on terminal failure.
/// On success the account is deduped by token fingerprint and persisted —
/// this is the ONLY place an OAuth login becomes a stored account.
#[tauri::command]
async fn cursor_oauth_poll(login_id: String) -> cursor_oauth::CursorOAuthPoll {
    let mut poll = cursor_oauth::poll_login(&login_id).await;
    if let Some(account) = poll.account.take() {
        let id = cursor_accounts::card_id_for_account(&account);
        let mut accounts = cursor_accounts::load_accounts();
        if accounts.iter().any(|a| cursor_accounts::card_id_for_account(a) == id) {
            poll.error = Some("该 Cursor 账号已存在".into());
        } else {
            accounts.push(account);
            if let Err(error) = cursor_accounts::save_accounts(&accounts) {
                poll.error = Some(error);
            }
        }
    }
    poll
}

/// Cancels a pending Cursor login.
#[tauri::command]
fn cursor_oauth_cancel(login_id: Option<String>) {
    cursor_oauth::cancel_login(login_id.as_deref());
}

/// Imports Cursor accounts from a cockpit-compatible JSON (single object,
/// array, or {accounts:[...]}); fields accept camelCase/snake aliases.
#[tauri::command]
fn cursor_import(json_content: String) -> Result<usize, String> {
    cursor_accounts::import_from_json(&json_content).map(|accounts| accounts.len())
}

/// One poll tick of a pending device-code login. Not an Err while the
/// user is still authorizing — the `done`/`error` fields carry the state.
#[tauri::command]
async fn oauth_poll(provider: String, device_auth_id: String) -> oauth::PollResponse {
    oauth::poll(&provider, &device_auth_id).await
}

/// Deletes Pane's own OAuth credential file for the provider. The CLI's
/// sign-in is untouched.
#[tauri::command]
fn oauth_logout(provider: String) -> Result<(), String> {
    oauth::logout(&provider)
}

#[tauri::command]
fn onenewapi_list_sites() -> Result<Vec<providers::onenewapi::SiteDto>, String> {
    providers::onenewapi::list_sites()
}

#[tauri::command]
async fn onenewapi_probe_site(base_url: String) -> Result<providers::onenewapi::ProbeDto, String> {
    providers::onenewapi::probe_site(base_url).await
}

#[tauri::command]
async fn onenewapi_create_site(
    name: String,
    base_url: String,
) -> Result<providers::onenewapi::CreateSiteResult, String> {
    providers::onenewapi::create_site(name, base_url).await
}

#[tauri::command]
async fn onenewapi_update_site(
    id: String,
    name: Option<String>,
    base_url: Option<String>,
) -> Result<providers::onenewapi::SiteDto, String> {
    let previous = providers::onenewapi::list_sites()?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| "site not found".to_string())?;
    let normalized_base_url = base_url
        .as_deref()
        .map(providers::onenewapi::normalize_site_url)
        .transpose()?;
    let url_changed = normalized_base_url
        .as_deref()
        .is_some_and(|candidate| candidate != previous.base_url);
    let (verified_base_url, display) = if url_changed {
        let raw = base_url
            .as_ref()
            .ok_or_else(|| "site URL is required".to_string())?;
        let (dto, display) = providers::onenewapi::probe_site_display(raw.clone()).await?;
        (Some(dto.base_url), Some(display))
    } else {
        (normalized_base_url, None)
    };
    let key_ids = previous
        .keys
        .iter()
        .map(|key| key.id.clone())
        .collect::<Vec<_>>();
    let affected_snapshot_ids = onenewapi_snapshot_ids(&key_ids);
    let _mutation = OneNewApiMutationGuard::begin(affected_snapshot_ids);
    providers::onenewapi::update_site_consistently(id, name, verified_base_url, display, |site| {
        if url_changed {
            forget_onenewapi_key_ids(key_ids)?;
            Ok(())
        } else {
            onenewapi_after_site_save(&previous, site)
        }
    })
}

#[tauri::command]
fn onenewapi_delete_site(id: String) -> Result<(), String> {
    let key_ids = providers::onenewapi::list_sites()?
        .into_iter()
        .find(|s| s.id == id)
        .map(|s| s.keys.into_iter().map(|k| k.id).collect::<Vec<_>>())
        .ok_or_else(|| "site not found".to_string())?;
    let _mutation = OneNewApiMutationGuard::begin(onenewapi_snapshot_ids(&key_ids));
    providers::onenewapi::delete_site_consistently(id, || purge_onenewapi_cards(&key_ids))
}

#[tauri::command]
fn onenewapi_set_site_access_token(
    site_id: String,
    access_token: Option<String>,
    user_id: Option<String>,
) -> Result<providers::onenewapi::SiteDto, String> {
    let key_ids = providers::onenewapi::list_sites()?
        .into_iter()
        .find(|s| s.id == site_id)
        .map(|s| s.keys.into_iter().map(|k| k.id).collect::<Vec<_>>())
        .unwrap_or_default();
    let _mutation = OneNewApiMutationGuard::begin(onenewapi_snapshot_ids(&key_ids));
    providers::onenewapi::set_site_access_token(site_id, access_token, user_id)
}

#[cfg_attr(not(test), allow(dead_code))]
fn onenewapi_apply_zero_to_one_enable(disabled: &mut Vec<Value>, key_id: &str) {
    let snap_id = format!("onenewapi@{key_id}");
    disabled.retain(|v| match v.as_str() {
        Some("onenewapi") => false,
        Some(id) if id == snap_id => false,
        _ => true,
    });
}

#[tauri::command]
fn onenewapi_create_key(
    site_id: String,
    label: String,
    api_key: String,
) -> Result<providers::onenewapi::CreatedKey, String> {
    let _mutation = OneNewApiMutationGuard::begin(Vec::new());
    providers::onenewapi::create_key(site_id, label, api_key)
}

#[tauri::command]
fn onenewapi_update_key(
    site_id: String,
    key_id: String,
    label: Option<String>,
    api_key: Option<String>,
) -> Result<providers::onenewapi::SiteDto, String> {
    let rotated = api_key
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());
    let label_changed = label.is_some();
    let snap_id = format!("onenewapi@{key_id}");
    let _mutation = OneNewApiMutationGuard::begin(vec![snap_id.clone()]);
    providers::onenewapi::update_key_consistently(site_id, key_id.clone(), label, api_key, |site| {
        if rotated {
            forget_provider_snapshot(&snap_id)?;
        } else if label_changed {
            if let Some(key) = site.keys.iter().find(|k| k.id == key_id) {
                rename_cached_snapshot(&snap_id, format!("{} · {}", site.name, key.label))?;
            }
        }
        Ok(())
    })
}

#[tauri::command]
fn onenewapi_delete_key(
    site_id: String,
    key_id: String,
) -> Result<providers::onenewapi::SiteDto, String> {
    let _mutation = OneNewApiMutationGuard::begin(vec![format!("onenewapi@{key_id}")]);
    let cleanup_key_id = key_id.clone();
    providers::onenewapi::delete_key_consistently(site_id, key_id, || {
        purge_onenewapi_cards(&[cleanup_key_id])
    })
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

/// A share card is a few hundred KB of PNG at 2x scale; 8 MB of base64
/// (6 MB decoded) leaves generous headroom while bounding what any code
/// running in the WebView can hand us.
const MAX_SHARE_PNG_BASE64: usize = 8 * 1024 * 1024;
/// Raw RGBA is 4 bytes per pixel, so 16 M pixels caps the expansion at
/// 64 MB. Real cards are ~1200x2400 (≈3 M pixels).
const MAX_SHARE_PNG_PIXELS: u64 = 16_000_000;

/// Reads width/height out of a PNG's IHDR chunk, which is always the first
/// chunk right after the 8-byte signature. Checking the declared dimensions
/// *before* handing the bytes to a decoder is what keeps a decompression
/// bomb (tiny file, billions of pixels) from being expanded at all.
fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes.len() < 24 || bytes[..8] != SIG || &bytes[12..16] != b"IHDR" {
        return Err("not a PNG".into());
    }
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if w == 0 || h == 0 {
        return Err("empty image".into());
    }
    Ok((w, h))
}

/// Puts a share-card PNG (rendered by the frontend on a canvas) onto the
/// Windows clipboard as a real image.
///
/// Every command is callable by whatever JavaScript runs in the WebView, so
/// the encoded size and the declared pixel count are both bounded before any
/// decoding happens — otherwise a crafted PNG could force a multi-gigabyte
/// RGBA allocation and take the tray process down.
#[tauri::command]
fn copy_share_image(png_base64: String) -> Result<(), String> {
    use base64::Engine;
    let png_base64 = png_base64.trim();
    if png_base64.len() > MAX_SHARE_PNG_BASE64 {
        return Err("share image too large".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64)
        .map_err(|e| format!("decode png: {e}"))?;
    let (dw, dh) = png_dimensions(&bytes)?;
    if u64::from(dw) * u64::from(dh) > MAX_SHARE_PNG_PIXELS {
        return Err("share image too large".into());
    }
    let img = tauri::image::Image::from_bytes(&bytes).map_err(|e| format!("parse png: {e}"))?;
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w != dw as usize || h != dh as usize {
        return Err("share image dimensions mismatch".into());
    }
    let rgba = img.rgba().to_vec();
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("clipboard: {e}"))?;
    clipboard
        .set_image(arboard::ImageData {
            width: w,
            height: h,
            bytes: rgba.into(),
        })
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
            toggle_popover_centered(app);
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
fn updater_endpoint_strings(version: &str) -> [String; 2] {
    [
        format!("https://trypane.xyz/api/update?v={version}"),
        "https://github.com/ItsJazii/pane/releases/latest/download/latest.json".into(),
    ]
}

fn build_updater(app: &tauri::AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    use tauri_plugin_updater::UpdaterExt;
    let version = app.package_info().version.to_string();
    let endpoints = updater_endpoint_strings(&version)
        .into_iter()
        .map(|endpoint| endpoint.parse().map_err(|e| format!("endpoint parse: {e}")))
        .collect::<Result<Vec<_>, _>>()?;
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
        None => Err("update no longer available — try again shortly".into()),
    }
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

/// Popover entry point with no anchor: show centered on the primary monitor.
/// Used by the global shortcut and by the second-instance signal (no click
/// position available). Centering avoids the popover drifting toward the
/// taskbar / off-screen edges.
fn toggle_popover_centered(app: &tauri::AppHandle) {
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

    let size = window
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(380, 600));
    // Primary monitor = the one whose work-area origin is closest to (0, 0).
    // On a single-monitor setup this is just that monitor. With multiples
    // we still want the popover on the screen where the user is reading
    // (the taskbar monitor), which is the one most apps consider "primary".
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let mon_pos = m.position();
        let mon_size = m.size();
        let x = mon_pos.x as f64
            + ((mon_size.width as f64 - size.width as f64) / 2.0).max(0.0);
        let y = mon_pos.y as f64
            + ((mon_size.height as f64 - size.height as f64) / 2.0).max(0.0);
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("popover-shown", ());
}

/// Hide the popover from the webview (Esc key): mirrors the tray toggle's
/// hide branch so the webview drops to low memory while parked in the tray.
#[tauri::command]
fn hide_popover(app: tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.hide();
    set_webview_memory_level(&window, true);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Second launches just poke the existing instance's popover open
        // instead of spawning a duplicate tray icon (Mac parity).
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            toggle_popover_centered(app);
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
            refresh_provider,
            cached_usage,
            fetch_spend,
            fetch_usage_history,
            antigravity_capture_account,
            cursor_oauth_start,
            cursor_oauth_poll,
            cursor_oauth_cancel,
            cursor_import,
            set_api_key,
            onenewapi_list_sites,
            onenewapi_probe_site,
            onenewapi_create_site,
            onenewapi_update_site,
            onenewapi_set_site_access_token,
            onenewapi_delete_site,
            onenewapi_create_key,
            onenewapi_update_key,
            onenewapi_delete_key,
            get_base_url,
            test_api_key,
            account_add,
            account_remove,
            account_rename,
            account_set_default,
            account_list,
            get_credential_status,
            oauth_start,
            oauth_poll,
            oauth_logout,
            get_config,
            set_config,
            system_ui_locale,
            get_autostart,
            set_autostart,
            sync_tray_surfaces,
            open_link,
            copy_share_image,
            set_shortcut,
            hide_popover,
            codex_redeem_credit,
            install_update,
            check_update
        ])
        .setup(|app| {
            spawn_update_checker(app.handle());
            spawn_share_watcher(app.handle().clone());
            spawn_auto_refresh(app.handle().clone());
            let quit = MenuItem::with_id(
                app,
                "quit",
                i18n::quit_label(&config_with_defaults(load_config())),
                true,
                None::<&str>,
            )?;
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

#[cfg(test)]
mod tests {
    use super::{
        cached_extra_account_id_is_configured, cached_kimi_ok_from,
        cached_onenewapi_id_is_configured, card_is_disabled,
        commit_strip_state_after_apply, fail_state, load_config_from, set_config_in,
        fold_moonshot_into_kimi, forget_onenewapi_key_ids, forget_provider_snapshot,
        is_kimi_wallet_label, last_ok, onenewapi_after_site_save,
        onenewapi_apply_zero_to_one_enable, onenewapi_snapshot_generations, persist_last_ok_at,
        onenewapi_purge_restore_patch, purge_onenewapi_cards, purge_onenewapi_cards_coordinated,
        purge_onenewapi_cards_with, purge_onenewapi_from_config,
        rename_cached_snapshot, rename_cached_snapshot_in, rename_cached_snapshots_in,
        refresh_minutes_from, restore_kimi_wallet_rows,
        restore_last_success_after_error,
        retain_current_onenewapi_results, strip_entry_application_order, strip_icon_ids_to_clear,
        strip_is_active, strip_reset_ids, updater_endpoint_strings, CachedSnap, FailState,
        OneNewApiMutationGuard, StripEntry, SNAPSHOT_CACHE_MS, STALE_GRACE_MS,
    };
    use crate::alerts;
    use crate::providers::{Metric, Snapshot};
    use serde_json::{json, Value};
    use std::collections::{HashMap, HashSet};

    fn strip_entry(id: &str, value: u32) -> StripEntry {
        StripEntry {
            id: id.into(),
            logo: vec![0; 32 * 32 * 4],
            values: vec![value],
            tooltip: id.into(),
        }
    }

    #[test]
    fn updater_prefers_trypane_then_github() {
        assert_eq!(
            updater_endpoint_strings("0.4.46"),
            [
                "https://trypane.xyz/api/update?v=0.4.46".to_string(),
                "https://github.com/ItsJazii/pane/releases/latest/download/latest.json".to_string(),
            ]
        );
    }

    #[test]
    fn kimi_wallet_labels() {
        assert!(is_kimi_wallet_label("API"));
        assert!(is_kimi_wallet_label("Balance"));
        assert!(!is_kimi_wallet_label("Session"));
        assert!(!is_kimi_wallet_label("Weekly"));
    }

    #[test]
    fn restore_wallet_rows_when_api_missing() {
        let mut current = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![Metric::progress("Session", 0.0, None)],
        );
        current.warning = Some("Moonshot API wallet couldn't refresh".into());
        let previous = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![
                Metric::progress("Session", 10.0, None),
                Metric::progress("API", 24.0, None),
                Metric::text("Balance", "$152.00".into()),
            ],
        );
        restore_kimi_wallet_rows(&mut current, &previous);
        let labels: Vec<_> = current.metrics.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, ["Session", "API", "Balance"]);
    }

    #[test]
    fn restore_wallet_rows_skips_when_api_present() {
        let mut current = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![
                Metric::progress("Session", 0.0, None),
                Metric::progress("API", 1.0, None),
            ],
        );
        let previous = Snapshot::ok(
            "kimi",
            "Kimi Code",
            None,
            vec![Metric::progress("API", 99.0, None)],
        );
        restore_kimi_wallet_rows(&mut current, &previous);
        let api = current.metrics.iter().find(|m| m.label == "API").unwrap();
        assert!((api.used_percent.unwrap() - 1.0).abs() < 0.01);
    }

    #[test]
    fn cached_kimi_ok_ignores_stale_or_missing_entries() {
        let now = 1_800_000_000_000i64;
        let fresh = json!({"kimi": {"at": now - 60_000, "snap": {"status": "ok"}}});
        assert!(cached_kimi_ok_from(&fresh, now));
        let old = json!({"kimi": {"at": now - SNAPSHOT_CACHE_MS - 1, "snap": {"status": "ok"}}});
        assert!(!cached_kimi_ok_from(&old, now));
        let err = json!({"kimi": {"at": now, "snap": {"status": "error"}}});
        assert!(!cached_kimi_ok_from(&err, now));
        assert!(!cached_kimi_ok_from(&json!({}), now));
    }

    #[test]
    fn refresh_interval_is_re_read_and_clamped_to_one_minute() {
        assert_eq!(refresh_minutes_from(&json!({"refreshMinutes": 7})), 7);
        assert_eq!(refresh_minutes_from(&json!({"refreshMinutes": 0})), 1);
        assert_eq!(refresh_minutes_from(&json!({"refreshMinutes": "7"})), 5);
        assert_eq!(refresh_minutes_from(&json!({})), 5);
    }

    #[test]
    fn tray_strip_order_change_rebuilds_all_pairs_right_to_left() {
        let previous = vec![strip_entry("claude", 50), strip_entry("codex", 60)];
        let next = vec![strip_entry("codex", 60), strip_entry("claude", 50)];

        let reset_ids = strip_reset_ids(&previous, &next);
        let application_ids: Vec<&str> = strip_entry_application_order(&next, true)
            .into_iter()
            .map(|entry| entry.id.as_str())
            .collect();

        assert_eq!(reset_ids, vec!["claude", "codex"]);
        assert_eq!(application_ids, vec!["claude", "codex"]);
    }

    #[test]
    fn tray_strip_value_change_keeps_existing_pairs() {
        let previous = vec![strip_entry("claude", 50), strip_entry("codex", 60)];
        let next = vec![strip_entry("claude", 40), strip_entry("codex", 30)];

        assert!(strip_reset_ids(&previous, &next).is_empty());
    }

    #[test]
    fn failed_tray_strip_clear_invalidates_cache_so_retry_rebuilds() {
        let previous = vec![strip_entry("claude", 50), strip_entry("codex", 60)];
        let same_order = previous.clone();
        let reordered = vec![strip_entry("codex", 60), strip_entry("claude", 50)];
        let mut cached = previous.clone();

        let result: Result<(), String> = Err("native tray update failed".into());
        assert!(commit_strip_state_after_apply(&mut cached, &same_order, result).is_err());
        cached.clear();

        assert!(!strip_reset_ids(&cached, &same_order).is_empty());
        assert!(!strip_reset_ids(&cached, &reordered).is_empty());
    }

    #[test]
    fn successful_tray_strip_apply_commits_the_new_state() {
        let previous = vec![strip_entry("claude", 50), strip_entry("codex", 60)];
        let next = vec![strip_entry("codex", 60), strip_entry("claude", 50)];
        let mut cached = previous;

        assert!(commit_strip_state_after_apply(&mut cached, &next, Ok(())).is_ok());
        assert!(strip_reset_ids(&cached, &next).is_empty());
    }

    #[test]
    fn strip_is_inactive_when_apply_failed() {
        assert!(!strip_is_active(false, &[strip_entry("claude", 50)]));
    }

    #[test]
    fn strip_is_inactive_when_entries_are_empty() {
        assert!(!strip_is_active(true, &[]));
        assert!(!strip_is_active(false, &[]));
    }

    #[test]
    fn strip_is_active_when_apply_succeeded_with_entries() {
        assert!(strip_is_active(true, &[strip_entry("claude", 50)]));
    }

    #[test]
    fn strip_clear_ids_include_family_and_account_cards() {
        let known = vec![strip_entry("claude@work", 50)];
        let attempted = vec![strip_entry("codex", 40)];
        let ids = strip_icon_ids_to_clear(&known, &attempted);
        assert!(ids.contains(&"claude".into()));
        assert!(ids.contains(&"claude@work".into()));
        assert!(ids.contains(&"codex".into()));
    }

    #[test]
    fn recent_error_fallback_within_grace_is_not_marked_stale() {
        let previous = Snapshot::ok(
            "codex",
            "Codex",
            None,
            vec![Metric::progress("Weekly", 25.0, None)],
        );
        let mut current = Snapshot::error("codex", "Codex", "timeout".into());

        assert!(restore_last_success_after_error(
            &mut current,
            &previous,
            1_000
        ));
        assert_eq!(current.status, "ok");
        assert!(!current.stale);
        assert_eq!(current.warning, None);
        assert_eq!(current.metrics[0].used_percent, Some(25.0));
    }

    #[test]
    fn recent_error_fallback_after_grace_is_marked_stale() {
        let previous = Snapshot::ok(
            "codex",
            "Codex",
            None,
            vec![Metric::progress("Weekly", 25.0, None)],
        );
        let mut current = Snapshot::error("codex", "Codex", "timeout".into());

        assert!(restore_last_success_after_error(
            &mut current,
            &previous,
            STALE_GRACE_MS + 1,
        ));
        assert_eq!(current.status, "ok");
        assert!(current.stale);
        assert_eq!(current.warning.as_deref(), Some("timeout"));
        assert_eq!(current.metrics[0].used_percent, Some(25.0));
    }

    #[test]
    fn expired_error_fallback_is_not_restored() {
        let previous = Snapshot::ok(
            "codex",
            "Codex",
            None,
            vec![Metric::progress("Weekly", 25.0, None)],
        );
        let mut current = Snapshot::error("codex", "Codex", "timeout".into());

        assert!(!restore_last_success_after_error(
            &mut current,
            &previous,
            SNAPSHOT_CACHE_MS + 1,
        ));
        assert_eq!(current.status, "error");
        assert!(!current.stale);
    }

    #[test]
    fn fold_keeps_moonshot_when_kimi_has_no_wallet() {
        let mut all = vec![
            Snapshot::ok(
                "kimi",
                "Kimi Code",
                None,
                vec![Metric::progress("Session", 0.0, None)],
            ),
            Snapshot::ok(
                "moonshot",
                "Kimi API",
                None,
                vec![Metric::progress("Credits used", 24.0, None)],
            ),
        ];
        fold_moonshot_into_kimi(&mut all);
        assert!(all.iter().any(|s| s.id == "moonshot"));
    }

    #[test]
    fn fold_hides_moonshot_when_kimi_has_wallet_or_moonshot_is_empty() {
        let mut with_api = vec![
            Snapshot::ok(
                "kimi",
                "Kimi Code",
                None,
                vec![
                    Metric::progress("Session", 0.0, None),
                    Metric::progress("API", 24.0, None),
                ],
            ),
            Snapshot::ok(
                "moonshot",
                "Kimi API",
                None,
                vec![Metric::progress("Credits used", 24.0, None)],
            ),
        ];
        fold_moonshot_into_kimi(&mut with_api);
        assert!(!with_api.iter().any(|s| s.id == "moonshot"));

        let mut empty_moon = vec![
            Snapshot::ok(
                "kimi",
                "Kimi Code",
                None,
                vec![Metric::progress("Session", 0.0, None)],
            ),
            Snapshot::no_credentials("moonshot", "Kimi API", "paste a key"),
        ];
        fold_moonshot_into_kimi(&mut empty_moon);
        assert!(!empty_moon.iter().any(|s| s.id == "moonshot"));
    }

    #[test]
    fn card_is_disabled_onenewapi_family_gates_keys_not_claude() {
        let family = vec!["onenewapi".into()];
        assert!(card_is_disabled("onenewapi@abc", &family));
        assert!(card_is_disabled("onenewapi", &family));
        assert!(!card_is_disabled("claude@home", &family));
        let one_key = vec!["onenewapi@abc".into()];
        assert!(card_is_disabled("onenewapi@abc", &one_key));
        assert!(!card_is_disabled("onenewapi@def", &one_key));
        let claude = vec!["claude".into()];
        assert!(card_is_disabled("claude", &claude));
        assert!(!card_is_disabled("claude@home", &claude));
    }

    #[test]
    fn cached_extra_account_cards_must_still_exist_in_the_account_store() {
        let current = crate::accounts::AccountEntry {
            label: "work".into(),
            api_key: "sk-current".into(),
            base_url: None,
        };
        let id = crate::accounts::card_id_for_account("deepseek", &current);
        let configured = [id.clone()].into_iter().collect();
        assert!(cached_extra_account_id_is_configured(&id, &configured));
        assert!(!cached_extra_account_id_is_configured("deepseek@deleted", &configured));
        assert!(cached_extra_account_id_is_configured("claude@home", &configured));
    }

    #[test]
    fn onenewapi_zero_to_one_auto_enable_clears_family_and_new_key() {
        let mut disabled = vec![
            json!("onenewapi"),
            json!("onenewapi@abc"),
            json!("onenewapi@other"),
            json!("claude"),
        ];
        onenewapi_apply_zero_to_one_enable(&mut disabled, "abc");
        assert_eq!(disabled, vec![json!("onenewapi@other"), json!("claude")]);
    }

    #[test]
    fn onenewapi_zero_to_one_does_not_add_the_new_key_to_disabled() {
        let mut disabled: Vec<Value> = vec![];
        onenewapi_apply_zero_to_one_enable(&mut disabled, "abc");
        assert!(disabled.is_empty());
    }

    struct TempConfig {
        dir: std::path::PathBuf,
    }

    impl TempConfig {
        fn new() -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let dir = std::env::temp_dir().join(format!(
                "pane-config-{}-{stamp}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self { dir }
        }
    }

    impl Drop for TempConfig {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn concurrent_config_patches_keep_both_updates() {
        let tmp = TempConfig::new();
        std::fs::write(tmp.dir.join("config.json"), "{}").unwrap();
        for round in 0..8 {
            std::fs::write(tmp.dir.join("config.json"), "{}").unwrap();
            let dir_a = tmp.dir.clone();
            let dir_b = tmp.dir.clone();
            let t1 = std::thread::spawn(move || set_config_in(&dir_a, json!({ "disabled": ["claude"] })));
            let t2 = std::thread::spawn(move || set_config_in(&dir_b, json!({ "locale": "zh" })));
            t1.join()
                .expect("disabled patch thread")
                .unwrap_or_else(|e| panic!("round {round} disabled patch: {e}"));
            t2.join()
                .expect("locale patch thread")
                .unwrap_or_else(|e| panic!("round {round} locale patch: {e}"));
            let cfg = load_config_from(&tmp.dir);
            assert_eq!(
                cfg["disabled"],
                json!(["claude"]),
                "round {round} lost disabled patch"
            );
            assert_eq!(cfg["locale"], json!("zh"), "round {round} lost locale patch");
        }
    }

    #[test]
    fn onenewapi_snapshot_cache_write_failure_is_reported() {
        let root =
            std::env::temp_dir().join(format!("pane-onenewapi-cache-fail-{}", std::process::id()));
        let _ = std::fs::remove_file(&root);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::write(&root, "not a directory").unwrap();
        let result = persist_last_ok_at(&root.join("last_snapshots.json"), &HashMap::new());
        let _ = std::fs::remove_file(&root);
        assert!(
            result.is_err(),
            "cache persistence errors must reach deletion cleanup"
        );
    }

    #[test]
    fn onenewapi_cached_cards_require_a_configured_key() {
        let configured = HashSet::from(["onenewapi@keep".to_string()]);
        assert!(cached_onenewapi_id_is_configured(
            "onenewapi@keep",
            &configured
        ));
        assert!(!cached_onenewapi_id_is_configured(
            "onenewapi@deleted",
            &configured
        ));
        assert!(cached_onenewapi_id_is_configured("claude", &configured));
    }

    #[test]
    fn onenewapi_stale_refresh_results_are_discarded() {
        let expected = onenewapi_snapshot_generations(["onenewapi@old".into()]);
        let mutation = OneNewApiMutationGuard::begin(vec!["onenewapi@old".into()]);
        drop(mutation);
        let current = onenewapi_snapshot_generations(["onenewapi@old".into()]);
        let mut snapshots = vec![
            Snapshot::ok("onenewapi@old", "Old · Key 1", None, vec![]),
            Snapshot::ok("claude", "Claude", None, vec![]),
        ];
        let stale = retain_current_onenewapi_results(&mut snapshots, &expected, &current);
        assert_eq!(stale, ["onenewapi@old"]);
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, "claude");
    }

    struct SnapCacheGuard(String);

    impl SnapCacheGuard {
        fn new(id: &str) -> Self {
            Self(id.to_string())
        }
    }

    impl Drop for SnapCacheGuard {
        fn drop(&mut self) {
            fail_state().lock().unwrap().remove(&self.0);
            last_ok().lock().unwrap().remove(&self.0);
        }
    }

    #[test]
    fn forget_provider_snapshot_clears_fail_state_and_last_ok() {
        let id = "onenewapi@ticket03-forget";
        let _guard = SnapCacheGuard::new(id);
        fail_state().lock().unwrap().insert(
            id.to_string(),
            FailState {
                until_ms: i64::MAX,
                note: "benched".into(),
            },
        );
        last_ok().lock().unwrap().insert(
            id.to_string(),
            CachedSnap {
                at: 1,
                snap: Snapshot::ok(
                    id,
                    "Panel · Old",
                    None,
                    vec![Metric::text("Limit", "$10.00".into())],
                ),
            },
        );
        forget_provider_snapshot(id).unwrap();
        assert!(!fail_state().lock().unwrap().contains_key(id));
        assert!(!last_ok().lock().unwrap().contains_key(id));
    }

    #[test]
    fn rename_cached_snapshot_updates_name_only() {
        let id = "onenewapi@ticket03-rename";
        let _guard = SnapCacheGuard::new(id);
        fail_state().lock().unwrap().insert(
            id.to_string(),
            FailState {
                until_ms: i64::MAX,
                note: "benched".into(),
            },
        );
        last_ok().lock().unwrap().insert(
            id.to_string(),
            CachedSnap {
                at: 42,
                snap: Snapshot::ok(
                    id,
                    "Panel · Old",
                    None,
                    vec![Metric::text("Limit", "$10.00".into())],
                ),
            },
        );
        rename_cached_snapshot(id, "Panel · New".into()).unwrap();
        let map = last_ok().lock().unwrap();
        let entry = map.get(id).unwrap();
        assert_eq!(entry.snap.name, "Panel · New");
        assert_eq!(entry.at, 42);
        assert_eq!(entry.snap.status, "ok");
        assert_eq!(entry.snap.metrics.len(), 1);
        assert_eq!(entry.snap.metrics[0].label, "Limit");
        assert_eq!(entry.snap.metrics[0].value.as_deref(), Some("$10.00"));
        drop(map);
        assert_eq!(
            fail_state().lock().unwrap().get(id).unwrap().note,
            "benched"
        );
    }

    #[test]
    fn onenewapi_cached_rename_write_failure_keeps_old_name() {
        let id = "onenewapi@ticket03-rename-fail";
        let mut map = HashMap::from([(
            id.to_string(),
            CachedSnap {
                at: 42,
                snap: Snapshot::ok(id, "Panel · Old", None, vec![]),
            },
        )]);
        let result = rename_cached_snapshot_in(&mut map, id, "Panel · New".into(), |_| {
            Err("snapshot cache locked".into())
        });
        assert_eq!(result.unwrap_err(), "snapshot cache locked");
        assert_eq!(map.get(id).unwrap().snap.name, "Panel · Old");
    }

    #[test]
    fn onenewapi_multi_key_rename_write_failure_keeps_all_old_names() {
        let a = "onenewapi@ticket06-rename-a";
        let b = "onenewapi@ticket06-rename-b";
        let mut map = HashMap::from([
            (
                a.to_string(),
                CachedSnap {
                    at: 1,
                    snap: Snapshot::ok(a, "Old · One", None, vec![]),
                },
            ),
            (
                b.to_string(),
                CachedSnap {
                    at: 2,
                    snap: Snapshot::ok(b, "Old · Two", None, vec![]),
                },
            ),
        ]);
        let result = rename_cached_snapshots_in(
            &mut map,
            &[
                (a.to_string(), "New · One".into()),
                (b.to_string(), "New · Two".into()),
            ],
            |_| Err("snapshot cache locked".into()),
        );
        assert_eq!(result.unwrap_err(), "snapshot cache locked");
        assert_eq!(map.get(a).unwrap().snap.name, "Old · One");
        assert_eq!(map.get(b).unwrap().snap.name, "Old · Two");
        assert_eq!(map.get(a).unwrap().at, 1);
        assert_eq!(map.get(b).unwrap().at, 2);
    }

    fn seed_onenewapi_cache(key_id: &str, name: &str) -> SnapCacheGuard {
        let id = format!("onenewapi@{key_id}");
        fail_state().lock().unwrap().insert(
            id.clone(),
            FailState {
                until_ms: i64::MAX,
                note: "benched".into(),
            },
        );
        last_ok().lock().unwrap().insert(
            id.clone(),
            CachedSnap {
                at: 42,
                snap: Snapshot::ok(
                    &id,
                    name,
                    None,
                    vec![Metric::text("Limit", "$10.00".into())],
                ),
            },
        );
        SnapCacheGuard::new(&id)
    }

    fn onenewapi_site(
        id: &str,
        name: &str,
        base_url: &str,
        keys: &[(&str, &str)],
    ) -> crate::providers::onenewapi::SiteDto {
        crate::providers::onenewapi::SiteDto {
            id: id.into(),
            name: name.into(),
            base_url: base_url.into(),
            has_access_token: false,
            user_id: String::new(),
            keys: keys
                .iter()
                .map(|(kid, label)| crate::providers::onenewapi::KeyDto {
                    id: (*kid).into(),
                    label: (*label).into(),
                    has_api_key: true,
                })
                .collect(),
        }
    }

    #[test]
    fn forget_onenewapi_key_ids_clears_listed_keys_only() {
        let _a = seed_onenewapi_cache("keep-a", "Panel · A");
        let _b = seed_onenewapi_cache("drop-b", "Panel · B");
        let _c = seed_onenewapi_cache("drop-c", "Panel · C");
        forget_onenewapi_key_ids(["drop-b".into(), "drop-c".into()]).unwrap();
        assert!(last_ok().lock().unwrap().contains_key("onenewapi@keep-a"));
        assert!(fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@keep-a"));
        assert!(!last_ok().lock().unwrap().contains_key("onenewapi@drop-b"));
        assert!(!fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@drop-b"));
        assert!(!last_ok().lock().unwrap().contains_key("onenewapi@drop-c"));
        assert!(!fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@drop-c"));
    }

    #[test]
    fn onenewapi_url_change_forgets_that_sites_keys() {
        let _a = seed_onenewapi_cache("site-a1", "Panel · One");
        let _b = seed_onenewapi_cache("site-a2", "Panel · Two");
        let _other = seed_onenewapi_cache("other-1", "Other · One");
        let previous = onenewapi_site(
            "site-a",
            "Panel",
            "http://127.0.0.1:1",
            &[("site-a1", "One"), ("site-a2", "Two")],
        );
        let updated = onenewapi_site(
            "site-a",
            "Panel",
            "http://127.0.0.1:2",
            &[("site-a1", "One"), ("site-a2", "Two")],
        );
        forget_onenewapi_key_ids(updated.keys.iter().map(|key| key.id.clone())).unwrap();
        onenewapi_after_site_save(&previous, &updated).unwrap();
        assert!(!last_ok().lock().unwrap().contains_key("onenewapi@site-a1"));
        assert!(!last_ok().lock().unwrap().contains_key("onenewapi@site-a2"));
        assert!(!fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@site-a1"));
        assert!(last_ok().lock().unwrap().contains_key("onenewapi@other-1"));
        assert!(fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@other-1"));
    }

    #[test]
    fn onenewapi_name_change_renames_child_cache_without_clearing() {
        let _a = seed_onenewapi_cache("site-n1", "Old · One");
        let _b = seed_onenewapi_cache("site-n2", "Old · Two");
        let previous = onenewapi_site(
            "site-n",
            "Old",
            "http://127.0.0.1:1",
            &[("site-n1", "One"), ("site-n2", "Two")],
        );
        let updated = onenewapi_site(
            "site-n",
            "New",
            "http://127.0.0.1:1",
            &[("site-n1", "One"), ("site-n2", "Two")],
        );
        onenewapi_after_site_save(&previous, &updated).unwrap();
        let map = last_ok().lock().unwrap();
        assert_eq!(map.get("onenewapi@site-n1").unwrap().snap.name, "New · One");
        assert_eq!(map.get("onenewapi@site-n2").unwrap().snap.name, "New · Two");
        assert_eq!(map.get("onenewapi@site-n1").unwrap().at, 42);
        assert_eq!(map.get("onenewapi@site-n1").unwrap().snap.metrics.len(), 1);
        drop(map);
        assert_eq!(
            fail_state()
                .lock()
                .unwrap()
                .get("onenewapi@site-n1")
                .unwrap()
                .note,
            "benched"
        );
    }

    fn sample_card_layout() -> Value {
        json!({
            "metricOrder": ["Usage"],
            "onDemand": [],
            "hidden": [],
            "starred": ["Usage"],
            "expanded": false
        })
    }

    #[test]
    fn purge_onenewapi_from_config_drops_only_those_snapshot_ids() {
        let mut cfg = json!({
            "disabled": ["onenewapi", "onenewapi@drop", "onenewapi@keep", "aihubmix"],
            "layout": {
                "providerOrder": [
                    "aihubmix",
                    "onenewapi",
                    "onenewapi@drop",
                    "onenewapi@keep",
                    "onenewapi@other"
                ],
                "providers": {
                    "aihubmix": sample_card_layout(),
                    "onenewapi": sample_card_layout(),
                    "onenewapi@drop": sample_card_layout(),
                    "onenewapi@keep": sample_card_layout(),
                    "onenewapi@other": sample_card_layout()
                }
            },
            "pinned": {"provider": "onenewapi@drop", "label": "Usage"},
            "trayProviders": ["onenewapi@drop", "aihubmix", "onenewapi@keep"]
        });
        let patch = purge_onenewapi_from_config(&mut cfg, &["onenewapi@drop".into()]);
        assert_eq!(
            cfg["disabled"],
            json!(["onenewapi", "onenewapi@keep", "aihubmix"])
        );
        assert_eq!(
            cfg["layout"]["providerOrder"],
            json!(["aihubmix", "onenewapi", "onenewapi@keep", "onenewapi@other"])
        );
        assert!(cfg["layout"]["providers"].get("onenewapi@drop").is_none());
        assert!(cfg["layout"]["providers"].get("onenewapi@keep").is_some());
        assert!(cfg["layout"]["providers"].get("onenewapi@other").is_some());
        assert!(cfg["layout"]["providers"].get("aihubmix").is_some());
        assert!(cfg["layout"]["providers"].get("onenewapi").is_some());
        assert_eq!(cfg["pinned"], Value::Null);
        assert_eq!(cfg["trayProviders"], json!(["aihubmix", "onenewapi@keep"]));
        assert!(patch.get("disabled").is_some());
        assert!(patch.get("layout").is_some());
        assert_eq!(patch["pinned"], Value::Null);
        assert!(patch.get("trayProviders").is_some());
    }

    #[test]
    fn purge_onenewapi_from_config_keeps_family_disabled_and_unrelated_pin() {
        let mut cfg = json!({
            "disabled": ["onenewapi", "onenewapi@drop"],
            "layout": {
                "providerOrder": ["aihubmix", "onenewapi", "onenewapi@drop"],
                "providers": {
                    "aihubmix": sample_card_layout(),
                    "onenewapi@drop": sample_card_layout()
                }
            },
            "pinned": {"provider": "aihubmix", "label": "Usage"},
            "trayProviders": ["aihubmix"]
        });
        let patch = purge_onenewapi_from_config(&mut cfg, &["onenewapi@drop".into()]);
        assert_eq!(cfg["disabled"], json!(["onenewapi"]));
        assert_eq!(
            cfg["pinned"],
            json!({"provider": "aihubmix", "label": "Usage"})
        );
        assert_eq!(cfg["trayProviders"], json!(["aihubmix"]));
        assert_eq!(
            cfg["layout"]["providerOrder"],
            json!(["aihubmix", "onenewapi"])
        );
        assert!(patch.get("pinned").is_none());
        assert!(patch.get("trayProviders").is_none());
    }

    #[test]
    fn purge_onenewapi_from_config_drops_all_site_keys_keeps_other_sites() {
        let mut cfg = json!({
            "disabled": ["onenewapi@a1", "onenewapi@a2", "onenewapi@b1", "aihubmix"],
            "layout": {
                "providerOrder": [
                    "aihubmix",
                    "onenewapi@a1",
                    "onenewapi@a2",
                    "onenewapi@b1"
                ],
                "providers": {
                    "aihubmix": sample_card_layout(),
                    "onenewapi@a1": sample_card_layout(),
                    "onenewapi@a2": sample_card_layout(),
                    "onenewapi@b1": sample_card_layout()
                }
            },
            "pinned": {"provider": "onenewapi@a2", "label": "Usage"},
            "trayProviders": ["onenewapi@a1", "onenewapi@b1", "aihubmix"]
        });
        let patch =
            purge_onenewapi_from_config(&mut cfg, &["onenewapi@a1".into(), "onenewapi@a2".into()]);
        assert_eq!(cfg["disabled"], json!(["onenewapi@b1", "aihubmix"]));
        assert_eq!(
            cfg["layout"]["providerOrder"],
            json!(["aihubmix", "onenewapi@b1"])
        );
        assert!(cfg["layout"]["providers"].get("onenewapi@a1").is_none());
        assert!(cfg["layout"]["providers"].get("onenewapi@a2").is_none());
        assert!(cfg["layout"]["providers"].get("onenewapi@b1").is_some());
        assert!(cfg["layout"]["providers"].get("aihubmix").is_some());
        assert_eq!(cfg["pinned"], Value::Null);
        assert_eq!(cfg["trayProviders"], json!(["onenewapi@b1", "aihubmix"]));
        assert!(patch.get("disabled").is_some());
    }

    #[test]
    fn purge_onenewapi_cards_drops_one_key_cache_and_alerts() {
        let _keep = seed_onenewapi_cache("ticket07-keep", "Panel · Keep");
        let _drop = seed_onenewapi_cache("ticket07-drop", "Panel · Drop");
        let _other = seed_onenewapi_cache("ticket07-other", "Other · One");
        alerts::insert_state_for_test("onenewapi@ticket07-drop:Usage");
        alerts::insert_state_for_test("onenewapi@ticket07-keep:Usage");
        alerts::insert_state_for_test("onenewapi@ticket07-other:Usage");
        purge_onenewapi_cards(&["ticket07-drop".into()]).unwrap();
        assert!(last_ok()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-keep"));
        assert!(fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-keep"));
        assert!(!last_ok()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-drop"));
        assert!(!fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-drop"));
        assert!(last_ok()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-other"));
        assert!(fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-other"));
        assert!(!alerts::has_state_for_test("onenewapi@ticket07-drop:Usage"));
        assert!(alerts::has_state_for_test("onenewapi@ticket07-keep:Usage"));
        assert!(alerts::has_state_for_test("onenewapi@ticket07-other:Usage"));
        alerts::forget_snapshot("onenewapi@ticket07-keep");
        alerts::forget_snapshot("onenewapi@ticket07-other");
    }

    #[test]
    fn purge_onenewapi_cards_config_save_failure_keeps_snapshots_and_alerts() {
        let _keep = seed_onenewapi_cache("ticket07-keep-cfg", "Panel · Keep");
        let _drop = seed_onenewapi_cache("ticket07-drop-cfg", "Panel · Drop");
        alerts::insert_state_for_test("onenewapi@ticket07-drop-cfg:Usage");
        alerts::insert_state_for_test("onenewapi@ticket07-keep-cfg:Usage");
        let result = purge_onenewapi_cards_with(&["ticket07-drop-cfg".into()], |_| {
            Err("config locked".into())
        });
        assert_eq!(result.unwrap_err(), "config locked");
        assert!(last_ok()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-drop-cfg"));
        assert!(fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-drop-cfg"));
        assert!(last_ok()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-keep-cfg"));
        assert!(fail_state()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-keep-cfg"));
        assert!(alerts::has_state_for_test(
            "onenewapi@ticket07-drop-cfg:Usage"
        ));
        assert!(alerts::has_state_for_test(
            "onenewapi@ticket07-keep-cfg:Usage"
        ));
        alerts::forget_snapshot("onenewapi@ticket07-drop-cfg");
        alerts::forget_snapshot("onenewapi@ticket07-keep-cfg");
    }

    #[test]
    fn purge_restores_card_settings_when_cache_cleanup_fails() {
        let cfg = std::cell::RefCell::new(json!({
            "disabled": ["onenewapi@drop", "aihubmix"],
            "layout": {
                "providerOrder": ["onenewapi@drop", "aihubmix"],
                "providers": {
                    "onenewapi@drop": {"starred": ["Usage"]},
                    "aihubmix": {"starred": ["Usage"]}
                }
            },
            "pinned": {"provider": "onenewapi@drop", "metric": "Usage"},
            "trayProviders": ["onenewapi@drop", "aihubmix"]
        }));
        let original = cfg.borrow().clone();
        let _keep = seed_onenewapi_cache("keep", "Panel · Keep");
        let _drop = seed_onenewapi_cache("drop", "Panel · Drop");
        alerts::insert_state_for_test("onenewapi@drop:Usage");
        alerts::insert_state_for_test("onenewapi@keep:Usage");
        let result = purge_onenewapi_cards_coordinated(
            &["drop".into()],
            |ids| {
                let mut cfg = cfg.borrow_mut();
                let before = cfg.clone();
                let patch = purge_onenewapi_from_config(&mut cfg, ids);
                assert!(!patch.as_object().unwrap().is_empty());
                assert_ne!(*cfg, before);
                Ok(onenewapi_purge_restore_patch(&before, &patch))
            },
            |_| Err("cache locked".into()),
            |restore| {
                let mut cfg = cfg.borrow_mut();
                if let Some(obj) = restore.as_object() {
                    for (k, v) in obj {
                        cfg[k.clone()] = v.clone();
                    }
                }
                Ok(())
            },
        );
        assert_eq!(result.unwrap_err(), "cache locked");
        assert_eq!(*cfg.borrow(), original);
        assert!(last_ok().lock().unwrap().contains_key("onenewapi@drop"));
        assert!(last_ok().lock().unwrap().contains_key("onenewapi@keep"));
        assert!(alerts::has_state_for_test("onenewapi@drop:Usage"));
        assert!(alerts::has_state_for_test("onenewapi@keep:Usage"));
        alerts::forget_snapshot("onenewapi@drop");
        alerts::forget_snapshot("onenewapi@keep");
    }

    #[test]
    fn purge_onenewapi_cards_drops_all_site_child_cache() {
        let _a1 = seed_onenewapi_cache("ticket07-a1", "Panel · One");
        let _a2 = seed_onenewapi_cache("ticket07-a2", "Panel · Two");
        let _b1 = seed_onenewapi_cache("ticket07-b1", "Other · One");
        alerts::insert_state_for_test("onenewapi@ticket07-a1:Usage");
        alerts::insert_state_for_test("onenewapi@ticket07-a2:Usage");
        alerts::insert_state_for_test("onenewapi@ticket07-b1:Usage");
        purge_onenewapi_cards(&["ticket07-a1".into(), "ticket07-a2".into()]).unwrap();
        assert!(!last_ok()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-a1"));
        assert!(!last_ok()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-a2"));
        assert!(last_ok()
            .lock()
            .unwrap()
            .contains_key("onenewapi@ticket07-b1"));
        assert!(!alerts::has_state_for_test("onenewapi@ticket07-a1:Usage"));
        assert!(!alerts::has_state_for_test("onenewapi@ticket07-a2:Usage"));
        assert!(alerts::has_state_for_test("onenewapi@ticket07-b1:Usage"));
        alerts::forget_snapshot("onenewapi@ticket07-b1");
    }
}
