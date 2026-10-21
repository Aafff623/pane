mod accounts;
mod identity;
mod secretstore;
mod antigravity_accounts;
mod auth_center;
mod codex_accounts;
mod codebuddy_login;
mod codex_login;
mod cursor_accounts;
mod cursor_oauth;
mod fonts;
mod keyvault;
mod keyvault_v3;
mod search_keys;
mod skin_packs;
mod font_packs;
mod reset_wake;
mod agent_skill;
mod alerts;
mod httpapi;
mod i18n;
mod kiro_login;
mod login_accounts;
mod local_logins;
mod qoder_login;
mod trae_login;
mod zed_login;
mod windsurf_login;
mod oauth;
mod oauth_util;
mod platform;
mod pricing;
mod providers;
pub(crate) mod provider_catalog;
mod snapshot_store;
mod provider_dispatch;
mod qoder_sessions;
mod spend;
mod telemetry;
mod tray_projection;
mod usage_history;
mod usage_units;
mod spend_history;

use platform::{hide_window_border, screen_is_being_shared, set_webview_memory_level};

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use rand_core::RngCore;

use serde_json::{json, Value};
use tauri::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use provider_catalog::family_of;
use provider_dispatch::{account_snapshot, key_snapshot_for};
use snapshot_store::{CachedSnap, FailState, SNAPSHOT_CACHE_MS,
    cached_kimi_ok_from, fold_moonshot_into_kimi, retain_rate_limited_benches};

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
    // Whether this install already has a layout decides the first-run
    // detection default: an install with a layout has been set up before
    // (upgrade path — the picker must never appear), a layout-less one is
    // brand new (the picker shows once). Computed BEFORE the layout key is
    // defaulted below, so it reflects what the user's file actually held.
    let already_set_up = cfg
        .get("layout")
        .is_some_and(|l| !l.is_null());
    let obj = cfg.as_object_mut().unwrap();
    // Out-of-the-box experience: 5-min refresh, all three quota alerts on,
    // dark + compact. (Autostart defaults on in setup; tray icon defaults
    // to Auto via pinned = null.)
    obj.entry("refreshMinutes").or_insert(json!(5));
    // Fresh installs start lean: mainstream subscriptions visible, niche
    // vendors wait disabled in Customize until the user enables them.
    // Existing configs already carry their own list — entry() never
    // overwrites it.
    obj.entry("disabled")
        .or_insert(json!(provider_catalog::default_disabled_families()));
    obj.entry("pinned").or_insert(Value::Null);
    obj.entry("trayProviders").or_insert(json!([]));
    obj.entry("notifyAlmostOut").or_insert(json!(true));
    obj.entry("notifyCuttingClose").or_insert(json!(true));
    obj.entry("notifyWillRunOut").or_insert(json!(true));
    // 5-hour windows announce themselves when an hour or less remains.
    obj.entry("notifyResetSoon").or_insert(json!(true));
    obj.entry("spendTab").or_insert(json!("today"));
    obj.entry("overviewTab").or_insert(json!("5h"));
    obj.entry("overviewCategory").or_insert(json!("coding"));
    obj.entry("categoryOverrides").or_insert(json!({}));
    obj.entry("spendMetric").or_insert(json!("cost"));
    obj.entry("spendGrouping").or_insert(json!("tool"));
    // Cost view folds sub-threshold providers into "Others"; this lifts it.
    obj.entry("spendShowAll").or_insert(json!(false));
    obj.entry("overviewStyle").or_insert(json!("bars"));
    obj.entry("showUsed").or_insert(json!(false));
    obj.entry("resetExact").or_insert(json!(false));
    obj.entry("timeFormat").or_insert(json!("auto"));
    obj.entry("layout").or_insert(Value::Null);
    obj.entry("appearance").or_insert(json!("dark"));
    obj.entry("density").or_insert(json!("compact"));
    // Empty = stock UI font stack; a family name replaces only the head of it.
    obj.entry("uiFont").or_insert(json!(""));
    obj.entry("uiFontPromptVersion").or_insert(json!(""));
    obj.entry("configNoticeVersion").or_insert(json!(""));
    obj.entry("glassEffects").or_insert(json!(true));
    obj.entry("shortcut").or_insert(json!("Alt+2"));
    obj.entry("categoryShortcut").or_insert(json!("Shift+1"));
    obj.entry("localShortcuts").or_insert(json!({}));
    obj.entry("proxy")
        .or_insert(json!({ "enabled": false, "url": "" }));
    obj.entry("showTotalSpend").or_insert(json!(true));
    obj.entry("welcomeDismissed").or_insert(json!(false));
    // First-run spotlight tour: false until the user finishes or skips it
    // (or replays it from Settings). Frontend-owned; seeded here so the
    // key survives a config rewrite and the Settings row reads a real value.
    obj.entry("tourSeen").or_insert(json!(false));
    obj.entry("tourPending").or_insert(json!(false));
    // Empty = "never recorded": the frontend uses it to tell a fresh
    // install (no What's-new popup) from an update (popup with the notes).
    obj.entry("lastSeenVersion").or_insert(json!(""));
    // Telemetry defaults ON and must SAY so: without this default the
    // Settings toggle read `undefined` (rendered off) while the sender's
    // own default kept transmitting — a switch that displays off while
    // data flows is the one state a privacy control must never be in.
    obj.entry("telemetry").or_insert(json!(true));
    obj.entry("reduceAnimations").or_insert(json!(false));
    obj.entry("jumpAnimation").or_insert(json!("smooth"));
    obj.entry("hideUsageWhileSharing").or_insert(json!(false));
    obj.entry("showTrend").or_insert(json!(true));
    obj.entry("locale").or_insert(json!("auto"));
    // Dual-form: "floating" (tray popover) stays the default; "panel" is the
    // large settings panel window. silentStart is stored-only for now —
    // the popover already launches hidden, so the flag carries no extra
    // behavior until the panel form learns auto-open-at-login semantics.
    obj.entry("windowForm").or_insert(json!("floating"));
    obj.entry("silentStart").or_insert(json!(false));
    // Board cleanliness: bare "no credentials" cards fold into one collapsed
    // section instead of flooding the dashboard (issue: fresh/legacy configs
    // with an effectively empty disabled list show 50+ dead cards). Stays a
    // user toggle in Settings → General.
    obj.entry("hideUnconfigured").or_insert(json!(true));
    // First-run detection: marked once the user has walked the
    // "detected providers" picker (or dismissed it — dismissals count, the
    // defaults apply). The picker itself runs in the frontend; this key
    // only records that it is done, so it never shows twice. An install
    // that already had a layout predates the picker → starts marked done.
    obj.entry("setupPicked").or_insert(json!(already_set_up));
    // Startup splash: on by default; lastStartupBootId is the boot session
    // it last played in (null = never). Seeded here so a frontend config
    // write can never drop the gate.
    obj.entry("startupAnimation").or_insert(json!(true));
    obj.entry("lastStartupBootId").or_insert(Value::Null);
    // Overview: multi-account families the user expanded (absent = folded
    // to the main account's ring/bar plus the N/M count).
    obj.entry("overviewExpanded").or_insert(json!([]));
    // Experimental features gate (skin market lives here): off means the
    // entry points don't even render.
    obj.entry("experimentalFeatures").or_insert(json!(false));
    // Spend-card lightning tiers: null = built-in (100M/250M/500M per day);
    // an object {medium, high, max} overrides the daily thresholds.
    obj.entry("spendIconTiers").or_insert(Value::Null);
    // Category chips: off = single compact row; on = full two-row grid (3 per row, 6 max + … board).
    obj.entry("overviewCatFull").or_insert(json!(false));
    // Dashboard memory: where the card list was scrolled to when the
    // popover was last dismissed (0 = top).
    obj.entry("mainScrollTop").or_insert(json!(0));
    // Qoder CN daily benefit check-in: OFF by default — it performs a write
    // (claim) on the user's account, so it only runs when explicitly asked.
    obj.entry("qoderCheckin").or_insert(json!(false));
    // Providers the user deleted from Customize: filtered out of the
    // drawer's list entirely (never offered as re-enable rows). Removal
    // also disables them, so they vanish from dashboards/tray; the drawer
    // footer offers restore.
    obj.entry("removedProviders").or_insert(json!([]));
    // Popover window chrome: pin (always-on-top) and the position lock.
    // Locked is the default — the popover is centered on every summon and
    // cannot be dragged. Unlocked, the user drags it anywhere and the
    // position survives dismissals.
    obj.entry("popoverPin").or_insert(json!(false));
    obj.entry("popoverLock").or_insert(json!(true));
    obj.entry("popoverPos").or_insert(Value::Null);
    // Which popover surface was open when it was dismissed: the next
    // summon lands back on it instead of the card list.
    obj.entry("lastView").or_insert(json!({ "kind": "main", "id": null }));
    cfg
}

#[tauri::command]
fn system_ui_locale() -> &'static str {
    i18n::system_ui_locale()
}

/// Boot-session id for the startup-animation gate. `None` where the OS
/// cannot answer — the frontend then plays once per app launch.
#[tauri::command]
fn get_boot_id() -> Option<u64> {
    platform::boot_id()
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
    // Seeded true by config_with_defaults and read by alerts.rs; without it
    // here the settings toggle's save was dropped ("ignoring unknown key")
    // and the switch could never take effect.
    "notifyResetSoon",
    "spendMetric",
    "spendGrouping",
    "spendShowAll",
    "overviewStyle",
    "spendTab",
    "overviewTab",
    "overviewCategory",
    "categoryOverrides",
    "showUsed",
    "resetExact",
    "timeFormat",
    "layout",
    "appearance",
    "density",
    "uiFont",
    "uiFontPromptVersion",
    "configNoticeVersion",
    "glassEffects",
    "shortcut",
    "categoryShortcut",
    "localShortcuts",
    "proxy",
    "showTotalSpend",
    "welcomeDismissed",
    "tourSeen",
    "tourPending",
    "lastSeenVersion",
    "telemetry",
    "reduceAnimations",
    "jumpAnimation",
    "hideUsageWhileSharing",
    "showTrend",
    "locale",
    "windowForm",
    "silentStart",
    "hideUnconfigured",
    "setupPicked",
    "startupAnimation",
    "lastStartupBootId",
    "overviewExpanded",
    "experimentalFeatures",
    "skinId",
    "spendIconTiers",
    "overviewCatFull",
    "mainScrollTop",
    "qoderCheckin",
    "removedProviders",
    // Frontend-owned: the popover's one-shot "open the panel here" request
    // (see applyPanelIntent in main.ts). Stored so the panel window can read
    // it on boot — the two windows share nothing else.
    "panelIntent",
    // Popover window chrome (pin / position lock / dragged position) and the
    // surface to restore on the next summon.
    "popoverPin",
    "popoverLock",
    "popoverPos",
    "lastView",
];

/// Providers hidden by default on a fresh install: everything outside the
/// mainstream subscriptions (Claude / Codex / Cursor / Grok / GLM / MiniMax /
/// Antigravity / Kimi). They stay one toggle away in Customize.

