
use rig::agent::{self, Text};
use rig::completion::ToolDefinition;
use rig::message::ToolCall;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;



// ---------- Tool Definition Conversion ----------
/// Ollama-required tool definition format.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct OlToolDefinition {
    #[serde(rename = "type")]
    pub type_field: String, // Fixed as "function"
    pub function: ToolDefinition,
}

/// Convert internal ToolDefinition (from the completion module) into Ollama's tool definition.
impl From<ToolDefinition> for OlToolDefinition {
    fn from(tool: ToolDefinition) -> Self {
        OlToolDefinition {
            type_field: "function".to_owned(),
            function: ToolDefinition {
                name: tool.name,
                description: tool.description,
                parameters: tool.parameters,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct OlToolCall {
    #[serde(default, rename = "type")]
    pub r#type: OlToolType,
    pub function: Function,
}
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OlToolType {
    #[default]
    Function,
}
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Function {
    pub name: String,
    pub arguments: Value,
}

// ---------- Additional Message Types ----------

impl From<ToolCall> for OlToolCall {
    fn from(tool_call: ToolCall) -> Self {
        Self {
            r#type: OlToolType::Function,
            function: Function {
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            },
        }
    }
}
