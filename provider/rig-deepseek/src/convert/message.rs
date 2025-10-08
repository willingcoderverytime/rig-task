// ---------- Provider Message Definition ----------
use rig::message::{
    Document, DocumentSourceKind,
};
use rig::{json_utils, message};
use serde::{Deserialize, Serialize};

use crate::convert::tool::DsToolCall;

/// Message 消息转换

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum DsMessage {
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    User {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Assistant {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(
            default,
            deserialize_with = "json_utils::null_or_vec",
            skip_serializing_if = "Vec::is_empty"
        )]
        tool_calls: Vec<DsToolCall>,
    },
    #[serde(rename = "tool")]
    ToolResult {
        tool_call_id: String,
        content: String,
    },
}

impl DsMessage {
    pub fn system(content: &str) -> Self {
        DsMessage::System {
            content: content.to_owned(),
            name: None,
        }
    }
}

pub struct RigMessage(pub message::Message);

impl TryFrom<RigMessage> for Vec<DsMessage> {
    type Error = message::MessageError;

    fn try_from(message: RigMessage) -> Result<Self, Self::Error> {
        match message.0 {
            message::Message::User { content } => {
                // extract tool results
                let mut messages = vec![];

                let tool_results = content
                    .clone()
                    .into_iter()
                    .filter_map(|content| match content {
                        message::UserContent::ToolResult(tool_result) => {
                            Some(DsMessage::from(tool_result))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                messages.extend(tool_results);

                // extract text results
                let text_messages = content
                    .into_iter()
                    .filter_map(|content| match content {
                        message::UserContent::Text(text) => Some(DsMessage::User {
                            content: text.text,
                            name: None,
                        }),
                        message::UserContent::Document(Document {
                            data:
                                DocumentSourceKind::Base64(content)
                                | DocumentSourceKind::String(content),
                            ..
                        }) => Some(DsMessage::User {
                            content,
                            name: None,
                        }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                messages.extend(text_messages);

                Ok(messages)
            }
            message::Message::Assistant { content, .. } => {
                let mut messages: Vec<DsMessage> = vec![];

                // extract text
                let text_content = content
                    .clone()
                    .into_iter()
                    .filter_map(|content| match content {
                        message::AssistantContent::Text(text) => Some(DsMessage::Assistant {
                            content: text.text,
                            name: None,
                            tool_calls: vec![],
                        }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                messages.extend(text_content);

                // extract tool calls
                let tool_calls = content
                    .clone()
                    .into_iter()
                    .filter_map(|content| match content {
                        message::AssistantContent::ToolCall(tool_call) => {
                            Some(DsToolCall::from(tool_call))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                // if we have tool calls, we add a new Assistant message with them
                if !tool_calls.is_empty() {
                    messages.push(DsMessage::Assistant {
                        content: "".to_string(),
                        name: None,
                        tool_calls,
                    });
                }

                Ok(messages)
            }
        }
    }
}
