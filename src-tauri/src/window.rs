use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Show the pre-created window (no cold start) or hide it if already open.
pub fn toggle(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
    } else {
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_focus();
        let _ = app.emit("spotlight://shown", ());
    }
}

pub fn setup_hotkey(app: &App) -> tauri::Result<()> {
    // Try these in order; the first one not already taken by another app wins.
    let candidates = [
        Shortcut::new(Some(Modifiers::ALT), Code::Space),
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space),
        Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::Space),
    ];
    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, _shortcut, event| {
                if event.state() == ShortcutState::Pressed {
                    toggle(app);
                }
            })
            .build(),
    )?;
    for hk in candidates {
        match app.global_shortcut().register(hk) {
            Ok(()) => {
                println!("Spotlight hotkey: {hk:?}");
                return Ok(());
            }
            Err(e) => eprintln!("Hotkey {hk:?} unavailable: {e}"),
        }
    }
    eprintln!("No hotkey available; use the tray icon to open Spotlight.");
    Ok(())
}

pub fn setup_tray(app: &App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Spotlight", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &quit])?;
    let mut tray = TrayIconBuilder::new()
        .tooltip("Spotlight (Alt+Space)")
        .menu(&menu)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => toggle(app),
            "settings" => open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Like Spotlight: clicking anywhere else dismisses it.
pub fn hide_on_blur(app: &App) {
    if let Some(win) = app.get_webview_window("main") {
        let w = win.clone();
        win.on_window_event(move |e| {
            if let WindowEvent::Focused(false) = e {
                let _ = w.hide();
            }
        });
    }
}

/// Open (or focus) the Settings window. It's a normal window, unlike the search bar.
pub fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html?view=settings".into()))
        .title("Spotlight Settings")
        .inner_size(760.0, 580.0)
        .min_inner_size(560.0, 420.0)
        .center()
        .build();
}
