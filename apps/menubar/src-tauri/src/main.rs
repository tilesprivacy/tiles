mod account;
mod clipboard;
mod daemon;
mod inference;
mod lifeline;
mod panel;
mod paths;
mod remote;
mod sessions;
mod tray;

use tauri::{ActivationPolicy, Manager, WindowEvent};

fn main() {
    tauri::Builder::default()
        // has to be registered first, so a second copy exits before it builds a
        // status item of its own. a daemon-owned copy displaces a manual one
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if lifeline::should_yield_to(&argv) {
                app.exit(0);
            }
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
            sessions::sessions_state,
            remote::remote_state,
            remote::remote_set
        ])
        .setup(|app| {
            // LSUIElement covers the launch window before this runs
            app.set_activation_policy(ActivationPolicy::Accessory);

            // first, so a daemon that dies mid-setup still takes us with it
            lifeline::init(app.handle());
            panel::init(app.handle())?;
            tray::init(app.handle())?;
            // before the watcher, its first tick already reports all three
            inference::init(app.handle());
            account::init(app.handle());
            sessions::init(app.handle());
            remote::init(app.handle());
            daemon::init(app.handle());

            panel::warm_up(app.handle());

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
