use tauri::{AppHandle, State};
use crate::{models::{Config, DeviceInfo, Output}, services};
use crate::AppState;

#[tauri::command]
pub fn load_config(state: State<AppState>) -> Config {
    state.config.read().clone()
}

#[tauri::command]
pub fn save_config(state: State<AppState>, config: Config) -> Result<(), String> {
    services::storage::save(&config).map_err(|e| e.to_string())?;
    *state.config.write() = config;
    Ok(())
}

#[tauri::command]
pub fn list_devices() -> Vec<DeviceInfo> {
    services::hid::enumerate()
}

#[tauri::command]
pub fn start_recording(state: State<AppState>) {
    state.engine.attach_recorder(state.recorder.clone());
    state.recorder.arm();
}

#[tauri::command]
pub fn stop_recording(state: State<AppState>) {
    state.recorder.stop();
}

#[tauri::command]
pub fn reload_engine(state: State<AppState>) {
    let cfg = state.config.read().clone();
    state.engine.apply(&cfg);
    state.engine.attach_recorder(state.recorder.clone());
}

#[tauri::command]
pub fn test_output(output: Output) -> Result<(), String> {
    services::macro_runner::run(&output).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    if enabled {
        services::autostart::enable().map_err(|e| e.to_string())
    } else {
        services::autostart::disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn is_autostart_enabled() -> bool {
    services::autostart::is_enabled()
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), String> {
    services::window::hide(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn show_window(app: AppHandle) -> Result<(), String> {
    services::window::show(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_config(reveal: bool) -> Result<(), String> {
    let path = if reveal {
        services::storage::config_dir()
    } else {
        services::storage::config_path()
    };
    std::process::Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("xdg-open failed: {e}"))?;
    Ok(())
}
