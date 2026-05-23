use tauri::{AppHandle, Manager};

pub fn show(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        w.show()?;
        w.set_focus()?;
        w.unminimize().ok();
    }
    Ok(())
}

pub fn hide(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        w.hide()?;
    }
    Ok(())
}

pub fn toggle(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            w.hide()?;
        } else {
            w.show()?;
            w.set_focus()?;
        }
    }
    Ok(())
}
