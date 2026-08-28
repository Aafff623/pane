mod alerts;
mod httpapi;
mod i18n;
mod pricing;
mod providers;
mod spend;
mod telemetry;
mod tray_projection;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

/// Last-good snapshots older than this are too misleading to show or to
/// use as a stand-in for a live Moonshot card.
const SNAPSHOT_CACHE_MS: i64 = 24 * 60 * 60 * 1000;
/// One failed cycle isn't "Outdated": vendors hiccup routinely.
const STALE_GRACE_MS: i64 = 3 * 60 * 1000;

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
    obj.entry("reduceAnimations").or_insert(json!(false));
    obj.entry("hideUsageWhileSharing").or_insert(json!(false));
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
    "reduceAnimations",
    "hideUsageWhileSharing",
    "locale",
];

fn set_config_inner(patch: Value) -> Result<Value, String> {
    let mut cfg = config_with_defaults(load_config());
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
        tray.set_tooltip(Some(&projection.tooltip))
            .map_err(|error| format!("set main tray tooltip: {error}"))?;
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
const STRIP_PROVIDER_IDS: [&str; 21] = [
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
        if clear_tray_strip_icons(app, &previous, &entries).await.is_ok() {
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
    let mut ids: Vec<String> = STRIP_PROVIDER_IDS.iter().map(|id| (*id).to_string()).collect();
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
    matches!(label, "API" | "Credits used" | "Balance" | "Vouchers" | "Cash")
}

fn restore_kimi_wallet_rows(current: &mut providers::Snapshot, previous: &providers::Snapshot) {
    if current.metrics.iter().any(|m| m.label == "API") {
        return;
    }
    for m in &previous.metrics {
        if is_kimi_wallet_label(&m.label)
            && !current.metrics.iter().any(|x| x.label == m.label)
        {
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

/// Called by the UI. Refreshes every enabled provider at the same time and
/// returns whatever each one found — data, "not signed in", or an error.
#[tauri::command]
async fn fetch_usage(
    app: tauri::AppHandle,
    disabled: Option<Vec<String>>,
) -> Vec<providers::Snapshot> {
    let cfg = config_with_defaults(load_config());
    let disabled = disabled.unwrap_or_else(|| {
        cfg.get("disabled")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default()
    });

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
        ("moonshot", Box::pin(guarded("moonshot".into(), "Kimi API".into(), providers::moonshot::snapshot()))),
        ("elevenlabs", Box::pin(guarded("elevenlabs".into(), "ElevenLabs".into(), providers::elevenlabs::snapshot()))),
        ("ollama", Box::pin(guarded("ollama".into(), "Ollama".into(), providers::ollama::snapshot()))),
        ("codebuff", Box::pin(guarded("codebuff".into(), "Codebuff".into(), providers::codebuff::snapshot()))),
        ("kilo", Box::pin(guarded("kilo".into(), "Kilo".into(), providers::kilo::snapshot()))),
        ("aihubmix", Box::pin(guarded("aihubmix".into(), "AihubMix".into(), providers::aihubmix::snapshot()))),
        ("qwen", Box::pin(guarded("qwen".into(), "Qwen Code".into(), providers::qwen::snapshot()))),
        ("hermes", Box::pin(guarded("hermes".into(), "Hermes".into(), providers::hermes::snapshot()))),
        ("kimi", Box::pin(guarded("kimi".into(), "Kimi Code".into(), providers::kimi::snapshot()))),
    ];
    // Skip the leftover Moonshot fetch only when the last Kimi card
    // actually painted — a credentials file alone is not enough (expired
    // login / network blip would otherwise hide the wallet with nothing
    // to fall back to). The post-fetch retain still drops it whenever
    // this cycle's Kimi snapshot is ok.
    let kimi_card_live = cached_kimi_ok();
    let mut futs: Vec<(String, BoxedSnap)> = base
        .into_iter()
        .filter(|(id, _)| {
            *id != "moonshot"
                || !providers::kimi::has_login()
                || disabled.iter().any(|d| d == "kimi")
                || !kimi_card_live
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
                    if cur.is_null() && !old.is_null() { old } else { cur },
                );
            }
            // Persist the PRUNED cache before the new stamp: if this
            // refresh finds nothing ok (offline launch) the on-disk cache
            // would otherwise keep the old account's entry while the stamp
            // already claims the new one, resurrecting the wrong numbers
            // next launch. Stamp last, so a failed write just re-prunes.
            let _ = std::fs::create_dir_all(providers::config_dir());
            if removed {
                if let Ok(serialized) = serde_json::to_string(&*map) {
                    let _ = std::fs::write(&cache_file, serialized);
                }
            }
            drop(map);
            let to_store = Value::Object(to_store);
            if to_store != stored {
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
            for s in all.iter_mut() {
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
                    map.insert(s.id.clone(), CachedSnap { at: now_ms, snap: s.clone() });
                    dirty = true;
                } else if s.status == "error" {
                    if let Some(previous) = map.get(&s.id) {
                        let age = now_ms - previous.at;
                        restore_last_success_after_error(s, &previous.snap, age);
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

    // One Kimi card: Session / Weekly / API. Hide the leftover Moonshot
    // wallet card whenever the plan card is actually showing.
    fold_moonshot_into_kimi(&mut all);

    httpapi::publish(&all);
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
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();

    // Same account-swap rule as the live path: if a different account
    // signed into a default home since the cache was written, that
    // family's bare-id entry belongs to the old account — never paint it,
    // not even for the seconds until the live fetch lands.
    let stored: Value = std::fs::read_to_string(providers::config_dir().join("cache_identities.json"))
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
                && !disabled.iter().any(|d| d == id)
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
            cached_usage,
            fetch_spend,
            set_api_key,
            get_config,
            set_config,
            system_ui_locale,
            get_autostart,
            set_autostart,
            sync_tray_surfaces,
            open_link,
            copy_share_image,
            set_shortcut,
            codex_redeem_credit,
            install_update,
            check_update
        ])
        .setup(|app| {
            spawn_update_checker(app.handle());
            spawn_share_watcher(app.handle().clone());
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
        cached_kimi_ok_from, commit_strip_state_after_apply, fold_moonshot_into_kimi,
        is_kimi_wallet_label, restore_kimi_wallet_rows, restore_last_success_after_error,
        strip_entry_application_order, strip_icon_ids_to_clear, strip_is_active, strip_reset_ids,
        StripEntry, SNAPSHOT_CACHE_MS, STALE_GRACE_MS,
    };
    use crate::providers::{Metric, Snapshot};
    use serde_json::json;

    fn strip_entry(id: &str, value: u32) -> StripEntry {
        StripEntry {
            id: id.into(),
            logo: vec![0; 32 * 32 * 4],
            values: vec![value],
            tooltip: id.into(),
        }
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

        assert!(restore_last_success_after_error(&mut current, &previous, 1_000));
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
}
