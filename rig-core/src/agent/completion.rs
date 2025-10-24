use super::prompt_request::{self, PromptRequest};
use crate::{
    agent::prompt_request::streaming::StreamingPromptRequest,
    completion::{
        Chat, Completion, CompletionError, CompletionModel, CompletionRequestBuilder, Document,
        GetTokenUsage, Message, Prompt, PromptError,
    },
    streaming::{StreamingChat, StreamingCompletion, StreamingPrompt},
};
use futures::{StreamExt, TryStreamExt, stream};
use rmcp::{
    RoleClient,
    model::{CallToolRequestParam, InitializeRequestParam},
    service::RunningService,
};
use serde_json::Value;
use std::{borrow::Cow, sync::Arc};

const UNKNOWN_AGENT_NAME: &str = "Unnamed Agent";

/// Struct representing an LLM agent. An agent is an LLM model combined with a preamble
/// (i.e.: system prompt) and a static set of context documents and tools.
/// All context documents and tools are always provided to the agent when prompted.
///
/// # Example
/// ```
/// use rig::{completion::Prompt, providers::openai};
///
/// let openai = openai::Client::from_env();
///
/// let comedian_agent = openai
///     .agent("gpt-4o")
///     .preamble("You are a comedian here to entertain the user using humour and jokes.")
///     .temperature(0.9)
///     .build();
///
/// let response = comedian_agent.prompt("Entertain me!")
///     .await
///     .expect("Failed to prompt the agent");
/// ```
#[derive(Clone)]
#[non_exhaustive]
pub struct Agent<M>
where
    M: CompletionModel,
{
    /// Name of the agent used for logging and debugging
    pub name: Option<String>,
    /// Agent description. Primarily useful when using sub-agents as part of an agent workflow and converting agents to other formats.
    pub description: Option<String>,
    /// Completion model (e.g.: OpenAI's gpt-3.5-turbo-1106, Cohere's command-r)
    pub model: Arc<M>,
    /// System prompt
    pub preamble: Option<String>,
    /// Context documents always available to the agent
    pub static_context: Vec<Document>,
    /// Tools that are always available to the agent (identified by their name)
    pub static_tools: Vec<String>,
    /// Temperature of the model
    pub temperature: Option<f64>,
    /// Maximum number of tokens for the completion
    pub max_tokens: Option<u64>,
    /// Additional parameters to be passed to the model
    pub additional_params: Option<serde_json::Value>,
    /// agent mcp server
    pub mcp_client: Option<Arc<RunningService<RoleClient, InitializeRequestParam>>>,
}

impl<M> Agent<M>
where
    M: CompletionModel,
{
    /// Returns the name of the agent.
    pub(crate) fn name(&self) -> &str {
        self.name.as_deref().unwrap_or(UNKNOWN_AGENT_NAME)
    }

    pub async fn call(&self, func_name: &str, args: &Value) -> Result<String, CompletionError> {
        if let Some(mcp_client) = self.mcp_client.clone() {
            let obj = args.as_object();
            let req = CallToolRequestParam {
                name: Cow::Owned(func_name.to_string()),
                arguments: obj.cloned(),
            };
            let result = mcp_client
                .call_tool(req)
                .await
                .map_err(|e| CompletionError::MCPError(e.to_string()))?;

            // Extract the result content as a string
            let result_str = result
                .content
                .iter()
                .map(|c| match &c.raw {
                    rmcp::model::RawContent::Text(text) => text.text.clone(),
                    rmcp::model::RawContent::Image(image) => {
                        format!("[Image: {}]", image.mime_type)
                    }
                    rmcp::model::RawContent::Resource(resource) => match &resource.resource {
                        rmcp::model::ResourceContents::TextResourceContents { text, .. } => {
                            text.clone()
                        }
                        rmcp::model::ResourceContents::BlobResourceContents { .. } => {
                            "[Binary Resource]".to_string()
                        }
                    },
                    rmcp::model::RawContent::Audio(_) => "[Audio]".to_string(),
                    rmcp::model::RawContent::ResourceLink(_) => "[Resource Link]".to_string(),
                })
                .collect::<Vec<_>>()
                .join("\n");

            return Ok(result_str);
        }

        Ok("".to_string())
    }
}

