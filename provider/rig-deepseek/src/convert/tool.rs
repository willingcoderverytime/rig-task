use rig::completion::{CompletionError, ToolDefinition};
use rig::json_utils;
use rig::message::{ToolCall, ToolChoice, ToolResult, ToolResultContent};
use serde::{Deserialize, Serialize};

use crate::convert::message::DsMessage;

// ---------- Tool Definition Conversion ----------
/// Ollama-required tool definition format.
/// 选中工具展示，即选中工具特性。
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "function")]
pub(crate) enum ToolChoiceFunctionKind {
    Function { name: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub(crate) enum DsToolChoice {
    None,
    Auto,
    Required,
    Function(Vec<ToolChoiceFunctionKind>),
}

impl TryFrom<ToolChoice> for DsToolChoice {
    type Error = CompletionError;
    /// tools 的信息转化。
    fn try_from(value: ToolChoice) -> Result<Self, Self::Error> {
        let res = match value {
            ToolChoice::None => Self::None,
            ToolChoice::Auto => Self::Auto,
            ToolChoice::Required => Self::Required,
            ToolChoice::Specific { function_names } => {
                let vec: Vec<ToolChoiceFunctionKind> = function_names
                    .into_iter()
                    .map(|name| ToolChoiceFunctionKind::Function { name })
                    .collect();

                Self::Function(vec)
            }
        };

        Ok(res)
    }
}
/// tools 格式信息转化。
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DsToolDefinition {
    pub r#type: String,
    pub function: ToolDefinition,
}

impl From<ToolDefinition> for DsToolDefinition {
    fn from(tool: ToolDefinition) -> Self {
        Self {
            r#type: "function".into(),
            function: tool,
        }
    }
}
// 工具ToolResult Message 转化
impl From<ToolResult> for DsMessage {
    fn from(tool_result: ToolResult) -> Self {
        let content = match tool_result.content.first() {
            ToolResultContent::Text(text) => text.text,
            ToolResultContent::Image(_) => String::from("[Image]"),
        };

        DsMessage::ToolResult {
            tool_call_id: tool_result.id,
            content,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DsToolCall {
    pub id: String,
    pub index: usize,
    #[serde(default)]
    pub r#type: DsToolType,
    pub function: DsFunction,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DsFunction {
    pub name: String,
    #[serde(with = "json_utils::stringified_json")]
    pub arguments: serde_json::Value,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum DsToolType {
    #[default]
    Function,
}

impl From<ToolCall> for DsToolCall {
    fn from(tool_call: ToolCall) -> Self {
        Self {
            id: tool_call.id,
            // TODO: update index when we have it
            index: 0,
            r#type: DsToolType::Function,
            function: DsFunction {
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            },
        }
    }
}
