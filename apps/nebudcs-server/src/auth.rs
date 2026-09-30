//! Authentication verification utilities, role resolution, and security middleware.

use axum::extract::{FromRequestParts, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;

use crate::config::ServerConfig;
use crate::error::{ServerError, ServerResult};
use crate::host::HostHandle;
use crate::security::{audit_log, constant_time_eq, UserRole};

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

/// Extracts bearer or query auth token from an incoming HTTP request.
pub fn extract_token_from_request<B>(req: &Request<B>) -> Option<String> {
    // 1. Check Authorization: Bearer <token>
    if let Some(auth_header) = req.headers().get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(bearer) = auth_str.strip_prefix("Bearer ") {
                return Some(bearer.trim().to_string());
            }
        }
    }

    // 2. Check query string: ?token=<token>
    if let Some(query) = req.uri().query() {
        if let Some(token) = extract_token_from_query(query) {
            return Some(token);
        }
    }

    None
}

/// Resolves the caller's role based on configured server tokens and the provided token.
pub fn resolve_token_role(config: &ServerConfig, token: Option<&str>) -> ServerResult<UserRole> {
    let has_any_token = config.auth_token.is_some()
        || config.admin_token.is_some()
        || config.instructor_token.is_some()
        || config.operator_token.is_some()
        || config.trainee_token.is_some();

    // If no tokens configured anywhere, authentication is disabled -> treat as Admin
    if !has_any_token {
        return Ok(UserRole::Admin);
    }

    let provided = match token {
        Some(t) if !t.is_empty() => t.as_bytes(),
        _ => return Err(ServerError::AuthenticationRequired),
    };

    // Check Admin tokens (dedicated admin_token or legacy auth_token)
    if let Some(ref t) = config.admin_token {
        if constant_time_eq(provided, t.as_bytes()) {
            return Ok(UserRole::Admin);
        }
    }
    if let Some(ref t) = config.auth_token {
        if constant_time_eq(provided, t.as_bytes()) {
            return Ok(UserRole::Admin);
        }
    }

    // Check Instructor token
    if let Some(ref t) = config.instructor_token {
        if constant_time_eq(provided, t.as_bytes()) {
            return Ok(UserRole::Instructor);
        }
    }

    // Check Operator token
    if let Some(ref t) = config.operator_token {
        if constant_time_eq(provided, t.as_bytes()) {
            return Ok(UserRole::Operator);
        }
    }

    // Check Trainee token
    if let Some(ref t) = config.trainee_token {
        if constant_time_eq(provided, t.as_bytes()) {
            return Ok(UserRole::Trainee);
        }
    }

    Err(ServerError::InvalidCredentials)
}

/// Validates whether the given request satisfies the configured auth token (if configured).
pub fn validate_request_auth<B>(config: &ServerConfig, req: &Request<B>) -> ServerResult<UserRole> {
    let token = extract_token_from_request(req);
    resolve_token_role(config, token.as_deref())
}

/// Validates a token supplied in a protocol message or handshake payload using constant-time comparison.
pub fn validate_token(
    expected_token: Option<&str>,
    provided_token: Option<&str>,
) -> ServerResult<()> {
    let expected = match expected_token {
        Some(t) if !t.is_empty() => t.as_bytes(),
        _ => return Ok(()),
    };

    match provided_token {
        Some(p) if constant_time_eq(expected, p.as_bytes()) => Ok(()),
        _ => Err(ServerError::InvalidCredentials),
    }
}

/// Axum extractor for resolving the authenticated UserRole from request extensions.
impl<S> FromRequestParts<S> for UserRole
where
    S: Send + Sync,
{
    type Rejection = ServerError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<UserRole>()
            .copied()
            .ok_or(ServerError::AuthenticationRequired)
    }
}

/// Axum middleware enforcing rate limiting, authentication, and role attachment on protected routes.
pub async fn auth_middleware(
    State(host): State<HostHandle>,
    mut req: Request,
    next: Next,
) -> Result<Response, ServerError> {
    let path = req.uri().path().to_string();

    // 1. Rate Limiting Check
    if host.config().rate_limit_enabled {
        let client_key = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "127.0.0.1".to_string());

        if !host.rate_limiter().check_allowed(&client_key) {
            audit_log(
                "RATE_LIMIT",
                &client_key,
                &path,
                "Rate limit exceeded",
                false,
            );
            return Err(ServerError::TooManyRequests);
        }
    }

    // 2. Paths exempt from authentication (Health & Metrics probes, WebSocket handshake, static assets)
    if path.starts_with("/api/v1/health")
        || path == "/api/v1/metrics"
        || path == "/ws"
        || !path.starts_with("/api/")
    {
        // Even for exempt routes, insert Admin role if unauthenticated for handler convenience
        req.extensions_mut().insert(UserRole::Admin);
        return Ok(next.run(req).await);
    }

    // 3. Resolve caller role from token
    let token = extract_token_from_request(&req);
    let role = match resolve_token_role(host.config(), token.as_deref()) {
        Ok(r) => r,
        Err(e) => {
            audit_log("AUTH_FAILURE", "anonymous", &path, &format!("{e}"), false);
            return Err(e);
        }
    };

    audit_log(
        "AUTH_SUCCESS",
        &format!("{role}"),
        &path,
        "Request authenticated",
        true,
    );

    // 4. Attach role to request extensions for downstream authorization
    req.extensions_mut().insert(role);
    Ok(next.run(req).await)
}
