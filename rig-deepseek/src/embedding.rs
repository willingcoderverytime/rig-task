use rig::embeddings::{Embedding, EmbeddingError, EmbeddingModel};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::convert::{ApiErrorResponse, ApiResponse};

use super::client::Client;

#[derive(Clone)]
pub struct OlEmbeddingModel {
    client: Client,
    pub model: String,
    ndims: usize,
}

impl OlEmbeddingModel {
    pub fn new(client: Client, model: &str, ndims: usize) -> Self {
        Self {
            client,
            model: model.to_owned(),
            ndims,
        }
    }
}

impl EmbeddingModel for OlEmbeddingModel {
    const MAX_DOCUMENTS: usize = 1024;
    fn ndims(&self) -> usize {
        self.ndims
    }
    #[cfg_attr(feature = "worker", worker::send)]
    async fn embed_texts(
        &self,
        documents: impl IntoIterator<Item = String>,
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        let docs: Vec<String> = documents.into_iter().collect();
        let payload = json!({
            "model": self.model,
            "input": docs,
        });
        let response = self.client.post("api/embed")?.json(&payload).send().await?;

        if !response.status().is_success() {
            return Err(EmbeddingError::ProviderError(response.text().await?));
        }

        let bytes = response.bytes().await?;

        let api_resp: EmbeddingResponse = serde_json::from_slice(&bytes)?;

        if api_resp.embeddings.len() != docs.len() {
            return Err(EmbeddingError::ResponseError(
                "Number of returned embeddings does not match input".into(),
            ));
        }
        Ok(api_resp
            .embeddings
            .into_iter()
            .zip(docs.into_iter())
            .map(|(vec, document)| Embedding { document, vec })
            .collect())
    }
}

pub const ALL_MINILM: &str = "all-minilm";
pub const NOMIC_EMBED_TEXT: &str = "nomic-embed-text";

#[derive(Debug, Serialize, Deserialize)]
struct EmbeddingResponse {
    pub model: String,
    pub embeddings: Vec<Vec<f64>>,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub load_duration: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u64>,
}

impl From<ApiErrorResponse> for EmbeddingError {
    fn from(err: ApiErrorResponse) -> Self {
        EmbeddingError::ProviderError(err.message)
    }
}

impl From<ApiResponse<EmbeddingResponse>> for Result<EmbeddingResponse, EmbeddingError> {
    fn from(value: ApiResponse<EmbeddingResponse>) -> Self {
        match value {
            ApiResponse::Ok(response) => Ok(response),
            ApiResponse::Err(err) => Err(EmbeddingError::ProviderError(err.message)),
        }
    }
}
