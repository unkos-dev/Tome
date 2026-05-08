//! Process-wide state shared across handlers, middleware, and background
//! workers.
//!
//! [`AppState`] is `Clone` (each field is itself cheaply-cloneable: pools are
//! `Arc`-backed; [`Config`] is owned data; [`OidcClient`] is an
//! `Arc`-wrapped metadata bundle). It is built once during
//! [`crate::run`] and threaded through Axum's `with_state`, the auth/session
//! layers, the ingestion watcher, the enrichment queue, and (read-only) the
//! writeback worker.

use sqlx::PgPool;

use crate::auth::oidc::OidcClient;
use crate::config::Config;

/// Cloneable handle to every dependency a request handler or background
/// task needs. Constructed once at startup; threaded through Axum via
/// `with_state` and into spawned tasks via per-task clones.
#[derive(Clone)]
pub struct AppState {
    /// Primary application pool. Connections run as `reverie_app`; every
    /// user-facing query MUST acquire via [`crate::db::acquire_with_rls`]
    /// so RLS policies see `app.current_user_id` set transaction-locally.
    pub pool: PgPool,
    /// Ingestion-pipeline pool. Connections run as `reverie_ingestion`
    /// and exercise the `*_ingestion_full_access` RLS policies. Used by
    /// the watcher, dry-run handlers, and metadata fetchers.
    pub ingestion_pool: PgPool,
    /// Resolved configuration loaded once at startup. Includes finalised
    /// CSP `HeaderValue`s on `config.security` (built in `run` before this
    /// state is constructed).
    pub config: Config,
    /// Pre-discovered OIDC client (issuer metadata + JWKS) for the
    /// login and callback routes. Discovery happens once at startup;
    /// reuse across requests is cheap.
    pub oidc_client: OidcClient,
}
