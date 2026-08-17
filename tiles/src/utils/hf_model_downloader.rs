/// Manages model snapshot downloading from HuggingFace
use anyhow::{Result, anyhow};
use hf_hub::api::{
    Siblings,
    tokio::{ApiBuilder, ApiError},
};

use crate::utils::config::get_or_create_model_download_path;

/// Default quantization pulled when a modelfile doesn't specify one
pub const DEFAULT_QUANT: &str = "Q4_K_M";

/// Download the model snapshot for the given model name and quantization.
/// `quant` selects the gguf variant (e.g. `Q8_0`); falls back to `Q4_K_M`.
pub async fn pull_model(model_name: &str, quant: Option<&str>) -> Result<()> {
    snapshot_download(model_name, quant).await
}

pub async fn snapshot_download(modelname: &str, quant: Option<&str>) -> Result<()> {
    let quant = quant.unwrap_or(DEFAULT_QUANT);
    let metadata_patterns = [".json", ".txt", ".md", ".gitattributes", "LICENSE"];
    let quant_gguf = format!("{}.gguf", quant.to_lowercase());
    let allow_patterns: Vec<String> = metadata_patterns
        .iter()
        .map(|p| (*p).to_owned())
        .chain([
            quant_gguf.clone(),
            // MTP head for speculative decoding, quant-independent
            "mtp-gemma-4-12b-it.gguf".to_owned(),
        ])
        .collect();
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
                        .filter(|sibling| {
                            allow_patterns.iter().any(|pat| {
                                sibling.rfilename.ends_with(pat.as_str())
                                    || sibling.rfilename.to_lowercase().ends_with(&quant_gguf)
                            })
                        })
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
    use super::is_main_gguf;

    #[test]
    fn test_is_main_gguf() {
        assert!(is_main_gguf("gemma-4-12b-it-Q4_K_M.gguf"));
        assert!(is_main_gguf("Some-Model-Q8_0.GGUF"));
        assert!(!is_main_gguf("mmproj-F16.gguf"));
        assert!(!is_main_gguf("mtp-gemma-4-12b-it.gguf"));
        assert!(!is_main_gguf("config.json"));
    }
}
