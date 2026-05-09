//! HTTP route handlers grouped by domain.
//!
//! Each submodule exposes a `pub fn router() -> Router<AppState>` (or a
//! `_enabled` variant returning `Option<Router<…>>` when the mount is
//! conditional) consumed by `crate::build_router_with_session_store`
//! to assemble the application router.

/// Authentication endpoints (OIDC login / callback / logout, session
/// `/auth/me` + theme update).
pub mod auth;
/// Manifestation enrichment trigger / dry-run / status endpoints.
pub mod enrichment;
/// Liveness + readiness probes.
pub mod health;
/// Library-scan trigger endpoint.
pub mod ingestion;
/// Metadata-version review endpoints (accept / reject / revert / lock).
pub mod metadata;
/// OPDS 1.2 catalog routes (feeds, downloads, cover dual-mount).
pub mod opds;
/// Single-page-app asset-serving router (`/assets/*` mount).
pub mod spa;
/// Per-user device-token issue / list / revoke endpoints.
pub mod tokens;
