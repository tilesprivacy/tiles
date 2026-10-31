//! The models Tiles ships a Modelfile for

use futures_util::future::join_all;
use serde::Serialize;

use std::path::{Path, PathBuf};

use super::{Models, Spec, download::ModelStatus, modelfile_spec};

// the Modelfile and the name the UI shows, smallest first
pub const BUNDLED: [(&str, &str); 5] = [
    ("gemma-4-e2b-gguf", "Gemma 4 E2B"),
    ("gemma-4-e4b-gguf", "Gemma 4 E4B"),
    ("gemma-4-12b-gguf", "Gemma 4 12B"),
    ("gemma-4-26b-a4b-gguf", "Gemma 4 26B-A4B"),
    ("gemma-4-31b-gguf", "Gemma 4 31B"),
];

#[derive(Debug, Serialize)]
pub struct BundledModel {
    id: &'static str,
    label: &'static str,
    default: bool,
    // last ran by `tiles run`
    current: bool,
    #[serde(flatten)]
    listing: Listing,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Listing {
    Status(ModelStatus),
    Failed {
        model: Option<String>,
        error: String,
    },
}

impl Models {
    pub async fn bundled(&self) -> Vec<BundledModel> {
        let current = self.current();
        join_all(BUNDLED.iter().map(|&(id, label)| {
            let spec = modelfile_spec(&self.modelfiles.join(id));
            let is = |other: &Option<Spec>| other.is_some() && spec.as_ref().ok() == other.as_ref();
            let (default, current) = (is(&self.default), is(&current));
            async move {
                BundledModel {
                    id,
                    label,
                    default,
                    current,
                    listing: self.listing(spec).await,
                }
            }
        }))
        .await
    }

    async fn listing(&self, spec: anyhow::Result<Spec>) -> Listing {
        let spec = match spec {
            Ok(spec) => spec,
            Err(err) => {
                return Listing::Failed {
                    model: None,
                    error: format!("{err:#}"),
                };
            }
        };
        match self.download(spec.clone()).status().await {
            Ok(status) => Listing::Status(status),
            Err(err) => Listing::Failed {
                model: Some(spec.to_string()),
                error: err.reason(),
            },
        }
    }
}

/// The Modelfile Tiles ships for `spec`, if any
pub fn modelfile_for(modelfiles: &Path, spec: &Spec) -> Option<PathBuf> {
    BUNDLED
        .iter()
        .map(|(id, _)| modelfiles.join(id))
        .find(|path| modelfile_spec(path).is_ok_and(|found| &found == spec))
}

/// The Modelfile of the model chats should use, when it's one of Tiles' own
pub fn picked_modelfile(modelfiles: &Path, picked: &str) -> Option<PathBuf> {
    modelfile_for(modelfiles, &Spec::parse(picked).ok()?)
}
