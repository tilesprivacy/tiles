/// All things configurations in Tiles
///
/// Tiles by default stores different type of data in 3 folders
///
/// - ~/.config/tiles (config dir) - Ofcourse the App configs
///     - config.toml
///     - server.pid - current server daemon pid
/// - ~/.local/share/tiles (data dir) - The User generated data should go here + app logs
///     - /logs
///     - /data (default, user can change this location tho)
///         - /memory (memory stored as PKM)
/// - /usr/local/share/tiles (lib dir) - Some internal App files, libraries etc go here..
///     - /modelfiles
///     - /server
use anyhow::{Context, Result, anyhow};
use std::fs::File;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fs};
use toml::Table;

pub trait ConfigProvider {
    fn get_config_dir(&self) -> Result<PathBuf>;
    fn get_or_create_config_dir(&self) -> Result<PathBuf>;
    fn get_data_dir(&self) -> Result<PathBuf>;
    fn get_or_create_data_dir(&self) -> Result<PathBuf>;
    fn get_user_data_dir(&self) -> Result<PathBuf>;
    fn get_lib_dir(&self) -> Result<PathBuf>;
}

#[derive(Debug, Default)]
pub struct DefaultProvider;

impl ConfigProvider for DefaultProvider {
    fn get_config_dir(&self) -> Result<PathBuf> {
        if cfg!(debug_assertions) {
            let base_dir = env::current_dir().context("Failed to fetch CURRENT_DIR")?;
            Ok(base_dir.join(".tiles_dev/tiles"))
        } else {
            let home_dir = env::home_dir().context("Failed to fetch $HOME")?;
            let config_dir = match env::var("XDG_CONFIG_HOME") {
                Ok(val) => PathBuf::from(val),
                Err(_err) => home_dir.join(".config"),
            };
            Ok(config_dir.join("tiles"))
        }
    }

    fn get_or_create_config_dir(&self) -> Result<PathBuf> {
        let config_dir = self.get_config_dir()?;
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        }

        Ok(config_dir)
    }

    fn get_data_dir(&self) -> Result<PathBuf> {
        if cfg!(debug_assertions) {
            let base_dir = env::current_dir().context("Failed to fetch CURRENT_DIR")?;
            Ok(base_dir.join(".tiles_dev/tiles"))
        } else {
            let home_dir = env::home_dir().context("Failed to fetch $HOME")?;
            let data_dir = match env::var("XDG_DATA_HOME") {
                Ok(val) => PathBuf::from(val),
                Err(_err) => home_dir.join(".local/share"),
            };
            Ok(data_dir.join("tiles"))
        }
    }

    fn get_or_create_data_dir(&self) -> Result<PathBuf> {
        let data_dir = self.get_data_dir()?;
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir).context("Failed to create data directory")?;
        }

        if !data_dir.join("logs").exists() {
            fs::create_dir(data_dir.join("logs"))?;
            File::create(data_dir.join("logs/server.out.log"))?;
            File::create(data_dir.join("logs/server.err.log"))?;
        }

        if !data_dir.join("data").exists() {
            fs::create_dir_all(data_dir.join("data/memory"))?;
        }
        Ok(data_dir)
    }

    fn get_user_data_dir(&self) -> Result<PathBuf> {
        let root_config = get_or_create_config()?;
        let data_config = root_config
            .get("data")
            .expect("Failed to get data")
            .as_table()
            .expect("Failed to parse to table (data)")
            .clone();

        if let Some(path) = data_config
            .get("path")
            .expect("failed to parse data -> path")
            .as_str()
        {
            if path.is_empty() {
                let data_dir = self.get_data_dir()?;
                Ok(data_dir.join("data"))
            } else {
                PathBuf::from_str(path).map_err(|_e| anyhow!("Failed to convert to pathbuf"))
            }
        } else {
            Err(anyhow!("Failed to get data path"))
        }
    }

    fn get_lib_dir(&self) -> Result<PathBuf> {
        if cfg!(debug_assertions) {
            let base_dir = env::current_dir().context("Failed to fetch CURRENT_DIR")?;
            Ok(base_dir)
        } else {
            let data_dir = PathBuf::from_str("/usr/local/share")?;
            Ok(data_dir.join("tiles"))
        }
    }
}
pub fn set_user_data_path(path: &str) -> Result<String> {
    set_user_data_path_with_provider(&DefaultProvider, path)
}

