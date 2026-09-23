//! free space on the volume models are kept on

/// Free and total bytes on the volume holding the model downloads.
#[cfg(unix)]
// macOS reports block counts as u32, linux as u64
#[allow(clippy::useless_conversion)]
pub fn model_volume_space() -> Option<(u64, u64)> {
    let dir = crate::utils::config::get_or_create_model_download_path().ok()?;
    let stat = nix::sys::statvfs::statvfs(&dir).ok()?;
    let unit = u64::from(stat.fragment_size());
    Some((
        u64::from(stat.blocks_available()) * unit,
        u64::from(stat.blocks()) * unit,
    ))
}

#[cfg(not(unix))]
pub fn model_volume_space() -> Option<(u64, u64)> {
    None
}
