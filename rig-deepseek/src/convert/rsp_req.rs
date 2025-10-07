use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use rig::{
    OneOrMany,
    completion::{self, CompletionError, CompletionRequest, CompletionResponse, Usage},
    json_utils,
    message::AssistantContent,
};

use crate::{
    client::Client,
    convert::{
        message::{DsMessage, RigMessage},
        tool::{DsToolChoice, DsToolDefinition},
    },
};

/// The response 转化
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DsCompletionResponse {
    // We'll match the JSON:
    pub choices: Vec<Choice>,
    pub usage: DsUsage,
    // you may want other fields
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DsUsage {
    pub completion_tokens: u32,
    pub prompt_tokens: u32,
    pub prompt_cache_hit_tokens: u32,
    pub prompt_cache_miss_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

impl DsUsage {
    pub(crate) fn new() -> Self {
        Self {
            completion_tokens: 0,
            prompt_tokens: 0,
            prompt_cache_hit_tokens: 0,
            prompt_cache_miss_tokens: 0,
            total_tokens: 0,
            completion_tokens_details: None,
            prompt_tokens_details: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CompletionTokensDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PromptTokensDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    pub index: usize,
    pub message: DsMessage,
    pub logprobs: Option<serde_json::Value>,
    pub finish_reason: String,
}

impl TryFrom<DsCompletionResponse> for CompletionResponse<DsCompletionResponse> {
    type Error = CompletionError;

    fn try_from(response: DsCompletionResponse) -> Result<Self, Self::Error> {
        let choice = response.choices.first().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no choices".to_owned())
        })?;
        let content = match &choice.message {
            DsMessage::Assistant {
                content,
                tool_calls,
                ..
            } => {
                let mut content = if content.trim().is_empty() {
                    vec![]
                } else {
                    vec![AssistantContent::text(content)]
                };

                content.extend(
                    tool_calls
                        .iter()
                        .map(|call| {
                            AssistantContent::tool_call(
                                &call.id,
                                &call.function.name,
                                call.function.arguments.clone(),
                            )
                        })
                        .collect::<Vec<_>>(),
                );
                Ok(content)
            }
            _ => Err(CompletionError::ResponseError(
                "Response did not contain a valid message or tool call".into(),
            )),
        }?;

        let choice = OneOrMany::many(content).map_err(|_| {
            CompletionError::ResponseError(
                "Response contained no message or tool call (empty)".to_owned(),
            )
        })?;

        let usage = Usage {
            input_tokens: response.usage.prompt_tokens as u64,
            output_tokens: response.usage.completion_tokens as u64,
            total_tokens: response.usage.total_tokens as u64,
        };

        Ok(CompletionResponse {
            choice,
            usage,
            raw_response: response,
        })
    }
}

pub fn create_completion_request(
    model: String,
    completion_request: CompletionRequest,
) -> Result<serde_json::Value, CompletionError> {
    // Build up the order of messages (context, chat_history, prompt)
    let mut partial_history = vec![];

    if let Some(docs) = completion_request.normalized_documents() {
        partial_history.push(docs);
    }

    partial_history.extend(completion_request.chat_history);

    // Initialize full history with preamble (or empty if non-existent)
    let mut full_history: Vec<DsMessage> = completion_request
        .preamble
        .map_or_else(Vec::new, |preamble| vec![DsMessage::system(&preamble)]);

    // Convert and extend the rest of the history
    full_history.extend(
        partial_history
            .into_iter()
            .map(|msg| RigMessage(msg).try_into())
            .collect::<Result<Vec<Vec<DsMessage>>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
    );

    let tool_choice = completion_request
        .tool_choice
        .map(DsToolChoice::try_from)
        .transpose()?;

    let request = if completion_request.tools.is_empty() {
        json!({
            "model": model,
            "messages": full_history,
            "temperature": completion_request.temperature,
        })
    } else {
        json!({
            "model": model,
            "messages": full_history,
            "temperature": completion_request.temperature,
            "tools": completion_request.tools.into_iter().map(DsToolDefinition::from).collect::<Vec<_>>(),
            "tool_choice": tool_choice,
        })
    };

    let request = if let Some(params) = completion_request.additional_params {
        json_utils::merge(request, params)
    } else {
        request
    };

    Ok(request)
}
