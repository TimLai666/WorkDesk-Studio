use crate::types::{ApiEnvelope, ApiMeta};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;
use thiserror::Error;

pub const AUTH_INVALID_CREDENTIALS: &str = "AUTH_INVALID_CREDENTIALS";
pub const AUTH_ACCOUNT_NOT_FOUND: &str = "AUTH_ACCOUNT_NOT_FOUND";
pub const WORKFLOW_NOT_FOUND: &str = "WORKFLOW_NOT_FOUND";
pub const PROPOSAL_NOT_FOUND: &str = "PROPOSAL_NOT_FOUND";
pub const RUN_NOT_FOUND: &str = "RUN_NOT_FOUND";
pub const RUN_NOT_CANCELABLE: &str = "RUN_NOT_CANCELABLE";
pub const VALIDATION_FAILED: &str = "VALIDATION_FAILED";
pub const FS_PATH_TRAVERSAL: &str = "FS_PATH_TRAVERSAL";
pub const BAD_REQUEST: &str = "BAD_REQUEST";
pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";

#[derive(Debug)]
pub struct ApiHttpError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub details: Option<Value>,
}

impl ApiHttpError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }
}

impl IntoResponse for ApiHttpError {
    fn into_response(self) -> Response {
        let body = Json(ApiEnvelope::<Value> {
            data: None,
            error: Some(crate::types::ApiErrorPayload {
                code: self.code.to_string(),
                message: self.message,
                details: self.details,
            }),
            meta: ApiMeta::new(),
        });
        (self.status, body).into_response()
    }
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("account not found")]
    AccountNotFound,
    #[error("workflow not found")]
    WorkflowNotFound,
    #[error("proposal not found")]
    ProposalNotFound,
    #[error("run not found")]
    RunNotFound,
    #[error("run is not cancelable in current state")]
    RunNotCancelable,
    #[error("proposal must be pending")]
    ProposalNotPending,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("path traversal is not allowed")]
    PathTraversal,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<CoreError> for ApiHttpError {
    fn from(value: CoreError) -> Self {
        match value {
            CoreError::InvalidCredentials => ApiHttpError::new(
                StatusCode::UNAUTHORIZED,
                AUTH_INVALID_CREDENTIALS,
                value.to_string(),
            ),
            CoreError::AccountNotFound => ApiHttpError::new(
                StatusCode::NOT_FOUND,
                AUTH_ACCOUNT_NOT_FOUND,
                value.to_string(),
            ),
            CoreError::WorkflowNotFound => {
                ApiHttpError::new(StatusCode::NOT_FOUND, WORKFLOW_NOT_FOUND, value.to_string())
            }
            CoreError::ProposalNotFound => {
                ApiHttpError::new(StatusCode::NOT_FOUND, PROPOSAL_NOT_FOUND, value.to_string())
            }
            CoreError::RunNotFound => {
                ApiHttpError::new(StatusCode::NOT_FOUND, RUN_NOT_FOUND, value.to_string())
            }
            CoreError::RunNotCancelable => {
                ApiHttpError::new(StatusCode::CONFLICT, RUN_NOT_CANCELABLE, value.to_string())
            }
            CoreError::ProposalNotPending => {
                ApiHttpError::new(StatusCode::CONFLICT, VALIDATION_FAILED, value.to_string())
            }
            CoreError::Validation(message) => {
                ApiHttpError::new(StatusCode::BAD_REQUEST, VALIDATION_FAILED, message)
            }
            CoreError::PathTraversal => ApiHttpError::new(
                StatusCode::BAD_REQUEST,
                FS_PATH_TRAVERSAL,
                value.to_string(),
            ),
            CoreError::BadRequest(message) => {
                ApiHttpError::new(StatusCode::BAD_REQUEST, BAD_REQUEST, message)
            }
            CoreError::Internal(message) => {
                ApiHttpError::new(StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR, message)
            }
        }
    }
}
