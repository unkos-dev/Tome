//! Liveness (`GET /health`) and readiness (`GET /health/ready`) probes.

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::state::AppState;

/// Build the `/health{,/ready}` router. Liveness returns `"ok"` once the
/// process is up; readiness additionally pings the application pool with
/// `SELECT 1` and 503s on failure so orchestrators withhold traffic
/// while the DB is unreachable.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/health/ready", get(ready))
}

async fn health() -> &'static str {
    "ok"
}

async fn ready(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    sqlx::query_scalar!("SELECT 1 AS \"one!: i32\"")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::warn!(error = ?e, "readiness probe DB check failed");
            StatusCode::SERVICE_UNAVAILABLE
        })?;
    Ok("ok")
}
