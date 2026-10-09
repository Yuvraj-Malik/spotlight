mod ai;
mod aliases;
mod commands;
mod icons;
mod providers;
mod ranking;
mod reminders;
mod timeparse;
mod window;

use parking_lot::RwLock;
use std::sync::Arc;
use tauri::Manager;

/// Everything search needs, kept in RAM so each keystroke is just a memory scan.
pub struct AppState {
    pub apps: RwLock<Vec<providers::apps::AppEntry>>,
    pub usage: ranking::Usage,
    pub aliases: aliases::Aliases,
    pub icon_dir: std::path::PathBuf,
    pub ai: ai::AiConfig,
    pub reminders: reminders::Reminders,
}

pub fn run() {
    // Load keys from the project's .env file (searched from the current folder upward).
    let _ = dotenvy::dotenv();

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let icon_dir = data_dir.join("icons");
            let state = Arc::new(AppState {
                apps: RwLock::new(Vec::new()),
                usage: ranking::Usage::load(data_dir.clone()),
                aliases: aliases::Aliases::load(data_dir.clone()),
                ai: ai::AiConfig::load(data_dir.clone()),
                reminders: reminders::Reminders::load(data_dir),
                icon_dir: icon_dir.clone(),
            });
            app.manage(state.clone());
            reminders::start(app.handle().clone(), state.clone());

            // Load the AI model in the background so the first sentence isn't slow.
            let ai_cfg = state.ai.get();
            if ai_cfg.enabled {
                std::thread::spawn(move || ai::warmup(&ai_cfg.model));
            }

            // Build the app index off the main thread so startup stays instant,
            // then refresh it every few minutes to pick up newly installed apps.
            std::thread::spawn(move || loop {
                let apps = providers::apps::scan();
                println!("Indexed {} apps", apps.len());
                let targets: Vec<String> = apps.iter().map(|a| a.path.clone()).collect();
                *state.apps.write() = apps;
                // Warm the icon cache so results appear with icons immediately.
                for t in targets {
                    icons::get(&t, &icon_dir);
                }
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
            commands::get_icon,
            commands::ai_interpret,
            commands::ai_answer,
            commands::snooze_reminder,
            commands::ai_models,
            commands::get_ai_settings,
            commands::set_ai_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Spotlight");
}
