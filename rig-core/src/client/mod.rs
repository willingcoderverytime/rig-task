//! This module provides traits for defining and creating provider clients.
//! Clients are used to create models for completion, embeddings, etc.
//! Dyn-compatible traits have been provided to allow for more provider-agnostic code.

pub mod completion;
pub mod embeddings;
pub mod verify;

#[cfg(feature = "derive")]
pub use rig_derive::ProviderClient;
use serde::Deserialize;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ClientBuilderError {
    #[error("reqwest error: {0}")]
    HttpError(
        #[from]
        #[source]
        reqwest::Error,
    ),
    #[error("invalid property: {0}")]
    InvalidProperty(&'static str),
}

#[derive(Clone, Deserialize)]
pub struct McpStdio {
    // cargo run | xxx.exe |
    pub command: String,
    // test.exe ---help xxx ....
    pub args: Vec<String>,
    // 必须是相对路径，绝对路径不能超过 cargo manifest  rutime currentdir。
    pub path: Option<String>,
}
/// McpType : 理论是上resource 应当是配置类型，当是stdio 形态的时候应当由args统一进行设定。
/// roots: 再这个client中应当是默认的 特定workspace中，应当再切换版本时进行指定。
///
///
#[derive(Clone, Deserialize)]
pub enum McpType {
    Nothing,
    STDIO(McpStdio),
    // 暂时先这样 StremHttp 以及 sse 暂时不用，且都是url 并不好区分，等后续再考虑。
    SHTTP(String),
    // SSE(String)
}

#[derive(Clone, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    // 需要独立的校验规则。
    pub code: String,
    pub desc: String,
    pub error:Option<String>,
    pub model: String,
    pub base_url: String,
    pub sys_promte: Option<String>,
    pub api_key: Option<String>,
    // todo 认证系统。主要针对可能得大模型
    // pub auth_map: Option<HashMap<String, Option<String>>>,
    pub mcp: McpType,
}

/// The base ProviderClient trait, facilitates conversion between client types
/// and creating a client from the environment.
///
/// All conversion traits must be implemented, they are automatically
/// implemented if the respective client trait is implemented.
pub trait ProviderClient: AsCompletion + AsEmbeddings + Debug {
    /// Create a client from the process's environment.
    /// Panics if an environment is improperly configured.
    fn from_config(config: AgentConfig) -> Box<dyn ProviderClient>
    where
        Self: Sized;
}

/// Attempt to convert a ProviderClient to a CompletionClient
pub trait AsCompletion {
    fn as_completion(&self) -> Option<Box<dyn CompletionClientDyn>> {
        None
    }
}

/// Attempt to convert a ProviderClient to a EmbeddingsClient
pub trait AsEmbeddings {
    fn as_embeddings(&self) -> Option<Box<dyn EmbeddingsClientDyn>> {
        None
    }
}

/// Attempt to convert a ProviderClient to a VerifyClient
pub trait AsVerify {
    fn as_verify(&self) -> Option<Box<dyn VerifyClientDyn>> {
        None
    }
}

/// Implements the conversion traits for a given struct
/// ```rust
/// pub struct Client;
/// impl ProviderClient for Client {
///     ...
/// }
/// impl_conversion_traits!(AsCompletion, AsEmbeddings for Client);
/// ```
#[macro_export]
macro_rules! impl_conversion_traits {
    ($( $trait_:ident ),* for $struct_:ident ) => {
        $(
            impl_conversion_traits!(@impl $trait_ for $struct_);
        )*
    };


    (@impl $trait_:ident for $struct_:ident) => {
        impl rig::client::$trait_ for $struct_ {}
    };
}
pub use impl_conversion_traits;

use crate::client::completion::CompletionClientDyn;
use crate::client::embeddings::EmbeddingsClientDyn;
use crate::client::verify::VerifyClientDyn;

pub use crate::client::completion::CompletionClient;
pub use crate::client::embeddings::EmbeddingsClient;
pub use crate::client::verify::{VerifyClient, VerifyError};
