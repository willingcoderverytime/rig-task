use std::{collections::HashMap, sync::Arc};

use once_cell::sync::OnceCell;
use rig::{
    agent::Agent,
    client::{AgentConfig, completion::CompletionModelHandle},
};
use rig_ollama::completion::OllamaCompletionModel;
use rmcp::handler::server::prompt;

use crate::{
    agent_builder::DynClientBuilder,
    agent_support::{AgentConfOwn, SupportFindTrait},
};

#[derive(Clone, Default)]
pub struct AgentManager {
    pub agent_map: HashMap<String, Arc<Agent<CompletionModelHandle<'static>>>>,
    pub agent_vec: Vec<Arc<AgentConfig>>,
}

// Static instance for global access
static INST: OnceCell<Arc<AgentManager>> = OnceCell::new();

impl AgentManager {
    /// Get or initialize the static RagApi instance
    pub fn global() -> Option<Arc<AgentManager>> {
        if INST.get().is_none() {
            None
        } else {
            let x = INST.get().expect("RagApi not initialized");
            Some(x.clone())
        }
    }

    /// Initialize the static RagApi instance
    /// Initialize the static RagApi instance
    pub async fn init_global(support: impl SupportFindTrait) -> Result<Arc<AgentManager>, String> {
        let mut api = AgentManager::default();
        let support_config = support.find_config();

        let build = DynClientBuilder::global();
        // let mut agent_futures = Vec::new();
        for AgentConfOwn {
            provider,
            mut config,
        } in support_config
        {
            let config_code = config.code.clone();
            let future = build.agent(provider, config.clone()).await;
            match future {
                Ok(agent) => {
                    api.agent_map.insert(config_code, Arc::new(agent));
                }
                // maybe log error info
                Err(e) => {
                    tracing::error!("init cmp client failed{e}");
                    config.error = Some(e.to_string())
                }
            }
            api.agent_vec.push(Arc::new(config));
        }

        let manager = Arc::new(api);
        if INST.set(manager.clone()).is_err() {
            return Err("agent manager init failed".to_string());
        }
        Ok(manager)
    }

    pub fn list_agent(&self) -> Vec<AgentVo> {
        let mut agent_info_vec = Vec::new();
        for ele in &self.agent_vec {
            let agent = AgentVo {
                name: ele.name.clone(),
                desc: ele.desc.clone(),
                error: ele.error.clone(),
            };
            agent_info_vec.push(agent);
        }
        agent_info_vec
    }
    /// 最终军事以string 吐出去，最终由task 取处理，前后置信息，无论是json diff。
    pub fn execute(prompt: String,/*  plan: WorkFlow */) -> String {
        String::new()
    }
}

pub struct AgentVo {
    pub name: String,
    pub desc: String,
    pub error: Option<String>,
}
