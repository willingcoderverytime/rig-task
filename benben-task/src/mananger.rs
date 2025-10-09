use std::{collections::HashMap, sync::Arc};

use once_cell::sync::OnceCell;

use crate::agent_support::{self, SuportAgent, SupportFindTrait};

#[derive(Clone, Default)]
pub struct AgentManager {
    pub agent_map: HashMap<String, Arc<SuportAgent>>,
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
    pub async fn init_global(support: impl SupportFindTrait) -> Result<Arc<AgentManager>, String> {
        let api: Arc<AgentManager> = Arc::new(AgentManager::default());
        let support_config = support.find_config();

        for ele in support_config {
            
        

        }

        let aa = INST.set(api.clone());
        if aa.is_err() {
            return Err("agent mananger init failed".to_string());
        }
        Ok(api)
    }

    pub fn list_agent() -> Vec<AgentVO> {
        vec![]
    }
    /// 最终军事以string 吐出去，最终由task 取处理，前后置信息，无论是json diff。
    pub fn execute() -> String {
        String::new()
    }
}
/// 代理 agent 多数信息是来源你与 supportconfig。
/// 但关键信息如addtion parameter api key 等等是不允许进行的。
pub struct AgentVO {}
