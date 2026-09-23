mod account;
mod atproto;
mod awake;
mod boot;
mod daemon;
mod deeplink;
mod inference;
mod lifeline;
mod paths;
mod remote;
mod sessions;
mod ui;

#[cfg(target_os = "macos")]
mod clipboard;
#[cfg(target_os = "macos")]
#[path = "lid_macos.rs"]
mod lid;
#[cfg(target_os = "macos")]
mod panel;
#[cfg(target_os = "macos")]
#[path = "power_macos.rs"]
mod power;
#[cfg(target_os = "macos")]
mod quit;
#[cfg(target_os = "macos")]
mod tray;

#[cfg(target_os = "linux")]
#[path = "clipboard_linux.rs"]
mod clipboard;
#[cfg(target_os = "linux")]
#[path = "lid_linux.rs"]
mod lid;
#[cfg(target_os = "linux")]
#[path = "panel_linux.rs"]
mod panel;
#[cfg(target_os = "linux")]
#[path = "power_linux.rs"]
mod power;
#[cfg(target_os = "linux")]
#[path = "quit_linux.rs"]
mod quit;
#[cfg(target_os = "linux")]
#[path = "tray_linux.rs"]
mod tray;

use tauri::{Manager, RunEvent, WindowEvent};

/// webkitgtk's dmabuf renderer trips explicit sync on nvidia under wayland
/// and the compositor disconnects us. must run before gtk initialises
#[cfg(target_os = "linux")]
fn accommodate_nvidia_wayland() {
    let nvidia = std::path::Path::new("/proc/driver/nvidia").exists();
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    if nvidia && wayland && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    accommodate_nvidia_wayland();

    let builder = tauri::Builder::default()
        // has to be registered first, so a second copy exits before it builds a
        // status item of its own. a daemon-owned copy displaces a manual one
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if lifeline::should_yield_to(&argv) {
                // the copy taking over opens the link this one held
                deeplink::stash(app);
                app.exit(0);
                return;
            }

            // the deep link plugin already took it
            if deeplink::in_args(&argv) {
                return;
            }

            // launching again is someone asking for the window back
            let _ = ui::open(app, "/");
        }))
        // after single instance, which forwards argv links to it
        .plugin(tauri_plugin_deep_link::init());

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
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
            if let Err(err) = tray::init(app.handle()) {
                eprintln!("[tray] no status item: {err}");
            }
            // before the watcher, its first tick already reports all three
            inference::init(app.handle());
            account::init(app.handle());
            atproto::init(app.handle());
            sessions::init(app.handle());
            remote::init(app.handle());
            awake::init(app.handle());
            daemon::init(app.handle());

            panel::warm_up(app.handle());
            ui::init(app.handle());
            deeplink::init(app.handle());
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
        .build(tauri::generate_context!())
        .expect("failed to start the Tiles menu bar app")
        .run(|app, event| match event {
            // the delegate only exists once the loop is up
            RunEvent::Ready => quit::init(app),
            // the dock's quit and cmd-q both land here, and killing the process
            // on the spot only gets us restarted by the daemon that owns us.
            // quitting is the daemon's to do, same as the tray item
            RunEvent::ExitRequested {
                code: None, api, ..
            } => {
                api.prevent_exit();
                daemon::quit(app);
            }
            // clicking the dock icon with the window closed. nothing else
            // brings it back, the tray item aside
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => ui::reopen(app),
            // the lifeline and a daemon quit both end here
            RunEvent::Exit => awake::shutdown(app),
            _ => {}
        });
}
