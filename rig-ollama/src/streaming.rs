use async_stream::try_stream;
use futures::StreamExt as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info_span;
use tracing_futures::Instrument;

use rig::{
    completion::{CompletionError, CompletionRequest, GetTokenUsage},
    json_utils::merge_inplace,
    streaming::{RawStreamingChoice, StreamingCompletionResponse},
};

use crate::{
    completion::OllamaCompletionModel,
    convert::{message::OlMessage, rsp_req::OllamaCompletionResponse},
};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OllamaStreamingCompletionResponse {
    pub done_reason: Option<String>,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u64>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u64>,
    pub eval_duration: Option<u64>,
}

impl GetTokenUsage for OllamaStreamingCompletionResponse {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        let mut usage = rig::completion::Usage::new();
        let input_tokens = self.prompt_eval_count.unwrap_or_default();
        let output_tokens = self.eval_count.unwrap_or_default();
        usage.input_tokens = input_tokens;
        usage.output_tokens = output_tokens;
        usage.total_tokens = input_tokens + output_tokens;

        Some(usage)
    }
}

impl OllamaCompletionModel {
    pub(super) async fn streams(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<OllamaStreamingCompletionResponse>, CompletionError>
    {
        let preamble = request.preamble.clone();
        let mut request = self.create_completion_request(request)?;
        merge_inplace(&mut request, json!({"stream": true}));

        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = "ollama",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = self.model,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("messages").unwrap()).unwrap(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        let response = self.client.post("api/chat")?.json(&request).send().await?;

        if !response.status().is_success() {
            return Err(CompletionError::ProviderError(response.text().await?));
        }

        let stream = Box::pin(try_stream! {
            let span = tracing::Span::current();
            let mut byte_stream = response.bytes_stream();
            let mut tool_calls_final = Vec::new();
            let mut text_response = String::new();

            while let Some(chunk) = byte_stream.next().await {
                let bytes = chunk?;

                for line in bytes.split(|&b| b == b'\n') {
                    if line.is_empty() {
                        continue;
                    }

                    tracing::debug!(target: "rig", "Received NDJSON line from Ollama: {}", String::from_utf8_lossy(line));

                    let response: OllamaCompletionResponse = serde_json::from_slice(line)?;

                    if response.done {
                        span.record("gen_ai.usage.input_tokens", response.prompt_eval_count);
                        span.record("gen_ai.usage.output_tokens", response.eval_count);
                        let message = OlMessage::Assistant {
                            content: text_response.clone(),
                            thinking: None,
                            images: None,
                            name: None,
                            tool_calls: tool_calls_final.clone()
                        };
                        span.record("gen_ai.output.messages", serde_json::to_string(&vec![message]).unwrap());
                        yield RawStreamingChoice::FinalResponse(
                            OllamaStreamingCompletionResponse {
                                total_duration: response.total_duration,
                                load_duration: response.load_duration,
                                prompt_eval_count: response.prompt_eval_count,
                                prompt_eval_duration: response.prompt_eval_duration,
                                eval_count: response.eval_count,
                                eval_duration: response.eval_duration,
                                done_reason: response.done_reason,
                            }
                        );
                        break;
                    }

                    if let OlMessage::Assistant { content, tool_calls, .. } = response.message {
                        if !content.is_empty() {
                            text_response += &content;
                            yield RawStreamingChoice::Message(content);
                        }
                        for tool_call in tool_calls {
                            tool_calls_final.push(tool_call.clone());
                            yield RawStreamingChoice::ToolCall {
                                id: String::new(),
                                name: tool_call.function.name,
                                arguments: tool_call.function.arguments,
                                call_id: None,
                            };
                        }
                    }
                }
            }
        }.instrument(span));

        Ok(StreamingCompletionResponse::stream(stream))
    }
}
