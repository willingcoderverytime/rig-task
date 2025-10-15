use crate::agent_support::DefaultProviders;
use rig::agent::{Agent, AgentBuilder};
use rig::client::completion::CompletionModelHandle;
use rig::client::{AgentConfig, McpStdio, McpType, ProviderClient};
use rig::completion::CompletionModelDyn;
use rig::embeddings::embedding::EmbeddingModelDyn;
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, InitializeRequestParam};
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt as _, TokioChildProcess};
use rmcp::{RoleClient, ServiceExt as _};
use std::collections::HashMap;
use std::panic::{RefUnwindSafe, UnwindSafe};
use thiserror::Error;
use tokio::process::Command;

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
    #[error("Stdio MCP Execute Failed")]
    MCPStidioExecuteFailed(std::io::Error),
    #[error("Stdio MCP Client Init Failed {}",.0)]
    MCPClinetInitError(rmcp::service::ClientInitializeError),
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
    pub async fn agent(
        &self,
        provider: DefaultProviders,
        config: AgentConfig,
    ) -> Result<Agent<CompletionModelHandle<'static>>, ClientBuildError> {
        let modle = config.model.clone();
        let client = self.build(provider, config.clone())?;

        let client = client
            .as_completion()
            .ok_or(ClientBuildError::UnsupportedFeature(
                provider.to_string(),
                "completion".to_string(),
            ))?;

        let mut build = client.agent(&modle);

        // 设置名称
        if !config.name.is_empty() {
            build = build.name(&config.name);
        }

        // 设置描述
        build = build.description( &config.desc);

        // 设定系统提示词。
        if let Some(sys_promte) = &config.sys_promte {
            build = build.preamble(sys_promte);
        }

        if let Some(sys_promte) = &config.sys_promte {
            build = build.preamble(sys_promte);
        }
        build = build.temperature(0.0);

        // 无论如何也需要进行roots 配置。
        match config.mcp {
            McpType::Nothing => {}
            McpType::STDIO(mcp_stdio) => {
                let client: RunningService<RoleClient, InitializeRequestParam> =
                    build_agent(mcp_stdio).await?;
                build = build.mcp_client(client);
            }
            McpType::SHTTP(_) => todo!(),
        }

        let agent = build.build();

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

async fn build_agent(
    mcp_stdio: McpStdio,
) -> Result<RunningService<RoleClient, InitializeRequestParam>, ClientBuildError> {
    let servers_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR is not set");

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "local stdio client".to_string(),
            title: None,
            version: "0.0.1".to_string(),
            website_url: None,
            icons: None,
        },
    };
    //mcp_stdio 判断是否存在...  bug ../容易形成漏洞攻击。 但是，本质上已经允许  stdio 启动了，可不在意这种级别的漏洞，因为已经透明了。
    let zhiding_loction = servers_dir.join(mcp_stdio.path.unwrap_or_default());
    let mut command = Command::new(mcp_stdio.command);

    for ele in mcp_stdio.args {
        command.arg(ele);
    }
    command.current_dir(zhiding_loction);

    let transport =
        TokioChildProcess::new(command).map_err(|e| ClientBuildError::MCPStidioExecuteFailed(e))?;

    let client = client_info
        .serve(transport)
        .await
        .inspect_err(|e| {
            tracing::error!("client error: {:?}", e);
        })
        .map_err(|e: rmcp::service::ClientInitializeError| {
            ClientBuildError::MCPClinetInitError(e)
        })?;
    // Ok("".to_string())
    Ok(client)
}

#[cfg(test)]
mod test {
    use std::fs;

    #[test]
    fn test_path() {
        let servers_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("CARGO_MANIFEST_DIR is not set")
            .join("servers");
        let dd = servers_dir.join("../benben-task/src/..");
        let xx = servers_dir.as_path();
        let yy = fs::canonicalize(dd.clone()).unwrap();
        println!("{}", servers_dir.to_str().unwrap_or_default());
        println!("{}", dd.to_str().unwrap_or_default());
        println!("{}", yy.to_str().unwrap_or_default());
    }
}
