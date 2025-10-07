//! This module provides traits for defining and creating provider clients.
//! Clients are used to create models for completion, embeddings, etc.
//! Dyn-compatible traits have been provided to allow for more provider-agnostic code.

pub mod builder;
pub mod completion;
pub mod embeddings;
pub mod verify;

#[cfg(feature = "derive")]
pub use rig_derive::ProviderClient;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ClientBuilderError {
    #[error("reqwest error: {0}")]
    HttpError(
        #[from]
        #[source]
        reqwest::Error,
    ),
    #[error("invalid property: {0}")]
    InvalidProperty(&'static str),
}

/// The base ProviderClient trait, facilitates conversion between client types
/// and creating a client from the environment.
///
/// All conversion traits must be implemented, they are automatically
/// implemented if the respective client trait is implemented.
pub trait ProviderClient: AsCompletion + AsEmbeddings + Debug {
    /// Create a client from the process's environment.
    /// Panics if an environment is improperly configured.
    fn from_env() -> Self
    where
        Self: Sized;

    /// A helper method to box the client.
    fn boxed(self) -> Box<dyn ProviderClient>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Create a boxed client from the process's environment.
    /// Panics if an environment is improperly configured.
    fn from_env_boxed<'a>() -> Box<dyn ProviderClient + 'a>
    where
        Self: Sized,
        Self: 'a,
    {
        Box::new(Self::from_env())
    }

    fn from_val(input: ProviderValue) -> Self
    where
        Self: Sized;

    /// Create a boxed client from the process's environment.
    /// Panics if an environment is improperly configured.
    fn from_val_boxed<'a>(input: ProviderValue) -> Box<dyn ProviderClient + 'a>
    where
        Self: Sized,
        Self: 'a,
    {
        Box::new(Self::from_val(input))
    }
}

#[derive(Clone)]
pub enum ProviderValue {
    Simple(String),
    ApiKeyWithOptionalKey(String, Option<String>),
    ApiKeyWithVersionAndHeader(String, String, String),
}

impl From<&str> for ProviderValue {
    fn from(value: &str) -> Self {
        Self::Simple(value.to_string())
    }
}

impl From<String> for ProviderValue {
    fn from(value: String) -> Self {
        Self::Simple(value)
    }
}

impl<P> From<(P, Option<P>)> for ProviderValue
where
    P: AsRef<str>,
{
    fn from((api_key, optional_key): (P, Option<P>)) -> Self {
        Self::ApiKeyWithOptionalKey(
            api_key.as_ref().to_string(),
            optional_key.map(|x| x.as_ref().to_string()),
        )
    }
}

impl<P> From<(P, P, P)> for ProviderValue
where
    P: AsRef<str>,
{
    fn from((api_key, version, header): (P, P, P)) -> Self {
        Self::ApiKeyWithVersionAndHeader(
            api_key.as_ref().to_string(),
            version.as_ref().to_string(),
            header.as_ref().to_string(),
        )
    }
}

/// Attempt to convert a ProviderClient to a CompletionClient
pub trait AsCompletion {
    fn as_completion(&self) -> Option<Box<dyn CompletionClientDyn>> {
        None
    }
}

/// Attempt to convert a ProviderClient to a EmbeddingsClient
pub trait AsEmbeddings {
    fn as_embeddings(&self) -> Option<Box<dyn EmbeddingsClientDyn>> {
        None
    }
}

/// Attempt to convert a ProviderClient to a VerifyClient
pub trait AsVerify {
    fn as_verify(&self) -> Option<Box<dyn VerifyClientDyn>> {
        None
    }
}

/// Implements the conversion traits for a given struct
/// ```rust
/// pub struct Client;
/// impl ProviderClient for Client {
///     ...
/// }
/// impl_conversion_traits!(AsCompletion, AsEmbeddings for Client);
/// ```
#[macro_export]
macro_rules! impl_conversion_traits {
    ($( $trait_:ident ),* for $struct_:ident ) => {
        $(
            impl_conversion_traits!(@impl $trait_ for $struct_);
        )*
    };


    (@impl $trait_:ident for $struct_:ident) => {
        impl rig::client::$trait_ for $struct_ {}
    };
}
pub use impl_conversion_traits;

use crate::client::completion::CompletionClientDyn;
use crate::client::embeddings::EmbeddingsClientDyn;
use crate::client::verify::VerifyClientDyn;

pub use crate::client::completion::CompletionClient;
pub use crate::client::embeddings::EmbeddingsClient;
pub use crate::client::verify::{VerifyClient, VerifyError};

#[cfg(test)]
mod tests {
    use crate::OneOrMany;
    use crate::client::ProviderClient;
    use crate::completion::{Completion, CompletionRequest, ToolDefinition};
    use crate::message::AssistantContent;
    use crate::providers::{cohere, gemini, huggingface, openai};
    use crate::streaming::StreamingCompletion;
    use crate::tool::Tool;
    use futures::StreamExt;
    use rig::message::Message;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::ProviderValue;

    struct ClientConfig {
        name: &'static str,
        factory_env: Box<dyn Fn() -> Box<dyn ProviderClient>>,
        // Not sure where we're going to be using this but I've added it for completeness
        #[allow(dead_code)]
        factory_val: Box<dyn Fn(ProviderValue) -> Box<dyn ProviderClient>>,
        env_variable: &'static str,
        completion_model: Option<&'static str>,
        embeddings_model: Option<&'static str>,
    }

    impl Default for ClientConfig {
        fn default() -> Self {
            Self {
                name: "",
                factory_env: Box::new(|| panic!("Not implemented")),
                factory_val: Box::new(|_| panic!("Not implemented")),
                env_variable: "",
                completion_model: None,
                embeddings_model: None,
            }
        }
    }

    impl ClientConfig {
        fn is_env_var_set(&self) -> bool {
            self.env_variable.is_empty() || std::env::var(self.env_variable).is_ok()
        }

        fn factory_env(&self) -> Box<dyn ProviderClient + '_> {
            self.factory_env.as_ref()()
        }
    }

    fn providers() -> Vec<ClientConfig> {
        vec![
            ClientConfig {
                name: "Cohere",
                factory_env: Box::new(cohere::Client::from_env_boxed),
                factory_val: Box::new(cohere::Client::from_val_boxed),
                env_variable: "COHERE_API_KEY",
                completion_model: Some(cohere::COMMAND_R),
                embeddings_model: Some(cohere::EMBED_ENGLISH_LIGHT_V2),
                ..Default::default()
            },
            ClientConfig {
                name: "Gemini",
                factory_env: Box::new(gemini::Client::from_env_boxed),
                factory_val: Box::new(gemini::Client::from_val_boxed),
                env_variable: "GEMINI_API_KEY",
                completion_model: Some(gemini::completion::GEMINI_2_0_FLASH),
                embeddings_model: Some(gemini::embedding::EMBEDDING_001),
                ..Default::default()
            },
            ClientConfig {
                name: "Huggingface",
                factory_env: Box::new(huggingface::Client::from_env_boxed),
                factory_val: Box::new(huggingface::Client::from_val_boxed),
                env_variable: "HUGGINGFACE_API_KEY",
                completion_model: Some(huggingface::PHI_4),
                ..Default::default()
            },
            ClientConfig {
                name: "OpenAI",
                factory_env: Box::new(openai::Client::from_env_boxed),
                factory_val: Box::new(openai::Client::from_val_boxed),
                env_variable: "OPENAI_API_KEY",
                completion_model: Some(openai::GPT_4O),
                embeddings_model: Some(openai::TEXT_EMBEDDING_ADA_002),
                ..Default::default()
            },
            // ClientConfig {
            //     name: "Deepseek",
            //     factory_env: Box::new(deepseek::client::Client::from_env_boxed),
            //     factory_val: Box::new(deepseek::client::Client::from_val_boxed),
            //     env_variable: "DEEPSEEK_API_KEY",
            //     completion_model: Some(deepseek::completion::DEEPSEEK_CHAT),
            //     ..Default::default()
            // },
 
        ]
    }

    async fn test_completions_client(config: &ClientConfig) {
        let client = config.factory_env();

        let Some(client) = client.as_completion() else {
            return;
        };

        let model = config
            .completion_model
            .unwrap_or_else(|| panic!("{} does not have completion_model set", config.name));

        let model = client.completion_model(model);

        let resp = model
            .completion_request(Message::user("Whats the capital of France?"))
            .send()
            .await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when prompting, {}",
            config.name,
            resp.err().unwrap()
        );

        let resp = resp.unwrap();

        match resp.choice.first() {
            AssistantContent::Text(text) => {
                assert!(text.text.to_lowercase().contains("paris"));
            }
            _ => {
                unreachable!(
                    "[{}]: First choice wasn't a Text message, {:?}",
                    config.name,
                    resp.choice.first()
                );
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_completions() {
        for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_completions_client(&p).await;
        }
    }

    async fn test_tools_client(config: &ClientConfig) {
        let client = config.factory_env();
        let model = config
            .completion_model
            .unwrap_or_else(|| panic!("{} does not have the model set.", config.name));

        let Some(client) = client.as_completion() else {
            return;
        };

        let model = client.agent(model)
            .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.")
            .max_tokens(1024)
            .tool(Adder)
            .tool(Subtract)
            .build();

        let request = model.completion("Calculate 2 - 5", vec![]).await;

        assert!(
            request.is_ok(),
            "[{}]: Error occurred when building prompt, {}",
            config.name,
            request.err().unwrap()
        );

        let resp = request.unwrap().send().await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when prompting, {}",
            config.name,
            resp.err().unwrap()
        );

        let resp = resp.unwrap();

        assert!(
            resp.choice.iter().any(|content| match content {
                AssistantContent::ToolCall(tc) => {
                    if tc.function.name != Subtract::NAME {
                        return false;
                    }

                    let arguments =
                        serde_json::from_value::<OperationArgs>((tc.function.arguments).clone())
                            .expect("Error parsing arguments");

                    arguments.x == 2.0 && arguments.y == 5.0
                }
                _ => false,
            }),
            "[{}]: Model did not use the Subtract tool.",
            config.name
        )
    }

    #[tokio::test]
    #[ignore]
    async fn test_tools() {
        for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_tools_client(&p).await;
        }
    }

    async fn test_streaming_client(config: &ClientConfig) {
        let client = config.factory_env();

        let Some(client) = client.as_completion() else {
            return;
        };

        let model = config
            .completion_model
            .unwrap_or_else(|| panic!("{} does not have the model set.", config.name));

        let model = client.completion_model(model);

        let resp = model.stream(CompletionRequest {
            preamble: None,
            tools: vec![],
            documents: vec![],
            temperature: None,
            max_tokens: None,
            additional_params: None,
            tool_choice: None,
            chat_history: OneOrMany::one(Message::user("What is the capital of France?")),
        });

        let mut resp = resp.await.unwrap();

        let mut received_chunk = false;

        while let Some(chunk) = resp.next().await {
            received_chunk = true;
            assert!(chunk.is_ok());
        }

        assert!(
            received_chunk,
            "[{}]: Failed to receive a chunk from stream",
            config.name
        );

        for choice in resp.choice {
            match choice {
                AssistantContent::Text(text) => {
                    assert!(
                        text.text.to_lowercase().contains("paris"),
                        "[{}]: Did not answer with Paris",
                        config.name
                    );
                }
                AssistantContent::ToolCall(_) => {}
                AssistantContent::Reasoning(_) => {}
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_streaming() {
        for provider in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_streaming_client(&provider).await;
        }
    }

    async fn test_streaming_tools_client(config: &ClientConfig) {
        let client = config.factory_env();
        let model = config
            .completion_model
            .unwrap_or_else(|| panic!("{} does not have the model set.", config.name));

        let Some(client) = client.as_completion() else {
            return;
        };

        let model = client.agent(model)
            .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.")
            .max_tokens(1024)
            .tool(Adder)
            .tool(Subtract)
            .build();

        let request = model.stream_completion("Calculate 2 - 5", vec![]).await;

        assert!(
            request.is_ok(),
            "[{}]: Error occurred when building prompt, {}",
            config.name,
            request.err().unwrap()
        );

        let resp = request.unwrap().stream().await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when prompting, {}",
            config.name,
            resp.err().unwrap()
        );

        let mut resp = resp.unwrap();

        let mut received_chunk = false;

        while let Some(chunk) = resp.next().await {
            received_chunk = true;
            assert!(chunk.is_ok());
        }

        assert!(
            received_chunk,
            "[{}]: Failed to receive a chunk from stream",
            config.name
        );

        assert!(
            resp.choice.iter().any(|content| match content {
                AssistantContent::ToolCall(tc) => {
                    if tc.function.name != Subtract::NAME {
                        return false;
                    }

                    let arguments =
                        serde_json::from_value::<OperationArgs>((tc.function.arguments).clone())
                            .expect("Error parsing arguments");

                    arguments.x == 2.0 && arguments.y == 5.0
                }
                _ => false,
            }),
            "[{}]: Model did not use the Subtract tool.",
            config.name
        )
    }

    #[tokio::test]
    #[ignore]
    async fn test_streaming_tools() {
        for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_streaming_tools_client(&p).await;
        }
    }

    fn assert_feature<F, M>(
        name: &str,
        feature_name: &str,
        model_name: &str,
        feature: Option<F>,
        model: Option<M>,
    ) {
        assert_eq!(
            feature.is_some(),
            model.is_some(),
            "{} has{} implemented {} but config.{} is {}.",
            name,
            if feature.is_some() { "" } else { "n't" },
            feature_name,
            model_name,
            if model.is_some() { "some" } else { "none" }
        );
    }

    #[test]
    #[ignore]
    pub fn test_polymorphism() {
        for config in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            let client = config.factory_env();
            assert_feature(
                config.name,
                "AsCompletion",
                "completion_model",
                client.as_completion(),
                config.completion_model,
            );

            assert_feature(
                config.name,
                "AsEmbeddings",
                "embeddings_model",
                client.as_embeddings(),
                config.embeddings_model,
            );
        }
    }

    async fn test_embed_client(config: &ClientConfig) {
        const TEST: &str = "Hello world.";

        let client = config.factory_env();

        let Some(client) = client.as_embeddings() else {
            return;
        };

        let model = config.embeddings_model.unwrap();

        let model = client.embedding_model(model);

        let resp = model.embed_text(TEST).await;

        assert!(
            resp.is_ok(),
            "[{}]: Error occurred when sending request, {}",
            config.name,
            resp.err().unwrap()
        );

        let resp = resp.unwrap();

        assert_eq!(resp.document, TEST);

        assert!(
            !resp.vec.is_empty(),
            "[{}]: Returned embed was empty",
            config.name
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_embed() {
        for config in providers().into_iter().filter(ClientConfig::is_env_var_set) {
            test_embed_client(&config).await;
        }
    }

    #[derive(Deserialize)]
    struct OperationArgs {
        x: f32,
        y: f32,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Math error")]
    struct MathError;

    #[derive(Deserialize, Serialize)]
    struct Adder;
    impl Tool for Adder {
        const NAME: &'static str = "add";

        type Error = MathError;
        type Args = OperationArgs;
        type Output = f32;

        async fn definition(&self, _prompt: String) -> ToolDefinition {
            ToolDefinition {
                name: "add".to_string(),
                description: "Add x and y together".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "x": {
                            "type": "number",
                            "description": "The first number to add"
                        },
                        "y": {
                            "type": "number",
                            "description": "The second number to add"
                        }
                    }
                }),
            }
        }

        async fn call(&self, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
            println!("[tool-call] Adding {} and {}", args.x, args.y);
            let result = args.x + args.y;
            Ok(result)
        }
    }

    #[derive(Deserialize, Serialize)]
    struct Subtract;
    impl Tool for Subtract {
        const NAME: &'static str = "subtract";

        type Error = MathError;
        type Args = OperationArgs;
        type Output = f32;

        async fn definition(&self, _prompt: String) -> ToolDefinition {
            serde_json::from_value(json!({
                "name": "subtract",
                "description": "Subtract y from x (i.e.: x - y)",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "x": {
                            "type": "number",
                            "description": "The number to subtract from"
                        },
                        "y": {
                            "type": "number",
                            "description": "The number to subtract"
                        }
                    }
                }
            }))
            .expect("Tool Definition")
        }

        async fn call(&self, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
            println!("[tool-call] Subtracting {} from {}", args.y, args.x);
            let result = args.x - args.y;
            Ok(result)
        }
    }
}
