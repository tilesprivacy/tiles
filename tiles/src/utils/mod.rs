use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub mod config;
pub mod crypto;
pub mod hf_model_downloader;
pub mod installer;
pub mod uninstaller;
pub fn get_unix_time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64
}

pub fn test_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub fn copy_recursive(src: &PathBuf, dest: &PathBuf) -> anyhow::Result<()> {
    fs::create_dir_all(dest)?;

    let ls = fs::read_dir(src)?;

    for dir_result in ls {
        let dir_path = dir_result?;
        let file_type = dir_path.file_type()?;
        let filename = dir_path.file_name();
        let src_path = dir_path.path();
        let dest_path = PathBuf::new().join(dest).join(filename);
        if file_type.is_symlink() {
            anyhow::bail!("Refusing to copy symlink entry {:?}", src_path);
        } else if file_type.is_dir() {
            copy_recursive(&src_path, &dest_path)?
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_valid_copy() {
        let tmp = tempdir().expect("created tmp dir");

        let src = tmp.path().join("source");
        let dest = tmp.path().join("dest");

        fs::create_dir_all(src.join("skills").join("flamez")).unwrap();

        fs::write(
            src.join("skills").join("flamez").join("SKILLS.md"),
            "skill".as_bytes(),
        )
        .unwrap();

        copy_recursive(&src, &dest).unwrap();

        assert!(
            dest.join("skills")
                .join("flamez")
                .join("SKILLS.md")
                .exists()
        )
    }
}
