//! Database pool factories and the RLS-context acquisition helper.
//!
//! Three connection-pool flavours feed the system:
//!
//! * [`init_pool`] — vanilla `PgPool` for the primary application
//!   (`reverie_app`) and the ingestion pipeline (`reverie_ingestion`).
//!   No per-connection setup; RLS context is set transaction-locally
//!   by [`acquire_with_rls`] from the user-facing handlers.
//! * [`init_writeback_pool`] — dedicated pool for the writeback worker
//!   that sets `app.system_context = 'writeback'` once at connect time.
//!   The `manifestations_*_system` RLS policies match only on this GUC,
//!   so user-facing handlers (which never set it) cannot cross into the
//!   system policies even by accident.
//! * [`acquire_with_rls`] — user-scoped transaction wrapper that injects
//!   `app.current_user_id` via `SET LOCAL`-equivalent
//!   `set_config(..., true)`. Auto-resets on commit/rollback so a
//!   recycled connection does not leak the caller's identity to the next
//!   borrower.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Build a vanilla `PgPool` against `database_url` capped at
/// `max_connections`.
///
/// No per-connection initialisation is run — callers needing RLS context
/// must wrap each user-scoped transaction in [`acquire_with_rls`]; the
/// writeback worker uses [`init_writeback_pool`] to set the
/// `app.system_context` GUC instead.
///
/// # Errors
///
/// Returns the underlying `sqlx::Error` when the pool cannot be opened
/// (DSN parse failure, TLS handshake failure, authentication failure,
/// connection refused, etc.).
pub async fn init_pool(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
}

/// Build a `reverie_app` pool dedicated to the writeback worker.
///
/// Every connection opened by this pool runs
/// `SELECT set_config('app.system_context', 'writeback', false)` once at
/// connect time, marking it as a system-context caller for the duration
/// of the connection.  The `manifestations_*_system` RLS policies match
/// only when this GUC is set to `'writeback'`, so no other code path
/// (in particular, no user-facing handler that forgets `SET LOCAL
/// app.current_user_id`) can reach those policies.
///
/// # Errors
///
/// Returns the underlying `sqlx::Error` when the pool cannot be opened
/// (DSN parse, TLS handshake, authentication, connection refused) or
/// when the per-connection
/// `SELECT set_config('app.system_context', 'writeback', false)` call
/// fails during the after-connect handshake — typically a transport-
/// level error, since `set_config` is a Postgres builtin requiring
/// no schema objects or elevated permissions.
pub async fn init_writeback_pool(
    database_url: &str,
    max_connections: u32,
) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SELECT set_config('app.system_context', 'writeback', false)")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
}

/// Acquire a transaction with RLS context set for the given user.
///
/// Uses `set_config('app.current_user_id', ..., true)` where the third
/// argument `true` means "local to current transaction" (equivalent to
/// `SET LOCAL`). The value auto-resets on commit/rollback — safe with
/// connection pools.
///
/// Every user-facing handler that touches RLS-gated tables (works,
/// manifestations, shelves, …) MUST acquire its transaction through this
/// helper. A bare `pool.acquire()` runs without a `current_user_id` and
/// the corresponding RLS policies will reject the read.
///
/// # Errors
///
/// Returns the underlying `sqlx::Error` when the transaction cannot be
/// begun or when the `set_config` call fails (rare — this is the seam
/// the rest of the request relies on, so failures here typically signal
/// pool exhaustion or a connectivity blip).
pub async fn acquire_with_rls(
    pool: &PgPool,
    user_id: uuid::Uuid,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT set_config('app.current_user_id', $1::text, true)")
        .bind(user_id.to_string())
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn acquire_with_rls_sets_session_variable(pool: PgPool) {
        let user_id = uuid::Uuid::new_v4();
        let mut tx = acquire_with_rls(&pool, user_id).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT current_setting('app.current_user_id')")
            .fetch_one(&mut *tx)
            .await
            .unwrap();

        assert_eq!(row.0, user_id.to_string());
        tx.rollback().await.unwrap();
    }
}
