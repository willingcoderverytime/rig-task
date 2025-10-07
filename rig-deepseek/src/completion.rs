use rig::json_utils::merge;
use rig::{completion::GetTokenUsage, streaming::StreamingCompletionResponse};

use reqwest_eventsource::{Event, RequestBuilderExt};
use rig::{
    completion::{self, CompletionError, CompletionRequest},
    json_utils, message,
};
use serde_json::json;
use std::collections::HashMap;
use tracing::{Instrument, info_span};

use crate::streaming::send_compatible_streaming_request;
use crate::{
    client::Client,
    convert::{
        ApiResponse,
        rsp_req::{DsCompletionResponse, create_completion_request},
    },
    streaming::DsStreamingCompletionResponse,
};
// ================================================================
// DeepSeek Completion API
// ================================================================

/// `deepseek-chat` completion model
pub const DEEPSEEK_CHAT: &str = "deepseek-chat";
/// `deepseek-reasoner` completion model
pub const DEEPSEEK_REASONER: &str = "deepseek-reasoner";
/// The struct implementing the `CompletionModel` trait
#[derive(Clone)]
pub struct DsCompletionModel {
    pub client: Client,
    pub model: String,
}

impl DsCompletionModel {}

impl completion::CompletionModel for DsCompletionModel {
    type Response = DsCompletionResponse;
    type StreamingResponse = DsStreamingCompletionResponse;

    #[cfg_attr(feature = "worker", worker::send)]
    async fn completion(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<
        completion::CompletionResponse<DsCompletionResponse>,
        crate::completion::CompletionError,
    > {
        let preamble = completion_request.preamble.clone();
        let request = create_completion_request(self.model.to_string(), completion_request)?;

        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = "deepseek",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("messages").unwrap()).unwrap(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        tracing::debug!("DeepSeek completion request: {request:?}");

        async move {
            let response = self
                .client
                .post("/chat/completions")
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let t = response.text().await?;
                tracing::debug!(target: "rig", "DeepSeek completion: {t}");

                match serde_json::from_str::<ApiResponse<DsCompletionResponse>>(&t)? {
                    ApiResponse::Ok(response) => {
                        let span = tracing::Span::current();
                        span.record(
                            "gen_ai.output.messages",
                            serde_json::to_string(&response.choices).unwrap(),
                        );
                        span.record("gen_ai.usage.input_tokens", response.usage.prompt_tokens);
                        span.record(
                            "gen_ai.usage.output_tokens",
                            response.usage.completion_tokens,
                        );
                        response.try_into()
                    }
                    ApiResponse::Err(err) => Err(CompletionError::ProviderError(err.message)),
                }
            } else {
                Err(CompletionError::ProviderError(response.text().await?))
            }
        }
        .instrument(span)
        .await
    }

    #[cfg_attr(feature = "worker", worker::send)]
    async fn stream(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let preamble = completion_request.preamble.clone();
        let mut request = create_completion_request(self.model.to_string(), completion_request)?;

        request = merge(
            request,
            json!({"stream": true, "stream_options": {"include_usage": true}}),
        );

        let builder = self.client.post("/chat/completions").json(&request);

        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = "deepseek",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("messages").unwrap()).unwrap(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        tracing::Instrument::instrument(send_compatible_streaming_request(builder), span).await
    }
}
