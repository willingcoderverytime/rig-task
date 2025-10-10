use rig::agent::{Agent, AgentBuilder};
use rig::client::completion::CompletionModelHandle;
use rig::client::{AgentConfig, ProviderClient};
use rig::completion::CompletionModelDyn;
use rig::embeddings::embedding::EmbeddingModelDyn;
use std::collections::HashMap;
use std::panic::{RefUnwindSafe, UnwindSafe};
use thiserror::Error;

use crate::agent_support::DefaultProviders;

#[derive(Debug, Error)]
pub enum ClientBuildError {
    #[error("factory error: {}", .0)]
    FactoryError(String),
    #[error("invalid id string: {}", .0)]
    InvalidIdString(String),
    #[error("unsupported feature: {} for {}", .1, .0)]
    UnsupportedFeature(String, String),
    #[error("unknown provider")]
    UnknownProvider,
}

pub type BoxCompletionModel<'a> = Box<dyn CompletionModelDyn + 'a>;
pub type BoxAgentBuilder<'a> = AgentBuilder<CompletionModelHandle<'a>>;
pub type BoxAgent<'a> = Agent<CompletionModelHandle<'a>>;
pub type BoxEmbeddingModel<'a> = Box<dyn EmbeddingModelDyn + 'a>;
#[derive(Default)]
pub struct DynClientBuilder {
    pub registry: HashMap<DefaultProviders, ClientFactory>,
}

impl<'a> DynClientBuilder {
    /// Generate a new instance of `DynClientBuilder`.
    /// By default, every single possible client that can be registered
    /// will be registered to the client builder.

    /// Register multiple ClientFactories
    pub fn register_all(mut self, factories: impl IntoIterator<Item = ClientFactory>) -> Self {
        for factory in factories {
            self.registry.insert(factory.name, factory);
        }
        self
    }

    /// Returns a (boxed) specific provider based on the given provider.
    fn build(
        &self,
        provider: DefaultProviders,
        agent_config: AgentConfig,
    ) -> Result<Box<dyn ProviderClient>, ClientBuildError> {
        let factory = self.get_factory(provider)?;
        factory.build(agent_config)
    }

    /// Returns a specific client factory (that exists in the registry).
    fn get_factory(&self, provider: DefaultProviders) -> Result<&ClientFactory, ClientBuildError> {
        self.registry
            .get(&provider)
            .ok_or(ClientBuildError::UnknownProvider)
    }

    /// Get a boxed agent based on the provider and model..
    pub fn agent(
        &self,
        provider: DefaultProviders,
        config: AgentConfig,
    ) -> Result<Agent<CompletionModelHandle<'_>>, ClientBuildError> {
        let modle = config.model.clone();
        let client = self.build(provider, config.clone())?;

        let client = client
            .as_completion()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "completion".to_string(),
            ))?;

        let build = client.agent(&modle);
        let mut agent_builder = build
            .preamble(&config.sys_promte.unwrap_or_default());

        // 设置名称
        if !config.name.is_empty() {
            agent_builder = agent_builder.name(&config.name);
        }

        // 设置描述
        if let Some(desc) = &config.desc {
            agent_builder = agent_builder.description(desc);
        }

        let agent = agent_builder.build();

        Ok(agent)
    }

    // pub fn embeddings(
    //     &self,
    //     provider: &str,
    //     model: &str,
    // ) -> Result<Box<dyn EmbeddingModelDyn + 'a>, ClientBuildError> {
    //     let client = self.build(provider)?;

    //     let embeddings = client
    //         .as_embeddings()
    //         .ok_or(ClientBuildError::UnsupportedFeature(
    //             provider.to_string(),
    //             "embeddings".to_owned(),
    //         ))?;

    //     Ok(embeddings.embedding_model(model))
    // }
}
pub struct ClientFactory {
    pub name: DefaultProviders,
    pub create_by_config: Box<dyn Fn(AgentConfig) -> Box<dyn ProviderClient> + Send + Sync>,
}

impl UnwindSafe for ClientFactory {}
impl RefUnwindSafe for ClientFactory {}

impl ClientFactory {
    pub fn new<F1>(name: DefaultProviders, create_by_config: F1) -> Self
    where
        F1: 'static + Fn(AgentConfig) -> Box<dyn ProviderClient> + Send + Sync,
    {
        Self {
            name,
            create_by_config: Box::new(create_by_config),
        }
    }

    fn build(&self, agent_conf: AgentConfig) -> Result<Box<dyn ProviderClient>, ClientBuildError> {
        std::panic::catch_unwind(|| (self.create_by_config)(agent_conf))
            .map_err(|e| ClientBuildError::FactoryError(format!("{e:?}")))
    }
}
