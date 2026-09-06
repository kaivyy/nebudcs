//! Structured protocol errors and error codes.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Standardized machine-readable error codes for API and WebSocket protocols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProtocolErrorCode {
    /// Incompatible protocol version during handshake.
    ProtocolVersionMismatch,
    /// Client authentication failed or missing credentials.
    AuthenticationFailed,
    /// Authenticated client lacks permission for requested operation.
    Unauthorized,
    /// Payload structure failed deserialization or validation.
    InvalidMessage,
    /// Payload exceeds maximum allowed frame or body size.
    PayloadTooLarge,
    /// Domain command rejected by validation rules or permissive guards.
    CommandRejected,
    /// Tag or subscription request failed (e.g. unknown tag).
    SubscriptionFailed,
    /// Rate limit or connection limit exceeded.
    RateLimited,
    /// Target entity or tag was not found.
    NotFound,
    /// Internal unexpected server error.
    InternalError,
}

/// Strongly typed protocol error envelope safe for transmission to external clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("[{code:?}] {message}")]
pub struct ProtocolError {
    pub code: ProtocolErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ProtocolError {
    pub fn new(code: ProtocolErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id: None,
            details: None,
        }
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

pub type ProtocolResult<T> = Result<T, ProtocolError>;
