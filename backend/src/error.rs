//! Application error type and HTTP response mapping.
//!
//! [`AppError`] is the single error returned by Axum handlers via
//! `Result<impl IntoResponse, AppError>`. Its [`axum::response::IntoResponse`]
//! impl is the only place where errors become HTTP status codes + JSON bodies
//! — handlers `?`-propagate into it, the impl flattens to a uniform
//! `{"error": "..."}` shape (or, for [`AppError::BasicAuthRequired`], an
//! empty body with a `WWW-Authenticate` challenge per RFC 7617).
//!
//! Internal errors (anything wrapped in [`AppError::Internal`]) deliberately
//! do **not** leak the inner cause's message to clients — the cause is
//! `tracing::error!`-logged and the response body is a fixed
//! `"internal server error"` string. This is a deliberate
//! information-disclosure mitigation: handlers may `?`-propagate errors
//! whose `Display` includes connection strings, file paths, or other
//! sensitive context that has no business reaching the network.

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

/// Errors returned from Axum handlers; converted to HTTP responses by the
/// [`IntoResponse`] impl on this type.
///
/// Handlers convert library errors via `?` (using `#[from]` on
/// [`Self::Internal`] for `anyhow::Error` and any `Into<anyhow::Error>`
/// type — `sqlx::Error` and friends). Domain-specific failures use the
/// dedicated variants ([`Self::NotFound`], [`Self::Validation`]) so the
/// HTTP mapping is explicit at the call site rather than buried inside an
/// `anyhow` chain.
///
/// Marked `#[non_exhaustive]` is intentionally **not** applied — the type
/// is crate-internal (the `reverie_api` library exposes it for embedders
/// but downstream `match`-on-error is not a supported integration mode);
/// adding a variant requires updating every handler regardless.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Resource not found. Maps to HTTP 404 with body `{"error":"not found"}`.
    #[error("not found")]
    NotFound,
    /// Caller is unauthenticated. Maps to HTTP 401 with body
    /// `{"error":"unauthorized"}` and **no** `WWW-Authenticate` challenge —
    /// for the OPDS Basic-auth challenge variant use
    /// [`Self::BasicAuthRequired`].
    #[error("unauthorized")]
    Unauthorized,
    /// 401 that emits a `WWW-Authenticate: Basic` challenge (RFC 7617). Used by
    /// the `BasicOnly` extractor to signal OPDS clients to prompt for
    /// credentials. `realm` is operator-configured and validated at startup
    /// (no embedded `"` allowed).
    #[error("basic auth required")]
    BasicAuthRequired {
        /// The `realm` value emitted in the `WWW-Authenticate: Basic`
        /// challenge. Pre-validated at config load (no embedded `"`).
        realm: String,
    },
    /// Caller is authenticated but lacks the role/policy to perform the
    /// action. Maps to HTTP 403.
    #[error("forbidden")]
    Forbidden,
    /// Request validation failed (malformed input, business-rule
    /// violation). Maps to HTTP 422; the inner string is included in the
    /// response body verbatim, so callers should keep it free of
    /// sensitive context.
    #[error("validation error: {0}")]
    Validation(String),
    /// Anything else — unhandled `sqlx::Error`, IO failure, etc. Mapped to
    /// HTTP 500 with a fixed `"internal server error"` body so the inner
    /// cause's message (potentially containing connection strings, file
    /// paths, or other operational detail) does not leak to clients. The
    /// inner error is `tracing::error!`-logged with full context for
    /// operator triage.
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let Self::BasicAuthRequired { realm } = &self {
            let challenge = format!("Basic realm=\"{realm}\", charset=\"UTF-8\"");
            let mut response = Response::new(axum::body::Body::empty());
            *response.status_mut() = StatusCode::UNAUTHORIZED;
            if let Ok(value) = HeaderValue::from_str(&challenge) {
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, value);
            }
            return response;
        }

        let (status, message) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "not found".to_owned()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_owned()),
            Self::BasicAuthRequired { .. } => unreachable!("handled above"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden".to_owned()),
            Self::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            Self::Internal(err) => {
                tracing::error!(error = %err, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_owned(),
                )
            }
        };

        let body = serde_json::json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn status_of(err: AppError) -> (StatusCode, String) {
        let response = err.into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn not_found_returns_404() {
        let (status, _) = status_of(AppError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unauthorized_returns_401() {
        let (status, _) = status_of(AppError::Unauthorized).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn validation_returns_422_with_message() {
        let (status, body) = status_of(AppError::Validation("bad input".into())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(body.contains("bad input"));
    }

    #[tokio::test]
    async fn forbidden_returns_403() {
        let (status, _) = status_of(AppError::Forbidden).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn basic_auth_required_emits_challenge() {
        let response = AppError::BasicAuthRequired {
            realm: "Reverie OPDS".into(),
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let challenge = response
            .headers()
            .get(axum::http::header::WWW_AUTHENTICATE)
            .expect("WWW-Authenticate header present")
            .to_str()
            .unwrap()
            .to_owned();
        assert_eq!(challenge, r#"Basic realm="Reverie OPDS", charset="UTF-8""#);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert!(body.is_empty(), "BasicAuthRequired body must be empty");
    }

    #[tokio::test]
    async fn internal_returns_500_without_leaking_details() {
        let inner = anyhow::anyhow!("secret database connection string leaked");
        let (status, body) = status_of(AppError::Internal(inner)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!body.contains("secret"));
        assert!(!body.contains("database"));
        assert!(body.contains("internal server error"));
    }
}
