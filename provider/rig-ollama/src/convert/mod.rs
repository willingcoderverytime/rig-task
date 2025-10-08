
pub mod message;

pub mod tool;

pub mod rsp_req;

// ---------- API Error and Response Structures ----------
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct ApiErrorResponse {
   pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ApiResponse<T> {
    Ok(T),
    Err(ApiErrorResponse),
}
