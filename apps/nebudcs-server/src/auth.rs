//! Authentication verification utilities and middleware.

use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;

use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;

/// Extracts the token query parameter from a raw query string.
pub fn extract_token_from_query(query: &str) -> Option<String> {
    for part in query.split('&') {
        if let Some((k, v)) = part.split_once('=') {
            if k == "token" {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Validates whether the given request satisfies the configured auth token (if configured).
pub fn validate_request_auth<B>(
    expected_token: Option<&str>,
    req: &Request<B>,
) -> ServerResult<()> {
    let expected = match expected_token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(()), // Authentication is optional/disabled
    };

    // 1. Check Authorization: Bearer <token>
    if let Some(auth_header) = req.headers().get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(bearer) = auth_str.strip_prefix("Bearer ") {
                if bearer.trim() == expected {
                    return Ok(());
                }
            }
        }
    }

    // 2. Check query string: ?token=<token>
    if let Some(query) = req.uri().query() {
        if let Some(token) = extract_token_from_query(query) {
            if token == expected {
                return Ok(());
            }
        }
    }

    Err(ServerError::InvalidCredentials)
}

/// Validates a token supplied in a protocol message or handshake payload.
pub fn validate_token(
    expected_token: Option<&str>,
    provided_token: Option<&str>,
) -> ServerResult<()> {
    let expected = match expected_token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(()),
    };

    match provided_token {
        Some(p) if p == expected => Ok(()),
        _ => Err(ServerError::InvalidCredentials),
    }
}

/// Axum middleware enforcing authentication on protected routes.
pub async fn auth_middleware(
    State(host): State<HostHandle>,
    req: Request,
    next: Next,
) -> Result<Response, ServerError> {
    let path = req.uri().path();
    // Allow health check, websocket upgrade handshake, and non-API static asset paths to pass through
    if path == "/api/v1/health" || path == "/ws" || !path.starts_with("/api/") {
        return Ok(next.run(req).await);
    }

    validate_request_auth(host.config().auth_token.as_deref(), &req)?;
    Ok(next.run(req).await)
}
