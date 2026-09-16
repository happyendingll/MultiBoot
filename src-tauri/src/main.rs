// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
fn relaunch_wayland_appimage_with_system_webkit() {
    use std::{
        collections::HashSet,
        env,
        os::unix::process::CommandExt,
        path::PathBuf,
        process::Command,
    };

    const RELAUNCHED: &str = "MULTIBOOT_APPIMAGE_SYSTEM_WEBKIT";
    const GTK_APPIMAGE_VARIABLES: &[&str] = &[
        "GDK_BACKEND",
        "GDK_PIXBUF_MODULE_FILE",
        "GIO_EXTRA_MODULES",
        "GSETTINGS_SCHEMA_DIR",
        "GST_PLUGIN_SYSTEM_PATH",
        "GST_PLUGIN_SYSTEM_PATH_1_0",
        "GTK_DATA_PREFIX",
        "GTK_EXE_PREFIX",
        "GTK_IM_MODULE_FILE",
        "GTK_PATH",
        "GTK_THEME",
    ];

    let wayland_session = env::var_os("WAYLAND_DISPLAY").is_some()
        || env::var("XDG_SESSION_TYPE").is_ok_and(|session| session == "wayland");
    if !wayland_session || env::var_os("APPIMAGE").is_none() || env::var_os(RELAUNCHED).is_some() {
        return;
    }

    let Some(app_dir) = env::var_os("APPDIR").map(PathBuf::from) else {
        return;
    };

    // Tauri's legacy AppImage bundler includes the build machine's GTK/WebKit
    // stack. On newer Wayland/Mesa systems, its old libwayland conflicts with
    // the host EGL driver and WebKitWebProcess exits with EGL_BAD_PARAMETER,
    // leaving a blank settings window. Prefer the host's coherent, updated
    // GTK/WebKit stack while retaining the AppImage executable and resources.
    let system_library_dirs = [
        PathBuf::from("/usr/lib/x86_64-linux-gnu"),
        PathBuf::from("/lib/x86_64-linux-gnu"),
        PathBuf::from("/usr/lib/aarch64-linux-gnu"),
        PathBuf::from("/lib/aarch64-linux-gnu"),
        PathBuf::from("/usr/lib64"),
        PathBuf::from("/lib64"),
        PathBuf::from("/usr/lib"),
        PathBuf::from("/lib"),
    ];
    let system_library_dirs = system_library_dirs
        .into_iter()
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    if !system_library_dirs
        .iter()
        .any(|path| path.join("libwebkit2gtk-4.1.so.0").exists())
    {
        return;
    }

    let mut library_dirs = env::var_os("LD_LIBRARY_PATH")
        .map(|value| {
            env::split_paths(&value)
                .filter(|path| !path.starts_with(&app_dir))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    library_dirs.extend(system_library_dirs);
    let mut seen = HashSet::new();
    library_dirs.retain(|path| seen.insert(path.clone()));

    let Ok(library_path) = env::join_paths(library_dirs) else {
        return;
    };
    let Ok(executable) = env::current_exe() else {
        return;
    };

    let mut command = Command::new(executable);
    command
        .args(env::args_os().skip(1))
        .env(RELAUNCHED, "1")
        .env("LD_LIBRARY_PATH", library_path);
    for variable in GTK_APPIMAGE_VARIABLES {
        command.env_remove(variable);
    }

    let error = command.exec();
    eprintln!("无法使用系统 WebKitGTK 重新启动 Linux AppImage：{error}");
}

fn main() {
    #[cfg(target_os = "linux")]
    relaunch_wayland_appimage_with_system_webkit();

    multiboot_lib::run()
}
