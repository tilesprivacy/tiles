//! power state from IOKit, and staying awake through caffeinate

use std::process::Command;

use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::{CFString, CFStringRef};

use crate::awake::DevicePower;

/// the tool holds the assertion, and `-w` releases it even on a kill we never
/// see coming, which a Drop impl cannot promise
const CAFFEINATE: &str = "/usr/bin/caffeinate";

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    fn IOPSCopyPowerSourcesList(blob: CFTypeRef) -> CFArrayRef;
    fn IOPSGetPowerSourceDescription(blob: CFTypeRef, source: CFTypeRef) -> CFDictionaryRef;
    fn IOPSGetProvidingPowerSourceType(blob: CFTypeRef) -> CFStringRef;
}

/// the snapshot and list are copy-rule values wrapped so every early return
/// releases them; the descriptions stay owned by IOKit
pub fn device_power() -> DevicePower {
    unsafe {
        let snapshot_ref = IOPSCopyPowerSourcesInfo();
        if snapshot_ref.is_null() {
            return DevicePower::default();
        }
        let snapshot = CFType::wrap_under_create_rule(snapshot_ref);

        let source_ref = IOPSGetProvidingPowerSourceType(snapshot.as_CFTypeRef());
        let plugged_in = !source_ref.is_null()
            && CFString::wrap_under_get_rule(source_ref).to_string() == "AC Power";

        let list_ref = IOPSCopyPowerSourcesList(snapshot.as_CFTypeRef());
        if list_ref.is_null() {
            return DevicePower {
                plugged_in,
                battery_percent: None,
            };
        }
        let sources: CFArray<CFType> = CFArray::wrap_under_create_rule(list_ref);
        let type_key = CFString::new("Type");
        let current_key = CFString::new("Current Capacity");
        let maximum_key = CFString::new("Max Capacity");

        let battery_percent = sources.iter().find_map(|source| {
            let description_ref =
                IOPSGetPowerSourceDescription(snapshot.as_CFTypeRef(), source.as_CFTypeRef());
            if description_ref.is_null() {
                return None;
            }
            let description: CFDictionary<CFString, CFType> =
                CFDictionary::wrap_under_get_rule(description_ref);

            let source_type = description
                .find(&type_key)
                .and_then(|value| value.downcast::<CFString>())?;
            if source_type.to_string() != "InternalBattery" {
                return None;
            }

            let number = |key: &CFString| {
                description
                    .find(key)
                    .and_then(|value| value.downcast::<CFNumber>())
                    .and_then(|value| value.to_i32())
            };
            let current = number(&current_key)?;
            let maximum = number(&maximum_key)?;
            (maximum > 0).then(|| (current.clamp(0, maximum) * 100 / maximum) as u8)
        });

        DevicePower {
            plugged_in,
            battery_percent,
        }
    }
}

pub fn inhibitor(left_ms: Option<u64>) -> Command {
    let mut command = Command::new(CAFFEINATE);
    // `-i` keeps the system up and leaves the display free to sleep
    command.arg("-i");
    if let Some(left) = left_ms {
        // the kernel holds the deadline, so it survives a sleep the way a
        // timer of our own would not
        command.args(["-t", &left.div_ceil(1000).max(1).to_string()]);
    }
    // no utility to run, so the assertion stands until the tool exits
    command.args(["-w", &std::process::id().to_string()]);
    command
}