impl<M> Completion<M> for Agent<M>
where
    M: CompletionModel,
{
    async fn completion(
        &self,
        prompt: impl Into<Message> + Send,
        chat_history: Vec<Message>,
    ) -> Result<CompletionRequestBuilder<M>, CompletionError> {
        let prompt = prompt.into();

        // Find the latest message in the chat history that contains RAG text
        // let rag_text = prompt.rag_text();
        // let rag_text = rag_text.or_else(|| {
        //     chat_history
        //         .iter()
        //         .rev()
        //         .find_map(|message| message.rag_text())
        // });

        let completion_request = self
            .model
            .completion_request(prompt)
            .messages(chat_history)
            .temperature_opt(self.temperature)
            .max_tokens_opt(self.max_tokens)
            .additional_params_opt(self.additional_params.clone())
            .documents(self.static_context.clone());
        let completion_request = if let Some(preamble) = &self.preamble {
            completion_request.preamble(preamble.to_owned())
        } else {
            completion_request
        };
        if let Some(client) = self.mcp_client.clone() {
            let tools = client
                .list_all_tools()
                .await
                .map_err(|_| CompletionError::MCPError("".to_string()))?;
            return Ok(completion_request.tools(tools));
        }
        Ok(completion_request)
        // todo  : If the agent has RAG text, we need to fetch the dynamic context and tools
    }
}

// Here, we need to ensure that usage of `.prompt` on agent uses these redefinitions on the opaque
//  `Prompt` trait so that when `.prompt` is used at the call-site, it'll use the more specific
//  `PromptRequest` implementation for `Agent`, making the builder's usage fluent.
//
// References:
//  - https://github.com/rust-lang/rust/issues/121718 (refining_impl_trait)

#[allow(refining_impl_trait)]
impl<M> Prompt for Agent<M>
where
    M: CompletionModel,
{
    fn prompt(
        &self,
        prompt: impl Into<Message> + Send,
    ) -> PromptRequest<'_, prompt_request::Standard, M, ()> {
        PromptRequest::new(self, prompt)
    }
}

#[allow(refining_impl_trait)]
impl<M> Prompt for &Agent<M>
where
    M: CompletionModel,
{
    #[tracing::instrument(skip(self, prompt), fields(agent_name = self.name()))]
    fn prompt(
        &self,
        prompt: impl Into<Message> + Send,
    ) -> PromptRequest<'_, prompt_request::Standard, M, ()> {
        PromptRequest::new(*self, prompt)
    }
}

#[allow(refining_impl_trait)]
impl<M> Chat for Agent<M>
where
    M: CompletionModel,
{
    #[tracing::instrument(skip(self, prompt, chat_history), fields(agent_name = self.name()))]
    async fn chat(
        &self,
        prompt: impl Into<Message> + Send,
        mut chat_history: Vec<Message>,
    ) -> Result<String, PromptError> {
        PromptRequest::new(self, prompt)
            .with_history(&mut chat_history)
            .await
    }
}

impl<M> StreamingCompletion<M> for Agent<M>
where
    M: CompletionModel,
{
    async fn stream_completion(
        &self,
        prompt: impl Into<Message> + Send,
        chat_history: Vec<Message>,
    ) -> Result<CompletionRequestBuilder<M>, CompletionError> {
        // Reuse the existing completion implementation to build the request
        // This ensures streaming and non-streaming use the same request building logic
        self.completion(prompt, chat_history).await
    }
}

impl<M> StreamingPrompt<M, M::StreamingResponse> for Agent<M>
where
    M: CompletionModel + 'static,
    M::StreamingResponse: GetTokenUsage,
{
    fn stream_prompt(&self, prompt: impl Into<Message> + Send) -> StreamingPromptRequest<M, ()> {
        let arc = Arc::new(self.clone());
        StreamingPromptRequest::new(arc, prompt)
    }
}

impl<M> StreamingChat<M, M::StreamingResponse> for Agent<M>
where
    M: CompletionModel + 'static,
    M::StreamingResponse: GetTokenUsage,
{
    fn stream_chat(
        &self,
        prompt: impl Into<Message> + Send,
        chat_history: Vec<Message>,
    ) -> StreamingPromptRequest<M, ()> {
        let arc = Arc::new(self.clone());
        StreamingPromptRequest::new(arc, prompt).with_history(chat_history)
    }
}
