mod account;
mod atproto;
mod awake;
mod boot;
mod clipboard;
mod daemon;
mod inference;
mod lifeline;
mod panel;
mod paths;
mod remote;
mod sessions;
mod tray;
mod ui;

use tauri::{Manager, WindowEvent};

fn main() {
    tauri::Builder::default()
        // has to be registered first, so a second copy exits before it builds a
        // status item of its own. a daemon-owned copy displaces a manual one
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if lifeline::should_yield_to(&argv) {
                app.exit(0);
                return;
            }

            // launching again is someone asking for the window back
            let _ = ui::open(app, "/");
        }))
        .plugin(tauri_nspanel::init())
        .invoke_handler(tauri::generate_handler![
            panel::hide_panel,
            panel::panel_ready,
            panel::resize_panel,
            panel::quit_app,
            paths::data_dir,
            paths::reveal_path,
            clipboard::copy_text,
            daemon::daemon_health,
            inference::inference_state,
            inference::inference_set,
            account::account_state,
            atproto::atproto_state,
            atproto::atproto_login,
            sessions::sessions_state,
            remote::remote_state,
            remote::remote_set,
            awake::awake_state,
            awake::awake_start,
            awake::awake_stop,
            awake::awake_pause,
            awake::awake_resume,
            ui::open_session,
            ui::open_ui
        ])
        .setup(|app| {
            // first, so a daemon that dies mid-setup still takes us with it
            lifeline::init(app.handle());
            panel::init(app.handle())?;
            tray::init(app.handle())?;
            // before the watcher, its first tick already reports all three
            inference::init(app.handle());
            account::init(app.handle());
            atproto::init(app.handle());
            sessions::init(app.handle());
            remote::init(app.handle());
            // before the watcher, whose every pass reconciles it
            awake::init(app.handle());
            daemon::init(app.handle());

            panel::warm_up(app.handle());
            ui::init(app.handle());
            boot::init(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            // has to stay a WindowEvent, nspanel's set_event_handler replaces
            // Tauri's NSWindowDelegate instead of chaining and kills this
            if matches!(event, WindowEvent::Focused(false)) && window.label() == panel::LABEL {
                let app = window.app_handle();
                let mode = if tray::pointer_over_item(app) {
                    panel::Dismiss::Fade
                } else {
                    panel::Dismiss::Instant
                };
                panel::dismiss(app, mode);
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to start the Tiles menu bar app");
}
