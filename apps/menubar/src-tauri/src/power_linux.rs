//! power state from sysfs, and staying awake through systemd-inhibit

use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::awake::DevicePower;

const SUPPLIES: &str = "/sys/class/power_supply";

/// ends with the app, or at the deadline (epoch seconds, 0 for none)
const HOLD: &str = r#"while kill -0 "$1" 2>/dev/null; do
  [ "$2" -gt 0 ] && [ "$(date +%s)" -ge "$2" ] && exit 0
  sleep 5
done"#;

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
}

pub fn device_power() -> DevicePower {
    let Ok(entries) = std::fs::read_dir(SUPPLIES) else {
        return DevicePower {
            plugged_in: true,
            battery_percent: None,
        };
    };

    let mut mains_online = None;
    let mut battery_percent = None;
    for entry in entries.flatten() {
        let dir = entry.path();
        match read(&dir.join("type")).as_deref() {
            Some("Battery") => {
                battery_percent = read(&dir.join("capacity")).and_then(|c| c.parse::<u8>().ok());
            }
            Some(_) => {
                let online = read(&dir.join("online")).as_deref() == Some("1");
                mains_online = Some(mains_online.unwrap_or(false) || online);
            }
            None => {}
        }
    }

    DevicePower {
        // no battery is a desktop
        plugged_in: mains_online.unwrap_or(battery_percent.is_none()),
        battery_percent,
    }
}

pub fn inhibitor(left_ms: Option<u64>) -> Command {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let deadline = left_ms.map_or(0, |ms| now + ms.div_ceil(1000).max(1));

    let mut command = Command::new("systemd-inhibit");
    command
        .args([
            "--what=idle:sleep",
            "--who=Tiles",
            "--why=Stay awake",
            "--mode=block",
        ])
        .args(["sh", "-c", HOLD, "tiles-awake"])
        .arg(std::process::id().to_string())
        .arg(deadline.to_string());
    command
}
