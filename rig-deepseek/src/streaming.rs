use std::collections::HashMap;

use async_stream::{stream, try_stream};
use futures::StreamExt as _;
use reqwest_eventsource::{Event, RequestBuilderExt as _};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info_span;
use tracing_futures::Instrument;

use rig::{
    completion::{CompletionError, CompletionRequest, GetTokenUsage, Usage},
    json_utils,
    providers::openai::StreamingToolCall,
    streaming::{RawStreamingChoice, StreamingCompletionResponse},
};

use crate::{
    completion::DsCompletionModel,
    convert::{
        message::DsMessage,
        rsp_req::{ DsUsage},
        tool::{DsFunction, DsToolCall, DsToolType},
    },
};

/// ----------- streaming --------------------

#[derive(Deserialize, Debug)]
pub struct StreamingDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default, deserialize_with = "json_utils::null_or_vec")]
    tool_calls: Vec<StreamingToolCall>,
    reasoning_content: Option<String>,
}

#[derive(Deserialize, Debug)]
struct StreamingChoice {
    delta: StreamingDelta,
}

#[derive(Deserialize, Debug)]
struct StreamingCompletionChunk {
    choices: Vec<StreamingChoice>,
    usage: Option<DsUsage>,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct DsStreamingCompletionResponse {
    pub usage: DsUsage,
}

impl GetTokenUsage for DsStreamingCompletionResponse {
    fn token_usage(&self) -> Option<Usage> {
        let mut usage = Usage::new();
        usage.input_tokens = self.usage.prompt_tokens as u64;
        usage.output_tokens = self.usage.completion_tokens as u64;
        usage.total_tokens = self.usage.total_tokens as u64;
        Some(usage)
    }
}

pub(crate) async fn send_compatible_streaming_request(
    request_builder: reqwest::RequestBuilder,
) -> Result<
    crate::streaming::StreamingCompletionResponse<DsStreamingCompletionResponse>,
    CompletionError,
> {
    let span = tracing::Span::current();
    let mut event_source = request_builder
        .eventsource()
        .expect("Cloning request must succeed");

    let stream = Box::pin(stream! {
        let mut final_usage = DsUsage::new();
        let mut text_response = String::new();
        let mut calls: HashMap<usize, (String, String, String)> = HashMap::new();

        while let Some(event_result) = event_source.next().await {
            match event_result {
                Ok(Event::Open) => {
                    tracing::trace!("SSE connection opened");
                    continue;
                }
                Ok(Event::Message(message)) => {
                    if message.data.trim().is_empty() || message.data == "[DONE]" {
                        continue;
                    }

                    let parsed = serde_json::from_str::<StreamingCompletionChunk>(&message.data);
                    let Ok(data) = parsed else {
                        let err = parsed.unwrap_err();
                        tracing::debug!("Couldn't parse SSE payload as StreamingCompletionChunk: {:?}", err);
                        continue;
                    };

                    if let Some(choice) = data.choices.first() {
                        let delta = &choice.delta;

                        if !delta.tool_calls.is_empty() {
                            for tool_call in &delta.tool_calls {
                                let function = &tool_call.function;

                                // Start of tool call
                                if function.name.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
                                    && function.arguments.is_empty()
                                {
                                    let id = tool_call.id.clone().unwrap_or_default();
                                    let name = function.name.clone().unwrap();
                                    calls.insert(tool_call.index, (id, name, String::new()));
                                }
                                // Continuation of tool call
                                else if function.name.as_ref().map(|s| s.is_empty()).unwrap_or(true)
                                    && !function.arguments.is_empty()
                                {
                                    if let Some((id, name, existing_args)) = calls.get(&tool_call.index) {
                                        let combined = format!("{}{}", existing_args, function.arguments);
                                        calls.insert(tool_call.index, (id.clone(), name.clone(), combined));
                                    } else {
                                        tracing::debug!("Partial tool call received but tool call was never started.");
                                    }
                                }
                                // Complete tool call
                                else {
                                    let id = tool_call.id.clone().unwrap_or_default();
                                    let name = function.name.clone().unwrap_or_default();
                                    let arguments_str = function.arguments.clone();

                                    let Ok(arguments_json) = serde_json::from_str::<serde_json::Value>(&arguments_str) else {
                                        tracing::debug!("Couldn't parse tool call args '{}'", arguments_str);
                                        continue;
                                    };

                                    yield Ok(crate::streaming::RawStreamingChoice::ToolCall {
                                        id,
                                        name,
                                        arguments: arguments_json,
                                        call_id: None,
                                    });
                                }
                            }
                        }

                        // DeepSeek-specific reasoning stream
                        if let Some(content) = &delta.reasoning_content {
                            yield Ok(crate::streaming::RawStreamingChoice::Reasoning {
                                reasoning: content.to_string(),
                                id: None,
                            });
                        }

                        if let Some(content) = &delta.content {
                            text_response += content;
                            yield Ok(crate::streaming::RawStreamingChoice::Message(content.clone()));
                        }
                    }

                    if let Some(usage) = data.usage {
                        final_usage = usage.clone();
                    }
                }
                Err(reqwest_eventsource::Error::StreamEnded) => {
                    break;
                }
                Err(err) => {
                    tracing::error!(?err, "SSE error");
                    yield Err(CompletionError::ResponseError(err.to_string()));
                    break;
                }
            }
        }

        let mut tool_calls = Vec::new();
        // Flush accumulated tool calls
        for (index, (id, name, arguments)) in calls {
            let Ok(arguments_json) = serde_json::from_str::<serde_json::Value>(&arguments) else {
                continue;
            };

            tool_calls.push(DsToolCall {
                id: id.clone(),
                index,
                r#type: DsToolType::Function,
                function: DsFunction {
                    name: name.clone(),
                    arguments: arguments_json.clone()
                }
            });
            yield Ok(crate::streaming::RawStreamingChoice::ToolCall {
                id,
                name,
                arguments: arguments_json,
                call_id: None,
            });
        }

        let message = DsMessage::Assistant {
            content: text_response,
            name: None,
            tool_calls
        };

        span.record("gen_ai.output.messages", serde_json::to_string(&message).unwrap());

        yield Ok(crate::streaming::RawStreamingChoice::FinalResponse(
            DsStreamingCompletionResponse { usage: final_usage.clone() }
        ));
    });

    Ok(crate::streaming::StreamingCompletionResponse::stream(
        stream,
    ))
}
