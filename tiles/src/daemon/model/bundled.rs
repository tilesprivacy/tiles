//! The models Tiles ships a Modelfile for, listed when `status` names no model

use futures_util::future::join_all;
use serde::Serialize;

use super::{Models, Spec, download::ModelStatus, modelfile_spec};

/// Smallest first, as the Modelfile's name and the name the UI shows
pub const BUNDLED: [(&str, &str); 5] = [
    ("gemma-4-e2b-gguf", "Gemma 4 E2B"),
    ("gemma-4-e4b-gguf", "Gemma 4 E4B"),
    ("gemma-4-12b-gguf", "Gemma 4 12B"),
    ("gemma-4-26b-a4b-gguf", "Gemma 4 26B-A4B"),
    ("gemma-4-31b-gguf", "Gemma 4 31B"),
];

#[derive(Debug, Serialize)]
pub struct BundledModel {
    /// The Modelfile's name
    id: &'static str,
    label: &'static str,
    /// The model used when a request names none
    default: bool,
    // whether `tiles run` last ran this model
    current: bool,
    #[serde(flatten)]
    listing: Listing,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Listing {
    Status(ModelStatus),
    /// Its Modelfile couldn't be read, or Hugging Face couldn't be asked
    Failed {
        model: Option<String>,
        error: String,
    },
}

impl Models {
    /// Every bundled model with its download, asking Hugging Face about all of them
    /// at once
    pub async fn bundled(&self) -> Vec<BundledModel> {
        let current = (self.current_model)().and_then(|model| Spec::parse(&model).ok());
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
