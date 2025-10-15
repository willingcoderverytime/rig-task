use std::{collections::HashMap, fmt, sync::Arc};

use once_cell::sync::OnceCell;
use rig::{
    agent::Agent,
    client::{AgentConfig, McpType, ProviderClient},
};
use rig_deepseek::completion::DsCompletionModel;
use rig_ollama::completion::OllamaCompletionModel;
use serde_json;

use crate::agent_builder::{ClientFactory, DynClientBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultProviders {
    Deepseek,
    Ollama,
}

impl fmt::Display for DefaultProviders {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DefaultProviders::Deepseek => write!(f, "deepseek"),
            DefaultProviders::Ollama => write!(f, "ollama"),
        }
    }
}

static INST: OnceCell<Arc<DynClientBuilder>> = OnceCell::new();
impl<'a> DynClientBuilder {
    pub fn global() -> Arc<DynClientBuilder> {
        if INST.get().is_none() {
            if INST.set(Arc::new(Self::new())).is_err() {
                panic!("Client Factory init Failed!")
            }
        }
        let x = INST.get().expect("Builder not initialized");
        x.clone()
    }

    fn new() -> Self {
        // 这里可以控制feature 进行条件装填。
        Self {
            registry: HashMap::new(),
        }
        .register_all(vec![
            ClientFactory::new(
                DefaultProviders::Ollama,
                rig_ollama::client::Client::from_config,
            ),
            ClientFactory::new(
                DefaultProviders::Deepseek,
                rig_deepseek::client::Client::from_config,
            ),
        ])
    }
}

pub struct AgentConfOwn {
    pub provider: DefaultProviders,
    pub config: AgentConfig,
}

pub trait SupportFindTrait {
    fn find_config(self) -> Vec<AgentConfOwn>;
}
/// EnvAgentFinder 默认实现。
#[derive(Default)]
pub struct EnvAgentFinder;
impl SupportFindTrait for EnvAgentFinder {
    fn find_config(self) -> Vec<AgentConfOwn> {
        let mut configs = Vec::new();

        // 遍历枚举实现 DefaultProviders并从env 中获取所有agent config
        // ollama1.    ollama2  ollama 作为前缀的方案确定一个完整agentconfig
        for provider in [DefaultProviders::Deepseek, DefaultProviders::Ollama] {
            let prefix = format!("{}", provider);
            // Try to load config with the provider name as prefix
            if let Some(config) = from_env(&prefix, provider) {
                configs.push(config);
            }

            // Also check for numbered variants (e.g., ollama1, ollama2, etc.)
            for i in 1..=10 {
                let numbered_prefix = format!("{}{}", prefix, i);
                if let Some(config) = from_env(&numbered_prefix, provider) {
                    configs.push(config);
                } else {
                    // If we can't find a numbered config, break the loop
                    // assuming there are no more numbered configs
                    break;
                }
            }
        }
        configs
    }
}

/// [from_env] 从环境变量中获取的默认实现。
/// ollama.model=
/// ollama.name=
/// ollama.api_key=
/// ollama.base_url=
/// ollama.addition_key={"",""}
/// ollama.sys_promte=
/// ollama.mcp=
/// ollama.mcp.path=
/// ollama.mcp.addtion_key={"",""}
/// ollama1.model=
/// ollama1.api_key=
/// ....
/// mcp stdio | sse | http
/// warming 这里的次序等各类均可以调整，包括整个配置分布都可以调整，但这不是重点。
///
/// 重点是满足agent 的动态创建，以及rmcp 整合  尤其是stdio 你只有一个人，不要想这么多，最扁平的接口
/// 已经流出来了，不要做无用功。
fn from_env(id: &str, provider: DefaultProviders) -> Option<AgentConfOwn> {
    let model = std::env::var(format!("{}.model", id)).unwrap_or_default();
    if model.is_empty() {
        return None;
    }
    let name = std::env::var(format!("{}.name", id)).unwrap_or_default();
    if name.is_empty() {
        return None;
    }

    let code = std::env::var(format!("{}.code", id)).unwrap_or_default();
    if code.is_empty() {
        return None;
    }

    let desc = std::env::var(format!("{}.desc", id)).unwrap_or_default();
    if desc.is_empty() {
        return None;
    }

    let api_key = std::env::var(format!("{}.api_key", id)).ok();
    let base_url = std::env::var(format!("{}.base_url", id)).unwrap_or_default();
    if base_url.is_empty() {
        return None;
    }
    let sys_promte = std::env::var(format!("{}.sys_promte", id)).ok();
    let mcp = std::env::var(format!("{}.mcp", id)).ok();

    let mcp: McpType = if let Some(mcp) = mcp {
        serde_json::from_str(&mcp).unwrap_or(McpType::Nothing)
    } else {
        McpType::Nothing
    };

    let mcp: McpType = serde_json::from_str("").unwrap();

    Some(AgentConfOwn {
        provider,
        config: AgentConfig {
            model,
            code,
            error: None,
            desc,
            name,
            base_url,
            api_key,
            sys_promte,
            mcp,
        },
    })
}
