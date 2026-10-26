//! Which files of a model repo to download

use std::path::{Component, Path};

use anyhow::{Result, bail};
use serde::Deserialize;

use super::{
    DownloadPlan, ModelFile,
    hub::{Hub, check},
};

/// Default quantization pulled when a modelfile doesn't specify one
pub const DEFAULT_QUANT: &str = "Q4_K_M";

const METADATA_SUFFIXES: [&str; 5] = [".json", ".txt", ".md", ".gitattributes", "LICENSE"];

#[derive(Deserialize)]
struct RepoInfo {
    sha: String,
    siblings: Vec<Sibling>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Sibling {
    rfilename: String,
    size: u64,
    blob_id: String,
    lfs: Option<Lfs>,
}

#[derive(Deserialize)]
struct Lfs {
    sha256: String,
}

impl From<Sibling> for ModelFile {
    // large files are stored by their SHA-256, small ones by their git id
    fn from(sibling: Sibling) -> Self {
        let sha256 = sibling.lfs.map(|lfs| lfs.sha256);
        Self {
            blob: sha256.clone().unwrap_or(sibling.blob_id),
            name: sibling.rfilename,
            size: sibling.size,
            sha256,
        }
    }
}

/// Lists the files of a model in huggingface API
pub async fn plan(hub: &Hub, repo: &str, quant: Option<&str>) -> Result<DownloadPlan> {
    let quant = quant.unwrap_or(DEFAULT_QUANT);
    let res = hub
        .get(&format!("api/models/{repo}/revision/main?blobs=true"))
        .send()
        .await?;
    let info: RepoInfo = check(res, repo)?.json().await?;

    let quant_gguf = format!("{}.gguf", quant.to_lowercase());
    let has_quant = info
        .siblings
        .iter()
        .any(|sibling| is_main_gguf(&sibling.rfilename) && wanted(&sibling.rfilename, &quant_gguf));
    if !has_quant {
        let available: Vec<&str> = info
            .siblings
            .iter()
            .map(|sibling| sibling.rfilename.as_str())
            .filter(|name| is_main_gguf(name))
            .collect();
        if available.is_empty() {
            bail!("No GGUF matching quant '{quant}' found in {repo}: the repo has no GGUF files");
        }
        bail!(
            "No GGUF matching quant '{quant}' found in {repo}. Available: {}. Pick one in the Modelfile, like `FROM {repo}:<quant>`",
            available.join(", ")
        );
    }

    let files: Vec<ModelFile> = info
        .siblings
        .into_iter()
        .filter(|sibling| wanted(&sibling.rfilename, &quant_gguf))
        .map(ModelFile::from)
        .collect();
    // these become paths in the cache, and HF_ENDPOINT can point at any mirror
    if !is_hex(&info.sha) {
        bail!(
            "Hugging Face sent {repo}'s commit as {:?}, which isn't a commit id",
            info.sha
        );
    }
    if let Some(file) = files
        .iter()
        .find(|file| !is_plain_path(&file.name) || !is_hex(&file.blob))
    {
        bail!(
            "Hugging Face listed a file in {repo} that isn't safe to save: {:?}",
            file.name
        );
    }
    Ok(DownloadPlan {
        repo: repo.to_owned(),
        commit: info.sha,
        files,
    })
}

// a relative path that stays inside the folder it's joined to
fn is_plain_path(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn is_hex(id: &str) -> bool {
    !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn wanted(name: &str, quant_gguf: &str) -> bool {
    let lower = name.to_lowercase();
    let top_level = !name.contains('/');
    METADATA_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
        || (is_main_gguf(name) && lower.ends_with(quant_gguf))
        // speculative decoding head, quant independent
        || (top_level && lower.ends_with(".gguf") && lower.contains("mtp"))
}

/// True for gguf files that can serve as the main model (excludes the
/// mmproj vision encoder and MTP draft heads, which are never main models).
fn is_main_gguf(filename: &str) -> bool {
    let name = filename.to_lowercase();
    name.ends_with(".gguf") && !name.contains("mmproj") && !name.contains("mtp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_paths_and_hex_ids_are_safe() {
        assert!(is_plain_path("MTP/README.md"));
        for name in ["", "../escape.json", "/etc/passwd", "MTP/../../x.gguf"] {
            assert!(!is_plain_path(name), "{name}");
        }
        assert!(is_hex("3b275b7a6957ac97aa4f5146652ab850ed212e5f"));
        assert!(!is_hex("../blob") && !is_hex(""));
    }

    #[test]
    fn test_is_main_gguf() {
        assert!(is_main_gguf("gemma-4-12b-it-Q4_K_M.gguf"));
        assert!(is_main_gguf("Some-Model-Q8_0.GGUF"));
        assert!(!is_main_gguf("mmproj-F16.gguf"));
        assert!(!is_main_gguf("mtp-gemma-4-12b-it.gguf"));
        assert!(!is_main_gguf("config.json"));
    }
}
