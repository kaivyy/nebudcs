//! Server error types, HTTP response mapping, and protocol error translation.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use nebudcs_protocol::{ProtocolError, ProtocolErrorCode};
use thiserror::Error;

/// Server error categories.
#[derive(Debug, Clone, Error)]
pub enum ServerError {
    #[error("Authentication credentials required")]
    AuthenticationRequired,

    #[error("Invalid authentication credentials")]
    InvalidCredentials,

    #[error("Permission denied: {0}")]
    Forbidden(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Invalid request payload: {0}")]
    InvalidRequest(String),

    #[error("Inbound payload exceeds maximum allowed size")]
    PayloadTooLarge,

    #[error("Command rejected by simulation logic: {0}")]
    CommandRejected(String),

    #[error("Too many requests; rate limit exceeded")]
    TooManyRequests,

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl ServerError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::AuthenticationRequired | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::CommandRejected(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn to_protocol_error(&self) -> ProtocolError {
        match self {
            Self::AuthenticationRequired | Self::InvalidCredentials => {
                ProtocolError::new(ProtocolErrorCode::AuthenticationFailed, self.to_string())
            }
            Self::Forbidden(_) => {
                ProtocolError::new(ProtocolErrorCode::Unauthorized, self.to_string())
            }
            Self::NotFound(_) => ProtocolError::new(ProtocolErrorCode::NotFound, self.to_string()),
            Self::InvalidRequest(_) => {
                ProtocolError::new(ProtocolErrorCode::InvalidMessage, self.to_string())
            }
            Self::PayloadTooLarge => {
                ProtocolError::new(ProtocolErrorCode::PayloadTooLarge, self.to_string())
            }
            Self::TooManyRequests => {
                ProtocolError::new(ProtocolErrorCode::RateLimited, self.to_string())
            }
            Self::CommandRejected(_) => {
                ProtocolError::new(ProtocolErrorCode::CommandRejected, self.to_string())
            }
            Self::Internal(_) => {
                ProtocolError::new(ProtocolErrorCode::InternalError, "Internal error")
            }
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let protocol_err = self.to_protocol_error();
        (status, Json(protocol_err)).into_response()
    }
}

pub type ServerResult<T> = Result<T, ServerError>;
