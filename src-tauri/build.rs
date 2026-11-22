fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let profile = std::env::var("PROFILE").unwrap_or_default();
    let force_admin = std::env::var("NEXTBOOT_REQUIRE_ADMIN")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // Embed requireAdministrator UAC manifest in Windows release builds so double-clicking
    // prompts UAC once at launch. In debug builds (tauri dev), skip manifest embedding so
    // non-elevated dev terminals can spawn the app without Win32 error 740, and the UI offers
    // a one-click "Re-launch as Administrator" button instead.
    if target_os == "windows" && (profile == "release" || force_admin) {
        let windows = tauri_build::WindowsAttributes::new()
            .app_manifest(include_str!("windows-app.manifest"));
        let attrs = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attrs).expect("failed to run tauri-build with Windows UAC manifest");
    } else {
        tauri_build::build();
    }
}
