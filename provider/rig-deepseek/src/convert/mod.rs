pub mod message;

pub mod tool;

pub mod rsp_req;

use rig::completion::CompletionError;
// ---------- API Error and Response Structures ----------
use serde::Deserialize;
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ApiResponse<T> {
    Ok(T),
    Err(ApiErrorResponse),
}
//--------- 错误信息格式
#[derive(Debug, Deserialize)]
pub(crate) struct ApiErrorResponse {
    pub message: String,
}
// 消息转化成统一错误信息
impl From<ApiErrorResponse> for CompletionError {
    fn from(err: ApiErrorResponse) -> Self {
        CompletionError::ProviderError(err.message)
    }
}
