/// Manages model snapshot downloading from HuggingFace
use anyhow::{Result, anyhow};
use hf_hub::api::{
    Siblings,
    tokio::{ApiBuilder, ApiError},
};
use serde::Deserialize;

use crate::utils::config::get_or_create_model_download_path;

/// Default quantization pulled when a modelfile doesn't specify one
pub const DEFAULT_QUANT: &str = "Q4_K_M";

const METADATA_PATTERNS: [&str; 5] = [".json", ".txt", ".md", ".gitattributes", "LICENSE"];

/// MTP head for speculative decoding, quant-independent
const MTP_HEAD: &str = "mtp-gemma-4-12b-it.gguf";

fn quant_gguf(quant: Option<&str>) -> String {
    format!("{}.gguf", quant.unwrap_or(DEFAULT_QUANT).to_lowercase())
}

/// the files a pull fetches, shared with the size shown before it
fn wanted(filename: &str, quant_gguf: &str) -> bool {
    METADATA_PATTERNS
        .iter()
        .any(|pattern| filename.ends_with(pattern))
        || filename.ends_with(MTP_HEAD)
        || filename.to_lowercase().ends_with(quant_gguf)
}

/// Download the model snapshot for the given model name and quantization.
/// `quant` selects the gguf variant (e.g. `Q8_0`); falls back to `Q4_K_M`.
pub async fn pull_model(model_name: &str, quant: Option<&str>) -> Result<()> {
    snapshot_download(model_name, quant).await
}

pub async fn snapshot_download(modelname: &str, quant: Option<&str>) -> Result<()> {
    let quant_gguf = quant_gguf(quant);
    let quant = quant.unwrap_or(DEFAULT_QUANT);
    let api_build_result = ApiBuilder::new()
        .with_progress(true)
        .with_cache_dir(get_or_create_model_download_path()?)
        .build();

    match api_build_result {
        Ok(api) => {
            let repo = api.model(modelname.to_owned());
            match repo.info().await {
                Ok(repo_info) => {
                    let filtered_siblings = repo_info
                        .siblings
                        .iter()
                        .filter(|sibling| wanted(&sibling.rfilename, &quant_gguf))
                        .collect::<Vec<&Siblings>>();

                    // failfast when the requested quant doesn't exist in the
                    // repo, instead of downloading metadata.
                    if !filtered_siblings
                        .iter()
                        .any(|sibling| is_main_gguf(&sibling.rfilename))
                    {
                        let available: Vec<&str> = repo_info
                            .siblings
                            .iter()
                            .map(|s| s.rfilename.as_str())
                            .filter(|name| is_main_gguf(name))
                            .collect();
                        let hint = if available.is_empty() {
                            "the repo contains no GGUF files".to_owned()
                        } else {
                            format!(
                                "available variants: {}. Select one in the modelfile, e.g. `FROM {}:<variant>`",
                                available.join(", "),
                                modelname
                            )
                        };
                        return Err(anyhow!(
                            "No GGUF matching quant '{}' found in {} ({})",
                            quant,
                            modelname,
                            hint
                        ));
                    }

                    for sibling in filtered_siblings {
                        if repo.get(&sibling.rfilename).await.is_err() {
                            return Err(anyhow!(
                                "{:?} failed to download, retry again",
                                &sibling.rfilename,
                            ));
                        }
                    }
                }
                Err(err) => return Err(anyhow!(format_hf_api_error(err))),
            };
        }
        Err(err) => return Err(anyhow!(format_hf_api_error(err))),
    }

    Ok(())
}

/// bytes a pull of this model would fetch, read off the hub's listing
pub async fn download_size(modelname: &str, quant: Option<&str>) -> Result<u64> {
    #[derive(Deserialize)]
    struct Listing {
        siblings: Vec<File>,
    }
    #[derive(Deserialize)]
    struct File {
        rfilename: String,
        size: Option<u64>,
    }

    let api = ApiBuilder::new()
        .with_cache_dir(get_or_create_model_download_path()?)
        .build()
        .map_err(|err| anyhow!(format_hf_api_error(err)))?;
    let listing: Listing = api
        .model(modelname.to_owned())
        .info_request()
        .query(&[("blobs", "true")])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let quant_gguf = quant_gguf(quant);
    Ok(listing
        .siblings
        .iter()
        .filter(|file| wanted(&file.rfilename, &quant_gguf))
        .filter_map(|file| file.size)
        .sum())
}

/// decimal units, the way finder counts
pub fn human_bytes(bytes: u64) -> String {
    let bytes = bytes as f64;
    if bytes >= 1e9 {
        format!("{:.1} GB", bytes / 1e9)
    } else {
        format!("{:.0} MB", bytes / 1e6)
    }
}

/// True for gguf files that can serve as the main model (excludes the
/// mmproj vision encoder and MTP draft heads, which are never main models).
fn is_main_gguf(filename: &str) -> bool {
    let name = filename.to_lowercase();
    name.ends_with(".gguf") && !name.contains("mmproj") && !name.contains("mtp")
}

fn format_hf_api_error(api_error: ApiError) -> String {
    match api_error {
        ApiError::RequestError(err) => err.to_string(),
        ApiError::TooManyRetries(err) => err.to_string(),
        _err => "Something unexpected happened, check your internet connection".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{human_bytes, is_main_gguf, quant_gguf, wanted};

    #[test]
    fn test_is_main_gguf() {
        assert!(is_main_gguf("gemma-4-12b-it-Q4_K_M.gguf"));
        assert!(is_main_gguf("Some-Model-Q8_0.GGUF"));
        assert!(!is_main_gguf("mmproj-F16.gguf"));
        assert!(!is_main_gguf("mtp-gemma-4-12b-it.gguf"));
        assert!(!is_main_gguf("config.json"));
    }

    #[test]
    fn the_size_counts_what_a_pull_fetches() {
        let quant = quant_gguf(None);
        assert!(wanted("gemma-4-12b-it-Q4_K_M.gguf", &quant));
        assert!(wanted("mtp-gemma-4-12b-it.gguf", &quant));
        assert!(wanted("config.json", &quant));
        assert!(!wanted("gemma-4-12b-it-Q8_0.gguf", &quant));
        assert!(!wanted("MTP/mtp-gemma-4-12b-it-BF16.gguf", &quant));
        assert!(wanted(
            "gemma-4-12b-it-Q8_0.gguf",
            &quant_gguf(Some("Q8_0"))
        ));
    }

    #[test]
    fn sizes_read_the_way_finder_shows_them() {
        assert_eq!(human_bytes(7_587_000_000), "7.6 GB");
        assert_eq!(human_bytes(465_109_248), "465 MB");
        assert_eq!(human_bytes(0), "0 MB");
    }
}
