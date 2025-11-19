// src/error.rs

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use log::error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AppError {
    #[error("Key not found")]
    KeyNotFound,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal cluster RPC error: {0}")]
    RpcError(#[from] RpcClientError),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InvalidInput(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::KeyNotFound => {
                (StatusCode::NOT_FOUND).into_response()
            }
            AppError::InvalidInput(msg) => {
                let body = Json(json!({ "error": msg }));
                (StatusCode::BAD_REQUEST, body).into_response()
            }
            AppError::RpcError(rpc_err) => {
                error!("Internal RPC error: {:?}", rpc_err);
                let body = Json(json!({ "error": "Internal cluster communication failed" }));
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
            AppError::InternalError(msg) => {
                error!("Internal server error: {}", msg);
                let body = Json(json!({ "error": "An internal error occurred" }));
                (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
            }
        }
    }
}


// --- RPC Client Error ---
// ------------------------
// `rpc_client.rs` 专用的错误类型。
// `thiserror` 的 `#[from]` 宏使得我们可以方便地
// 使用 `?` 来转换 tonic 和 serde 的错误。

#[derive(Error, Debug)]
pub enum RpcClientError {
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("gRPC call failed: {0}")]
    Status(#[from] tonic::Status),

    #[error("JSON serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),
}