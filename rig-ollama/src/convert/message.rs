// ---------- Provider Message Definition ----------
use rig::agent::Text;
use rig::message::{
    AssistantContent, Document, DocumentSourceKind, Message, MessageError, Reasoning, ToolResult,
    ToolResultContent, UserContent,
};
use rig::{OneOrMany, json_utils, message};
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum OlMessage {
    User {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Assistant {
        #[serde(default)]
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        // #[serde(default, deserialize_with = "json_utils::null_or_vec")]
        #[serde(default, deserialize_with = "json_utils::null_or_vec")]
        tool_calls: Vec<super::tool::OlToolCall>,
    },
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        images: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    #[serde(rename = "tool")]
    ToolResult {
        #[serde(rename = "tool_name")]
        name: String,
        content: String,
    },
}
pub struct RigMessage(pub Message);
// extern crate rig::agent::completion::Message;
/// -----------------------------
/// Provider Message Conversions
/// -----------------------------
/// Conversion from an internal Rig message (Message) to a provider Message.
/// (Only User and Assistant variants are supported.)
impl TryFrom<RigMessage> for Vec<OlMessage> {
    type Error = MessageError;
    fn try_from(internal_msg: RigMessage) -> Result<Self, Self::Error> {
        use Message as InternalMessage;
        match internal_msg.0 {
            InternalMessage::User { content, .. } => {
                let (tool_results, other_content): (Vec<_>, Vec<_>) = content
                    .into_iter()
                    .partition(|content| matches!(content, UserContent::ToolResult(_)));

                if !tool_results.is_empty() {
                    tool_results
                        .into_iter()
                        .map(|content| match content {
                            UserContent::ToolResult(ToolResult { id, content, .. }) => {
                                // Ollama expects a single string for tool results, so we concatenate
                                let content_string = content
                                    .into_iter()
                                    .map(|content| match content {
                                        ToolResultContent::Text(text) => text.text,
                                        _ => "[Non-text content]".to_string(),
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");

                                Ok::<_, MessageError>(OlMessage::ToolResult {
                                    name: id,
                                    content: content_string,
                                })
                            }
                            _ => unreachable!(),
                        })
                        .collect::<Result<Vec<_>, _>>()
                } else {
                    // Ollama requires separate text content and images array
                    let texts = other_content
                        .into_iter()
                        .fold(Vec::new(), |mut texts, content| {
                            match content {
                                UserContent::Text(Text { text }) => texts.push(text),

                                UserContent::Document(Document {
                                    data:
                                        DocumentSourceKind::Base64(data)
                                        | DocumentSourceKind::String(data),
                                    ..
                                }) => texts.push(data),
                                _ => {} // Audio not supported by Ollama
                            }
                            texts
                        });

                    Ok(vec![OlMessage::User {
                        content: texts.join(" "),
                        images: None,
                        name: None,
                    }])
                }
            }
            InternalMessage::Assistant { content, .. } => {
                let mut thinking: Option<String> = None;
                let (text_content, tool_calls) = content.into_iter().fold(
                    (Vec::new(), Vec::new()),
                    |(mut texts, mut tools), content| {
                        match content {
                            AssistantContent::Text(text) => texts.push(text.text),
                            AssistantContent::ToolCall(tool_call) => tools.push(tool_call),
                            AssistantContent::Reasoning(Reasoning { reasoning, .. }) => {
                                thinking =
                                    Some(reasoning.first().cloned().unwrap_or(String::new()));
                            }
                        }
                        (texts, tools)
                    },
                );

                // `OneOrMany` ensures at least one `AssistantContent::Text` or `ToolCall` exists,
                //  so either `content` or `tool_calls` will have some content.
                Ok(vec![OlMessage::Assistant {
                    content: text_content.join(" "),
                    thinking,
                    images: None,
                    name: None,
                    tool_calls: tool_calls
                        .into_iter()
                        .map(|tool_call| tool_call.into())
                        .collect::<Vec<_>>(),
                }])
            }
        }
    }
}

/// Conversion from provider Message to a completion message.
/// This is needed so that responses can be converted back into chat history.
impl From<OlMessage> for Message {
    fn from(msg: OlMessage) -> Self {
        match msg {
            OlMessage::User { content, .. } => Message::User {
                content: OneOrMany::one(message::UserContent::Text(Text { text: content })),
            },
            OlMessage::Assistant {
                content,
                tool_calls,
                ..
            } => {
                let mut assistant_contents =
                    vec![message::AssistantContent::Text(Text { text: content })];
                for tc in tool_calls {
                    assistant_contents.push(message::AssistantContent::tool_call(
                        tc.function.name.clone(),
                        tc.function.name,
                        tc.function.arguments,
                    ));
                }
                Message::Assistant {
                    id: None,
                    content: OneOrMany::many(assistant_contents).unwrap(),
                }
            }
            // System and ToolResult are converted to User message as needed.
            OlMessage::System { content, .. } => Message::User {
                content: OneOrMany::one(message::UserContent::Text(Text { text: content })),
            },
            OlMessage::ToolResult { name, content } => Message::User {
                content: OneOrMany::one(message::UserContent::tool_result(
                    name,
                    OneOrMany::one(message::ToolResultContent::text(content)),
                )),
            },
        }
    }
}

impl OlMessage {
    /// Constructs a system message.
    pub fn system(content: &str) -> Self {
        OlMessage::System {
            content: content.to_owned(),
            images: None,
            name: None,
        }
    }
}
