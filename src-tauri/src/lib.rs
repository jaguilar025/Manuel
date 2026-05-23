mod models;
mod services;
mod commands;

use std::sync::Arc;

use parking_lot::RwLock;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

pub struct AppState {
    pub config: RwLock<models::Config>,
    pub engine: Arc<services::engine::Engine>,
    pub recorder: Arc<services::recorder::Recorder>,
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Launch ydotoold as a child of this process; gets killed when we exit.
    services::ydotoold::start();

    let config = services::storage::load_or_default();
    let engine = Arc::new(services::engine::Engine::new());
    engine.apply(&config);

    let recorder = Arc::new(services::recorder::Recorder::new());

    let state = AppState {
        config: RwLock::new(config),
        engine: engine.clone(),
        recorder: recorder.clone(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(move |app| {
            // Tray
            let toggle_i = MenuItem::with_id(app, "toggle", "Show/Hide", true, None::<&str>)?;
            let quit_i   = MenuItem::with_id(app, "quit",   "Exit",       true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_i, &quit_i])?;

            // Use the bundled window icon if present; otherwise fall back to a 1x1 stub.
            let icon = app
                .default_window_icon()
                .cloned()
                .unwrap_or_else(|| Image::new_owned(vec![0, 0, 0, 0], 1, 1));

            let _tray = TrayIconBuilder::with_id("main")
                .tooltip("Manuel")
                .icon(icon)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle" => { let _ = services::window::toggle(app); }
                    "quit"   => { app.exit(0); }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let _ = services::window::toggle(tray.app_handle());
                    }
                })
                .build(app)?;

            // Recorder event bridge → frontend
            let app_handle = app.handle().clone();
            recorder.set_emitter(Box::new(move |trigger| {
                let _ = app_handle.emit("input-recorded", trigger);
            }));

            // Engine event bridge for device changes
            let app_handle2 = app.handle().clone();
            engine.set_devices_emitter(Box::new(move |devices| {
                let _ = app_handle2.emit("devices-changed", devices);
            }));

            // Start initial state
            let cfg = app.state::<AppState>().config.read().clone();
            if cfg.settings.start_minimized {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing main window only hides it (background mode)
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.config.read().settings.run_in_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::list_devices,
            commands::start_recording,
            commands::stop_recording,
            commands::reload_engine,
            commands::test_output,
            commands::set_autostart,
            commands::is_autostart_enabled,
            commands::hide_window,
            commands::show_window,
            commands::open_config,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                services::ydotoold::stop();
            }
        });
}

