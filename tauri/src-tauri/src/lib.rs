use tauri::AppHandle;
use tauri_plugin_tvmate::{FullScreenRequest, TvmateExt};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(app_handle: AppHandle, name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn fullscreen(app_handle: AppHandle) -> String {
    let result = app_handle.tvmate().fullscreen(FullScreenRequest {});
    return format!("{result:?}");
}

#[tauri::command]
fn is_fullscreen(app_handle: AppHandle) -> bool {
    let result = app_handle.tvmate().is_fullscreen();

    result.map(|r| r.is_fullscreen).unwrap_or_default()
}

#[tauri::command]
fn exit_fullscreen(app_handle: AppHandle) -> String {
    let result = app_handle.tvmate().exit_fullscreen();
    return format!("{result:?}");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_tvmate::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            fullscreen,
            is_fullscreen,
            exit_fullscreen
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
