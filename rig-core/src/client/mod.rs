//! This module provides traits for defining and creating provider clients.
//! Clients are used to create models for completion, embeddings, etc.
//! Dyn-compatible traits have been provided to allow for more provider-agnostic code.

pub mod completion;
pub mod embeddings;
pub mod verify;

#[cfg(feature = "derive")]
pub use rig_derive::ProviderClient;
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

/// The base ProviderClient trait, facilitates conversion between client types
/// and creating a client from the environment.
///
/// All conversion traits must be implemented, they are automatically
/// implemented if the respective client trait is implemented.
pub trait ProviderClient: AsCompletion + AsEmbeddings + Debug {
    /// Create a client from the process's environment.
    /// Panics if an environment is improperly configured.
    fn from_env() -> Self
    where
        Self: Sized;

    fn name(self) -> String;

    fn api_key(self) -> Option<String>;
    /// A helper method to box the client.
    fn boxed(self) -> Box<dyn ProviderClient>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }

    /// Create a boxed client from the process's environment.
    /// Panics if an environment is improperly configured.
    fn from_env_boxed<'a>() -> Box<dyn ProviderClient + 'a>
    where
        Self: Sized,
        Self: 'a,
    {
        Box::new(Self::from_env())
    }

    fn from_val(input: ProviderValue) -> Self
    where
        Self: Sized;

    /// Create a boxed client from the process's environment.
    /// Panics if an environment is improperly configured.
    fn from_val_boxed<'a>(input: ProviderValue) -> Box<dyn ProviderClient + 'a>
    where
        Self: Sized,
        Self: 'a,
    {
        Box::new(Self::from_val(input))
    }
}

#[derive(Clone)]
pub enum ProviderValue {
    Simple(String),
    ApiKeyWithOptionalKey(String, Option<String>),
    ApiKeyWithVersionAndHeader(String, String, String),
}

impl From<&str> for ProviderValue {
    fn from(value: &str) -> Self {
        Self::Simple(value.to_string())
    }
}

impl From<String> for ProviderValue {
    fn from(value: String) -> Self {
        Self::Simple(value)
    }
}

impl<P> From<(P, Option<P>)> for ProviderValue
where
    P: AsRef<str>,
{
    fn from((api_key, optional_key): (P, Option<P>)) -> Self {
        Self::ApiKeyWithOptionalKey(
            api_key.as_ref().to_string(),
            optional_key.map(|x| x.as_ref().to_string()),
        )
    }
}

impl<P> From<(P, P, P)> for ProviderValue
where
    P: AsRef<str>,
{
    fn from((api_key, version, header): (P, P, P)) -> Self {
        Self::ApiKeyWithVersionAndHeader(
            api_key.as_ref().to_string(),
            version.as_ref().to_string(),
            header.as_ref().to_string(),
        )
    }
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