static CONFIG_WRITE: Mutex<()> = Mutex::new(());
static CONFIG_TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn apply_config_patch(cfg: &mut Value, patch: &Value) {
    if let (Some(target), Some(source)) = (cfg.as_object_mut(), patch.as_object()) {
        for (k, v) in source {
            if CONFIG_KEYS.contains(&k.as_str()) {
                if k == "locale" {
                    let ok = matches!(v.as_str(), Some("auto" | "en" | "zh" | "ru"));
                    target.insert(k.clone(), if ok { v.clone() } else { json!("auto") });
                } else if k == "windowForm" {
                    let ok = matches!(v.as_str(), Some("floating" | "panel"));
                    target.insert(k.clone(), if ok { v.clone() } else { json!("floating") });
                } else if k == "skinId" {
                    let skin = match v.as_str() {
                        Some(id) if skin_packs::valid_skin_id(id) => v.clone(),
                        _ => Value::Null,
                    };
                    target.insert(k.clone(), skin);
                } else if k == "uiFont" {
                    // A font family name at most — trim and cap so a stray
                    // paste can't bloat config.json.
                    let name = v.as_str().map(str::trim).unwrap_or("");
                    let ok = name.chars().count() <= 100;
                    target.insert(k.clone(), json!(if ok { name } else { "" }));
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
    // Cross-window sync (dual-form M3): the other window's in-memory config
    // is stale the moment this write lands — tell every window to reload.
    // Empty payload: receivers pull get_config themselves and skip the event
    // when their own write just went out, so no echo loop.
    let _ = app.emit("config-updated", ());
    Ok(cfg)
}

#[tauri::command]
fn list_system_fonts() -> Vec<String> {
    fonts::system_font_families()
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
    rebuild_tray_menu(app);
}

// ---------------------------------------------------------------------------
// Tray menu (dual-form M2): open panel · show popover (check) · refresh all ·
// settings · autostart (check) · quit. Rebuilt wholesale on state changes —
// Windows set_text is fine, but a rebuild is simpler and the menu is closed
// at every rebuild point anyway (after a click, or before it opens).
// ---------------------------------------------------------------------------

/// The popover's on-screen state, treating a Win+D-iconified window as
/// hidden — the same rule toggle_popover uses.
fn main_popover_visible(app: &tauri::AppHandle) -> bool {
    app.get_webview_window("main")
        .map(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
        .unwrap_or(false)
}

fn autostart_now_enabled(app: &tauri::AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

/// The tray menu keeps a compact, read-only status list like ccSwitch. It is
fn effective_disabled_list(cfg: &Value) -> Vec<String> {
    let mut out: Vec<String> = cfg
        .get("disabled")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if let Some(removed) = cfg.get("removedProviders").and_then(Value::as_array) {
        for r in removed.iter().filter_map(Value::as_str) {
            if !out.iter().any(|d| d == r) {
                out.push(r.to_string());
            }
        }
    }
    out
}

/// built from Pane's last-good snapshot cache, so opening the native menu does
/// not wait on network requests or the WebView. A click on one of these rows
/// returns to the live floating dashboard; the full management surface lives
/// behind the separate Settings… item below.
fn cached_tray_provider_rows(cfg: &Value) -> Vec<(String, String)> {
    const MAX_ROWS: usize = 8;
    let disabled: HashSet<String> = effective_disabled_list(cfg).into_iter().collect();
    let mut rows: Vec<(String, String)> = Vec::new();
    let Ok(cache) = last_ok().lock() else {
        return rows;
    };
    for (id, cached) in cache.iter() {
        if disabled.contains(id) || cached.snap.status != "ok" {
            continue;
        }
        let metric = cached
            .snap
            .metrics
            .iter()
            .find(|m| m.kind == "progress" && m.used_percent.is_some());
        let suffix = metric
            .and_then(|m| m.used_percent)
            .map(|used| {
                format!(
                    " · {}",
                    i18n::tray_left_label(cfg, (100.0 - used.clamp(0.0, 100.0)).round() as i64)
                )
            })
            .unwrap_or_default();
        rows.push((
            format!("tray_provider:{id}"),
            format!("{}{}", cached.snap.name, suffix),
        ));
    }
    rows.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    rows.truncate(MAX_ROWS);
    rows
}

fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let cfg = config_with_defaults(load_config());
    // "Open main panel" stays on top: when every window got lost, the tray
    // is the only way back (cc-switch's hard-won lesson).
    let open_panel = MenuItem::with_id(app, "open_panel", i18n::open_panel_label(&cfg), true, None::<&str>)?;
    let show_popover = CheckMenuItem::with_id(
        app,
        "show_popover",
        i18n::show_popover_label(&cfg),
        true,
        main_popover_visible(app),
        None::<&str>,
    )?;
    let refresh_all = MenuItem::with_id(app, "refresh_all", i18n::refresh_all_label(&cfg), true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let provider_items: Vec<MenuItem<tauri::Wry>> = cached_tray_provider_rows(&cfg)
        .into_iter()
        .map(|(id, label)| MenuItem::with_id(app, id, label, true, None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
    let provider_refs: Vec<&dyn IsMenuItem<tauri::Wry>> = provider_items
        .iter()
        .map(|item| item as &dyn IsMenuItem<tauri::Wry>)
        .collect();
    let sep_providers = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "open_settings", i18n::settings_label(&cfg), true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        i18n::autostart_label(&cfg),
        true,
        autostart_now_enabled(app),
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", i18n::quit_label(&cfg), true, None::<&str>)?;
    let mut items: Vec<&dyn IsMenuItem<tauri::Wry>> = vec![
        &open_panel,
        &show_popover,
        &refresh_all,
        &sep1,
    ];
    items.extend(provider_refs);
    if !provider_items.is_empty() {
        items.push(&sep_providers);
    }
    items.extend([&settings as &dyn IsMenuItem<tauri::Wry>, &autostart, &sep2, &quit]);
    Menu::with_items(app, &items)
}

fn rebuild_tray_menu(app: &tauri::AppHandle) {
    let Some(tray) = app.tray_by_id("tray") else {
        return;
    };
    match build_tray_menu(app) {
        Ok(menu) => {
            let _ = tray.set_menu(Some(menu));
        }
        Err(e) => eprintln!("[pane] rebuild tray menu: {e}"),
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
    let mut ids: Vec<String> = provider_catalog::provider_definitions()
        .iter()
        .map(|definition| definition.family_id.to_string())
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
        if !provider_catalog::valid_card_id(&entry.id) {
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
            for definition in provider_catalog::provider_definitions() {
                let id = definition.family_id;
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
/// `rate_limited` benches survive a user-forced refresh; ordinary ones
/// must not stand between the user's click and a real fetch.
fn clear_soft_benches() {
    retain_rate_limited_benches(&mut fail_state().lock().unwrap_or_else(|error| error.into_inner()));
}

fn fail_state() -> &'static Mutex<HashMap<String, FailState>> {
    static STATE: OnceLock<Mutex<HashMap<String, FailState>>> = OnceLock::new();
    STATE.get_or_init(Default::default)
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
    accounts::write_json_atomic(path, serialized)
}

fn persist_last_ok(map: &HashMap<String, CachedSnap>) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    let cache_file = providers::config_dir().join("last_snapshots.json");
    persist_last_ok_at(&cache_file, map)
}

fn forget_provider_snapshots(ids: &[String]) -> Result<(), String> {
    let mut map = last_ok().lock().unwrap_or_else(|e| e.into_inner());
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
    let mut failures = fail_state().lock().unwrap_or_else(|e| e.into_inner());
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
    // Pane-managed Codex logins are accounts too (their own store file).
    ids.extend(
        codex_accounts::load_with_imported_single_login()
            .iter()
            .map(codex_accounts::card_id_for_account),
    );
    // Pane-managed Copilot/Grok logins (login_accounts stores, same deal).
    for family in login_accounts::LOGIN_FAMILIES {
        ids.extend(
            login_accounts::load_with_imported_single_login(family)
                .iter()
                .map(|login| login_accounts::card_id_for_account(family, login)),
        );
    }
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
    let mut map = last_ok().lock().unwrap_or_else(|e| e.into_inner());
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

// Qoder CN daily benefit check-in (default off, `qoderCheckin`): runs where
// long-wall mirroring runs — the global cycle and the manual ⟳ — so both
// paths agree; the connectivity test never claims. The config read is paid
// only by qodercn cards.
async fn apply_qoder_checkin(card_id: &str, snap: &mut providers::Snapshot) {
    if !card_id.starts_with("qodercn") {
        return;
    }
    let on = config_with_defaults(load_config())
        .get("qoderCheckin")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if on {
        providers::qodercn::maybe_checkin(card_id, snap).await;
    }
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
        let map = fail_state().lock().unwrap_or_else(|e| e.into_inner());
        map.get(id)
            .filter(|f| now < f.until_ms)
            .map(|f| f.note.clone())
    };
    if let Some(note) = benched {
        return providers::Snapshot::error(id, name, note);
    }
    let mut snap = fut.await;
    // Long-window wall mirroring: while a weekly/monthly cap stands, short
    // windows would show a green mirage. Family-gated in
    // providers::WEEK_WALL_MIRROR_FAMILIES.
    providers::mirror_week_wall(id, &mut snap);
    apply_qoder_checkin(id, &mut snap).await;
    let mut map = fail_state().lock().unwrap_or_else(|e| e.into_inner());
    if snap.status == "error" {
        let interval_ms = (refresh_minutes_from(&load_config()) as i64).saturating_mul(60_000);
        map.insert(id.to_string(), snapshot_store::bench_after_error(
            snap.error.as_deref().unwrap_or_default(), now, interval_ms,
        ));
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

/// Consecutive failed fetches per card id — the degrade threshold needs
/// two in a row, so one network blip never paints ⚠ (the cache serves
/// silently). A live result clears the entry.
fn failure_streaks() -> &'static std::sync::Mutex<std::collections::HashMap<String, u32>> {
    static STREAKS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, u32>>> =
        std::sync::OnceLock::new();
    STREAKS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
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
    let base_url = if provider_catalog::takes_base_url(family) {
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

// ── Key vault commands: thin wrappers over keyvault.rs's plain API. ────────

#[tauri::command]
fn keyvault_list() -> Result<Vec<keyvault::VaultRow>, String> {
    keyvault::list()
}

#[tauri::command]
fn keyvault_add(
    service: String,
    label: String,
    key: String,
    note: String,
) -> Result<Vec<keyvault::VaultRow>, String> {
    let rows = keyvault::add(&service, &label, &key, &note)?;
    // The vault is the primary key source for the search/MCP quota
    // providers — drop their cached snapshots so the next tick picks the
    // new key set up instead of serving the old pool for 45 more minutes.
    providers::searchquota::invalidate_service(service.trim());
    Ok(rows)
}

#[tauri::command]
fn keyvault_remove(id: String) -> Result<Vec<keyvault::VaultRow>, String> {
    // Which service the id belongs to must be read BEFORE removal.
    let service = keyvault::service_of(&id);
    let rows = keyvault::remove(&id)?;
    if let Some(service) = service {
        providers::searchquota::invalidate_service(&service);
    }
    Ok(rows)
}

#[tauri::command]
fn keyvault_copy(id: String) -> Result<String, String> {
    keyvault::copy(&id)
}

#[tauri::command]
fn keyvault_status() -> keyvault::VaultStatus {
    keyvault::status()
}

#[tauri::command]
fn keyvault_set_password(password: String) -> Result<keyvault::VaultStatus, String> {
    keyvault::set_password(&password)
}

/// (Re)set the Q&A recovery questions — the master password is verified
/// against the on-disk canary first, so a stale session cannot rewrite them.
#[tauri::command]
fn keyvault_set_recovery(
    password: String,
    questions: Vec<(String, String)>,
) -> Result<keyvault::VaultStatus, String> {
    keyvault::set_recovery(&password, &questions)
}

/// The challenge surface for the forgot-password flow: (id, question) pairs,
/// readable while locked. Answers never leave this function unsealed.
#[tauri::command]
fn keyvault_recovery_questions() -> Vec<(String, String)> {
    keyvault::recovery_questions()
}

/// Forgot-password reset: every answer must verify, then the recovered old
/// password is used internally to re-seal the vault under `new_password`.
/// Rate-limited (5 misses → 5-minute cooldown) because answers are words.
#[tauri::command]
fn keyvault_recovery_reset(
    answers: Vec<String>,
    new_password: String,
) -> Result<keyvault::VaultStatus, String> {
    keyvault::recovery_reset(&answers, &new_password)
}

/// Native "Save as…" for an exported text file (recovery-code TXT). The
/// dialog runs on a blocking thread; a cancelled dialog returns null, not
/// an error. The webview never touches the dialog plugin directly — this
/// app command is the only seam, same rule as the rest of the IPC surface.
#[tauri::command]
async fn save_text_file(
    app: tauri::AppHandle,
    contents: String,
    suggested_name: String,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        let picked = app
            .dialog()
            .file()
            .set_file_name(&suggested_name)
            .add_filter("Text", &["txt"])
            .blocking_save_file();
        let Some(picked) = picked else {
            return Ok(None);
        };
        let path = picked.into_path().map_err(|e| e.to_string())?;
        std::fs::write(&path, contents).map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(Some(path.display().to_string()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// v3 setup wizard, one shot: create the vault (unlock code + exactly three
/// questions + enrolled recovery code) in a single atomic file write. The
/// printed recovery code comes back exactly once. Only valid while no vault
/// exists at all.
/// Plan / execute the AI-agent troubleshooting-skill install. `plan` is
/// read-only and shown to the user before anything is written; `install`
/// backs up replaced files and is idempotent.
#[tauri::command]
fn agent_skill_plan() -> Result<Vec<String>, String> {
    agent_skill::plan()
}

#[tauri::command]
fn agent_skill_install() -> Result<Vec<String>, String> {
    agent_skill::install()
}

/// Skin resource packs (workshop): download/verify/install a pack zip,
/// list installed packs with their self-contained CSS, or remove one.
#[tauri::command]
async fn skin_pack_install(zip_path: String) -> Result<skin_packs::SkinPackManifest, String> {
    skin_packs::skin_pack_install(zip_path).await
}

#[tauri::command]
async fn skin_pack_download(url: String, sha256: String) -> Result<skin_packs::SkinPackManifest, String> {
    skin_packs::skin_pack_download(url, sha256).await
}

#[tauri::command]
async fn skin_pack_list() -> Result<Vec<skin_packs::InstalledPack>, String> {
    skin_packs::skin_pack_list().await
}

#[tauri::command]
async fn skin_pack_remove(id: String) -> Result<(), String> {
    skin_packs::skin_pack_remove(id).await
}

#[tauri::command]
async fn font_pack_list() -> Result<Vec<font_packs::InstalledFontPack>, String> {
    font_packs::font_pack_list().await
}

#[tauri::command]
async fn font_pack_download(app: tauri::AppHandle, url: String, sha256: String) -> Result<font_packs::FontPackManifest, String> {
    let pack = font_packs::font_pack_download(url, sha256).await?;
    let _ = app.emit("font-packs-updated", ());
    Ok(pack)
}

#[tauri::command]
async fn font_pack_install(app: tauri::AppHandle, zip_path: String) -> Result<font_packs::FontPackManifest, String> {
    let pack = font_packs::font_pack_install(zip_path).await?;
    let _ = app.emit("font-packs-updated", ());
    Ok(pack)
}

#[tauri::command]
async fn font_pack_remove(app: tauri::AppHandle, id: String) -> Result<(), String> {
    font_packs::font_pack_remove(id).await?;
    let _ = app.emit("font-packs-updated", ());
    Ok(())
}

#[tauri::command]
async fn reset_wake_settings() -> Result<reset_wake::Settings, String> {
    reset_wake::settings().await
}

#[tauri::command]
async fn reset_wake_set(app: tauri::AppHandle, settings: reset_wake::Settings, consent: bool) -> Result<(), String> {
    reset_wake::set_settings(settings, consent).await?;
    let _ = app.emit("reset-wake-settings-updated", ());
    Ok(())
}

#[tauri::command]
async fn reset_wake_status() -> Result<Vec<reset_wake::WakeStatus>, String> {
    reset_wake::statuses().await
}

/// WorkBuddy's web-session fallback (ticket 06): the Customize panel's
/// explicit enable / import / clear surface. The cookie itself never
/// travels back to the frontend — only its presence does.
#[tauri::command]
fn workbuddy_web_status() -> providers::workbuddy_web::WebStatus {
    providers::workbuddy_web::status()
}

#[tauri::command]
fn workbuddy_web_set_enabled(enabled: bool) -> Result<(), String> {
    providers::workbuddy_web::set_enabled(enabled)
}

#[tauri::command]
fn workbuddy_web_import_chrome() -> Result<u32, String> {
    providers::workbuddy_web::import_from_chrome()
}

#[tauri::command]
fn workbuddy_web_save_cookie(cookie: String) -> Result<(), String> {
    providers::workbuddy_web::save_cookie(cookie)
}

#[tauri::command]
fn workbuddy_web_clear() -> Result<(), String> {
    providers::workbuddy_web::clear()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopImport {
    label: String,
    already_existed: bool,
}

/// Import the official desktop client's CURRENT sign-in as a Pane-managed
/// account (explicit user action). Read-only on the vendor side: Pane reads
/// the plaintext auth store, upserts the account into its own login store
/// (vault-first) and never writes the vendor file. An encrypted store, a
/// missing file, or a credential that belongs to the other WorkBuddy
/// variant all fail with a readable error and change nothing.
#[tauri::command]
fn workbuddy_import_desktop(family: String) -> Result<DesktopImport, String> {
    let f = codebuddy_login::family(&family)
        .ok_or_else(|| format!("{family} has no desktop sign-in import"))?;
    if f.local_auth_file.is_none() {
        return Err(format!("{} has no desktop sign-in store", f.display_name));
    }
    let session = match providers::workbuddy_local::read_for(f) {
        providers::workbuddy_local::LocalRead::Session(session) => *session,
        providers::workbuddy_local::LocalRead::Encrypted => {
            return Err(format!(
                "{}'s desktop sign-in is encrypted at rest — Pane does not decrypt it. Sign in from the auth center instead.",
                f.display_name
            ))
        }
        providers::workbuddy_local::LocalRead::Missing => {
            return Err(format!(
                "No {} desktop sign-in found on this machine",
                f.display_name
            ))
        }
        providers::workbuddy_local::LocalRead::Unreadable(reason) => {
            return Err(format!("{} desktop sign-in could not be read: {reason}", f.display_name))
        }
    };
    if !f.matches_domain(&session.domain) {
        return Err(format!(
            "This desktop sign-in belongs to the other WorkBuddy product ({}) — not imported into {}",
            session.domain, f.display_name
        ));
    }
    let label = if !session.email.trim().is_empty() {
        session.email.clone()
    } else if !session.nickname.trim().is_empty() {
        session.nickname.clone()
    } else {
        format!("{} — desktop app", f.display_name)
    };
    let already_existed = login_accounts::load(f.family_id)
        .iter()
        .any(|account| account.account_id == session.uid);
    let mut extra = serde_json::json!({
        "uid": session.uid,
        "domain": session.domain,
        "sourceKind": "desktop",
    });
    if !session.enterprise_id.trim().is_empty() {
        extra["enterpriseId"] = serde_json::Value::String(session.enterprise_id.clone());
    }
    login_accounts::record_login(
        f.family_id,
        login_accounts::LoginAccount {
            account_id: session.uid,
            email: session.email,
            label: label.clone(),
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            id_token: String::new(),
            expires_at: session
                .expires_at_ms
                .and_then(chrono::DateTime::from_timestamp_millis)
                .map(|at| at.to_rfc3339())
                .unwrap_or_default(),
            added_at: chrono::Utc::now().timestamp(),
            extra,
        },
    )?;
    Ok(DesktopImport { label, already_existed })
}

/// 基元律动 (tokenrhythm.studio) web-session login surface. The console's
/// money endpoints reject the API key (verified 2026-10-19), so the user
/// signs in inside a Pane-hosted webview and the session persists in
/// Pane's own WebView2 profile — the cookie never travels to the frontend.
#[tauri::command]
fn tokenrhythm_login_status() -> providers::tokenrhythm::LoginStatus {
    providers::tokenrhythm::status()
}

/// Opens the sign-in window and waits for the console to report a session.
/// Async: WebviewWindowBuilder::build() deadlocks inside synchronous
/// commands on Windows (see ensure_panel_window).
#[tauri::command]
async fn tokenrhythm_login(app: tauri::AppHandle) -> Result<(), String> {
    providers::tokenrhythm::login(&app).await
}

#[tauri::command]
fn tokenrhythm_login_clear() -> Result<(), String> {
    providers::tokenrhythm::clear()
}

#[tauri::command]
fn keyvault_v3_setup(
    code: String,
    questions: Vec<(String, String)>,
) -> Result<(keyvault::VaultStatus, String), String> {
    keyvault::setup_v3(&code, &questions)
}

/// v3: enroll a fresh recovery code. The printed code is returned exactly
/// once (never stored); re-enrolling rotates the id/salt, revoking the old
/// code against the new file.
#[tauri::command]
fn keyvault_v3_enable_recovery_code(
    password: String,
) -> Result<(keyvault::VaultStatus, String), String> {
    keyvault::enable_recovery_code(&password)
}

#[tauri::command]
fn keyvault_v3_disable_recovery_code(password: String) -> Result<keyvault::VaultStatus, String> {
    keyvault::disable_recovery_code(&password)
}

/// v3 forgot-code reset. Exactly one factor: `answers` are (question_id,
/// answer) pairs — any two of three suffice — or `recovery_code` (typed or
/// lifted from the exported TXT; the same normalization applies). The DEK
/// never changes, so entries/questions stay byte-identical. Rate-limited
/// like the v2 flow.
#[tauri::command]
fn keyvault_v3_reset_unlock_code(
    answers: Option<Vec<(String, String)>>,
    recovery_code: Option<String>,
    new_code: String,
) -> Result<keyvault::VaultStatus, String> {
    if answers.is_none() == recovery_code.is_none() {
        return Err("provide exactly one recovery factor (answers or recovery code)".into());
    }
    keyvault::reset_unlock_code_v3(
        answers.as_deref().unwrap_or(&[]),
        recovery_code.as_deref(),
        &new_code,
    )
}

#[tauri::command]
fn keyvault_unlock(password: String) -> Result<keyvault::VaultStatus, String> {
    let status = keyvault::unlock(&password)?;
    // Newly readable keys change what the MCP/search cards can query.
    providers::searchquota::invalidate_all();
    Ok(status)
}

#[tauri::command]
fn keyvault_lock() -> keyvault::VaultStatus {
    let status = keyvault::lock();
    providers::searchquota::invalidate_all();
    status
}

#[tauri::command]
fn keyvault_reveal(id: String) -> Result<String, String> {
    keyvault::reveal(&id)
}

#[tauri::command]
fn keyvault_set_note(id: String, note: String) -> Result<Vec<keyvault::VaultRow>, String> {
    keyvault::set_note(&id, &note)
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
    clear_benches: Option<bool>,
    probe_all: Option<bool>,
) -> Vec<providers::Snapshot> {
    // Only an explicit user click (Refresh button, Ctrl+R, the overview ⟳)
    // may knock again on ordinary-error benches; timer and refocus passes
    // stay bench-respecting so a failing provider isn't re-probed on every
    // focus. 429 cooldowns survive either way.
    if clear_benches.unwrap_or(false) {
        clear_soft_benches();
    }
    let snapshots = run_usage_fetch_inner_mode(&app, disabled, probe_all.unwrap_or(false)).await;
    let _ = app.emit("usage-updated", &snapshots);
    snapshots
}

/// Refreshes ONE provider card on demand (the ⟳ button in the card head).
/// Dispatches to the same snapshot function the full refresh cycle uses,
/// but skips alert checking, telemetry, and the snapshot cache — those
/// belong to the global cycle. The returned snapshot is merged into the
/// frontend's `lastSnapshots` by the caller.
#[tauri::command]
async fn refresh_provider(provider_id: String) -> Result<providers::Snapshot, String> {
    let mut snap = fetch_provider_snapshot(provider_id.clone(), false).await?;
    // Same long-wall mirroring the global cycle applies in guarded() — a
    // manual ⟳ must not paint the green mirage back over the wall.
    providers::mirror_week_wall(&provider_id, &mut snap);
    apply_qoder_checkin(&provider_id, &mut snap).await;
    Ok(snap)
}

/// Connectivity test from the card menu: one real authenticated request with
/// the stored credential, timed. Disabled cards are allowed — the point is
/// testing a key BEFORE enabling the provider. For vendors whose quota
/// window only starts ticking on the first request (GLM 5h, Doubao), this
/// is also what opens the clock without burning a chat turn by hand.
/// Returns "ok:<ms>" or "<error>:<ms>" so the frontend can localize.
#[tauri::command]
async fn test_provider(provider_id: String) -> Result<String, String> {
    let started = std::time::Instant::now();
    let snap = fetch_provider_snapshot(provider_id, true).await?;
    let ms = started.elapsed().as_millis();
    Ok(match snap.status.as_str() {
        "ok" => format!("ok:{ms}"),
        // A credential that connects but cannot read quota is not a failed
        // test — the Customize panel must say what is missing instead of
        // painting it red (user rule 2026-10-15).
        "needs_setup" => format!(
            "needs_setup:{}:{ms}",
            snap.error.unwrap_or_else(|| "one more step needed".into())
        ),
        _ => format!("{}:{ms}", snap.error.unwrap_or_else(|| "unknown error".into())),
    })
}

async fn fetch_provider_snapshot(provider_id: String, allow_disabled: bool) -> Result<providers::Snapshot, String> {
    let family = family_of(&provider_id);
    let cfg = config_with_defaults(load_config());
    let disabled = effective_disabled_list(&cfg);
    if !allow_disabled
        && disabled.iter().any(|d| d == &family)
        && family_of(&provider_id) != "onenewapi"
    {
        return Err(format!("{family} is disabled"));
    }

    // The bare card is accounts[0] whenever this family has pasted accounts.
    if provider_id == family && provider_catalog::provider_definition(&family)
        .is_some_and(|p| p.supports_api_key && p.supports_extra_accounts) {
        if let Some(entry) = accounts_with_imported_main_key(&family).first().cloned() {
            let identity = accounts::card_id_for_account(&family, &entry);
            let snap = account_snapshot(family.clone(), family.clone(), family.clone(),
                entry.api_key, entry.base_url).await;
            if accounts_with_imported_main_key(&family).first()
                .is_none_or(|current| accounts::card_id_for_account(&family, current) != identity) {
                return Err("account changed while the query was running; retry".into());
            }
            return Ok(snap);
        }
    }
    // No pasted default → the native provider path.
    if provider_id == family {
        let snap = provider_dispatch::native_snapshot(&family)
            .ok_or_else(|| format!("no single-provider refresh for {family}"))?.await;
        if provider_catalog::provider_definition(&family)
            .is_some_and(|p| p.supports_api_key && p.supports_extra_accounts)
            && !accounts_with_imported_main_key(&family).is_empty() {
            return Err("account changed while the query was running; retry".into());
        }
        return Ok(snap);
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
        if let Some(account) = providers::codex::discover_extra_accounts()
            .into_iter()
            .find(|a| a.id == provider_id)
        {
            return Ok(
                providers::codex::snapshot_at(account.dir, provider_id.clone(), account.name).await,
            );
        }
        // Pane-managed logins (auth center) mint their codex@ cards from the
        // account store; discovery only covers CLI homes.
        let login = codex_accounts::load_with_imported_single_login()
            .into_iter()
            .find(|l| codex_accounts::card_id_for_account(l) == provider_id)
            .ok_or_else(|| format!("no codex account {provider_id}"))?;
        return Ok(providers::codex::snapshot_with_login(login).await);
    }

    // Pane-managed Copilot/Grok logins (auth center): one card per stored
    // account, fetched under its own identity.
    if login_accounts::takes_login_accounts(&family) {
        let login = login_accounts::load_with_imported_single_login(&family)
            .into_iter()
            .find(|l| login_accounts::card_id_for_account(&family, &l) == provider_id)
            .ok_or_else(|| format!("no {family} account {provider_id}"))?;
        return Ok(match family.as_str() {
            "copilot" => providers::copilot::snapshot_with_login(login).await,
            "kiro" => providers::kiro::snapshot_with_login(login).await,
            "qoder" => providers::qoder::snapshot_with_login(login).await,
            // Every CodeBuddy-family member is one row in the family table.
            fam if codebuddy_login::family(fam).is_some() => {
                let f = codebuddy_login::known_family(fam);
                providers::codebuddy::snapshot_with_login(f, login).await
            }
            "windsurf" => providers::windsurf::snapshot_with_login(login).await,
            "trae" => providers::trae::snapshot_with_login(login).await,
            "zed" => providers::zed::snapshot_with_login(login).await,
            _ => providers::grok::snapshot_with_login(login).await,
        });
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
        let snap = account_snapshot(
            family.clone(),
            provider_id.clone(),
            name,
            acct.api_key.clone(),
            acct.base_url.clone(),
        )
        .await;
        if accounts::snapshot_identities(&family, &accounts::load_accounts(&family))
            .get(&provider_id) != Some(&provider_id) {
            return Err("account changed while the query was running; retry".into());
        }
        return Ok(snap);
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
    run_usage_fetch_inner(app, None).await
}

async fn run_usage_fetch_inner(app: &tauri::AppHandle, extra_disabled: Option<Vec<String>>) -> Vec<providers::Snapshot> {
    run_usage_fetch_inner_mode(app, extra_disabled, false).await
}

/// `probe_all` ignores the config's own disabled list and probes every
/// provider that is not in `extra_disabled`. It exists for the first-run
/// detection pass, which must see providers the static default-disabled
/// list would otherwise skip before anything is spawned (that list is a
/// convenience for untouched installs, never a detection gate).
async fn run_usage_fetch_inner_mode(
    app: &tauri::AppHandle,
    extra_disabled: Option<Vec<String>>,
    probe_all: bool,
) -> Vec<providers::Snapshot> {
    let _guard = usage_fetch_lock().lock().await;
    let cfg = config_with_defaults(load_config());
    let mut disabled: Vec<String> = if probe_all {
        Vec::new()
    } else {
        effective_disabled_list(&cfg)
    };
    if let Some(extra) = extra_disabled {
        for e in extra {
            if !disabled.iter().any(|d| d == &e) {
                disabled.push(e);
            }
        }
    }

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
    let base: Vec<(&str, BoxedSnap)> = provider_dispatch::registrations().map(|definition| {
        let future = provider_dispatch::native_snapshot(definition.family_id)
            .expect("registered provider has a snapshot adapter");
        (definition.family_id, Box::pin(guarded(
            definition.family_id.into(), definition.display_name.into(), future,
        )) as BoxedSnap)
    }).collect();
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
    let mut expected_key_identities: HashMap<String, String> = extra_accounts.iter()
        .flat_map(|(family, entries)| accounts::snapshot_identities(family, entries)).collect();
    for definition in provider_catalog::provider_definitions().iter()
        .filter(|p| p.supports_api_key && p.supports_extra_accounts) {
        expected_key_identities.entry(definition.family_id.into()).or_default();
    }
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
    // Pane-managed Codex logins (auth center): one card per stored account,
    // deduped against the CLI homes above and the default login — the same
    // ChatGPT account must never double-card.
    let mut codex_known: HashSet<String> = providers::codex::discover_extra_accounts()
        .into_iter()
        .map(|a| a.id)
        .collect();
    if let Some(default) = providers::codex::default_identity() {
        codex_known.insert(codex_accounts::card_id_for_account_id(&default));
    }
    for login in codex_accounts::load_with_imported_single_login() {
        let id = codex_accounts::card_id_for_account(&login);
        if !codex_known.insert(id.clone()) {
            continue;
        }
        let name = providers::codex::login_card_name(&login);
        futs.push((
            id.clone(),
            Box::pin(guarded(id.clone(), name, providers::codex::snapshot_with_login(login))),
        ));
    }
    // Pane-managed Copilot/Grok/Kiro logins (auth center): one card per
    // stored account. Dedup rules per family:
    // - the single OAuth slot renders as the BARE family card whenever it
    //   wins the default card's credential order, so a store entry that IS
    //   the slot's account (same identity, or same token for legacy slots
    //   that carry no account id) never double-cards;
    // - grok's CLI auth.json names its account uuid, and kiro's local
    //   database its profileArn, so a login for THAT account is skipped
    //   too (the bare card already renders it).
    for family in login_accounts::LOGIN_FAMILIES {
        let slot = oauth::load(family).map(|t| {
            (
                t.account_id.clone().unwrap_or_default(),
                t.access_token.clone(),
            )
        });
        let mut known: HashSet<String> = match family {
            "grok" => providers::grok::default_identity().into_iter().collect(),
            "kiro" => providers::kiro::default_identity().into_iter().collect(),
            "qoder" => providers::qoder::default_identity().await.into_iter().collect(),
            _ => HashSet::new(),
        };
        for login in login_accounts::load_with_imported_single_login(family) {
            if let Some((slot_acct, slot_tok)) = &slot {
                let same_as_slot = (!slot_acct.is_empty() && slot_acct == &login.account_id)
                    || slot_tok == &login.access_token;
                if same_as_slot {
                    continue;
                }
            }
            if !known.insert(login.account_id.clone()) {
                continue;
            }
            let id = login_accounts::card_id_for_account(family, &login);
            let (name, fut): (
                String,
                std::pin::Pin<Box<dyn std::future::Future<Output = providers::Snapshot> + Send>>,
            ) = match family {
                "copilot" => (
                    providers::copilot::login_card_name(&login),
                    Box::pin(providers::copilot::snapshot_with_login(login)),
                ),
                "kiro" => (
                    providers::kiro::login_card_name(&login),
                    Box::pin(providers::kiro::snapshot_with_login(login)),
                ),
                "qoder" => (
                    providers::qoder::login_card_name(&login),
                    Box::pin(providers::qoder::snapshot_with_login(login)),
                ),
                // CodeBuddy family: the table owns host, platform and name.
                fam if codebuddy_login::family(fam).is_some() => {
                    let f = codebuddy_login::known_family(fam);
                    (
                        providers::codebuddy::login_card_name(f, &login),
                        Box::pin(providers::codebuddy::snapshot_with_login(f, login)),
                    )
                }
                "windsurf" => (
                    providers::windsurf::login_card_name(&login),
                    Box::pin(providers::windsurf::snapshot_with_login(login)),
                ),
                "trae" => (
                    providers::trae::login_card_name(&login),
                    Box::pin(providers::trae::snapshot_with_login(login)),
                ),
                "zed" => (
                    providers::zed::login_card_name(&login),
                    Box::pin(providers::zed::snapshot_with_login(login)),
                ),
                _ => (
                    providers::grok::login_card_name(&login),
                    Box::pin(providers::grok::snapshot_with_login(login)),
                ),
            };
            futs.push((id.clone(), Box::pin(guarded(id.clone(), name, fut))));
        }
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
    retain_current_key_accounts(&mut all, &expected_key_identities);
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
        let mut failures = fail_state().lock().unwrap_or_else(|e| e.into_inner());
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
            let mut map = cache.lock().unwrap_or_else(|e| e.into_inner());
            let _ = snapshot_store::reconcile_identities(
                &mut map, &stored, &current, persist_last_ok, |stamp| {
                    accounts::write_json_atomic(
                        &stamp_file,
                        serde_json::to_string_pretty(stamp).map_err(|error| error.to_string())?,
                    )
                },
            );
        }
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Ok(mut map) = cache.lock() {
            retain_current_key_accounts(&mut all, &expected_key_identities);
            let mut dirty = false;
            let mut credit_samples = Vec::new();
            let mut daily_samples = Vec::new();
            let mut failures = failure_streaks().lock().unwrap_or_else(|error| error.into_inner());
            for s in all.iter_mut() {
                if family_of(&s.id) == "onenewapi" {
                    let current = onenewapi_snapshot_generations([s.id.clone()]);
                    if expected_onenewapi_generations.get(&s.id) != current.get(&s.id) {
                        continue;
                    }
                }
                let (changed, credits, daily) = snapshot_store::merge_snapshot(
                    s, &mut map, &mut failures, now_ms,
                );
                dirty |= changed;
                credit_samples.extend(credits.into_iter().map(|reading| (s.id.clone(), reading)));
                daily_samples.extend(daily.into_iter().map(|days| (s.id.clone(), days)));
            }
            if dirty {
                if let Err(error) = persist_last_ok(&map) {
                    eprintln!("[pane] snapshot cache refresh: {error}");
                }
                usage_history::record_meter_samples(&credit_samples);
                usage_history::record_daily_samples(&daily_samples);
            }
        }
    }

    retain_current_key_accounts(&mut all, &expected_key_identities);
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
        let mut failures = fail_state().lock().unwrap_or_else(|e| e.into_inner());
        for id in stale_onenewapi_ids {
            failures.remove(&id);
        }
    }

    // One Kimi card: Session / Weekly / API. Hide the leftover Moonshot
    // wallet card whenever the plan card is actually showing.
    fold_moonshot_into_kimi(&mut all);

    if let Ok(mut latest) = wake_snapshots().lock() {
        *latest = all.clone();
    }
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
                                    // Qoder CN package rows carry vendor
                                    // free text as their label; the telemetry
                                    // contract is stable IDs only, so they
                                    // fold into one bucket.
                                    .map(|m| {
                                        if family_of(pid) == "qodercn" && m != "Credits" {
                                            "qodercn/<package>".to_string()
                                        } else {
                                            format!("{}/{m}", family_of(pid))
                                        }
                                    })
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

/// One full refresh pass without a webview: fetch every provider, sync the
/// main tray numbers, and emit "usage-updated" so any open window can adopt
/// the fresh snapshots. Shared by the background loop and the tray menu's
/// "Refresh all quotas".
async fn fetch_and_broadcast_usage(app: &tauri::AppHandle) -> Vec<providers::Snapshot> {
    let snapshots = run_usage_fetch(app).await;
    // The tray strip's provider logos are rasterized by the webview,
    // so the background loop only refreshes the main tray icon and
    // treats the last applied strip as the strip state.
    let cfg = config_with_defaults(load_config());
    if let Some(projection_cfg) = tray_projection_config_from(&cfg) {
        let strip_active = last_strip().lock().map(|s| !s.is_empty()).unwrap_or(false);
        let projection =
            tray_projection::project_main_tray(&snapshots, &projection_cfg, strip_active);
        if let Err(error) = apply_main_tray_projection(app, &projection) {
            eprintln!("[pane] background tray sync: {error}");
        }
    }
    // Rebuild outside the native right-click event. Replacing a Win32 menu
    // while it is opening causes the white flash / immediate dismissal seen
    // when `set_menu` runs from MouseButtonState::Down.
    rebuild_tray_menu(&app);
    let _ = app.emit("usage-updated", &snapshots);
    snapshots
}

/// Auto-refresh that does not depend on the webview: fetches first (an
/// immediate live pass at launch — the webview's startup refresh dies with
/// the network at boot), then sleeps refreshMinutes (re-read every cycle,
/// so a Settings change applies without a restart).
fn wake_snapshots() -> &'static Mutex<Vec<providers::Snapshot>> {
    static LATEST: OnceLock<Mutex<Vec<providers::Snapshot>>> = OnceLock::new();
    LATEST.get_or_init(|| Mutex::new(Vec::new()))
}

fn spawn_reset_wake(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let snapshots = wake_snapshots().lock().map(|latest| latest.clone()).unwrap_or_default();
            if let Ok(settings) = reset_wake::settings().await {
                match reset_wake::tick(&settings, &snapshots).await {
                    Ok(statuses) => { let _ = app.emit("reset-wake-updated", statuses); }
                    Err(error) => eprintln!("[pane] automatic wake: {error}"),
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}

fn spawn_auto_refresh(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut outage_retries = 0u32;
        loop {
            let snapshots = fetch_and_broadcast_usage(&app).await;

            // Nothing came back live (boot with the network still coming up,
            // Wi-Fi down, …): the failed pass just benched every provider,
            // so retry soon instead of idling a full interval. Bounded, and
            // rate-limit cooldowns survive so a 429 is never hammered.
            let got_live = snapshots
                .iter()
                .any(|s| s.status == "ok" && !s.stale);
            let minutes = refresh_minutes_from(&config_with_defaults(load_config()));
            if !snapshots.is_empty() && !got_live && outage_retries < 5 {
                outage_retries += 1;
                clear_soft_benches();
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            } else {
                outage_retries = 0;
                // ±10% jitter on the cadence so a fleet of providers never
                // lines up against one rate-limit window on every tick.
                let jitter = 90 + (rand_core::OsRng.next_u64() % 21) as u64;
                let seconds = (minutes as u64).saturating_mul(60).saturating_mul(jitter) / 100;
                tokio::time::sleep(std::time::Duration::from_secs(seconds.max(60))).await;
            }
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
    let disabled: Vec<String> = effective_disabled_list(&cfg);
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
    let current = json!({
        "claude": providers::claude::default_identity(),
        "codex": providers::codex::default_identity(),
    });

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
                && !snapshot_store::identity_changed(&stored, &current, id)
        })
        .map(|(_, c)| {
            let mut s = c.snap;
            s.stale = true;
            // A warm start is a preheat, not a fault: the neutral
            // "cached · refreshing" chip replaces the failure-driven ⚠.
            s.boot_cache = true;
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
    let cfg = config_with_defaults(load_config());
    let cursor_disabled = effective_disabled_list(&cfg).iter().any(|v| v == "cursor");
    let cursor_csv = if cursor_disabled {
        None
    } else {
        providers::cursor::fetch_usage_csv().await
    };
    let result = tauri::async_runtime::spawn_blocking(move || spend::collect_daily(cursor_csv))
        .await
        .unwrap_or_default();
    if let Err(err) = spend_history::merge_daily(&result.1) {
        eprintln!("[pane] spend history: {err}");
    }
    let result = result.0;
    eprintln!(
        "[pane] spend: {} providers in {:?}",
        result.len(),
        started.elapsed()
    );
    result
}

/// Returns the local token-ledger inventory without scanning or contacting
/// providers. Used by diagnostics and future coverage UI.
#[tauri::command]
fn fetch_spend_sources() -> Vec<spend::SpendSourceStatus> {
    spend::source_statuses()
}

/// Daily credit consumption per card id. Qoder CN / Trae CN bill in credits,
/// not tokens — this series is theirs alone and never feeds token or
/// dollar totals.
#[tauri::command]
fn fetch_credit_history() -> std::collections::BTreeMap<String, Vec<Option<f64>>> {
    usage_history::credit_trend_map()
}

#[tauri::command]
fn fetch_meter_history() -> Vec<usage_history::MeterTrend> {
    usage_history::meter_trends()
}

/// Long-range spend from permanent daily rollups. `None` means all retained history.
#[tauri::command]
fn fetch_spend_history(range_days: Option<u32>) -> Result<Vec<spend_history::RangeSpend>, String> {
    spend_history::range_spend(range_days)
}

/// Credits-denominated providers (Qoder CN / Trae CN): credits cannot be
/// converted into tokens or dollars honestly, so the spend panel lists them
/// separately instead of blending them into the totals.
#[tauri::command]
fn fetch_credit_extras() -> Vec<usage_history::CreditExtra> {
    usage_history::credit_extras()
}

/// Daily provider/model rows for the spend detail heatmap. Read-only.
#[tauri::command]
fn fetch_spend_daily(range_days: Option<u32>) -> Result<Vec<spend_history::DailySpendRow>, String> {
    spend_history::daily_spend(range_days)
}

/// Qoder CN local transcripts (GitHub #28): measured credits plus derived
/// billed input tokens. Kept out of Total Spend and the dollar donut —
/// credits are a separate unit and the derived tokens are not measured.
#[tauri::command]
async fn fetch_qoder_spend() -> qoder_sessions::QoderSpend {
    tauri::async_runtime::spawn_blocking(qoder_sessions::collect)
        .await
        .unwrap_or_default()
}

/// Validates a relay base URL for a provider that takes one; the rejection
/// message names the provider's own card, never the other relay family.
fn validate_relay_base_url(provider: &str, url: &str) -> Result<(), String> {
    let name = provider_catalog::provider_definition(provider)
        .map(|definition| definition.display_name)
        .unwrap_or("relay");
    providers::relaybalance::validate_base_url_for(name, url)
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
        // Deleting a key deletes both the plaintext fallback and the vault copy.
        let _ = std::fs::remove_file(&path);
        secretstore::remove(&secretstore::provider_key(&provider));
        return Ok(());
    }
    // Validate the optional relay URL before any write.
    let base_url = base_url.as_deref().map(str::trim).filter(|u| !u.is_empty());
    if let Some(url) = base_url {
        if !provider_catalog::takes_base_url(&provider) {
            return Err("this provider does not take a base URL".into());
        }
        validate_relay_base_url(&provider, url)?;
    }
    // Keys live in the OS credential vault; the file carries only non-secret
    // fields. A failed vault write falls back to the plaintext file — the
    // exact behavior from before the migration, so nothing can be lost.
    let vaulted = secretstore::put(&secretstore::provider_key(&provider), key).is_ok();
    let mut doc = serde_json::json!({});
    if !vaulted {
        doc["apiKey"] = serde_json::Value::from(key);
    }
    if let Some(url) = base_url {
        doc["baseUrl"] = serde_json::Value::from(url);
    }
    if doc.as_object().is_some_and(|o| o.is_empty()) {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    std::fs::write(&path, doc.to_string()).map_err(|e| format!("write key file: {e}"))
}

/// The reveal gate's "set a password first" spelling — the frontend matches
/// this to route the user into the vault setup instead of showing an error.
const NO_MASTER_PASSWORD: &str = "no master password is set";

/// Shared prologue for the reveal commands: no password → guidance error;
/// unlocked session → pass; otherwise verify and open the session. One
/// verification covers the vault and every provider/account reveal until a
/// lock.
fn reveal_gate(password: &str) -> Result<(), String> {
    if !keyvault::status().has_password {
        return Err(NO_MASTER_PASSWORD.into());
    }
    if keyvault::session_open() {
        return Ok(());
    }
    keyvault::verify_for_reveal(password)
}

/// Reveal a provider's saved API key after master-password verification.
#[tauri::command]
fn reveal_provider_key(provider: String, password: String) -> Result<String, String> {
    let family = family_of(&provider);
    reveal_gate(&password)?;
    providers::stored_key_file(&family)
        .ok_or_else(|| "no saved key for this provider".to_string())
}

/// Reveal one account key (addressed by its stable card id) after
/// master-password verification.
#[tauri::command]
fn reveal_account_key(provider: String, id: String, password: String) -> Result<String, String> {
    let family = family_of(&provider);
    reveal_gate(&password)?;
    if let Some(key) = accounts::load_accounts(&family)
        .into_iter()
        .find(|a| accounts::card_id_for_account(&family, a) == id)
        .map(|a| a.api_key)
        .filter(|k| !k.trim().is_empty())
    {
        return Ok(key);
    }
    let base = crate::providers::config_dir();
    accounts::load_archived_from(&base)
        .into_iter()
        .find(|a| a.provider == family && a.card_id == id)
        .map(|a| a.api_key)
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| "no saved key for this account".to_string())
}

/// Change the master password: verify the old one first, then re-seal the
/// whole vault under the new one.
#[tauri::command]
fn change_master_password(old: String, new: String) -> Result<keyvault::VaultStatus, String> {
    keyvault::change_password(&old, &new)
}

/// The base URL saved alongside a provider's API key (Custom Balance's and
/// Linkso's user-chosen relay host), so Settings can pre-fill its input.
#[tauri::command]
fn get_base_url(provider: String) -> Option<String> {
    if provider_catalog::takes_base_url(&provider) {
        providers::stored_base_url(&provider)
    } else {
        None
    }
}

/// One family's live snapshot for a bare key — the shared probe behind
/// the Settings test-before-save button and the generic extra-account
/// fetch (which re-brands the result with the account's card id/name).
/// Outcome of a live test_api_key probe, shown in the Customize ⚙ panel:
/// the metric count on success, the backend's error verbatim on failure.
#[derive(serde::Serialize)]
struct TestResult {
    ok: bool,
    metrics: usize,
    message: String,
    snapshot: providers::Snapshot,
}

/// Live test of a pasted API key against its provider (Customize "Test
/// connection"). Pure probe — nothing is written, the key never touches
/// disk. Custom Balance and Linkso additionally need the relay's base URL;
/// testing always uses the pasted values, never the stored ones.
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
    let snap = key_snapshot_for(&provider, key, base_url).await?;
    Ok(TestResult {
        ok: snap.status == "ok",
        metrics: snap.metrics.len(),
        message: snap.error.clone().unwrap_or_default(),
        snapshot: snap,
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
/// %APPDATA%\Pane\accounts\<provider>.json. Failed probes may also be
/// saved for later repair; nothing here touches the network.
#[tauri::command]
fn account_add(
    provider: String,
    label: String,
    api_key: String,
    base_url: Option<String>,
) -> Result<String, String> {
    if provider == "codex" {
        // Codex accounts are Pane sign-ins, not pasted keys — the store is
        // codex_accounts and the entry point is the auth center.
        return Err(
            "Codex accounts are added by signing in: Auth center → Codex → Add account".into(),
        );
    }
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
        if provider_catalog::takes_base_url(&provider) {
            validate_relay_base_url(&provider, url)?;
        } else {
            return Err("this provider does not take a base URL".into());
        }
    }
    if provider_catalog::takes_base_url(&provider) && base_url.is_none() {
        return Err("a base URL is required for this provider".into());
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
    accounts::save_accounts(&provider, &entries)?;
    let _ = forget_provider_snapshot(&provider);
    Ok(candidate_id)
}

/// Replaces a pasted credential by stable identity, never by a stale row
/// index. The new fingerprint is returned so the UI can reconcile its tabs.
#[tauri::command]
fn account_update(provider: String, card_id: String, label: String, api_key: String,
    base_url: Option<String>) -> Result<String, String> {
    if !provider_catalog::provider_definition(&provider).is_some_and(|p| p.supports_api_key && p.supports_extra_accounts) {
        return Err("this provider requires its own sign-in flow".into());
    }
    let mut entries = accounts::load_accounts(&provider);
    let index = entries.iter().position(|entry| accounts::card_id_for_account(&provider, entry) == card_id)
        .ok_or("account no longer exists")?;
    if api_key.trim().is_empty() { return Err("credential is empty".into()); }
    let base_url = base_url.filter(|url| !url.trim().is_empty());
    if provider_catalog::takes_base_url(&provider) {
        validate_relay_base_url(&provider, base_url.as_deref().ok_or("a base URL is required")?)?;
    } else if base_url.is_some() { return Err("this provider does not take a base URL".into()); }
    let candidate = accounts::AccountEntry { label: label.trim().into(), api_key: api_key.trim().into(), base_url };
    let new_id = accounts::card_id_for_account(&provider, &candidate);
    if entries.iter().enumerate().any(|(i, entry)| i != index && accounts::card_id_for_account(&provider, entry) == new_id) {
        return Err("this credential is already added".into());
    }
    entries[index] = candidate;
    accounts::save_accounts(&provider, &entries)?;
    let _ = forget_provider_snapshot(&card_id);
    if index == 0 { let _ = forget_provider_snapshot(&provider); }
    Ok(new_id)
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
    if provider == "codex" {
        let mut logins = codex_accounts::load_with_imported_single_login();
        let has_cli = providers::codex::default_account_info().is_some();
        let target_idx = if has_cli {
            if index == 0 {
                return Err("默认 CLI 账号由本地环境管理，无法在面板中删除".into());
            }
            index - 1
        } else {
            index
        };
        if target_idx >= logins.len() {
            return Err(format!("no codex account #{index}"));
        }
        logins.remove(target_idx);
        return codex_accounts::save_accounts(&logins);
    }
    if login_accounts::takes_login_accounts(&provider) {
        let mut logins = login_accounts::load_with_imported_single_login(&provider);
        if index >= logins.len() {
            return Err(format!("no {provider} account #{index}"));
        }
        let removed = logins.remove(index);
        // The legacy single OAuth slot imports lazily, so if the removed
        // account IS the slot's, delete the file too — otherwise the next
        // load resurrects the row and the bare family card keeps rendering
        // the very account the user just deleted.
        let removed_slot = oauth::load(&provider).is_some_and(|t| {
            t.account_id
                .as_deref()
                .is_some_and(|id| !id.is_empty() && id == removed.account_id)
                || t.access_token == removed.access_token
        });
        login_accounts::save(&provider, &logins)?;
        if removed_slot {
            let _ = oauth::logout(&provider);
        }
        return Ok(());
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

/// Archives an account before removing it from the active cards. The stable
/// card id is retained so credit history stays attached to this identity.
#[tauri::command]
fn account_archive(provider: String, index: usize) -> Result<(), String> {
    let base = crate::providers::config_dir();
    if provider == "antigravity" {
        let mut slots = antigravity_accounts::load_slots();
        if index >= slots.len() {
            return Err(format!("no antigravity slot #{index}"));
        }
        let slot = slots.remove(index);
        let card_id = antigravity_accounts::card_id_for_slot(&slot);
        let label = slot.label.clone();
        let payload = serde_json::to_value(&slot).map_err(|e| format!("archive antigravity slot: {e}"))?;
        accounts::archive_external_to(&base, accounts::ArchivedExternalAccount {
            provider: provider.clone(), card_id, label, payload,
            archived_at: chrono::Utc::now().timestamp_millis(),
        })?;
        return antigravity_accounts::save_slots(&slots);
    }
    if provider == "cursor" {
        let mut entries = cursor_accounts::load_accounts();
        if index >= entries.len() {
            return Err(format!("no cursor account #{index}"));
        }
        let entry = entries.remove(index);
        let card_id = cursor_accounts::card_id_for_account(&entry);
        let label = entry.label.clone();
        let payload = serde_json::to_value(&entry).map_err(|e| format!("archive cursor account: {e}"))?;
        accounts::archive_external_to(&base, accounts::ArchivedExternalAccount {
            provider: provider.clone(), card_id, label, payload,
            archived_at: chrono::Utc::now().timestamp_millis(),
        })?;
        return cursor_accounts::save_accounts(&entries);
    }
    if !accounts::provider_takes_accounts(&provider) {
        return Err(format!("unknown multi-account provider: {provider}"));
    }
    let mut entries = accounts::load_accounts(&provider);
    if index >= entries.len() {
        return Err(format!("no account #{index} for {provider}"));
    }
    if index == 0 {
        if let Some(entry) = entries.first() {
            let stable = accounts::card_id_for_account(&provider, entry);
            usage_history::migrate_card_id(&provider, &stable);
        }
    }
    let entry = entries.remove(index);
    let card_id = accounts::card_id_for_account(&provider, &entry);
    accounts::archive_account(accounts::ArchivedAccount {
        provider: provider.clone(),
        label: entry.label.clone(),
        api_key: entry.api_key.clone(),
        base_url: entry.base_url.clone(),
        card_id,
        archived_at: chrono::Utc::now().timestamp_millis(),
    })?;
    accounts::save_accounts(&provider, &entries)?;
    if index == 0 {
        let _ = forget_provider_snapshot(&provider);
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct ArchivedAccountSummary {
    provider: String,
    card_id: String,
    label: String,
    archived_at: i64,
    masked_key: String,
}

/// Lists the family's archived accounts without returning retained
/// credentials to the WebView.
#[tauri::command]
fn archived_accounts(provider: String) -> Vec<ArchivedAccountSummary> {
    let base = crate::providers::config_dir();
    let mut result: Vec<_> = accounts::load_archived_from(&base)
        .into_iter()
        .filter(|a| a.provider == provider)
        .map(|a| ArchivedAccountSummary {
            masked_key: accounts::mask_key(&a.api_key),
            provider: a.provider,
            card_id: a.card_id,
            label: a.label,
            archived_at: a.archived_at,
        })
        .collect();
    result.extend(
        accounts::load_archived_external_from(&base)
            .into_iter()
            .filter(|a| a.provider == provider)
            .map(|a| ArchivedAccountSummary {
                provider: a.provider,
                card_id: a.card_id,
                label: a.label,
                archived_at: a.archived_at,
                masked_key: "***".to_string(),
            }),
    );
    result
}

/// Deletes recorded usage history for a card when the user explicitly unchecks
/// the keep-usage option during card removal.
#[tauri::command]
fn remove_usage_history(card_id: String) {
    usage_history::remove_card_history(&card_id);
}

/// Restores an archived account: its kept credential re-enters the active
/// accounts file under the original stable card id, so usage history and
/// layout reattach without migration. Accounts archived before credentials
/// were kept have an empty key and are rejected (nothing to restore).
#[tauri::command]
fn account_restore(provider: String, card_id: String) -> Result<(), String> {
    let base = crate::providers::config_dir();
    if provider == "antigravity" {
        let archived = accounts::load_archived_external_from(&base)
            .into_iter()
            .find(|a| a.provider == provider && a.card_id == card_id)
            .ok_or_else(|| format!("no archived account {card_id} for {provider}"))?;
        let slot: antigravity_accounts::AgSlot = serde_json::from_value(archived.payload)
            .map_err(|e| format!("restore antigravity slot: {e}"))?;
        let mut slots = antigravity_accounts::load_slots();
        if slots.iter().any(|s| antigravity_accounts::card_id_for_slot(s) == card_id) {
            return Err(format!("account {card_id} already exists"));
        }
        slots.push(slot);
        antigravity_accounts::save_slots(&slots)?;
        return accounts::remove_archived_external_from(&base, &card_id);
    }
    if provider == "cursor" {
        let archived = accounts::load_archived_external_from(&base)
            .into_iter()
            .find(|a| a.provider == provider && a.card_id == card_id)
            .ok_or_else(|| format!("no archived account {card_id} for {provider}"))?;
        let entry: cursor_oauth::CursorAccount = serde_json::from_value(archived.payload)
            .map_err(|e| format!("restore cursor account: {e}"))?;
        let mut entries = cursor_accounts::load_accounts();
        if entries.iter().any(|e| cursor_accounts::card_id_for_account(e) == card_id) {
            return Err(format!("account {card_id} already exists"));
        }
        entries.push(entry);
        cursor_accounts::save_accounts(&entries)?;
        return accounts::remove_archived_external_from(&base, &card_id);
    }
    if !accounts::provider_takes_accounts(&provider) {
        return Err(format!("unknown multi-account provider: {provider}"));
    }
    let base = crate::providers::config_dir();
    let archived = accounts::load_archived_from(&base)
        .into_iter()
        .find(|a| a.provider == provider && a.card_id == card_id)
        .ok_or_else(|| format!("no archived account {card_id} for {provider}"))?;
    if archived.api_key.trim().is_empty() {
        return Err(format!("archived account {card_id} kept no credential"));
    }
    let mut entries = accounts::load_accounts(&provider);
    if entries
        .iter()
        .any(|e| accounts::card_id_for_account(&provider, e) == card_id)
    {
        return Err(format!("account {card_id} already exists"));
    }
    entries.push(accounts::AccountEntry {
        label: archived.label,
        api_key: archived.api_key,
        base_url: archived.base_url,
    });
    accounts::save_accounts(&provider, &entries)?;
    accounts::remove_archived(&card_id)?;
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
    if index > 0 {
        let old_default = accounts::card_id_for_account(&provider, &entries[0]);
        usage_history::migrate_card_id(&provider, &old_default);
        let promoted = accounts::card_id_for_account(&provider, &entries[index]);
        usage_history::migrate_card_id(&promoted, &provider);
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
    if provider == "codex" {
        let mut logins = codex_accounts::load_with_imported_single_login();
        let has_cli = providers::codex::default_account_info().is_some();
        let target_idx = if has_cli {
            if index == 0 {
                return Err("默认 CLI 账号标签跟随本地环境，如需重命名请在看板卡片备注中设置".into());
            }
            index - 1
        } else {
            index
        };
        if target_idx >= logins.len() {
            return Err(format!("no codex account #{index}"));
        }
        logins[target_idx].label = label.trim().to_string();
        return codex_accounts::save_accounts(&logins);
    }
    if login_accounts::takes_login_accounts(&provider) {
        let mut logins = login_accounts::load_with_imported_single_login(&provider);
        if index >= logins.len() {
            return Err(format!("no {provider} account #{index}"));
        }
        logins[index].label = label.trim().to_string();
        return login_accounts::save(&provider, &logins);
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
    if provider == "codex" {
        let mut list = Vec::new();
        if let Some((_account_id, email)) = providers::codex::default_account_info() {
            list.push(json!({
                "id": "codex",
                "label": email.clone().unwrap_or_else(|| "默认 (CLI)".into()),
                "email": email,
                "maskedKey": "CLI 登录",
                "baseUrl": null,
            }));
        }
        for login in codex_accounts::load_with_imported_single_login() {
            let id = codex_accounts::card_id_for_account(&login);
            if list.iter().any(|existing| existing.get("id").and_then(Value::as_str) == Some(&id)) {
                continue;
            }
            list.push(json!({
                "id": id,
                "label": if login.label.trim().is_empty() {
                    if login.email.trim().is_empty() {
                        "Codex 账号".into()
                    } else {
                        login.email.clone()
                    }
                } else {
                    login.label.clone()
                },
                "email": login.email,
                "maskedKey": codex_accounts::mask_token(&login.access_token),
                "baseUrl": null,
            }));
        }
        return Ok(list);
    }
    // Login-backed families (auth center) keep their accounts in the login
    // store; the generic branch below reads the API-key store, which these
    // families never use (supports_api_key: false).
    if login_accounts::takes_login_accounts(&provider) {
        let logins = login_accounts::load_with_imported_single_login(&provider);
        return Ok(login_accounts::account_rows(&provider, &logins));
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

fn retain_current_key_accounts(snapshots: &mut Vec<providers::Snapshot>, expected: &HashMap<String, String>) {
    let families: HashSet<String> = expected.keys().map(|id| family_of(id)).collect();
    let current: HashMap<String, String> = families.iter().flat_map(|family| {
        let mut identities = accounts::snapshot_identities(family, &accounts_with_imported_main_key(family));
        identities.entry(family.clone()).or_default();
        identities
    }).collect();
    snapshots.retain(|snapshot| expected.get(&snapshot.id)
        .is_none_or(|identity| current.get(&snapshot.id) == Some(identity)));
}

/// The panel window's auth center (M3): every auth family with its account
/// rows — antigravity slots, cursor accounts, and the single OAuth login of
/// codex/copilot/grok. Read-only assembly over the existing stores; quota
/// and needs-reauth state come from the snapshot cache on the frontend.
#[tauri::command]
fn auth_center_list() -> Vec<auth_center::AuthFamilyGroup> {
    auth_center::collect()
}

/// Known env-var fallbacks per provider, for get_credential_status. The
/// saved-file probe is providers::stored_key_file; local-CLI detection is
/// each provider's own local_credential_hint (dispatched just below).
fn provider_env_vars(provider: &str) -> &'static [&'static str] {
    match provider {
        "openrouter" => &["OPENROUTER_API_KEY"],
        "zai" => &["ZAI_API_KEY", "GLM_API_KEY"],
        "commandcode" => &["COMMAND_CODE_API_KEY"],
        "clawsgo" => &["CLAWSGO_TOKEN"],
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
        "stepfun-plan" => &[],
        "bocha" => &["BOCHA_API_KEY"],
        "tavily" => &["TAVILY_API_KEY"],
        "clinepass" => &["CLINE_API_KEY"],
        "apigoto" => &["APIGOTO_API_KEY"],
        "amp" => &["AMP_API_KEY"],
        "chutes" => &["CHUTES_API_KEY"],
        "deepgram" => &["DEEPGRAM_API_KEY"],
        "openai-api" => &["OPENAI_ADMIN_KEY"],
        "poe" => &["POE_API_KEY"],
        "venice" => &["VENICE_API_KEY"],
        "warp" => &["WARP_API_KEY", "WARP_TOKEN"],
        "mimo" => &["MIMO_API_KEY"],
        "factory" => &["FACTORY_API_KEY"],
        "groq" => &["GROQ_API_KEY"],
        "huggingface" => &["HF_TOKEN", "HUGGINGFACE_API_KEY", "HUGGING_FACE_HUB_TOKEN"],
        "longcat" => &["LONGCAT_COOKIE"],
        "sub2api" => &["SUB2API_API_KEY"],
        "mistral" => &["MISTRAL_COOKIE"],
        "perplexity" => &["PERPLEXITY_SESSION_TOKEN", "PERPLEXITY_COOKIE"],
        "volcengine" => &["VOLC_ACCESS_KEY"],

        "firecrawl" => &["FIRECRAWL_API_KEY", "FIRECRAWL_FIRECRAWL_API_KEY"],
        "brave" => &["BRAVE_API_KEY", "BRAVE_SEARCH_API_KEY"],
        "giphy" => &["GIPHY_PARTNER_TOKEN", "GIPHY_ACCESS_TOKEN"],
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
    // "stored key" means "the accounts list has at least one entry". Login
    // families keep their accounts in login-accounts/<family>.json, NOT in
    // accounts/<family>.json — reading the wrong store made a signed-in
    // CodeBuddy/WorkBuddy look unconfigured.
    let stored_key = if login_accounts::takes_login_accounts(&family) {
        !login_accounts::load(&family).is_empty()
    } else if accounts::provider_takes_accounts(&family) {
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
        "stepfun-plan" => providers::stepfun_plan::local_credential_hint(),
        "clinepass" => providers::clinepass::local_credential_hint(),
        "sensenova" => providers::sensenova::local_credential_hint(),
        "apigoto" => providers::apigoto::local_credential_hint(),
        "amp" => providers::amp::local_credential_hint(),
        "chutes" => providers::chutes::local_credential_hint(),
        "deepgram" => providers::deepgram::local_credential_hint(),
        "openai-api" => providers::openai_api::local_credential_hint(),
        "poe" => providers::poe::local_credential_hint(),
        "venice" => providers::venice::local_credential_hint(),
        "warp" => providers::warp::local_credential_hint(),
        "shandianshuo" => providers::shandianshuo::local_credential_hint(),
        "bocha" => providers::searchquota::local_credential_hint("bocha", &["BOCHA_API_KEY"]),
        "tavily" => providers::searchquota::local_credential_hint("tavily", &["TAVILY_API_KEY"]),
        "firecrawl" => providers::searchquota::local_credential_hint(
            "firecrawl",
            &["FIRECRAWL_API_KEY", "FIRECRAWL_FIRECRAWL_API_KEY"],
        ),
        "brave" => providers::searchquota::local_credential_hint("brave", &["BRAVE_API_KEY", "BRAVE_SEARCH_API_KEY"]),
        "giphy" => providers::giphy::local_credential_hint(),
        "siliconflow" => providers::siliconflow::local_credential_hint(),
        "novita" => providers::novita::local_credential_hint(),
        "relaybalance" => providers::relaybalance::local_credential_hint(),
        "qodercn" => providers::qodercn::local_credential_hint(),
        "traecn" => providers::traecn::local_credential_hint(),
        "commandcode" => providers::commandcode::local_credential_hint(),
        "doubao" => providers::doubao::local_credential_hint(),
        "clawsgo" => providers::clawsgo::local_credential_hint(),
        "kiro" => providers::kiro::local_credential_hint(),
        "trae" => providers::trae::local_credential_hint(),
        "qoder" => providers::qoder::local_credential_hint(),
        "zed" => providers::zed::local_credential_hint(),
        "mimo" => providers::mimo::local_credential_hint(),
        "factory" => providers::factory::local_credential_hint(),
        "jetbrains" => providers::jetbrains::local_credential_hint(),
        "groq" => providers::groq::local_credential_hint(),
        "huggingface" => providers::huggingface::local_credential_hint(),
        "longcat" => providers::longcat::local_credential_hint(),
        "sub2api" => providers::sub2api::local_credential_hint(),
        "mistral" => providers::mistral::local_credential_hint(),
        "perplexity" => providers::perplexity::local_credential_hint(),
        "volcengine" => providers::volcengine::local_credential_hint(),
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
    let masked_key = providers::stored_key_file(&family).map(|key| accounts::mask_key(&key));
    json!({ "storedKey": stored_key, "maskedKey": masked_key, "envKey": env_key, "localCli": local_cli, "oauth": oauth_label, "activeSource": active_source, "membership": membership })
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
    let response = oauth::poll(&provider, &device_auth_id).await;
    // Every completed Codex sign-in becomes its own account in the store, so
    // N ChatGPT subscriptions can be monitored side by side; the single
    // oauth/codex.json slot stays as the default card's fallback.
    if provider == "codex" && response.done {
        if let Some(tokens) = oauth::load("codex") {
            let _ = codex_accounts::record_login(&tokens);
        }
    }
    response
}

/// Starts the Codex browser PKCE login (127.0.0.1:1455 loopback). Errors
/// with "CODEX_OAUTH_PORT_IN_USE" when the Codex CLI owns the port — the
/// frontend then falls back to the device-code flow.
#[tauri::command]
fn codex_login_start() -> Result<codex_login::LoginStart, String> {
    codex_login::start()
}

#[tauri::command]
async fn codex_login_poll(login_id: String) -> codex_login::LoginPoll {
    codex_login::poll(&login_id).await
}

#[tauri::command]
fn codex_login_cancel(login_id: String) {
    codex_login::cancel(&login_id);
}

/// Kiro browser PKCE login (app.kiro.dev portal + fixed-port callback):
/// start → open the auth URL → poll until the token exchange completes.
#[tauri::command]
fn kiro_login_start() -> Result<kiro_login::LoginStart, String> {
    kiro_login::start()
}

#[tauri::command]
async fn kiro_login_poll(login_id: String) -> kiro_login::LoginPoll {
    kiro_login::poll(&login_id).await
}

#[tauri::command]
fn kiro_login_cancel(login_id: String) {
    kiro_login::cancel(&login_id);
}

/// Qoder device login (qoder.com selection page + openapi deviceToken
/// poll): start → open the auth URL → poll until the token lands.
#[tauri::command]
fn qoder_login_start() -> Result<qoder_login::LoginStart, String> {
    qoder_login::start()
}

#[tauri::command]
async fn qoder_login_poll(login_id: String) -> qoder_login::LoginPoll {
    qoder_login::poll(&login_id).await
}

#[tauri::command]
fn qoder_login_cancel(login_id: String) {
    qoder_login::cancel(&login_id);
}

/// CodeBuddy family (Tencent) scan login: auth/state → browser page →
/// auth/token poll. The family table supplies host + platform, so each
/// member gets its own command triple under the frontend's
/// `<family>_login_*` naming convention.
#[tauri::command]
async fn codebuddy_login_start() -> Result<codebuddy_login::LoginStart, String> {
    codebuddy_login::start(codebuddy_login::known_family("codebuddy")).await
}

#[tauri::command]
async fn codebuddycn_login_start() -> Result<codebuddy_login::LoginStart, String> {
    codebuddy_login::start(codebuddy_login::known_family("codebuddycn")).await
}

#[tauri::command]
async fn workbuddy_login_start() -> Result<codebuddy_login::LoginStart, String> {
    codebuddy_login::start(codebuddy_login::known_family("workbuddy")).await
}

#[tauri::command]
async fn workbuddyai_login_start() -> Result<codebuddy_login::LoginStart, String> {
    codebuddy_login::start(codebuddy_login::known_family("workbuddyai")).await
}

#[tauri::command]
async fn codebuddy_login_poll(login_id: String) -> codebuddy_login::LoginPoll {
    codebuddy_login::poll(&login_id).await
}

#[tauri::command]
async fn codebuddycn_login_poll(login_id: String) -> codebuddy_login::LoginPoll {
    codebuddy_login::poll(&login_id).await
}

#[tauri::command]
async fn workbuddy_login_poll(login_id: String) -> codebuddy_login::LoginPoll {
    codebuddy_login::poll(&login_id).await
}

#[tauri::command]
async fn workbuddyai_login_poll(login_id: String) -> codebuddy_login::LoginPoll {
    codebuddy_login::poll(&login_id).await
}

#[tauri::command]
fn codebuddy_login_cancel(login_id: String) {
    codebuddy_login::cancel(&login_id);
}

#[tauri::command]
fn codebuddycn_login_cancel(login_id: String) {
    codebuddy_login::cancel(&login_id);
}

#[tauri::command]
fn workbuddy_login_cancel(login_id: String) {
    codebuddy_login::cancel(&login_id);
}

#[tauri::command]
fn workbuddyai_login_cancel(login_id: String) {
    codebuddy_login::cancel(&login_id);
}

/// Windsurf browser login (implicit flow on an ephemeral loopback port,
/// then the seat-management key exchange): start → open the auth URL →
/// poll until the callback lands.
#[tauri::command]
fn windsurf_login_start() -> Result<windsurf_login::LoginStart, String> {
    windsurf_login::start()
}

#[tauri::command]
async fn windsurf_login_poll(login_id: String) -> windsurf_login::LoginPoll {
    windsurf_login::poll(&login_id).await
}

#[tauri::command]
fn windsurf_login_cancel(login_id: String) {
    windsurf_login::cancel(&login_id);
}

/// Trae (international) browser login: GetLoginGuidance → the IDE's
/// authorization page → loopback /authorize → auth-code exchange.
#[tauri::command]
async fn trae_login_start() -> Result<trae_login::LoginStart, String> {
    trae_login::start().await
}

#[tauri::command]
async fn trae_login_poll(login_id: String) -> trae_login::LoginPoll {
    trae_login::poll(&login_id).await
}

#[tauri::command]
fn trae_login_cancel(login_id: String) {
    trae_login::cancel(&login_id);
}

/// Zed native-app sign-in (ephemeral loopback port + an RSA-sealed
/// access token): start → open the auth URL → poll until the callback
/// lands.
#[tauri::command]
fn zed_login_start() -> Result<zed_login::LoginStart, String> {
    zed_login::start()
}

#[tauri::command]
async fn zed_login_poll(login_id: String) -> zed_login::LoginPoll {
    zed_login::poll(&login_id).await
}

#[tauri::command]
fn zed_login_cancel(login_id: String) {
    zed_login::cancel(&login_id);
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
    let previous = load_config()
        .get("shortcut")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    match register_shortcut(&app, &shortcut) {
        Ok(()) => Ok(()),
        Err(err) => {
            // register_shortcut clears the global registry before attempting
            // the new binding. Restore the last working shortcut so a failed
            // Alt+2 change does not leave the app unreachable.
            let _ = register_shortcut(&app, &previous);
            Err(format!("shortcut unavailable: {err}"))
        }
    }
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

/// Update manifest endpoints, tried in order. The original upstream's
/// trypane.xyz server and ItsJazii releases were dropped when the project
/// detached — their builds are signed with a key this app no longer trusts,
/// so querying them could only produce signature failures.
///
/// The first endpoint is our own edge route: the same GitHub manifest,
/// except CN clients get the Windows installer URL rewritten to the Gitee
/// mirror (GitHub release assets measure ~40 KB/s from Chinese networks
/// against ~1.9 MB/s from Gitee). The edge decides by the request's country
/// — but a CN user behind a global proxy exits abroad, so Cloudflare sees
/// the proxy's country and the geo branch misses exactly the users who
/// suffer most. A client that resolves a proxy therefore asks for the
/// mirror explicitly (`?source=gitee`); the route still probes the mirror
/// before offering it. Second comes the plain edge route (geo by IP), then
/// GitHub's own manifest as the reachable-everywhere fallback.
///
/// Ordering matters: tauri-plugin-updater only advances to the next URL when
/// a request fails or answers non-2xx, never for slowness.
fn updater_endpoint_strings() -> [String; 3] {
    let edge = "https://pane.threetwoa.live/update/latest.json";
    let first = if providers::outbound_proxy_url().is_some() {
        format!("{edge}?source=gitee")
    } else {
        edge.to_string()
    };
    let endpoints = [
        first,
        edge.to_string(),
        "https://github.com/Aafff623/pane/releases/latest/download/latest.json".into(),
    ];
    // Support aid: PANE_UPDATE_DEBUG=1 makes the resolved order visible in
    // the log, which is the only way to tell which manifest (and therefore
    // which download host) a machine actually took.
    if std::env::var_os("PANE_UPDATE_DEBUG").is_some() {
        eprintln!("[pane] updater endpoints: {endpoints:?}");
    }
    endpoints
}

fn build_updater(app: &tauri::AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    use tauri_plugin_updater::UpdaterExt;
    let endpoints = updater_endpoint_strings()
        .into_iter()
        .map(|endpoint| endpoint.parse().map_err(|e| format!("endpoint parse: {e}")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut builder = app
        .updater_builder()
        .endpoints(endpoints)
        .map_err(|e| e.to_string())?;
    // The plugin's client reads proxy env vars only — a Clash-style Windows
    // system proxy is invisible to it (the plugin builds reqwest without the
    // system-proxy feature), so manifest fetches rode a direct github.com
    // connection and died on flaky links with "error sending request". Feed
    // it the same chain every provider client uses. An unparsable proxy
    // value is skipped, not fatal — same tolerance as providers' http_builder.
    if let Some(url) = providers::outbound_proxy_url() {
        if let Ok(proxy) = url.parse() {
            builder = builder.proxy(proxy);
        }
    }
    // Cap the manifest check so a dead link errors in seconds instead of
    // parking the install button on OS-level TCP timeouts. The plugin
    // builds the download client without a total timeout, so this never
    // cuts a slow installer download short.
    builder = builder.timeout(std::time::Duration::from_secs(20));
    builder.build().map_err(|e| e.to_string())
}

/// Downloads and installs a pending update, then restarts the app. Only
/// called from the frontend banner after check_for_update announced one.
/// Byte progress streams as `update-progress` events so the spend-header
/// indicator can fill a real ring — the mainstream download-progress
/// detail (electron-updater exposes the same event).
#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let updater = build_updater(&app)?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => {
            let handle = app.clone();
            update
                .download_and_install(
                    move |chunk_len, content_len| {
                        let _ = handle.emit(
                            "update-progress",
                            serde_json::json!({ "chunk": chunk_len, "total": content_len }),
                        );
                    },
                    || {},
                )
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

/// StepFun Step Plan: test pasted credentials (token pair, or account +
/// password which runs a throwaway login). Never writes anything.
#[tauri::command]
async fn stepfun_plan_test(
    token: String,
    username: String,
    password: String,
) -> Result<serde_json::Value, String> {
    let snap = providers::stepfun_plan::snapshot_with_creds(&token, &username, &password).await;
    Ok(serde_json::json!({
        "ok": snap.status == "ok",
        "status": snap.status,
        "metrics": snap.metrics.len(),
        "message": snap.error.unwrap_or_default(),
    }))
}

/// StepFun Step Plan: persist credentials. Account + password without a
/// token runs one real login so the stored pair is born here (and a bad
/// password fails at save time, not at the next refresh).
#[tauri::command]
async fn stepfun_plan_save(
    token: String,
    username: String,
    password: String,
) -> Result<(), String> {
    providers::stepfun_plan::save_creds_interactive(&token, &username, &password).await
}

/// StepFun Step Plan: drop the stored pair + account.
#[tauri::command]
async fn stepfun_plan_clear() {
    providers::stepfun_plan::clear_creds().await;
}

/// SenseNova: build the PKCE authorization URL for the one-time browser
/// sign-in (the verifier/state stay in memory for `sensenova_oauth_finish`).
#[tauri::command]
fn sensenova_oauth_start() -> String {
    providers::sensenova::oauth_start()
}

/// SenseNova: exchange/persist whatever the browser yielded (redirect URL,
/// token payload, bare access token, or raw code) and probe the pools.
#[tauri::command]
async fn sensenova_oauth_finish(input: String) -> Result<providers::Snapshot, String> {
    providers::sensenova::oauth_finish(&input).await
}

// ---------------------------------------------------------------------------
// Panel window bounds — the desktop form remembers its last size/position.
// Stored separately from config.json (pure local UI state, like a window
// manager's session file), in physical pixels because that is exactly what
// outer_position/inner_size report and restore.
// ---------------------------------------------------------------------------

const PANEL_DEFAULT_SIZE: (f64, f64) = (1350.0, 900.0);
const PANEL_MIN_SIZE: (f64, f64) = (960.0, 640.0);

fn panel_bounds_path() -> PathBuf {
    providers::config_dir().join("panel-window.json")
}

fn load_panel_bounds() -> Option<(i32, i32, u32, u32)> {
    let raw = std::fs::read_to_string(panel_bounds_path()).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    Some((
        i32::try_from(value.get("x")?.as_i64()?).ok()?,
        i32::try_from(value.get("y")?.as_i64()?).ok()?,
        u32::try_from(value.get("width")?.as_u64()?).ok()?,
        u32::try_from(value.get("height")?.as_u64()?).ok()?,
    ))
}

fn save_panel_bounds(window: &tauri::Window) {
    // A minimized window reports an off-screen position — persisting that
    // would restore the panel in the void on the next open.
    if window.is_minimized().unwrap_or(false) {
        return;
    }
    let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) else {
        return;
    };
    let bounds = json!({
        "x": pos.x,
        "y": pos.y,
        "width": size.width,
        "height": size.height,
    });
    let _ = std::fs::write(panel_bounds_path(), bounds.to_string());
}

/// Coalesces the resize/move stream into one write: a drag emits dozens of
/// events per second, and every one of them would otherwise hit the disk.
fn schedule_panel_bounds_save(window: tauri::Window) {
    use std::sync::mpsc;
    static TX: OnceLock<mpsc::Sender<tauri::Window>> = OnceLock::new();
    let tx = TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<tauri::Window>();
        std::thread::spawn(move || {
            let mut latest = None;
            while let Ok(window) = rx.recv() {
                latest = Some(window);
                while let Ok(window) = rx.recv_timeout(std::time::Duration::from_millis(350)) {
                    latest = Some(window);
                }
                if let Some(window) = latest.take() {
                    save_panel_bounds(&window);
                }
            }
        });
        tx
    });
    let _ = tx.send(window);
}

/// The saved rect is only usable while a meaningful piece of it still sits
/// on a connected monitor — unplugging the display that held the panel must
/// not strand the window off-screen. The result is clamped into that
/// monitor, so a resolution change cannot leave it hanging off an edge.
fn restorable_panel_bounds(
    app: &tauri::AppHandle,
) -> Option<(tauri::PhysicalPosition<i32>, tauri::PhysicalSize<u32>)> {
    let (x, y, width, height) = load_panel_bounds()?;
    if width < 320 || height < 240 {
        return None;
    }
    let mut best: Option<(i64, i32, i32, u32, u32)> = None;
    for monitor in app.available_monitors().ok()? {
        let mp = monitor.position();
        let ms = monitor.size();
        let overlap_w = (i64::from(x) + i64::from(width))
            .min(i64::from(mp.x) + i64::from(ms.width))
            - i64::from(x).max(i64::from(mp.x));
        let overlap_h = (i64::from(y) + i64::from(height))
            .min(i64::from(mp.y) + i64::from(ms.height))
            - i64::from(y).max(i64::from(mp.y));
        let area = overlap_w.max(0) * overlap_h.max(0);
        if area > best.map_or(0, |b| b.0) {
            best = Some((area, mp.x, mp.y, ms.width, ms.height));
        }
    }
    let (area, mx, my, mw, mh) = best?;
    if area < 120 * 80 {
        return None;
    }
    let width = width.min(mw);
    let height = height.min(mh);
    let max_x = (mx + mw as i32 - width as i32).max(mx);
    let max_y = (my + mh as i32 - height as i32).max(my);
    Some((
        tauri::PhysicalPosition::new(x.clamp(mx, max_x), y.clamp(my, max_y)),
        tauri::PhysicalSize::new(width, height),
    ))
}

/// Shared panel-window opener: focus the existing window instead of stacking
/// duplicates, build it otherwise. Used by the open_panel_window command and
/// the tray menu ("Open main panel" / "Settings…").
///
/// MUST only be reached from async contexts: on Windows,
/// WebviewWindowBuilder::build() deadlocks inside synchronous commands /
/// run_on_main_thread (wry's own docs) — and the deadlock left a live frame
/// with a permanently blank webview, the exact white screen this PoC first
/// produced. Tray menu callbacks therefore forward here via
/// tauri::async_runtime::spawn.
async fn ensure_panel_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("panel") {
        // A prior close/minimize can leave the native handle alive while the
        // window is not visible. Make the tray action idempotent and visibly
        // recover the existing panel instead of flashing and disappearing.
        let _ = w.unminimize();
        let _ = w.show();
        w.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    // Built hidden so restoring a saved rect happens before the first paint
    // instead of flashing a centered window and then jumping.
    let restored = restorable_panel_bounds(app);
    let window =
        tauri::WebviewWindowBuilder::new(app, "panel", tauri::WebviewUrl::App("index.html".into()))
            .title("Pane")
            .inner_size(PANEL_DEFAULT_SIZE.0, PANEL_DEFAULT_SIZE.1)
            .min_inner_size(PANEL_MIN_SIZE.0, PANEL_MIN_SIZE.1)
            .center()
            .resizable(true)
            .decorations(true)
            .always_on_top(false)
            .skip_taskbar(false)
            .visible(false)
            .build()
            .map_err(|e| e.to_string())?;
    if let Some((pos, size)) = restored {
        let _ = window.set_size(size);
        let _ = window.set_position(pos);
    }
    let _ = window.show();
    let _ = window.set_focus();
    match window.url() {
        Ok(url) => eprintln!("[pane] panel window opened, url={url}"),
        Err(_) => eprintln!("[pane] panel window opened (url unreadable)"),
    }
    Ok(())
}

/// Panel-form PoC (dual-form product work): a second window over the same
/// frontend bundle — resizable, decorated, taskbar-visible, not always-on-top.
/// Must stay an ASYNC command — see ensure_panel_window for the deadlock note.
#[tauri::command]
async fn open_panel_window(app: tauri::AppHandle) -> Result<(), String> {
    ensure_panel_window(&app).await
}

/// Window-form switch to "floating" closes the panel window; from inside the
/// panel window this is a self-close. No-op when it is not open.
#[tauri::command]
async fn close_panel_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("panel") {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn get_window_form() -> String {
    config_with_defaults(load_config())
        .get("windowForm")
        .and_then(Value::as_str)
        .unwrap_or("floating")
        .to_string()
}

/// Persist the preferred window form ("floating" | "panel"). Async like the
/// other window commands: config writes take the CONFIG_WRITE mutex, which
/// must never be held on the main thread across window work.
#[tauri::command]
async fn set_window_form(app: tauri::AppHandle, form: String) -> Result<(), String> {
    let form = match form.as_str() {
        "panel" => "panel",
        _ => "floating",
    };
    set_config_inner(json!({ "windowForm": form }))?;
    // Same cross-window note as set_config: windowForm is a frontend key.
    let _ = app.emit("config-updated", ());
    Ok(())
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
/// When the "main" window last GAINED focus (ms epoch).
static LAST_FOCUS_GAINED_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The position the next summon should use: the one the user dragged the
/// popover to, but only while the position lock is off. `None` means "place
/// it the usual way" (tray anchor / monitor center).
fn saved_popover_position() -> Option<tauri::PhysicalPosition<i32>> {
    let cfg = load_config();
    if cfg.get("popoverLock").and_then(Value::as_bool).unwrap_or(true) {
        return None; // locked: the popover is always re-placed, never remembered
    }
    let pos = cfg.get("popoverPos")?;
    let x = i32::try_from(pos.get("x")?.as_i64()?).ok()?;
    let y = i32::try_from(pos.get("y")?.as_i64()?).ok()?;
    Some(tauri::PhysicalPosition::new(x, y))
}

/// Remembers where the popover sits so an unlocked summon can put it back.
/// Called on every dismiss path; a no-op while the lock is on, because a
/// locked popover's position is not the user's to keep.
/// Re-apply the pin preference on every summon. The window builder starts
/// unpinned; the stored config is the only authority, so a prior
/// always-on-top state cannot leak across hide/show.
fn apply_popover_pin(window: &tauri::WebviewWindow) {
    let pinned = load_config()
        .get("popoverPin")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let _ = window.set_always_on_top(pinned);
}

fn remember_popover_position(window: &tauri::WebviewWindow) {
    let cfg = load_config();
    if cfg.get("popoverLock").and_then(Value::as_bool).unwrap_or(true) {
        return;
    }
    let Ok(pos) = window.outer_position() else {
        return;
    };
    let _ = set_config_inner(json!({ "popoverPos": { "x": pos.x, "y": pos.y } }));
}

/// Dismisses the popover, remembering its position first when unlocked.
fn dismiss_popover(window: &tauri::WebviewWindow) {
    remember_popover_position(window);
    let _ = window.hide();
    set_webview_memory_level(window, true);
}

fn toggle_popover(app: &tauri::AppHandle, click: tauri::PhysicalPosition<f64>) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    // A minimized window still reports is_visible() == true: Win+D / "show
    // desktop" iconifies without clearing WS_VISIBLE, so the naive check
    // would keep routing every toggle into the hide branch and the popover
    // could never be summoned again. Treat iconic as hidden.
    let minimized = window.is_minimized().unwrap_or(false);
    if window.is_visible().unwrap_or(false) && !minimized {
        dismiss_popover(&window);
        return;
    }

    if now_ms().saturating_sub(LAST_AUTO_HIDE_MS.load(Ordering::Relaxed)) < 300 {
        return;
    }

    set_webview_memory_level(&window, false);
    apply_popover_pin(&window);

    // Anchor the popover's bottom-right corner near the tray click,
    // which sits next to the clock on a standard bottom taskbar — unless
    // the user dragged it somewhere and left the lock off.
    let size = window
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(380, 600));
    let placed = if let Some(pos) = saved_popover_position() {
        window.set_position(tauri::PhysicalPosition::new(pos.x, pos.y)).is_ok()
    } else {
        false
    };
    if !placed {
        let x = (click.x - f64::from(size.width)).max(0.0);
        let y = (click.y - f64::from(size.height) - 8.0).max(0.0);
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
    // show() alone leaves an iconic window iconic (SW_SHOW does not restore),
    // so the restore must be explicit before focusing.
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    hide_window_border(&window);
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

    // Same iconic-as-hidden rule as toggle_popover: a Win+D-minimized
    // popover still counts as "visible" to is_visible(), which deadlocked
    // the Alt+2 / second-launch toggle in the hide branch forever.
    let minimized = window.is_minimized().unwrap_or(false);
    if window.is_visible().unwrap_or(false) && !minimized {
        dismiss_popover(&window);
        return;
    }

    if now_ms().saturating_sub(LAST_AUTO_HIDE_MS.load(Ordering::Relaxed)) < 300 {
        return;
    }

    set_webview_memory_level(&window, false);
    apply_popover_pin(&window);

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
    // A dragged position wins over centering while the lock is off.
    let placed = if let Some(pos) = saved_popover_position() {
        window.set_position(pos).is_ok()
    } else {
        false
    };
    if !placed {
        if let Some(m) = monitor {
            let mon_pos = m.position();
            let mon_size = m.size();
            let x = mon_pos.x as f64
                + ((mon_size.width as f64 - size.width as f64) / 2.0).max(0.0);
            let y = mon_pos.y as f64
                + ((mon_size.height as f64 - size.height as f64) / 2.0).max(0.0);
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        }
    }
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    hide_window_border(&window);
    let _ = window.emit("popover-shown", ());
}

/// Shows the popover (never toggles it off) and asks it to replay the
/// first-run spotlight tour. The tour highlights popover elements, so a
/// replay requested from the settings panel window must land in the
/// popover's own document rather than the panel's.
#[tauri::command]
fn replay_tour(app: tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    set_webview_memory_level(&window, false);
    apply_popover_pin(&window);
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("tour-replay", ());
}

/// Hide the popover from the webview (Esc key): mirrors the tray toggle's
/// hide branch so the webview drops to low memory while parked in the tray.
#[tauri::command]
fn hide_popover(app: tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    remember_popover_position(&window);
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            keyvault_list,
            keyvault_add,
            keyvault_remove,
            keyvault_copy,
            keyvault_status,
            keyvault_set_password,
            keyvault_set_recovery,
            keyvault_recovery_questions,
            keyvault_recovery_reset,
            keyvault_v3_setup,
            agent_skill_plan,
            agent_skill_install,
            skin_pack_install,
            skin_pack_download,
            skin_pack_list,
            skin_pack_remove,
            font_pack_list,
            font_pack_download,
            font_pack_install,
            font_pack_remove,
            reset_wake_settings,
            reset_wake_set,
            reset_wake_status,
            workbuddy_web_status,
            workbuddy_web_set_enabled,
            workbuddy_web_import_chrome,
            workbuddy_web_save_cookie,
            workbuddy_web_clear,
            workbuddy_import_desktop,
            tokenrhythm_login_status,
            tokenrhythm_login,
            tokenrhythm_login_clear,
            save_text_file,
            keyvault_v3_enable_recovery_code,
            keyvault_v3_disable_recovery_code,
            keyvault_v3_reset_unlock_code,
            keyvault_unlock,
            keyvault_lock,
            keyvault_reveal,
            keyvault_set_note,
            fetch_usage,
            refresh_provider,
            test_provider,
            cached_usage,
            fetch_spend,
            fetch_spend_sources,
            fetch_credit_history,
            fetch_meter_history,
            get_boot_id,
            fetch_spend_history,
            fetch_credit_extras,
            fetch_spend_daily,
            fetch_qoder_spend,
            antigravity_capture_account,
            cursor_oauth_start,
            cursor_oauth_poll,
            cursor_oauth_cancel,
            cursor_import,
            set_api_key,
            reveal_provider_key,
            reveal_account_key,
            change_master_password,
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
            account_update,
            account_remove,
            account_archive,
            account_restore,
            remove_usage_history,
            archived_accounts,
            account_rename,
            account_set_default,
            account_list,
            auth_center_list,
            get_credential_status,
            oauth_start,
            oauth_poll,
            codex_login_start,
            codex_login_poll,
            codex_login_cancel,
            kiro_login_start,
            kiro_login_poll,
            kiro_login_cancel,
            qoder_login_start,
            qoder_login_poll,
            qoder_login_cancel,
            codebuddy_login_start,
            codebuddy_login_poll,
            codebuddy_login_cancel,
            codebuddycn_login_start,
            codebuddycn_login_poll,
            codebuddycn_login_cancel,
            workbuddy_login_start,
            workbuddy_login_poll,
            workbuddy_login_cancel,
            workbuddyai_login_start,
            workbuddyai_login_poll,
            workbuddyai_login_cancel,
            windsurf_login_start,
            windsurf_login_poll,
            windsurf_login_cancel,
            trae_login_start,
            trae_login_poll,
            trae_login_cancel,
            zed_login_start,
            zed_login_poll,
            zed_login_cancel,
            oauth_logout,
            get_config,
            set_config,
            system_ui_locale,
            list_system_fonts,
            get_autostart,
            set_autostart,
            sync_tray_surfaces,
            open_link,
            copy_share_image,
            set_shortcut,
            hide_popover,
            replay_tour,
            codex_redeem_credit,
            install_update,
            check_update,
            open_panel_window,
            close_panel_window,
            get_window_form,
            set_window_form,
            stepfun_plan_test,
            stepfun_plan_save,
            stepfun_plan_clear,
            sensenova_oauth_start,
            sensenova_oauth_finish
        ])
        .setup(|app| {
            spawn_update_checker(app.handle());
            spawn_share_watcher(app.handle().clone());
            spawn_auto_refresh(app.handle().clone());
            spawn_reset_wake(app.handle().clone());
            let menu = build_tray_menu(app.handle())?;

            TrayIconBuilder::with_id("tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Pane")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => app.exit(0),
                        // Menu callbacks are synchronous main-thread code —
                        // building a window here deadlocks (see
                        // ensure_panel_window), so hop to the async runtime.
                        "open_panel" | "open_settings" => {
                            let handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = ensure_panel_window(&handle).await {
                                    eprintln!("[pane] tray open panel: {e}");
                                }
                            });
                        }
                        "show_popover" => {
                            toggle_popover_centered(app);
                            rebuild_tray_menu(app);
                        }
                        "refresh_all" => {
                            let handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                // Same courtesy as the frontend Refresh
                                // button: ordinary error benches must not
                                // stand between the click and a real fetch.
                                clear_soft_benches();
                                fetch_and_broadcast_usage(&handle).await;
                            });
                        }
                        "autostart" => {
                            use tauri_plugin_autostart::ManagerExt;
                            let enable = !autostart_now_enabled(app);
                            // Same pair as the set_autostart command: persist
                            // the choice, then flip the registry entry.
                            let _ = set_config_inner(json!({ "autostart": enable }));
                            let manager = app.autolaunch();
                            let result = if enable { manager.enable() } else { manager.disable() };
                            if let Err(e) = result {
                                eprintln!("[pane] tray autostart: {e}");
                            }
                            rebuild_tray_menu(app);
                        }
                        id if id.starts_with("tray_provider:") => {
                            // Provider rows are status shortcuts, not a
                            // second settings surface. Return to the live
                            // dashboard so the user can inspect that card.
                            toggle_popover_centered(app);
                            rebuild_tray_menu(app);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            position,
                            ..
                        } => toggle_popover(tray.app_handle(), position),
                        // Refresh check states (popover visible, autostart)
                        // right before the menu opens.
                        TrayIconEvent::Click {
                            button: MouseButton::Right,
                            button_state: MouseButtonState::Down,
                            ..
                        } => {
                            // The native menu is already attached to the tray.
                            // Do not replace it during the opening gesture.
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Leave the first WebView2 load at the normal memory target. Some
            // WebView2 versions can paint a blank/black surface when the
            // target is lowered before the hidden document finishes loading.
            // Later hide transitions still lower it after a real paint.
            if let Some(wv) = app.get_webview_window("main") {
                hide_window_border(&wv);
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
            if window.label() == "panel" {
                match event {
                    WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
                        schedule_panel_bounds_save(window.clone());
                    }
                    WindowEvent::CloseRequested { .. } => save_panel_bounds(window),
                    _ => {}
                }
                return;
            }
            if window.label() == "main" {
                if let WindowEvent::Focused(gained) = event {
                    if *gained {
                        LAST_FOCUS_GAINED_MS.store(now_ms(), Ordering::Relaxed);
                        return;
                    }
                    // When the pane process has no foreground rights (e.g.
                    // launched by an agent/automation), Windows grants the
                    // freshly shown window focus for a few milliseconds and
                    // immediately yanks it back. That gain→yank roundtrip is
                    // not "the user clicked away" — hiding on it is what made
                    // the popover impossible to summon. A real click-away
                    // happens after focus was held for a noticeable time.
                    let gained_at = LAST_FOCUS_GAINED_MS.load(Ordering::Relaxed);
                    if now_ms().saturating_sub(gained_at) < 500 {
                        return;
                    }
                    if let Some(wv) = window.app_handle().get_webview_window("main") {
                        dismiss_popover(&wv);
                    }
                    LAST_AUTO_HIDE_MS.store(now_ms(), Ordering::Relaxed);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{
        cached_extra_account_id_is_configured,
        cached_onenewapi_id_is_configured, card_is_disabled,
        commit_strip_state_after_apply, fail_state, load_config_from, set_config_in,
        forget_onenewapi_key_ids, forget_provider_snapshot,
        last_ok, onenewapi_after_site_save,
        onenewapi_apply_zero_to_one_enable, onenewapi_snapshot_generations, persist_last_ok_at,
        onenewapi_purge_restore_patch, purge_onenewapi_cards, purge_onenewapi_cards_coordinated,
        purge_onenewapi_cards_with, purge_onenewapi_from_config,
        rename_cached_snapshot, rename_cached_snapshot_in, rename_cached_snapshots_in,
        refresh_minutes_from,
        retain_current_onenewapi_results, strip_entry_application_order, strip_icon_ids_to_clear,
        strip_is_active, strip_reset_ids, updater_endpoint_strings, CachedSnap, FailState,
        OneNewApiMutationGuard, StripEntry,
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
    fn updater_points_at_own_github_releases() {
        // Edge-first ladder (CN mirror + direct edge), the project's own
        // GitHub releases last; no upstream project endpoint anywhere.
        let endpoints = updater_endpoint_strings();
        assert_eq!(endpoints.len(), 3, "ladder shape: {endpoints:?}");
        assert_eq!(
            endpoints.last().unwrap(),
            "https://github.com/Aafff623/pane/releases/latest/download/latest.json"
        );
        assert!(
            endpoints.iter().all(|e| !e.contains("ItsJazii")),
            "an upstream endpoint survived: {endpoints:?}"
        );
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
                rate_limited: false,
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
                rate_limited: false,
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
                rate_limited: false,
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
