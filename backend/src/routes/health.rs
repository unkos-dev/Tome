use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/health/ready", get(ready))
}

async fn health() -> &'static str {
    "ok"
}

async fn ready(State(state): State<AppState>) -> Result<impl IntoResponse, StatusCode> {
    sqlx::query!("SELECT 1 AS one")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::warn!(error = ?e, "readiness probe DB check failed");
            StatusCode::SERVICE_UNAVAILABLE
        })?;
    Ok("ok")
}
