/// All things configurations in Tiles
///
/// Tiles by default stores different type of data in 3 folders
///
/// - ~/.config/tiles (config dir) - Ofcourse the App configs
///     - config.toml
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
use tiles_engine::Engine;
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
    /// How llama.cpp loads the model (`--load-mode`).
    pub load_mode: Option<LoadMode>,
    pub context_checkpoints: Option<u32>,
}

/// llama.cpp's `--load-mode`, with llama.cpp's own value names
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LoadMode {
    /// mmap, unless a device doesn't support it (llama.cpp's default)
    Auto,
    /// read the model into memory
    None,
    /// memory-map the model
    Mmap,
    /// keep the model in RAM, never swapped out
    Mlock,
    /// mmap, and keep the model in RAM
    #[serde(rename = "mmap+mlock")]
    #[value(name = "mmap+mlock")]
    MmapMlock,
    /// DirectIO, where the system has it
    Dio,
}

impl LlamaConfig {
    /// Fills what's unset with Tiles' defaults. Without a context length llama.cpp uses
    /// the model's maximum and runs out of memory on most machines
    pub fn with_defaults(mut self) -> Self {
        self.context_length = self.context_length.or(Some(DEFAULT_CONTEXT_LENGTH));
        self.flash_attn = self.flash_attn.or(Some(true));
        self.context_checkpoints = self
            .context_checkpoints
            .or(Some(DEFAULT_CONTEXT_CHECKPOINTS));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.context_length.is_none()
            && self.gpu_layers.is_none()
            && self.offload_kqv.is_none()
            && self.batch_size.is_none()
            && self.mtp.is_none()
            && self.n_cpu_moe.is_none()
            && self.flash_attn.is_none()
            && self.load_mode.is_none()
            && self.context_checkpoints.is_none()
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct EngineConfig {
    /// The llama.cpp build replies run on, like "vulkan", or "auto" for Tiles to pick one
    pub backend: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RootConfig {
    #[serde(rename = "root-user")]
    pub root_user: Option<RootUserConfig>,
    pub data: Option<DataConfig>,
    pub model: Option<ModelConfig>,
    pub inference: Option<InferenceConfig>,
    pub llama: Option<LlamaConfig>,
    pub engine: Option<EngineConfig>,
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
pub const DEFAULT_CONTEXT_LENGTH: u32 = 65536;
// llama.cpp keeps 32 per slot, and with Gemma they fill RAM
pub const DEFAULT_CONTEXT_CHECKPOINTS: u32 = 4;

const MODEL_SUB_PATH: &str = "models/huggingface/hub";
pub const SYSTEM_BIN_DIR: &str = "/usr/local/bin";
pub const SYSTEM_BIN_PATH: &str = "/usr/local/bin/tiles";
pub const SYSTEM_LIB_DIR: &str = "/usr/local/share/tiles";
pub const PY_PORT: u32 = 6969;
// Used in remote inference, this is port where we open a TCP connection to proxy
pub const REMOTE_BOUND_PORT: u32 = 9271;

/// Left in the lib dir by `install.sh --headless`, for updates to stay headless
pub const HEADLESS_MARKER: &str = ".headless";

/// Bundled runtime directories under lib_dir removed on default uninstall.
pub const LIB_RUNTIME_DIRS_TO_REMOVE: &[&str] = &["server", "modelfiles", "pi", "ui", "models"];

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

// smallest context length the settings take
const MIN_CONTEXT_LENGTH: u32 = 4_096;

#[derive(Debug, Default, Clone, Copy)]
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
        let root_config = get_or_create_config(DefaultProvider)?;
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
    // an empty path in the config means the default data folder
    DefaultProvider
        .get_user_data_dir()?
        .join("memory")
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("failed to convert path to str"))
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
        let mut root_config = get_or_create_config(DefaultProvider)?;
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
pub fn get_or_create_config(provider: impl ConfigProvider) -> Result<Table> {
    let tiles_config_dir = provider.get_or_create_config_dir()?;
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
    save_config_with_provider(config, DefaultProvider)
}

pub fn save_config_with_provider(config: &Table, provider: impl ConfigProvider) -> Result<()> {
    let tiles_config_dir = provider.get_config_dir()?;
    let config_path = tiles_config_dir.join("config.toml");
    let tmp_path = tiles_config_dir.join("config.tmp.toml");
    fs::write(&tmp_path, config.to_string())?;
    // readers never see half a file
    fs::rename(&tmp_path, &config_path)?;
    Ok(())
}

/// Saves the root config toml `RootConfig` type
// #[warn(private_interfaces)]
fn save_root_config(config: &RootConfig) -> Result<()> {
    let tiles_config_dir = DefaultProvider.get_config_dir()?;
    let config_path = tiles_config_dir.join("config.toml");
    let tmp_path = tiles_config_dir.join("config.tmp.toml");
    fs::write(&tmp_path, toml::to_string(config)?)?;
    // readers never see half a file
    fs::rename(&tmp_path, &config_path)?;
    Ok(())
}
/// Get the apt path where the model in the system.
/// `model_name` may carry a `:quant` tag (e.g. `unsloth/gemma-4-12b-it-GGUF:Q8_0`),
/// which is stripped before resolving the HF cache dir.
/// Why a model lookup failed when it's not a disk error, so the API can answer 400 or
/// 404 instead of 500
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCacheError {
    /// The name is not in `owner/repo` form
    InvalidSpec,
    /// The model is not downloaded
    NotCached,
}

impl std::fmt::Display for ModelCacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidSpec => "Model spec should look like owner/repo",
            Self::NotCached => "Model is not downloaded",
        })
    }
}

