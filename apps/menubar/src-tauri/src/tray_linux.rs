//! a StatusNotifierItem. the host owns the click, so everything is a menu entry

use tauri::AppHandle;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::{daemon, panel, ui};

const ID: &str = "tiles";

static LIVE: &[u8] = include_bytes!("../icons/tray-live.png");
static IDLE: &[u8] = include_bytes!("../icons/tray-idle.png");

fn icon(bytes: &[u8]) -> tauri::Result<Image<'static>> {
    Image::from_bytes(bytes).map(Image::to_owned)
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Tiles", true, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", "Status", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Tiles", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&open, &status, &PredefinedMenuItem::separator(app)?, &quit],
    )?;

    TrayIconBuilder::with_id(ID)
        .icon(icon(IDLE)?)
        .tooltip("Tiles")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                if let Err(err) = ui::open(app, "/") {
                    eprintln!("[tray] could not open the chat window: {err}");
                }
            }
            "status" => panel::toggle(app),
            "quit" => daemon::quit(app),
            _ => {}
        })
        // only x11 hosts without an indicator deliver clicks
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                panel::toggle(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn pointer_over_item(_app: &AppHandle) -> bool {
    false
}

pub fn set_live(app: &AppHandle, live: bool) {
    let Some(tray) = app.tray_by_id(ID) else {
        return;
    };
    if let Ok(image) = icon(if live { LIVE } else { IDLE }) {
        let _ = tray.set_icon(Some(image));
    }
}
