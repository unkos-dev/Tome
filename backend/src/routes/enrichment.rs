//! Enrichment control endpoints.
//!
//! * `POST /api/manifestations/:id/enrichment/trigger` — re-queue this
//!   manifestation for a fresh enrichment pass.
//! * `POST /api/manifestations/:id/enrichment/dry-run`  — synchronous preview
//!   of what the pipeline would change.
//! * `GET  /api/enrichment/status` — aggregate queue counters.

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::middleware::CurrentUser;
use crate::db;
use crate::error::AppError;
use crate::services;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/manifestations/{id}/enrichment/trigger", post(trigger))
        .route("/api/manifestations/{id}/enrichment/dry-run", post(dry_run))
        .route("/api/enrichment/status", get(status))
}

async fn trigger(
    current_user: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    current_user.require_not_child()?;

    let mut tx = db::acquire_with_rls(&state.pool, current_user.user_id)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let rows = sqlx::query(
        "UPDATE manifestations \
         SET enrichment_status = 'pending', \
             enrichment_attempt_count = 0, \
             enrichment_attempted_at = NULL, \
             enrichment_error = NULL \
         WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    if rows.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    Ok(StatusCode::ACCEPTED)
}

async fn dry_run(
    current_user: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    current_user.require_not_child()?;

    // RLS-gated visibility check: hide manifestations the user can't see.
    // The check runs on the user's pool with `app.current_user_id` set; the
    // subsequent fan-out runs on `ingestion_pool` (matching the queue's
    // pattern) because the dry-run service reads through several joined
    // tables without re-checking RLS at each step.
    let mut tx = db::acquire_with_rls(&state.pool, current_user.user_id)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    let visible: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM manifestations WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
    drop(tx);
    if visible.is_none() {
        return Err(AppError::NotFound);
    }

    let diff = services::enrichment::dry_run::preview(&state.ingestion_pool, &state.config, id)
        .await
        .map_err(AppError::Internal)?;
    Ok(axum::Json(diff))
}

#[derive(Debug, Serialize)]
struct StatusSummary {
    pending: i64,
    in_progress: i64,
    complete: i64,
    failed: i64,
    skipped: i64,
}

async fn status(
    current_user: CurrentUser,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    current_user.require_not_child()?;

    let mut tx = db::acquire_with_rls(&state.pool, current_user.user_id)
        .await
        .map_err(|e| AppError::Internal(e.into()))?;

    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT enrichment_status::text, COUNT(*)::bigint \
         FROM manifestations \
         GROUP BY enrichment_status",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    let mut summary = StatusSummary {
        pending: 0,
        in_progress: 0,
        complete: 0,
        failed: 0,
        skipped: 0,
    };
    for (k, v) in rows {
        match k.as_str() {
            "pending" => summary.pending = v,
            "in_progress" => summary.in_progress = v,
            "complete" => summary.complete = v,
            "failed" => summary.failed = v,
            "skipped" => summary.skipped = v,
            _ => {}
        }
    }
    Ok(axum::Json(summary))
}

#[cfg(test)]
mod tests {
    use crate::test_support;
    use axum::http::StatusCode;
    use uuid::Uuid;

    #[tokio::test]
    async fn trigger_requires_auth() {
        let server = test_support::test_server();
        let id = Uuid::new_v4();
        let response = server
            .post(&format!("/api/manifestations/{id}/enrichment/trigger"))
            .await;
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn status_requires_auth() {
        let server = test_support::test_server();
        let response = server.get("/api/enrichment/status").await;
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn dry_run_requires_auth() {
        let server = test_support::test_server();
        let id = Uuid::new_v4();
        let response = server
            .post(&format!("/api/manifestations/{id}/enrichment/dry-run"))
            .await;
        assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    }
}
