//! Authentication subsystem for Reverie.
//!
//! Provides cookie-or-Basic credential resolution ([`middleware`]), an
//! axum-login [`AuthnBackend`](axum_login::AuthnBackend) that upserts users
//! from OIDC claims ([`backend`]), OIDC provider discovery and client
//! construction ([`oidc`]), role-assertion helpers ([`middleware::CurrentUser`]),
//! device-token generation and constant-time verification ([`token`]),
//! a Basic-only extractor for OPDS routes ([`basic_only`]), and the FOUC
//! theme-preference cookie ([`theme_cookie`]).
//!
//! # Tier 2 — security-critical
//!
//! All modules in this directory are Tier 2 under the comment policy
//! (`adr/2026-05-08-tiered-comment-policy.md`). Threat annotations
//! (`// THREAT:`) are present on any non-obvious mitigation.

/// Authentication backend: upserts users from validated OIDC claims.
pub mod backend;

/// `BasicOnly` extractor: rejects session cookies, requires `Authorization: Basic`.
pub mod basic_only;

/// `CurrentUser` extractor: resolves identity via session cookie or Basic auth.
pub mod middleware;

/// OIDC provider discovery and `OidcClient` construction.
pub mod oidc;

/// FOUC theme-preference cookie (`reverie_theme`): set/read helpers.
pub mod theme_cookie;

/// Device-token generation and SHA-256 constant-time verification.
pub mod token;
