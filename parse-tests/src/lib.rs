//! Compiles the real provider sources so their own `#[cfg(test)]` modules
//! run here. Nothing is copied — every `#[path]` below points at the file
//! the app ships, so an assertion that passes here is an assertion about
//! production code.

#[path = "../../src-tauri/src/platform/mod.rs"]
pub mod platform;

#[path = "../../src-tauri/src/i18n.rs"]
pub mod i18n;

#[path = "../../src-tauri/src/alerts.rs"]
pub mod alerts;

#[path = "../../src-tauri/src/pace.rs"]
pub mod pace;

#[path = "../../src-tauri/src/resets.rs"]
pub mod resets;

#[path = "../../src-tauri/src/oauth.rs"]
pub mod oauth;

#[path = "../../src-tauri/src/oauth_util.rs"]
pub mod oauth_util;

#[path = "../../src-tauri/src/providers/mod.rs"]
pub mod providers;

#[path = "../../src-tauri/src/keyvault.rs"]
pub mod keyvault;
#[path = "../../src-tauri/src/keyvault_v3/mod.rs"]
pub mod keyvault_v3;
#[path = "../../src-tauri/src/search_keys.rs"]
pub mod search_keys;

#[path = "../../src-tauri/src/pricing.rs"]
pub mod pricing;

#[path = "../../src-tauri/src/pricing_timeline.rs"]
pub mod pricing_timeline;

#[path = "../../src-tauri/src/refresh_gate.rs"]
pub mod refresh_gate;

#[path = "../../src-tauri/src/adaptive_interval.rs"]
pub mod adaptive_interval;

#[path = "../../src-tauri/src/account_token_lock.rs"]
pub mod account_token_lock;

#[path = "../../src-tauri/src/error.rs"]
pub mod error;

#[path = "../../src-tauri/src/share_image.rs"]
pub mod share_image;

#[path = "../../src-tauri/src/fetch_strategy.rs"]
pub mod fetch_strategy;

#[path = "../../src-tauri/src/identity_guard.rs"]
pub mod identity_guard;

#[path = "../../src-tauri/src/meter_samples.rs"]
pub mod meter_samples;

#[path = "../../src-tauri/src/limits.rs"]
pub mod limits;

#[path = "../../src-tauri/src/skill_usage.rs"]
pub mod skill_usage;

#[path = "../../src-tauri/src/wsl.rs"]
pub mod wsl;

#[path = "../../src-tauri/src/spend.rs"]
pub mod spend;

#[path = "../../src-tauri/src/qoder_sessions.rs"]
pub mod qoder_sessions;

#[path = "../../src-tauri/src/source_probe.rs"]
pub mod source_probe;

#[path = "../../src-tauri/src/httpapi.rs"]
pub mod httpapi;

#[path = "../../src-tauri/src/agent_skill.rs"]
pub mod agent_skill;

#[path = "../../src-tauri/src/skin_packs.rs"]
pub mod skin_packs;

#[path = "../../src-tauri/src/font_packs.rs"]
pub mod font_packs;

#[path = "../../src-tauri/src/usage_history.rs"]
pub mod usage_history;

#[path = "../../src-tauri/src/usage_units.rs"]
pub mod usage_units;

#[path = "../../src-tauri/src/spend_history.rs"]
pub mod spend_history;

#[path = "../../src-tauri/src/rtk_savings.rs"]
pub mod rtk_savings;

#[path = "../../src-tauri/src/provider_catalog.rs"]
pub mod provider_catalog;

#[path = "../../src-tauri/src/accounts.rs"]
pub mod accounts;

#[path = "../../src-tauri/src/backup.rs"]
pub mod backup;

#[path = "../../src-tauri/src/identity.rs"]
pub mod identity;

#[path = "../../src-tauri/src/secretstore.rs"]
pub mod secretstore;

#[path = "../../src-tauri/src/antigravity_accounts.rs"]
pub mod antigravity_accounts;

#[path = "../../src-tauri/src/cursor_oauth.rs"]
pub mod cursor_oauth;

#[path = "../../src-tauri/src/cursor_accounts.rs"]
pub mod cursor_accounts;

#[path = "../../src-tauri/src/codex_accounts.rs"]
pub mod codex_accounts;

#[path = "../../src-tauri/src/codex_login.rs"]
pub mod codex_login;

#[path = "../../src-tauri/src/login_accounts.rs"]
pub mod login_accounts;

#[path = "../../src-tauri/src/local_logins.rs"]
pub mod local_logins;

#[path = "../../src-tauri/src/kiro_login.rs"]
pub mod kiro_login;

#[path = "../../src-tauri/src/qoder_login.rs"]
pub mod qoder_login;

#[path = "../../src-tauri/src/codebuddy_login.rs"]
pub mod codebuddy_login;

#[path = "../../src-tauri/src/windsurf_login.rs"]
pub mod windsurf_login;

#[path = "../../src-tauri/src/trae_login.rs"]
pub mod trae_login;

#[path = "../../src-tauri/src/zed_login.rs"]
pub mod zed_login;

#[path = "../../src-tauri/src/auth_center.rs"]
pub mod auth_center;

#[path = "../../src-tauri/src/tray_projection.rs"]
pub mod tray_projection;

mod catalog_consistency;
mod recovery_consistency;
mod config_consistency;

#[path = "../../src-tauri/src/snapshot_store.rs"]
pub mod snapshot_store;

#[path = "../../src-tauri/src/provider_dispatch.rs"]
pub mod provider_dispatch;

#[path = "../../src-tauri/src/account_lifecycle.rs"]
pub mod account_lifecycle;

#[path = "../../src-tauri/src/popover_geometry.rs"]
pub mod popover_geometry;
