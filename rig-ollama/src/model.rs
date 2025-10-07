use rig::agent::{self, Text};
use rig::completion::ToolDefinition;
use rig::message::{
    Document, DocumentSourceKind, Message, MessageError, Reasoning, ToolCall, ToolResult,
    ToolResultContent,
};
use rig::{OneOrMany, json_utils, message};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;



#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SystemContent {
    #[serde(default)]
    r#type: SystemContentType,
    text: String,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SystemContentType {
    #[default]
    Text,
}

impl From<String> for SystemContent {
    fn from(s: String) -> Self {
        SystemContent {
            r#type: SystemContentType::default(),
            text: s,
        }
    }
}

impl FromStr for SystemContent {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SystemContent {
            r#type: SystemContentType::default(),
            text: s.to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AssistantContent {
    pub text: String,
}

impl FromStr for AssistantContent {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(AssistantContent { text: s.to_owned() })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum UserContent {
    Text { text: String },
}

impl FromStr for UserContent {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UserContent::Text { text: s.to_owned() })
    }
}
