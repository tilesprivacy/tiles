//! The models Tiles offers to download, each with its own Modelfile in the lib dir

use futures_util::future::join_all;
use serde::Serialize;

use super::{Models, Spec, download::ModelStatus, modelfile_spec};

pub const LINEUP: [(&str, &str); 5] = [
    ("gemma-4-e2b-gguf", "Gemma 4 E2B"),
    ("gemma-4-e4b-gguf", "Gemma 4 E4B"),
    ("gemma-4-12b-gguf", "Gemma 4 12B"),
    ("gemma-4-26b-a4b-gguf", "Gemma 4 26B-A4B"),
    ("gemma-4-31b-gguf", "Gemma 4 31B"),
];

#[derive(Debug, Serialize)]
pub struct LineupModel {
    id: &'static str,
    label: &'static str,
    // whether `tiles run` last ran this model
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
    /// Every model in the lineup with its download, asking Hugging Face about all of
    /// them at once. `current` is the model `tiles run` last ran
    pub async fn lineup(&self, current: Option<&str>) -> Vec<LineupModel> {
        let current = current.and_then(|model| Spec::parse(model).ok());
        join_all(LINEUP.iter().map(|&(id, label)| {
            let spec = modelfile_spec(&self.modelfiles.join(id));
            let current = spec.as_ref().ok() == current.as_ref() && current.is_some();
            async move {
                LineupModel {
                    id,
                    label,
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