impl std::error::Error for ModelCacheError {}

pub fn get_model_cache(model_name: &str) -> Result<PathBuf> {
    let (repo, _quant) = tilekit::modelfile::split_model_spec(model_name);
    let hf_model_dir = if repo.contains("/") {
        let model_spec_parts = repo.split("/").collect::<Vec<&str>>();
        format!("models--{}--{}", model_spec_parts[0], model_spec_parts[1])
    } else {
        return Err(ModelCacheError::InvalidSpec.into());
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
        Err(ModelCacheError::NotCached.into())
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

pub fn get_pi_context_window() -> Option<u32> {
    get_llama_config()
        .ok()
        .and_then(|config| config.context_length)
}

pub fn create_pi_provider_config(model_name: &str, enpoint_base_url: &str) -> Result<String> {
    create_pi_provider_config_with_context(model_name, enpoint_base_url, get_pi_context_window())
}

fn create_pi_provider_config_with_context(
    model_name: &str,
    enpoint_base_url: &str,
    context_window: Option<u32>,
) -> Result<String> {
    let max_tokens = context_window.unwrap_or(MAX_TOKENS);
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
        let max_tokens = context_window.unwrap_or(MAX_TOKENS);
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

/// With the defaults filled in, matching what the model server gets
pub fn get_llama_config() -> Result<LlamaConfig> {
    let root_config = get_or_create_root_config()?;
    Ok(root_config.llama.unwrap_or_default().with_defaults())
}

/// The engine picked by hand, `None` for auto or a name this Tiles doesn't know
pub fn get_engine_pick() -> Option<Engine> {
    let backend = get_or_create_root_config().ok()?.engine?.backend?;
    Engine::from_name(&backend)
}

pub fn set_engine_pick(pick: Option<Engine>) -> Result<()> {
    let mut root_config = get_or_create_root_config()?;
    root_config.engine = Some(EngineConfig {
        backend: Some(pick.map_or("auto", |engine| engine.name()).to_owned()),
    });
    save_root_config(&root_config)
}

/// The model chats use, last picked in the UI or run by `tiles run`
pub fn get_current_model() -> Option<String> {
    Some(get_or_create_root_config().ok()?.model?.current)
}

/// The config for the model server, with the llama defaults filled in
pub fn get_config_json() -> Result<serde_json::Value> {
    let mut root_config = get_or_create_root_config()?;
    let llama = root_config.llama.take().unwrap_or_default().with_defaults();
    let llama_server_args = llama_server_arguments(&llama);
    root_config.llama = Some(llama);
    let mut config = serde_json::to_value(root_config)?;
    config["llama_server_args"] = json!(llama_server_args);
    Ok(config)
}

pub fn update_llama_config(config: &LlamaConfig) -> Result<()> {
    let mut root_config = get_or_create_root_config()?;
    let mut llama_config = root_config.llama.unwrap_or_default();

    llama_config.context_length = config.context_length.or(llama_config.context_length);
    llama_config.gpu_layers = config.gpu_layers.or(llama_config.gpu_layers);
    llama_config.offload_kqv = config.offload_kqv.or(llama_config.offload_kqv);
    llama_config.batch_size = config.batch_size.or(llama_config.batch_size);
    llama_config.mtp = config.mtp.or(llama_config.mtp);
    llama_config.n_cpu_moe = config.n_cpu_moe.or(llama_config.n_cpu_moe);
    llama_config.flash_attn = config.flash_attn.or(llama_config.flash_attn);
    llama_config.load_mode = config.load_mode.or(llama_config.load_mode);
    llama_config.context_checkpoints = config
        .context_checkpoints
        .or(llama_config.context_checkpoints);
    root_config.llama = Some(llama_config.with_defaults());
    save_root_config(&root_config)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LlamaSetting {
    pub name: String,
    pub value: Option<serde_json::Value>,
    pub default: String,
    // number, flag or choice
    pub kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
    pub help: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSetting(String);

impl std::fmt::Display for InvalidSetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InvalidSetting {}

enum SettingType {
    Number { min: i64 },
    Flag,
    LoadMode,
}

enum ServerArgument {
    FlagThenValue(&'static str),
    FlagThenOnOrOff(&'static str),
    FlagForEach {
        enabled: &'static str,
        disabled: &'static str,
    },
    // mtp, the model server finds the head file first
    PassedByModelServer,
}

struct SettingDefinition {
    name: &'static str,
    setting_type: SettingType,
    default: &'static str,
    help: &'static str,
    server_argument: ServerArgument,
}

const LLAMA_SETTINGS: [SettingDefinition; 9] = [
    SettingDefinition {
        name: "context_length",
        setting_type: SettingType::Number {
            min: MIN_CONTEXT_LENGTH as i64,
        },
        default: "65536",
        help: "Size of the context window, in tokens",
        server_argument: ServerArgument::FlagThenValue("-c"),
    },
    SettingDefinition {
        name: "gpu_layers",
        setting_type: SettingType::Number { min: -1 },
        default: "auto",
        help: "Model layers to run on the GPU",
        server_argument: ServerArgument::FlagThenValue("-ngl"),
    },
    SettingDefinition {
        name: "offload_kqv",
        setting_type: SettingType::Flag,
        default: "true",
        help: "Keep the KV cache on the GPU",
        server_argument: ServerArgument::FlagForEach {
            enabled: "--kv-offload",
            disabled: "--no-kv-offload",
        },
    },
    SettingDefinition {
        name: "batch_size",
        setting_type: SettingType::Number { min: 1 },
        default: "2048",
        help: "Prompt tokens processed at once",
        server_argument: ServerArgument::FlagThenValue("-b"),
    },
    SettingDefinition {
        name: "mtp",
        setting_type: SettingType::Flag,
        default: "false",
        help: "MTP speculative decoding, for models that come with an MTP head",
        server_argument: ServerArgument::PassedByModelServer,
    },
    SettingDefinition {
        name: "n_cpu_moe",
        setting_type: SettingType::Number { min: 0 },
        default: "0",
        help: "MoE models: layers whose expert weights stay on the CPU",
        server_argument: ServerArgument::FlagThenValue("--n-cpu-moe"),
    },
    SettingDefinition {
        name: "flash_attn",
        setting_type: SettingType::Flag,
        default: "true",
        help: "Flash attention",
        server_argument: ServerArgument::FlagThenOnOrOff("--flash-attn"),
    },
    SettingDefinition {
        name: "load_mode",
        setting_type: SettingType::LoadMode,
        default: "auto",
        help: "How llama.cpp loads the model",
        server_argument: ServerArgument::FlagThenValue("--load-mode"),
    },
    SettingDefinition {
        name: "context_checkpoints",
        setting_type: SettingType::Number { min: 0 },
        default: "4",
        help: "Chat snapshots kept in RAM, so a reply doesn't re-read the chat",
        server_argument: ServerArgument::FlagThenValue("--ctx-checkpoints"),
    },
];

pub fn llama_server_arguments(config: &LlamaConfig) -> Vec<String> {
    let fields = serde_json::to_value(config).unwrap_or_default();
    let mut arguments = vec![];
    for definition in &LLAMA_SETTINGS {
        let value = match fields.get(definition.name) {
            Some(serde_json::Value::String(text)) => text.clone(),
            Some(serde_json::Value::Null) | None => continue,
            Some(value) => value.to_string(),
        };
        let enabled = value == "true";
        match definition.server_argument {
            ServerArgument::FlagThenValue(flag) => arguments.extend([flag.to_owned(), value]),
            ServerArgument::FlagThenOnOrOff(flag) => {
                let on_or_off = if enabled { "on" } else { "off" };
                arguments.extend([flag.to_owned(), on_or_off.to_owned()]);
            }
            ServerArgument::FlagForEach { enabled: flag, .. } if enabled => {
                arguments.push(flag.to_owned());
            }
            ServerArgument::FlagForEach { disabled: flag, .. } => arguments.push(flag.to_owned()),
            ServerArgument::PassedByModelServer => {}
        }
    }
    arguments
}

fn load_mode_names() -> Vec<String> {
    use clap::ValueEnum;
    LoadMode::value_variants()
        .iter()
        .filter_map(|mode| mode.to_possible_value())
        .map(|mode| mode.get_name().to_owned())
        .collect()
}

pub fn get_llama_settings() -> Result<Vec<LlamaSetting>> {
    Ok(llama_settings_from_config(&get_llama_config()?))
}

pub fn llama_setting_names() -> impl Iterator<Item = &'static str> {
    LLAMA_SETTINGS.iter().map(|definition| definition.name)
}

/// `None` resets a setting
pub fn update_llama_settings(
    changes: &[(String, Option<serde_json::Value>)],
) -> Result<Vec<LlamaSetting>> {
    let changes = parse_setting_changes(changes)?;
    let mut root_config = get_or_create_root_config()?;
    let current = root_config.llama.take().unwrap_or_default();
    let config = apply_setting_changes(&current, changes)?.with_defaults();
    let settings = llama_settings_from_config(&config);
    root_config.llama = Some(config);
    save_root_config(&root_config)?;
    Ok(settings)
}

fn llama_settings_from_config(config: &LlamaConfig) -> Vec<LlamaSetting> {
    let fields = serde_json::to_value(config).unwrap_or_default();
    LLAMA_SETTINGS
        .iter()
        .map(|definition| LlamaSetting {
            name: definition.name.to_owned(),
            value: fields
                .get(definition.name)
                .filter(|v| !v.is_null())
                .cloned(),
            default: definition.default.to_owned(),
            kind: match definition.setting_type {
                SettingType::Number { .. } => "number",
                SettingType::Flag => "flag",
                SettingType::LoadMode => "choice",
            }
            .to_owned(),
            choices: match definition.setting_type {
                SettingType::LoadMode => load_mode_names(),
                _ => vec![],
            },
            help: definition.help.to_owned(),
        })
        .collect()
}

fn parse_setting_changes(
    changes: &[(String, Option<serde_json::Value>)],
) -> Result<Vec<(&'static str, serde_json::Value)>, InvalidSetting> {
    changes
        .iter()
        .map(|(name, value)| {
            let definition = LLAMA_SETTINGS
                .iter()
                .find(|definition| definition.name == name)
                .ok_or_else(|| InvalidSetting(format!("No setting named {name}")))?;
            let value = match value {
                Some(value) => parse_setting_value(definition, value)?,
                None => serde_json::Value::Null,
            };
            Ok((definition.name, value))
        })
        .collect()
}

fn apply_setting_changes(
    config: &LlamaConfig,
    changes: Vec<(&'static str, serde_json::Value)>,
) -> Result<LlamaConfig, InvalidSetting> {
    let invalid = |err: serde_json::Error| InvalidSetting(err.to_string());
    let serde_json::Value::Object(mut fields) = serde_json::to_value(config).map_err(invalid)?
    else {
        return Err(InvalidSetting(
            "The llama settings aren't a table".to_owned(),
        ));
    };
    for (name, value) in changes {
        fields.insert(name.to_owned(), value);
    }
    serde_json::from_value(serde_json::Value::Object(fields)).map_err(invalid)
}

// JSON from the API, text from the CLI
fn parse_setting_value(
    definition: &SettingDefinition,
    value: &serde_json::Value,
) -> Result<serde_json::Value, InvalidSetting> {
    let name = definition.name;
    let text = match value {
        serde_json::Value::String(text) => text.trim().to_owned(),
        other => other.to_string(),
    };
    match definition.setting_type {
        SettingType::Number { min } => match text.parse::<i64>() {
            Ok(number) if number >= min => Ok(json!(number)),
            Ok(_) => Err(InvalidSetting(format!("{name} can't be less than {min}"))),
            Err(_) => Err(InvalidSetting(format!(
                "{name} needs a whole number, not {text}"
            ))),
        },
        SettingType::Flag => match text.as_str() {
            "true" | "on" => Ok(json!(true)),
            "false" | "off" => Ok(json!(false)),
            _ => Err(InvalidSetting(format!(
                "{name} needs true or false, not {text}"
            ))),
        },
        SettingType::LoadMode => {
            let modes = load_mode_names();
            if modes.contains(&text) {
                Ok(json!(text))
            } else {
                Err(InvalidSetting(format!(
                    "{name} needs one of {}, not {text}",
                    modes.join(", ")
                )))
            }
        }
    }
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

    // old configs still have no_mmap, and nothing migrates them. the next save drops it
    #[test]
    fn a_config_with_retired_keys_still_loads() {
        let config: RootConfig = toml::from_str(
            r#"
                [llama]
                context_length = 32768
                no_mmap = true
            "#,
        )
        .unwrap();

        let llama = config.llama.unwrap();
        assert_eq!(llama.context_length, Some(32768));
        assert_eq!(llama.load_mode, None);
    }

    #[test]
    fn load_mode_takes_llama_cpp_names_only() {
        for (name, mode) in [
            ("auto", LoadMode::Auto),
            ("none", LoadMode::None),
            ("mmap", LoadMode::Mmap),
            ("mlock", LoadMode::Mlock),
            ("mmap+mlock", LoadMode::MmapMlock),
            ("dio", LoadMode::Dio),
        ] {
            let config: LlamaConfig = toml::from_str(&format!("load_mode = \"{name}\"")).unwrap();
            assert_eq!(config.load_mode, Some(mode));
        }
        assert!(toml::from_str::<LlamaConfig>("load_mode = \"no-mmap\"").is_err());
    }

    fn one_setting_change(name: &str, value: Value) -> Vec<(String, Option<Value>)> {
        vec![(name.to_owned(), Some(value))]
    }

    #[test]
    fn there_is_a_setting_for_every_llama_config_field() {
        let Value::Object(fields) = serde_json::to_value(LlamaConfig::default()).unwrap() else {
            panic!("LlamaConfig isn't a table");
        };
        let mut fields: Vec<_> = fields.keys().map(String::as_str).collect();
        let mut names: Vec<_> = llama_setting_names().collect();
        fields.sort_unstable();
        names.sort_unstable();
        assert_eq!(names, fields);
    }

    #[test]
    fn settings_take_text_from_the_cli_and_json_from_the_api() {
        let changes = [
            ("context_length".to_owned(), Some(json!("64000"))),
            ("gpu_layers".to_owned(), Some(json!(-1))),
            ("flash_attn".to_owned(), Some(json!("off"))),
            ("mtp".to_owned(), Some(json!(true))),
            ("load_mode".to_owned(), Some(json!("mmap+mlock"))),
        ];
        let changes = parse_setting_changes(&changes).unwrap();
        let config = apply_setting_changes(&LlamaConfig::default(), changes).unwrap();
        assert_eq!(config.context_length, Some(64000));
        assert_eq!(config.gpu_layers, Some(-1));
        assert_eq!(config.flash_attn, Some(false));
        assert_eq!(config.mtp, Some(true));
        assert_eq!(config.load_mode, Some(LoadMode::MmapMlock));
        assert_eq!(config.batch_size, None);
    }

    #[test]
    fn a_wrong_name_or_value_says_what_is_wrong() {
        fn rejection_reason(name: &str, value: Value) -> String {
            parse_setting_changes(&one_setting_change(name, value))
                .unwrap_err()
                .to_string()
        }
        assert_eq!(
            rejection_reason("context_size", json!(1)),
            "No setting named context_size"
        );
        assert_eq!(
            rejection_reason("context_length", json!("big")),
            "context_length needs a whole number, not big"
        );
        assert_eq!(
            rejection_reason("context_length", json!(2048)),
            "context_length can't be less than 4096"
        );
        assert_eq!(
            rejection_reason("flash_attn", json!("maybe")),
            "flash_attn needs true or false, not maybe"
        );
        assert_eq!(
            rejection_reason("load_mode", json!("no-mmap")),
            "load_mode needs one of auto, none, mmap, mlock, mmap+mlock, dio, not no-mmap"
        );
    }

    #[test]
    fn set_settings_become_llama_server_arguments() {
        assert!(llama_server_arguments(&LlamaConfig::default()).is_empty());

        let config = LlamaConfig {
            context_length: Some(8192),
            gpu_layers: Some(-1),
            offload_kqv: Some(false),
            batch_size: Some(512),
            mtp: Some(true),
            n_cpu_moe: Some(4),
            flash_attn: Some(true),
            load_mode: Some(LoadMode::MmapMlock),
            context_checkpoints: Some(2),
        };
        let arguments = llama_server_arguments(&config).join(" ");
        assert_eq!(
            arguments,
            "-c 8192 -ngl -1 --no-kv-offload -b 512 --n-cpu-moe 4 --flash-attn on --load-mode mmap+mlock --ctx-checkpoints 2"
        );

        let flipped = LlamaConfig {
            offload_kqv: Some(true),
            flash_attn: Some(false),
            ..LlamaConfig::default()
        };
        assert_eq!(
            llama_server_arguments(&flipped).join(" "),
            "--kv-offload --flash-attn off"
        );
    }

    #[test]
    fn a_reset_setting_goes_back_to_its_default() {
        let set = LlamaConfig {
            context_length: Some(8192),
            gpu_layers: Some(10),
            ..LlamaConfig::default()
        };
        let resets = [
            ("context_length".to_owned(), None),
            ("gpu_layers".to_owned(), None),
        ];
        let config = apply_setting_changes(&set, parse_setting_changes(&resets).unwrap())
            .unwrap()
            .with_defaults();
        assert_eq!(config.context_length, Some(DEFAULT_CONTEXT_LENGTH));
        assert_eq!(config.gpu_layers, None);

        let settings = llama_settings_from_config(&config);
        let gpu_layers = settings.iter().find(|s| s.name == "gpu_layers").unwrap();
        assert_eq!(gpu_layers.value, None);
        assert_eq!(gpu_layers.default, "auto");
        let load_mode = settings.iter().find(|s| s.name == "load_mode").unwrap();
        assert_eq!(load_mode.choices, load_mode_names());
    }

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

        if let Some(max_tokens) = get_pi_context_window()
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
    fn unset_llama_settings_get_the_defaults_and_set_ones_stay() {
        let filled = LlamaConfig::default().with_defaults();
        assert_eq!(filled.context_length, Some(65536));
        // --n-cpu-moe turns off llama.cpp's --fit for every model
        assert_eq!(filled.n_cpu_moe, None);
        assert_eq!(filled.flash_attn, Some(true));
        // llama.cpp picks how to load the model
        assert_eq!(filled.load_mode, None);
        assert_eq!(filled.context_checkpoints, Some(4));

        let mine = LlamaConfig {
            context_length: Some(8192),
            flash_attn: Some(false),
            ..LlamaConfig::default()
        }
        .with_defaults();
        assert_eq!(mine.context_length, Some(8192));
        assert_eq!(mine.flash_attn, Some(false));
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
    fn a_small_context_length_in_config_toml_is_passed_on_unchanged() {
        let config_str = create_pi_provider_config_with_context(
            "unsloth/gpt-oss-20b-GGUF",
            "http://127.0.0.1:6969/v1",
            Some(2_048),
        )
        .unwrap();
        let config: Value = serde_json::from_str(&config_str).unwrap();
        let model = &config["providers"]["tiles"]["models"][0];

        assert_eq!(model["contextWindow"], 2_048);
        assert_eq!(model["maxTokens"], 2_048);
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
