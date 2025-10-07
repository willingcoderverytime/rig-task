use futures::StreamExt as _;
use serde_json::{Value, json};
use tracing::info_span;

use rig::{completion::{self, CompletionError, CompletionRequest}, json_utils, streaming::StreamingCompletionResponse};

use crate::{
    client::Client,
    convert::{
        message::{OlMessage, RigMessage},
        rsp_req::{OllamaCompletionResponse, create_completion_request},
        tool::OlToolDefinition,
    },
    streaming::OllamaStreamingCompletionResponse,
};

// ---------- Completion Model ----------

#[derive(Clone)]
pub struct OllamaCompletionModel {
    pub(super) client: Client,
    pub model: String,
}

impl OllamaCompletionModel {
    pub fn new(client: Client, model: &str) -> Self {
        Self {
            client,
            model: model.to_owned(),
        }
    }

    pub(super) fn create_completion_request(
        &self,
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
            "model": self.model,
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
}

// ---------- CompletionModel Implementation ----------

impl completion::CompletionModel for OllamaCompletionModel {
    type Response = OllamaCompletionResponse;
    type StreamingResponse = OllamaStreamingCompletionResponse;

    #[cfg_attr(feature = "worker", worker::send)]
    async fn completion(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<completion::CompletionResponse<Self::Response>, CompletionError> {
        let preamble = completion_request.preamble.clone();
        let request = create_completion_request(self.model.to_string(), completion_request)?;

        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = "ollama",
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

        let async_block = async move {
            let response = self.client.post("api/chat")?.json(&request).send().await?;

            if !response.status().is_success() {
                return Err(CompletionError::ProviderError(response.text().await?));
            }

            let bytes = response.bytes().await?;

            tracing::debug!(target: "rig", "Received response from Ollama: {}", String::from_utf8_lossy(&bytes));

            let response: OllamaCompletionResponse = serde_json::from_slice(&bytes)?;
            let span = tracing::Span::current();
            span.record("gen_ai.response.model_name", &response.model);
            span.record(
                "gen_ai.output.messages",
                serde_json::to_string(&vec![&response.message]).unwrap(),
            );
            span.record(
                "gen_ai.usage.input_tokens",
                response.prompt_eval_count.unwrap_or_default(),
            );
            span.record(
                "gen_ai.usage.output_tokens",
                response.eval_count.unwrap_or_default(),
            );

            let response: completion::CompletionResponse<OllamaCompletionResponse> =
                response.try_into()?;

            Ok(response)
        };

        tracing::Instrument::instrument(async_block, span).await
    }

    #[cfg_attr(feature = "worker", worker::send)]
    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>
    {
        self.streams(request).await
    }
}
