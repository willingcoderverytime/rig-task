use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use rig::{
    OneOrMany,
    completion::{self, CompletionError, CompletionRequest, Usage},
    json_utils,
};

use crate::convert::{
        message::{OlMessage, RigMessage},
        tool::OlToolDefinition,
    };

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaCompletionResponse {
    pub model: String,
    pub created_at: String,
    pub message: OlMessage,
    pub done: bool,
    #[serde(default)]
    pub done_reason: Option<String>,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub load_duration: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u64>,
    #[serde(default)]
    pub prompt_eval_duration: Option<u64>,
    #[serde(default)]
    pub eval_count: Option<u64>,
    #[serde(default)]
    pub eval_duration: Option<u64>,
}

impl TryFrom<OllamaCompletionResponse>
    for completion::CompletionResponse<OllamaCompletionResponse>
{
    type Error = CompletionError;
    fn try_from(resp: OllamaCompletionResponse) -> Result<Self, Self::Error> {
        match resp.message {
            // Process only if an assistant message is present.
            OlMessage::Assistant {
                content,
                thinking,
                tool_calls,
                ..
            } => {
                let mut assistant_contents = Vec::new();
                // Add the assistant's text content if any.
                if !content.is_empty() {
                    assistant_contents.push(completion::AssistantContent::text(&content));
                }
                // Process tool_calls following Ollama's chat response definition.
                // Each ToolCall has an id, a type, and a function field.
                for tc in tool_calls.iter() {
                    assistant_contents.push(completion::AssistantContent::tool_call(
                        tc.function.name.clone(),
                        tc.function.name.clone(),
                        tc.function.arguments.clone(),
                    ));
                }
                let choice = OneOrMany::many(assistant_contents).map_err(|_| {
                    CompletionError::ResponseError("No content provided".to_owned())
                })?;
                let prompt_tokens = resp.prompt_eval_count.unwrap_or(0);
                let completion_tokens = resp.eval_count.unwrap_or(0);

                let raw_response = OllamaCompletionResponse {
                    model: resp.model,
                    created_at: resp.created_at,
                    done: resp.done,
                    done_reason: resp.done_reason,
                    total_duration: resp.total_duration,
                    load_duration: resp.load_duration,
                    prompt_eval_count: resp.prompt_eval_count,
                    prompt_eval_duration: resp.prompt_eval_duration,
                    eval_count: resp.eval_count,
                    eval_duration: resp.eval_duration,
                    message: OlMessage::Assistant {
                        content,
                        thinking,
                        images: None,
                        name: None,
                        tool_calls,
                    },
                };

                Ok(completion::CompletionResponse {
                    choice,
                    usage: Usage {
                        input_tokens: prompt_tokens,
                        output_tokens: completion_tokens,
                        total_tokens: prompt_tokens + completion_tokens,
                    },
                    raw_response,
                })
            }
            _ => Err(CompletionError::ResponseError(
                "Chat response does not include an assistant message".into(),
            )),
        }
    }
}

pub(crate) fn create_completion_request(
    model: String,
    completion_request: CompletionRequest,
) -> Result<Value, CompletionError> {
    if completion_request.tool_choice.is_some() {
        tracing::warn!("WARNING: `tool_choice` not supported for Ollama");
    }

    // Build up the order of messages (context, chat_history)
    let mut partial_history = vec![];
    if let Some(docs) = completion_request.normalized_documents() {
        partial_history.push(docs);
    }
    partial_history.extend(completion_request.chat_history);

    // Initialize full history with preamble (or empty if non-existent)
    let mut full_history: Vec<OlMessage> = completion_request
        .preamble
        .map_or_else(Vec::new, |preamble| vec![OlMessage::system(&preamble)]);

    // Convert and extend the rest of the history
    full_history.extend(
        partial_history
            .into_iter()
            .map(|msg| RigMessage(msg).try_into())
            .collect::<Result<Vec<Vec<OlMessage>>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<OlMessage>>(),
    );

    // Convert internal prompt into a provider Message
    let options = if let Some(extra) = completion_request.additional_params {
        json_utils::merge(
            json!({ "temperature": completion_request.temperature }),
            extra,
        )
    } else {
        json!({ "temperature": completion_request.temperature })
    };

    let mut request_payload = json!({
        "model": model,
        "messages": full_history,
        "options": options,
        "stream": false,
    });
    if !completion_request.tools.is_empty() {
        request_payload["tools"] = json!(
            completion_request
                .tools
                .into_iter()
                .map(|tool| tool.into())
                .collect::<Vec<OlToolDefinition>>()
        );
    }

    tracing::debug!(target: "rig", "Chat mode payload: {}", request_payload);

    Ok(request_payload)
}
