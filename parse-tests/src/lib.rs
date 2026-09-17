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
