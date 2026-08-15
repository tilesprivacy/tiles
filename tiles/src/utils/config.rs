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
/// - /usr/local/share/tiles or ~/.local/share/tiles (lib dir) - Some internal App files, libraries etc go here..
///     - /modelfiles
///     - /server
///     - /models - Where the pre-downloaded models.
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;
use std::{env, fs};
use toml::Table;

use crate::core::agent::types::{CompactionSettings, PiSettings};

#[derive(Serialize, Deserialize, Debug)]
struct ModelConfig {
    pub current: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct RootUserConfig {
    id: String,
    nickname: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DataConfig {
    path: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct InferenceConfig {
    // setting this to true, will prevent repl auto-exiting inference
    pub daemon: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct LlamaConfig {
    pub context_length: Option<u32>,
    pub gpu_layers: Option<i32>,
    pub offload_kqv: Option<bool>,
    pub batch_size: Option<u32>,
    pub mtp: Option<bool>,
    /// MoE only: keep expert weights for the first N layers on CPU (`--n-cpu-moe`).
    pub n_cpu_moe: Option<u32>,
    /// Enable flash attention (`--flash-attn`).
    pub flash_attn: Option<bool>,
    /// Disable memory-mapping the GGUF (`--no-mmap`).
    pub no_mmap: Option<bool>,
}

impl LlamaConfig {
    pub fn is_empty(&self) -> bool {
        self.context_length.is_none()
            && self.gpu_layers.is_none()
            && self.offload_kqv.is_none()
            && self.batch_size.is_none()
            && self.mtp.is_none()
            && self.n_cpu_moe.is_none()
            && self.flash_attn.is_none()
            && self.no_mmap.is_none()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct RootConfig {
    #[serde(rename = "root-user")]
    pub root_user: Option<RootUserConfig>,
    pub data: Option<DataConfig>,
    pub model: Option<ModelConfig>,
    pub inference: Option<InferenceConfig>,
    pub llama: Option<LlamaConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PiModelConfig {
    pub providers: HashMap<String, PiProviderConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PiProviderConfig {
    api: String,
    #[serde(rename = "apiKey")]
    api_key: String,
    #[serde(rename = "baseUrl")]
    base_url: String,
    pub models: Vec<PiProviderModelConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PiProviderModelConfig {
    pub id: String,
    pub reasoning: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "contextWindow")]
    pub context_window: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxTokens")]
    pub max_tokens: Option<u32>,
}
const MODEL_SUB_PATH: &str = "models/huggingface/hub";
pub const SYSTEM_BIN_DIR: &str = "/usr/local/bin";
pub const SYSTEM_BIN_PATH: &str = "/usr/local/bin/tiles";
pub const SYSTEM_LIB_DIR: &str = "/usr/local/share/tiles";
pub const PY_PORT: u32 = 6969;
// Used in remote inference, this is port where we open a TCP connection to proxy
pub const REMOTE_BOUND_PORT: u32 = 9271;

/// Bundled runtime directories under lib_dir removed on default uninstall.
pub const LIB_RUNTIME_DIRS_TO_REMOVE: &[&str] = &["server", "modelfiles", "pi", "models"];

pub trait ConfigProvider {
    fn get_config_dir(&self) -> Result<PathBuf>;
    fn get_or_create_config_dir(&self) -> Result<PathBuf>;
    fn get_data_dir(&self) -> Result<PathBuf>;
    fn get_or_create_data_dir(&self) -> Result<PathBuf>;
    fn get_user_data_dir(&self) -> Result<PathBuf>;
    fn get_lib_dir(&self) -> Result<PathBuf>;
    fn get_user_bin_dir(&self) -> Result<PathBuf>;
    fn get_user_bin_path(&self) -> Result<PathBuf>;
}

// Default MAX_TOKENS passed to Pi, incase not configued
const MAX_TOKENS: u32 = 30_000;

// Hard default MIN_TOKENS passed to Pi, incase configured with something way less
const MIN_TOKENS: u32 = 4_096;

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
            .expect("Failed to parse to table (data)");

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
            Ok(base_dir.join(".tiles_dev/tiles"))
        } else {
            // first check is that, let's say I have all the 3 folders (pi, modelfiles, server) in a portable folder somewhere,
            // and then we run the tiles binary from there like ./tiles
            // so first it checks if these folders exist right beside the binary (aka some portable folder and not in /usr or /.local)
            // then it should have no issues running
            if let Ok(current_exe) = env::current_exe()
                && let Some(exe_dir) = current_exe.parent()
                && is_tiles_lib_dir(exe_dir)
            {
                return Ok(exe_dir.to_path_buf());
            }

            // If it's not next to the executable, then check if these folders are in /usr/local/share/tiles
            let system_lib_dir = PathBuf::from(SYSTEM_LIB_DIR);
            if is_tiles_lib_dir(&system_lib_dir) {
                return Ok(system_lib_dir);
            }

            // If not in the global root share files, then finally, if these files are in ~/.local/share/tiles
            // then tiles can pick that up
            let user_lib_dir = self.get_data_dir()?;
            if is_tiles_lib_dir(&user_lib_dir) {
                return Ok(user_lib_dir);
            }

            Ok(PathBuf::from(SYSTEM_LIB_DIR))
        }
    }

    fn get_user_bin_dir(&self) -> Result<PathBuf> {
        let home_dir = env::home_dir().context("Failed to fetch $HOME")?;
        Ok(home_dir.join(".local/bin"))
    }

    fn get_user_bin_path(&self) -> Result<PathBuf> {
        Ok(self.get_user_bin_dir()?.join("tiles"))
    }
}

pub fn is_tiles_lib_dir(path: &Path) -> bool {
    path.join("modelfiles").is_dir() && path.join("server").is_dir() && path.join("pi").is_dir()
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

// TODO: This fn is very rigid and should be eventually replaced by
// `get_or_create_root_config`
pub fn get_or_create_config() -> Result<Table> {
    let tiles_config_dir = DefaultProvider.get_config_dir()?;
    let config_toml_path = tiles_config_dir.join("config.toml");

    if config_toml_path
        .try_exists()
        .context("config.toml path doesn't exist")?
    {
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

fn get_or_create_root_config() -> Result<RootConfig> {
    let tiles_config_dir = DefaultProvider.get_config_dir()?;
    let config_toml_path = tiles_config_dir.join("config.toml");

    if config_toml_path
        .try_exists()
        .context("config.toml path doesn't exist")?
    {
        let config_str = fs::read_to_string(config_toml_path)?;
        Ok(toml::from_str(&config_str)?)
    } else {
        let init_table: RootConfig = toml::from_str(
            r#"
                [root-user]
                id = ''
                nickname = ''

                [data]
                path = ''
            "#,
        )?;
        fs::write(config_toml_path, toml::to_string(&init_table)?)?;
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

/// Saves the root config toml `RootConfig` type
// #[warn(private_interfaces)]
fn save_root_config(config: &RootConfig) -> Result<()> {
    let tiles_config_dir = DefaultProvider.get_config_dir()?;
    let config_path = tiles_config_dir.join("config.toml");
    let tmp_path = tiles_config_dir.join("config.tmp.toml");
    fs::write(&tmp_path, toml::to_string(config)?)?;
    fs::copy(&tmp_path, &config_path)?;
    fs::remove_file(tmp_path)?;
    Ok(())
}
/// Get the apt path where the model in the system.
/// `model_name` may carry a `:quant` tag (e.g. `unsloth/gemma-4-12b-it-GGUF:Q8_0`),
/// which is stripped before resolving the HF cache dir.
pub fn get_model_cache(model_name: &str) -> Result<PathBuf> {
    let (repo, _quant) = tilekit::modelfile::split_model_spec(model_name);
    let hf_model_dir = if repo.contains("/") {
        let model_spec_parts = repo.split("/").collect::<Vec<&str>>();
        format!("models--{}--{}", model_spec_parts[0], model_spec_parts[1])
    } else {
        return Err(anyhow!("Modelfile not found"));
        // TODO: Check for a better Modilefile search instead of relying on checking "/"
    };

    let lib_dir = DefaultProvider.get_lib_dir()?;
    let pre_downloaded_model_path = lib_dir.join(MODEL_SUB_PATH).join(&hf_model_dir);
    let data_dir = DefaultProvider.get_user_data_dir()?;
    let user_data_dir_model_path = data_dir.join(MODEL_SUB_PATH).join(&hf_model_dir);

    let legacy_model_path = PathBuf::from(format!(
        "{}/.cache/huggingface/hub",
        env::home_dir().unwrap().to_str().unwrap()
    ))
    .join(&hf_model_dir);

    if pre_downloaded_model_path.exists() {
        get_commit_path(pre_downloaded_model_path)
    } else if user_data_dir_model_path.exists() {
        get_commit_path(user_data_dir_model_path)
    } else if legacy_model_path.exists() {
        get_commit_path(legacy_model_path)
    } else {
        Err(anyhow!("Model doesnt exist"))
    }
}

fn get_commit_path(base_path: PathBuf) -> Result<PathBuf> {
    let mut snapshots: Vec<(PathBuf, SystemTime)> = vec![];
    let snapshot_path = base_path.join("snapshots");
    if snapshot_path.exists() {
        for entry in snapshot_path.read_dir()? {
            if let Ok(item) = entry
                && item.path().is_dir()
            {
                snapshots.push((item.path(), item.path().metadata()?.modified()?));
            }
        }
        if snapshots.is_empty() {
            Ok(base_path)
        } else {
            let latest_snapshot = snapshots
                .iter()
                .max_by_key(|a| a.1)
                .expect("Failed fetching latest snapshot");
            Ok(latest_snapshot.0.clone())
        }
    } else {
        Ok(base_path)
    }
}

pub fn get_or_create_model_download_path() -> Result<PathBuf> {
    let data_dir = DefaultProvider.get_user_data_dir()?;
    let model_dir = data_dir.join(MODEL_SUB_PATH);
    if !model_dir.exists() {
        fs::create_dir_all(&model_dir)?;
    }
    Ok(model_dir)
}

pub fn get_app_name() -> String {
    if cfg!(debug_assertions) {
        "tiles_dev".to_owned()
    } else {
        "tiles".to_owned()
    }
}

pub fn update_current_model(model_name: &str) -> Result<()> {
    let mut root_config = get_or_create_root_config()?;
    // No toml file writes, if model is same
    if let Some(model_config) = &root_config.model
        && model_config.current == model_name
    {
        return Ok(());
    }
    do_update_current_model(&mut root_config, model_name)?;
    save_root_config(&root_config)
}

fn do_update_current_model(config: &mut RootConfig, model_name: &str) -> Result<()> {
    if let Some(_model_config) = &config.model {
        let model_config_v2 = ModelConfig {
            current: model_name.to_owned(),
        };
        config.model = Some(model_config_v2)
    } else {
        let model_config = ModelConfig {
            current: model_name.to_owned(),
        };

        config.model = Some(model_config);
    }
    Ok(())
}

fn get_pi_context_window() -> Option<u32> {
    get_llama_config()
        .ok()
        .and_then(|config| config.context_length)
}

fn get_pi_max_tokens(context_window: Option<u32>) -> Option<u32> {
    context_window.map(|context_window| context_window.clamp(MIN_TOKENS, context_window))
}

pub fn create_pi_provider_config(model_name: &str, enpoint_base_url: &str) -> Result<String> {
    create_pi_provider_config_with_context(model_name, enpoint_base_url, get_pi_context_window())
}

fn create_pi_provider_config_with_context(
    model_name: &str,
    enpoint_base_url: &str,
    context_window: Option<u32>,
) -> Result<String> {
    let max_tokens = get_pi_max_tokens(context_window).unwrap_or(MAX_TOKENS);
    let provider_config = PiProviderConfig {
        api: String::from("openai-responses"),
        api_key: String::from("tiles"),
        base_url: enpoint_base_url.to_string(),
        models: vec![PiProviderModelConfig {
            id: model_name.to_string(),
            reasoning: true,
            context_window,
            max_tokens: Some(max_tokens),
        }],
    };

    let mut provider: HashMap<String, PiProviderConfig> = HashMap::new();

    provider.insert("tiles".to_owned(), provider_config);
    let pi_model = PiModelConfig {
        providers: provider,
    };
    let config = json!(pi_model);

    serde_json::to_string(&config).map_err(Into::<anyhow::Error>::into)
}

#[allow(dead_code)]
fn try_update_pi_provider_model(config: &str, model_name: &str) -> Result<String> {
    let mut pi_model_config: PiModelConfig = serde_json::from_str(config)?;
    let mut tiles_provider_config: PiProviderConfig = pi_model_config
        .providers
        .get("tiles")
        .expect("Expected tiles key in under provider in models.json")
        .clone();

    if tiles_provider_config.models[0].id != model_name {
        let context_window = get_pi_context_window();
        let max_tokens = get_pi_max_tokens(context_window).unwrap_or(MAX_TOKENS);
        tiles_provider_config.models = vec![PiProviderModelConfig {
            id: model_name.to_owned(),
            reasoning: true,
            context_window,
            max_tokens: Some(max_tokens),
        }];
        let mut provider: HashMap<String, PiProviderConfig> = HashMap::new();
        provider.insert("tiles".to_owned(), tiles_provider_config);
        pi_model_config.providers = provider;
        serde_json::to_string(&pi_model_config).map_err(Into::<anyhow::Error>::into)
    } else {
        Ok(config.to_owned())
    }
}

pub fn get_inference_config() -> Result<Option<InferenceConfig>> {
    let root_config = get_or_create_root_config()?;
    Ok(root_config.inference)
}

pub fn update_inference_config(config: InferenceConfig) -> Result<()> {
    let mut root_config = get_or_create_root_config()?;
    root_config.inference = Some(config);
    save_root_config(&root_config)
}

pub fn get_llama_config() -> Result<LlamaConfig> {
    let root_config = get_or_create_root_config()?;
    Ok(root_config.llama.unwrap_or_default())
}

pub fn get_config_json() -> Result<serde_json::Value> {
    let root_config = get_or_create_root_config()?;
    serde_json::to_value(root_config).map_err(Into::<anyhow::Error>::into)
}

pub fn update_llama_config(config: &LlamaConfig) -> Result<()> {
    let mut root_config = get_or_create_root_config()?;
    let mut llama_config = root_config.llama.unwrap_or_default();

    llama_config.context_length = config
        .context_length
        .or(llama_config.context_length)
        .or(Some(32768));
    llama_config.gpu_layers = config.gpu_layers.or(llama_config.gpu_layers);
    llama_config.offload_kqv = config.offload_kqv.or(llama_config.offload_kqv);
    llama_config.batch_size = config.batch_size.or(llama_config.batch_size);
    llama_config.mtp = config.mtp.or(llama_config.mtp);
    llama_config.n_cpu_moe = config.n_cpu_moe.or(llama_config.n_cpu_moe).or(Some(12));
    llama_config.flash_attn = config.flash_attn.or(llama_config.flash_attn).or(Some(true));
    llama_config.no_mmap = config.no_mmap.or(llama_config.no_mmap).or(Some(true));
    root_config.llama = Some(llama_config);
    save_root_config(&root_config)
}

pub fn handle_pi_settings_config(settings_path: &PathBuf) -> Result<()> {
    let mut settings_config: PiSettings = if !settings_path.exists() {
        PiSettings::default()
    } else {
        let settings_str =
            fs::read_to_string(settings_path).context("Failed to read Pi settings path")?;
        serde_json::from_str(&settings_str).context("Failed to parse to settings")?
    };

    if settings_config.compaction.is_none() {
        settings_config.compaction = Some(CompactionSettings { enabled: false })
    }

    fs::write(
        settings_path,
        serde_json::to_string_pretty(&settings_config)?,
    )
    .map_err(Into::<anyhow::Error>::into)
    .context("Failed to write to Pi settings.json")
}

#[cfg(test)]
mod tests {

    use crate::core::agent::types::ReasoningEffort;

    use super::*;
    use serde_json::Value;
    use tempfile::tempdir;

    fn expected_pi_provider_json(model_name: &str, endpoint_base_url: &str) -> Value {
        let mut model = json!({
            "id": model_name,
            "reasoning": true,
            "maxTokens": MAX_TOKENS
        });

        if let Some(context_window) = get_pi_context_window()
            && let Value::Object(model_object) = &mut model
        {
            model_object.insert("contextWindow".to_owned(), json!(context_window));
        }

        if let Some(max_tokens) = get_pi_max_tokens(get_pi_context_window())
            && let Value::Object(model_object) = &mut model
        {
            model_object.insert("maxTokens".to_owned(), json!(max_tokens));
        }

        json!({
          "providers": {
            "tiles": {
              "api": "openai-responses",
              "apiKey": "tiles",
              "baseUrl": endpoint_base_url,
              "models": [model]
            }
          }
        })
    }

    #[test]
    fn test_updating_current_model_first_time() {
        let mut config: RootConfig = toml::from_str(
            r#"
                [root-user]
                id = 'did:key:xyz'
                nickname = ''
            "#,
        )
        .unwrap();

        do_update_current_model(&mut config, "model_name").unwrap();

        assert_eq!("model_name", config.model.unwrap().current);
    }

    #[test]
    fn test_updating_current_model_not_first_time() {
        let mut config: RootConfig = toml::from_str(
            r#"
                [root-user]
                id = 'did:key:xyz'
                nickname = ''

                [model]
                current = 'mlx-wahtever'
            "#,
        )
        .unwrap();

        do_update_current_model(&mut config, "model_name").unwrap();

        assert_eq!("model_name", config.model.unwrap().current);
    }

    #[test]
    fn test_valid_create_pi_provider_config() {
        let config_str = create_pi_provider_config(
            "mlx-community/Qwen3.5-4B-MLX-4bit",
            "http://127.0.0.1:0000/v1",
        )
        .unwrap();

        let config: PiModelConfig = serde_json::from_str(&config_str).unwrap();

        let expected_json = expected_pi_provider_json(
            "mlx-community/Qwen3.5-4B-MLX-4bit",
            "http://127.0.0.1:0000/v1",
        );

        assert_eq!(expected_json, serde_json::to_value(&config).unwrap())
    }

    #[test]
    fn test_pi_provider_uses_llama_context_length() {
        let config_str = create_pi_provider_config_with_context(
            "unsloth/gpt-oss-20b-GGUF",
            "http://127.0.0.1:6969/v1",
            Some(12_000),
        )
        .unwrap();
        let config: Value = serde_json::from_str(&config_str).unwrap();
        let model = &config["providers"]["tiles"]["models"][0];

        assert_eq!(model["contextWindow"], 12_000);
        assert_eq!(model["maxTokens"], 12_000);
    }

    #[test]
    fn test_valid_model_config_update() {
        let config_str = create_pi_provider_config(
            "mlx-community/Qwen3.5-4B-MLX-4bit",
            "http://127.0.0.1:0000/v1",
        )
        .unwrap();

        let config: PiModelConfig = serde_json::from_str(&config_str).unwrap();

        let expected_json = expected_pi_provider_json(
            "mlx-community/Qwen3.5-4B-MLX-4bit",
            "http://127.0.0.1:0000/v1",
        );

        assert_eq!(expected_json, serde_json::to_value(&config).unwrap());

        let new_config_str = try_update_pi_provider_model(&config_str, "new_model").unwrap();

        let new_config: PiModelConfig = serde_json::from_str(&new_config_str).unwrap();

        let expected_json = expected_pi_provider_json("new_model", "http://127.0.0.1:0000/v1");

        assert_eq!(expected_json, serde_json::to_value(&new_config).unwrap());
        assert_ne!(config_str, new_config_str);
    }

    #[test]
    fn test_no_model_config_update() {
        let config_str = create_pi_provider_config(
            "mlx-community/Qwen3.5-4B-MLX-4bit",
            "http://127.0.0.1:0000/v1",
        )
        .unwrap();

        let config: PiModelConfig = serde_json::from_str(&config_str).unwrap();

        let expected_json = expected_pi_provider_json(
            "mlx-community/Qwen3.5-4B-MLX-4bit",
            "http://127.0.0.1:0000/v1",
        );

        assert_eq!(expected_json, serde_json::to_value(&config).unwrap());

        let new_config_str =
            try_update_pi_provider_model(&config_str, "mlx-community/Qwen3.5-4B-MLX-4bit").unwrap();

        assert_eq!(config_str, new_config_str);
    }

    #[test]
    fn test_no_settings_file_exits() {
        let tmp = tempdir().expect("created tmp dir");
        let src = tmp.path().join("settings.json");

        handle_pi_settings_config(&src).unwrap();

        let settings_str = fs::read_to_string(&src)
            .context("Failed to read Pi settings path")
            .unwrap();

        let settings_config: PiSettings = serde_json::from_str(&settings_str).unwrap();

        assert_eq!(
            settings_config.compaction.unwrap(),
            CompactionSettings { enabled: false }
        );
        assert_eq!(
            settings_config.default_thinking_level.unwrap(),
            ReasoningEffort::Medium
        );
    }

    #[test]
    fn test_settings_file_exits_but_no_compaction_settings() {
        let tmp = tempdir().expect("created tmp dir");
        let src = tmp.path().join("settings.json");
        let pi_settings = PiSettings {
            default_thinking_level: Some(ReasoningEffort::Low),
            compaction: None,
        };
        fs::write(&src, serde_json::to_string_pretty(&pi_settings).unwrap()).unwrap();

        handle_pi_settings_config(&src).unwrap();

        let settings_str = fs::read_to_string(&src)
            .context("Failed to read Pi settings path")
            .unwrap();

        let settings_config: PiSettings = serde_json::from_str(&settings_str).unwrap();

        assert_eq!(
            settings_config.compaction.unwrap(),
            CompactionSettings { enabled: false }
        );
        assert_eq!(
            settings_config.default_thinking_level.unwrap(),
            ReasoningEffort::Low
        );
    }

    #[test]
    fn test_settings_file_exits_but_all_settings_exist() {
        let tmp = tempdir().expect("created tmp dir");
        let src = tmp.path().join("settings.json");
        let pi_settings = PiSettings {
            default_thinking_level: Some(ReasoningEffort::Low),
            compaction: Some(CompactionSettings { enabled: true }),
        };
        fs::write(&src, serde_json::to_string_pretty(&pi_settings).unwrap()).unwrap();

        handle_pi_settings_config(&src).unwrap();

        let settings_str = fs::read_to_string(&src)
            .context("Failed to read Pi settings path")
            .unwrap();

        let settings_config: PiSettings = serde_json::from_str(&settings_str).unwrap();

        assert_eq!(
            settings_config.compaction.unwrap(),
            CompactionSettings { enabled: true }
        );
        assert_eq!(
            settings_config.default_thinking_level.unwrap(),
            ReasoningEffort::Low
        );
    }
}
