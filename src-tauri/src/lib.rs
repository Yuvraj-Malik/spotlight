mod aliases;
mod commands;
mod providers;
mod ranking;
mod window;

use parking_lot::RwLock;
use std::sync::Arc;
use tauri::Manager;

/// Everything search needs, kept in RAM so each keystroke is just a memory scan.
pub struct AppState {
    pub apps: RwLock<Vec<providers::apps::AppEntry>>,
    pub usage: ranking::Usage,
    pub aliases: aliases::Aliases,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let state = Arc::new(AppState {
                apps: RwLock::new(Vec::new()),
                usage: ranking::Usage::load(data_dir.clone()),
                aliases: aliases::Aliases::load(data_dir),
            });
            app.manage(state.clone());

            // Build the app index off the main thread so startup stays instant,
            // then refresh it every few minutes to pick up newly installed apps.
            std::thread::spawn(move || loop {
                let apps = providers::apps::scan();
                println!("Indexed {} apps", apps.len());
                *state.apps.write() = apps;
                std::thread::sleep(std::time::Duration::from_secs(300));
            });

            window::setup_hotkey(app)?;
            window::setup_tray(app)?;
            window::hide_on_blur(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::execute,
            commands::hide_window,
            commands::open_settings,
            commands::list_aliases,
            commands::set_alias,
            commands::remove_alias,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Spotlight");
}
