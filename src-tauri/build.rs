fn main() {
    // Debug builds must not share the installed app's identifier: the
    // single-instance mutex and the WebView2 data directory are derived from
    // it, so a dev launch would toggle the installed popover instead of
    // opening its own — and both would fight over one WebView2 profile.
    //
    // TAURI_CONFIG is merged over tauri.conf.json by both tauri-build and
    // generate_context!, so the override applies to plain `cargo build` as
    // well as to the Tauri CLI. Release builds keep the shipped identifier.
    if std::env::var("PROFILE").as_deref() == Ok("debug") {
        let mut patch = std::env::var("TAURI_CONFIG")
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .filter(serde_json::Value::is_object)
            .unwrap_or_else(|| serde_json::json!({}));
        patch["identifier"] = serde_json::json!("com.aafff623.pane.dev");
        let patch = patch.to_string();
        std::env::set_var("TAURI_CONFIG", &patch);
        // The proc macro reads the same variable while compiling lib.rs.
        println!("cargo:rustc-env=TAURI_CONFIG={patch}");
    }
    tauri_build::build()
}
