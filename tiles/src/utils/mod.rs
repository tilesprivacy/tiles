use std::time::{SystemTime, UNIX_EPOCH};

pub mod config;
pub mod crypto;
pub mod hf_model_downloader;
pub mod installer;
pub fn get_unix_time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64
}

pub fn test_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}
