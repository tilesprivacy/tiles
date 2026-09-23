/// Manages model snapshot downloading from HuggingFace
use anyhow::{Result, anyhow};
use hf_hub::api::{
    Siblings,
    tokio::{ApiBuilder, ApiError},
};
use serde::Deserialize;
use std::{io::Read, path::Path};

use crate::utils::config::get_or_create_model_download_path;

/// Default quantization pulled when a modelfile doesn't specify one
pub const DEFAULT_QUANT: &str = "Q4_K_M";

const METADATA_PATTERNS: [&str; 5] = [".json", ".txt", ".md", ".gitattributes", "LICENSE"];

fn quant_gguf(quant: Option<&str>) -> String {
    format!("{}.gguf", quant.unwrap_or(DEFAULT_QUANT).to_lowercase())
}

/// the repo's MTP head for speculative decoding, quant-independent. the
/// higher precision ones under `MTP/` are not what we run
fn is_mtp_head(filename: &str) -> bool {
    !filename.contains('/') && filename.starts_with("mtp-") && filename.ends_with(".gguf")
}

/// the files a pull fetches, shared with the size shown before it
fn wanted(filename: &str, quant_gguf: &str) -> bool {
    METADATA_PATTERNS
        .iter()
        .any(|pattern| filename.ends_with(pattern))
        || is_mtp_head(filename)
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

/// One file of a pull, as the hub lists it.
#[derive(Debug, Clone)]
pub struct RepoFile {
    pub name: String,
    pub size: u64,
    /// what hf-hub names the blob: the sha256 for lfs files, the git id otherwise
    pub blob: String,
}

impl RepoFile {
    pub fn is_main_gguf(&self) -> bool {
        is_main_gguf(&self.name)
    }
}

/// the files a pull of this model fetches, read off the hub's listing
pub async fn repo_files(modelname: &str, quant: Option<&str>) -> Result<Vec<RepoFile>> {
    Ok(repo_snapshot(modelname, quant).await?.1)
}

/// the commit the hub is serving, and the files a pull fetches from it
pub async fn repo_snapshot(
    modelname: &str,
    quant: Option<&str>,
) -> Result<(String, Vec<RepoFile>)> {
    #[derive(Deserialize)]
    struct Listing {
        sha: String,
        siblings: Vec<File>,
    }
    #[derive(Deserialize)]
    struct File {
        rfilename: String,
        size: Option<u64>,
        #[serde(rename = "blobId")]
        blob_id: Option<String>,
        lfs: Option<Lfs>,
    }
    #[derive(Deserialize)]
    struct Lfs {
        sha256: String,
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
    let files = listing
        .siblings
        .into_iter()
        .filter(|file| wanted(&file.rfilename, &quant_gguf))
        .map(|file| RepoFile {
            size: file.size.unwrap_or(0),
            blob: file
                .lfs
                .map(|lfs| lfs.sha256)
                .or(file.blob_id)
                .unwrap_or_default(),
            name: file.rfilename,
        })
        .collect();
    Ok((listing.sha, files))
}

/// where hf-hub keeps a model: `blobs/`, `snapshots/<commit>/` and `refs/`
pub fn repo_dir(modelname: &str) -> Result<std::path::PathBuf> {
    Ok(get_or_create_model_download_path()?
        .join(format!("models--{}", modelname.replace('/', "--"))))
}

/// bytes a pull of this model would fetch
pub async fn download_size(modelname: &str, quant: Option<&str>) -> Result<u64> {
    Ok(repo_files(modelname, quant)
        .await?
        .iter()
        .map(|file| file.size)
        .sum())
}

/// How far a pull has got on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Downloaded {
    pub bytes: u64,
    /// every file is complete, which is what a model needs to load
    pub complete: bool,
}

/// Bytes already on disk for `files`. hf-hub writes a file as
/// `<blob>.sync.part`, sized to the file plus 8 bytes that record how much of
/// it is written, and renames it to `<blob>` once done, so both are readable
/// without having to track anything ourselves.
pub fn downloaded(modelname: &str, files: &[RepoFile]) -> Result<Downloaded> {
    Ok(downloaded_in(&repo_dir(modelname)?.join("blobs"), files))
}

fn downloaded_in(blobs: &Path, files: &[RepoFile]) -> Downloaded {
    let mut bytes = 0;
    let mut complete = true;
    for file in files {
        let blob = blobs.join(&file.blob);
        if blob.metadata().is_ok_and(|meta| meta.len() == file.size) {
            bytes += file.size;
            continue;
        }
        complete = false;
        bytes += committed(&blobs.join(format!("{}.sync.part", file.blob)), file.size).unwrap_or(0);
    }
    Downloaded { bytes, complete }
}

/// bytes of `part` already written, from the marker in its last 8 bytes
pub fn committed(part: &Path, size: u64) -> Option<u64> {
    let mut file = std::fs::File::open(part).ok()?;
    if file.metadata().ok()?.len() != size + 8 {
        return None;
    }
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(size)).ok()?;
    let mut marker = [0u8; 8];
    file.read_exact(&mut marker).ok()?;
    Some(u64::from_le_bytes(marker).min(size))
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
    use super::{
        Downloaded, RepoFile, downloaded_in, human_bytes, is_main_gguf, quant_gguf, wanted,
    };

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
        // every repo ships its own head, not just the 12b
        assert!(wanted("mtp-gemma-4-E2B-it.gguf", &quant));
        assert!(!wanted("mmproj-F16.gguf", &quant));
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

    #[test]
    fn progress_is_read_off_finished_and_partial_blobs() {
        let blobs = tempfile::tempdir().unwrap();
        let file = |name: &str, size, blob: &str| RepoFile {
            name: name.to_owned(),
            size,
            blob: blob.to_owned(),
        };
        let files = [
            file("config.json", 10, "aaa"),
            file("model.gguf", 100, "bbb"),
        ];

        std::fs::write(blobs.path().join("aaa"), [0u8; 10]).unwrap();
        // 40 of 100 written: the file plus an 8 byte marker holding 40
        let mut part = vec![0u8; 100];
        part.extend_from_slice(&40u64.to_le_bytes());
        std::fs::write(blobs.path().join("bbb.sync.part"), part).unwrap();
        assert_eq!(
            downloaded_in(blobs.path(), &files),
            Downloaded {
                bytes: 50,
                complete: false
            }
        );

        std::fs::remove_file(blobs.path().join("bbb.sync.part")).unwrap();
        std::fs::write(blobs.path().join("bbb"), [0u8; 100]).unwrap();
        assert_eq!(
            downloaded_in(blobs.path(), &files),
            Downloaded {
                bytes: 110,
                complete: true
            }
        );

        assert_eq!(
            downloaded_in(&blobs.path().join("nothing-here"), &files),
            Downloaded {
                bytes: 0,
                complete: false
            }
        );
    }
}
