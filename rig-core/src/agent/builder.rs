use std::{collections::HashMap, sync::Arc};

use rmcp::{RoleClient, model::InitializeRequestParam, service::RunningService};
use tokio::time::error::Elapsed;

use crate::{
    completion::{CompletionModel, Document},
    message::ToolChoice,
};

use super::Agent;

/// A builder for creating an agent
///
/// # Example
/// ```
/// use rig::{providers::openai, agent::AgentBuilder};
///
/// let openai = openai::Client::from_env();
///
/// let gpt4o = openai.completion_model("gpt-4o");
///
/// // Configure the agent
/// let agent = AgentBuilder::new(model)
///     .preamble("System prompt")
///     .context("Context document 1")
///     .context("Context document 2")
///     .tool(tool1)
///     .tool(tool2)
///     .temperature(0.8)
///     .additional_params(json!({"foo": "bar"}))
///     .build();
/// ```
pub struct AgentBuilder<M>
where
    M: CompletionModel,
{
    /// Name of the agent used for logging and debugging
    name: Option<String>,
    /// Agent description. Primarily useful when using sub-agents as part of an agent workflow and converting agents to other formats.
    description: Option<String>,
    /// Completion model (e.g.: OpenAI's gpt-3.5-turbo-1106, Cohere's command-r)
    model: M,
    /// System prompt
    preamble: Option<String>,
    /// Context documents always available to the agent
    static_context: Vec<Document>,
    /// Tools that are always available to the agent (by name)
    static_tools: Vec<String>,
    /// Additional parameters to be passed to the model
    additional_params: Option<serde_json::Value>,
    /// Maximum number of tokens for the completion
    max_tokens: Option<u64>,

    /// Temperature of the model
    temperature: Option<f64>,

    mcp_client: Option<RunningService<RoleClient, InitializeRequestParam>>,
}

impl<M> AgentBuilder<M>
where
    M: CompletionModel,
{
    pub fn new(model: M) -> Self {
        Self {
            name: None,
            description: None,
            model,
            preamble: None,
            static_context: vec![],
            static_tools: vec![],
            temperature: None,
            max_tokens: None,
            additional_params: None,
            mcp_client: None,
        }
    }

    /// Set the name of the agent
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description of the agent
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the system prompt
    pub fn preamble(mut self, preamble: &str) -> Self {
        self.preamble = Some(preamble.into());
        self
    }

    /// Remove the system prompt
    pub fn without_preamble(mut self) -> Self {
        self.preamble = None;
        self
    }

    /// Append to the preamble of the agent
    pub fn append_preamble(mut self, doc: &str) -> Self {
        self.preamble = Some(format!(
            "{}\n{}",
            self.preamble.unwrap_or_else(|| "".into()),
            doc
        ));
        self
    }

    /// Add a static context document to the agent
    pub fn context(mut self, doc: &str) -> Self {
        self.static_context.push(Document {
            id: format!("static_doc_{}", self.static_context.len()),
            text: doc.into(),
            additional_props: HashMap::new(),
        });
        self
    }

    /// Set the temperature of the model
    pub fn temperature(mut self, _temperature: f64) -> Self {
        self.temperature = Some(0 as f64);
        self
    }

    /// Set the maximum number of tokens for the completion
    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set additional parameters to be passed to the model
    pub fn additional_params(mut self, params: serde_json::Value) -> Self {
        self.additional_params = Some(params);
        self
    }

    /// Set Mcp Client
    pub fn mcp_client(
        mut self,
        client: RunningService<RoleClient, InitializeRequestParam>,
    ) -> Self {
        self.mcp_client = Some(client);
        self
    }

    /// Build the agent
    pub fn build(self) -> Agent<M> {
        let mcp = if let Some(mcp_rc) = self.mcp_client {
            Some(Arc::new(mcp_rc))
        } else {
            None
        };

        Agent {
            name: self.name,
            description: self.description,
            model: Arc::new(self.model),
            preamble: self.preamble,
            static_context: self.static_context,
            static_tools: self.static_tools,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            additional_params: self.additional_params,
            mcp_client: mcp,
        }
    }
}
