use arboard::{Clipboard, SetExtLinux};

/// the selection lives only as long as its owner, so a thread holds it until
/// another application takes the clipboard
#[tauri::command]
pub fn copy_text(text: String) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|err| err.to_string())?;
    std::thread::spawn(move || {
        let _ = clipboard.set().wait().text(text);
    });
    Ok(())
}
