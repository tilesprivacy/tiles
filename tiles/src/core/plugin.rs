//! Plugin system

use std::{
    env,
    fs::{File, remove_dir_all},
    io::Write,
    path::PathBuf,
    process::Command,
    str::FromStr,
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use log::info;
use reqwest::Client;
use tempfile::tempdir;

use crate::utils::{
    config::{ConfigProvider, DefaultProvider},
    copy_recursive,
};

pub async fn install(path: String) -> Result<String> {
    if let Ok(url) = reqwest::Url::parse(&path) {
        info!("Online Plugin");
        let valid_filename = if let Some(mut segment_iterator) = url.path_segments()
            && let Some(filename) = segment_iterator.next_back()
            && !filename.is_empty()
            && is_valid_file_by_extension(filename)
        {
            filename
        } else {
            return Err(anyhow!(
                "Invalid plugin url, url should end with a .zip or tar.gz filename"
            ));
        };

        // We dont want to download big files
        let client_builder = Client::builder().timeout(Duration::from_secs(60));

        let client = client_builder.build()?;
        let plugin_name = valid_filename.split(".").collect::<Vec<&str>>()[0];

        println!("Downloading the plugin {}..", plugin_name);

        let result = client.get(path).send().await;

        match result {
            Err(err) => Err(anyhow!("Failed to download the plugin due to {:?}", err)),
            Ok(response) => {
                let mut tmp_path = env::temp_dir();

                tmp_path.push(valid_filename);
                let mut plugin_file = File::create(&tmp_path)?;
                let data = response.bytes().await?;
                plugin_file.write_all(&data)?;
                info!("Wrote the plugin to tmp path {:?}", tmp_path);
                install_from_local_source(tmp_path)
            }
        }
    } else {
        info!("Local Plugin");
        let local_path = PathBuf::from_str(&path).context("Invalid local path")?;
        install_from_local_source(local_path)
    }
}

fn install_from_local_source(local_path: PathBuf) -> Result<String> {
    if let Ok(true) = local_path.try_exists()
        && is_valid_file_by_extension(&local_path.to_string_lossy())
        && let Some(file_name) = local_path.file_name()
    {
        let file_name = file_name.to_string_lossy();
        let plugin_name = file_name.split(".").collect::<Vec<&str>>()[0];
        let user_data_dir = DefaultProvider.get_user_data_dir()?;
        let pi_skills_dir = user_data_dir.join("pi/agent/skills");
        std::fs::create_dir_all(&pi_skills_dir).context("Failed to create Pi skills directory")?;

        let tmp_dir = tempdir().expect("Failed to create tmp dir");

        let mut tmp_path = tmp_dir.path().to_path_buf();

        tmp_path.push("tmp_tiles_plugins");
        std::fs::create_dir_all(&tmp_path)
            .context("Failed to create temporary plugins directory")?;

        let output = if local_path.ends_with(".zip") {
            Command::new("unzip")
                .arg(&local_path)
                .arg("-d")
                .arg(&tmp_path)
                .output()?
        } else {
            Command::new("tar")
                .arg("-xzf")
                .arg(&local_path)
                .arg("-C")
                .arg(&tmp_path)
                .output()?
        };

        if !output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("{}", output_str))
        } else {
            // For now we are only copying skills
            if tmp_path.join(plugin_name).join("skills").is_dir() {
                copy_recursive(&tmp_path.join(plugin_name).join("skills"), &pi_skills_dir)?;
                Ok(format!("Successfully installed plugin {}", plugin_name))
            } else {
                Ok("Skills not found in this plugin".to_string())
            }
        }
    } else {
        Err(anyhow!(
            "{:?} not a valid local path. Please check if file exists and has valid extensions (.tar.gz, .zip)",
            local_path
        ))
    }
}

fn is_valid_file_by_extension(filename: &str) -> bool {
    filename.ends_with(".zip") || filename.ends_with(".tar.gz")
}

pub fn uninstall(plugin_name: &str) -> Result<String> {
    let user_data_dir = DefaultProvider.get_user_data_dir()?;
    let pi_skills_dir = user_data_dir.join("pi/agent/skills");

    if let Ok(true) = plugin_exist(plugin_name, &pi_skills_dir)
        && let Ok(_) = remove_dir_all(pi_skills_dir.join(plugin_name))
    {
        Ok(format!("Uninstalled plugin {} successfully", plugin_name))
    } else {
        Err(anyhow!(
            "Failed to uninstall plugin {}, please check the plugin name is correct and try again",
            plugin_name
        ))
    }
}

fn plugin_exist(plugin_name: &str, plugin_dir: &PathBuf) -> Result<bool> {
    let read_dir = std::fs::read_dir(plugin_dir)?;

    for dir_result in read_dir {
        match dir_result {
            Ok(dir) => {
                if dir.file_name() == plugin_name {
                    return Ok(true);
                }
            }
            Err(_err) => continue,
        }
    }
    Ok(false)
}

pub fn list() -> Result<()> {
    let user_data_dir = DefaultProvider.get_user_data_dir()?;
    let pi_skills_dir = user_data_dir.join("pi/agent/skills");
    let read_dir = std::fs::read_dir(pi_skills_dir)?;
    for dir_result in read_dir {
        match dir_result {
            Ok(dir) => {
                println!("{}", dir.file_name().to_string_lossy())
            }
            Err(_err) => continue,
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_file_extension() {
        assert!(is_valid_file_by_extension("flame.zip"));
        assert!(!is_valid_file_by_extension("flame.zi"));
        assert!(!is_valid_file_by_extension("flame.tar"));
        assert!(is_valid_file_by_extension("flame.tar.gz"));
        assert!(!is_valid_file_by_extension("flame.gz"));
        assert!(!is_valid_file_by_extension("flame.mp3"));
        assert!(!is_valid_file_by_extension("flame"));
    }
}
