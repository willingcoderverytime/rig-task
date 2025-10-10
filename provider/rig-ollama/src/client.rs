//! Ollama API client and Rig integration
//!
//! # Example
//! ```rust
//! use rig::providers::ollama;
//!
//! // Create a new Ollama client (defaults to http://localhost:11434)
//! let client = ollama::Client::new();
//!
//! // Create a completion model interface using, for example, the "llama3.2" model
//! let comp_model = client.completion_model("llama3.2");
//!
//! let req = rig::completion::CompletionRequest {
//!     preamble: Some("You are now a humorous AI assistant.".to_owned()),
//!     chat_history: vec![],  // internal messages (if any)
//!     prompt: rig::message::Message::User {
//!         content: rig::one_or_many::OneOrMany::one(rig::message::UserContent::text("Please tell me why the sky is blue.")),
//!         name: None
//!     },
//!     temperature: 0.7,
//!     additional_params: None,
//!     tools: vec![],
//! };
//!
//! let response = comp_model.completion(req).await.unwrap();
//! println!("Ollama completion response: {:?}", response.choice);
//!
//! // Create an embedding interface using the "all-minilm" model
//! let emb_model = ollama::Client::new().embedding_model("all-minilm");
//! let docs = vec![
//!     "Why is the sky blue?".to_owned(),
//!     "Why is the grass green?".to_owned()
//! ];
//! let embeddings = emb_model.embed_texts(docs).await.unwrap();
//! println!("Embedding response: {:?}", embeddings);
//!
//! // Also create an agent and extractor if needed
//! let agent = client.agent("llama3.2");
//! let extractor = client.extractor::<serde_json::Value>("llama3.2");
//! ```
use rig::client::{
    ClientBuilderError, CompletionClient, EmbeddingsClient, ProviderClient, VerifyClient,
    VerifyError,
};
use rig::embeddings::EmbeddingsBuilder;

use reqwest;
use rig::Embed;
// use reqwest_eventsource::{Event, RequestBuilderExt}; // (Not used currently as Ollama does not support SSE)
use url::Url;

use crate::completion::OllamaCompletionModel;
use crate::embedding::OlEmbeddingModel;
// ---------- Main Client ----------

const OLLAMA_API_BASE_URL: &str = "http://localhost:11434";

pub struct ClientBuilder<'a> {
    base_url: &'a str,
    http_client: Option<reqwest::Client>,
}

impl<'a> ClientBuilder<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            base_url: OLLAMA_API_BASE_URL,
            http_client: None,
        }
    }

    pub fn base_url(mut self, base_url: &'a str) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn custom_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn build(self) -> Result<Client, ClientBuilderError> {
        let http_client = if let Some(http_client) = self.http_client {
            http_client
        } else {
            reqwest::Client::builder().build()?
        };

        Ok(Client {
            base_url: Url::parse(self.base_url)
                .map_err(|_| ClientBuilderError::InvalidProperty("base_url"))?,
            http_client,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    base_url: Url,
    http_client: reqwest::Client,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Create a new Ollama client builder.
    ///
    /// # Example
    /// ```
    /// use rig::providers::ollama::{ClientBuilder, self};
    ///
    /// // Initialize the Ollama client
    /// let client = Client::builder()
    ///    .build()
    /// ```
    pub fn builder() -> ClientBuilder<'static> {
        ClientBuilder::new()
    }

    /// Create a new Ollama client. For more control, use the `builder` method.
    ///
    /// # Panics
    /// - If the reqwest client cannot be built (if the TLS backend cannot be initialized).
    pub fn new() -> Self {
        Self::builder().build().expect("Ollama client should build")
    }

    pub(crate) fn post(&self, path: &str) -> Result<reqwest::RequestBuilder, url::ParseError> {
        let url = self.base_url.join(path)?;
        Ok(self.http_client.post(url))
    }

    pub(crate) fn get(&self, path: &str) -> Result<reqwest::RequestBuilder, url::ParseError> {
        let url = self.base_url.join(path)?;
        Ok(self.http_client.get(url))
    }
}

impl ProviderClient for Client {
    fn from_config(config: rig::client::AgentConfig) -> Box<dyn ProviderClient>
    where
        Self: Sized,
    {
        Box::new(Self::builder().base_url(&config.base_url).build().unwrap())
    }
}

impl CompletionClient for Client {
    type CompletionModel = OllamaCompletionModel;

    fn completion_model(&self, model: &str) -> Self::CompletionModel {
        OllamaCompletionModel::new(self.clone(), model)
    }
}

impl EmbeddingsClient for Client {
    type EmbeddingModel = OlEmbeddingModel;
    fn embedding_model(&self, model: &str) -> Self::EmbeddingModel {
        OlEmbeddingModel::new(self.clone(), model, 0)
    }
    fn embedding_model_with_ndims(&self, model: &str, ndims: usize) -> Self::EmbeddingModel {
        OlEmbeddingModel::new(self.clone(), model, ndims)
    }
    fn embeddings<D: Embed>(&self, model: &str) -> EmbeddingsBuilder<Self::EmbeddingModel, D> {
        EmbeddingsBuilder::new(self.embedding_model(model))
    }
}

impl VerifyClient for Client {
    #[cfg_attr(feature = "worker", worker::send)]
    async fn verify(&self) -> Result<(), VerifyError> {
        let response = self
            .get("api/tags")
            .expect("Failed to build request")
            .send()
            .await?;
        match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => {
                response.error_for_status()?;
                Ok(())
            }
        }
    }
}