pub fn get_memory_path() -> Result<String> {
    let root_config = get_or_create_config()?;
    let memory_config = root_config
        .get("data")
        .ok_or_else(|| anyhow!("memory section doesnt exist"))?
        .as_table()
        .expect("Failed to parse to table (data)");

    let path = memory_config
        .get("path")
        .ok_or_else(|| anyhow!("path doesnt exist (data)"))?
        .as_str()
        .expect("parse failed (memory)");
    if path.is_empty() {
        Err(anyhow::anyhow!(format!("NOT SET")))
    } else {
        Ok(PathBuf::from_str(path)?
            .join("memory")
            .to_str()
            .ok_or_else(|| anyhow!("failed to convert path to str"))?
            .to_owned())
    }
}

pub fn get_default_memory_path() -> Result<PathBuf> {
    let tiles_data_dir = DefaultProvider.get_data_dir()?;
    let memory_path = tiles_data_dir.join("memory");
    Ok(memory_path)
}

pub fn create_default_memory_folder() -> Result<PathBuf> {
    let memory_path = get_default_memory_path()?;
    fs::create_dir_all(&memory_path).context("Failed to create tiles user data directory")?;
    Ok(memory_path)
}

pub fn is_memory_model(modelname: &str) -> bool {
    if modelname.contains("mem") {
        return true;
    }
    false
}

fn set_user_data_path_with_provider<P: ConfigProvider>(
    _provider: &P,
    path: &str,
) -> Result<String> {
    let path_buf = PathBuf::from_str(path)?;
    if path_buf.try_exists()? {
        let mut root_config = get_or_create_config()?;
        let mut memory_config = root_config
            .get("data")
            .ok_or_else(|| anyhow!("data section doesnt exist"))?
            .as_table()
            .expect("Failed to parse to table (data)")
            .clone();
        memory_config.insert(String::from("path"), toml::Value::String(path.to_owned()));
        root_config.insert(String::from("data"), toml::Value::Table(memory_config));
        save_config(&root_config)?;
    } else {
        return Err(anyhow::anyhow!(format!(
            "Not a valid path {}",
            path_buf.to_str().unwrap()
        )));
    }
    Ok(format!(
        "Memory path set successfully at {}",
        path_buf.to_str().unwrap()
    ))
}

pub fn get_or_create_config() -> Result<Table> {
    let tiles_config_dir = DefaultProvider.get_config_dir()?;
    let config_toml_path = tiles_config_dir.join("config.toml");

    if config_toml_path.try_exists()? {
        let config_str = fs::read_to_string(config_toml_path)?;
        Ok(config_str.parse::<Table>()?)
    } else {
        let init_table: Table = toml::from_str(
            r#"
                [root-user]
                id = ''
                nickname = ''

                [data]
                path = ''
            "#,
        )?;
        fs::write(config_toml_path, init_table.to_string())?;
        Ok(init_table)
    }
}

/// Saves the root config toml `Table` type
pub fn save_config(config: &Table) -> Result<()> {
    let tiles_config_dir = DefaultProvider.get_config_dir()?;
    let config_path = tiles_config_dir.join("config.toml");
    let tmp_path = tiles_config_dir.join("config.tmp.toml");
    fs::write(&tmp_path, config.to_string())?;
    fs::copy(&tmp_path, &config_path)?;
    fs::remove_file(tmp_path)?;
    Ok(())
}

//TODO: Add more tests for config.toml
#[cfg(test)]
mod tests {

    // use super::*;

    // #[test]
    // fn test_create_config_file() -> Result<()> {
    //     let _config_table = get_or_create_config()?;
    //     Ok(())
    // }
}
