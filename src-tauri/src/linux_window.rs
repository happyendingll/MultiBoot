// This module is compiled only on Linux; other platforms retain their window behavior.

pub fn refresh_decorations(window: &tauri::Window) -> tauri::Result<()> {
    let wayland_session = std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|session| session == "wayland");
    if !wayland_session || std::env::var("GDK_BACKEND").is_ok_and(|backend| backend == "x11") {
        return Ok(());
    }

    // Tao 0.35.3 can leave Wayland title-bar buttons unresponsive after showing
    // a window created with visible(false), including subsequent tray restores.
    // Refresh the decorations after focus, when the window has been mapped.
    // https://github.com/tauri-apps/tauri/issues/11856
    // Preserve intentionally fixed-size windows and avoid a maximize/restore cycle.
    if window.is_resizable()? {
        window.set_resizable(false)?;
        window.set_resizable(true)?;
    }
    Ok(())
}
