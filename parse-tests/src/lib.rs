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

#[path = "../../src-tauri/src/oauth.rs"]
pub mod oauth;

#[path = "../../src-tauri/src/providers/mod.rs"]
pub mod providers;

#[path = "../../src-tauri/src/keyvault.rs"]
pub mod keyvault;

#[path = "../../src-tauri/src/pricing.rs"]
pub mod pricing;

#[path = "../../src-tauri/src/spend.rs"]
pub mod spend;

#[path = "../../src-tauri/src/usage_history.rs"]
pub mod usage_history;

#[path = "../../src-tauri/src/spend_history.rs"]
pub mod spend_history;

#[path = "../../src-tauri/src/provider_catalog.rs"]
pub mod provider_catalog;

#[path = "../../src-tauri/src/accounts.rs"]
pub mod accounts;

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

#[path = "../../src-tauri/src/auth_center.rs"]
pub mod auth_center;


mod catalog_consistency;
mod config_consistency;
