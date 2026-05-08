//! axum-login `AuthnBackend` implementation for Reverie.
//!
//! Bridges the OIDC callback path into axum-login's session management.
//! After the OIDC provider validates the ID token and the handler extracts
//! claims, [`crate::auth::backend::AuthBackend`] upserts the user via
//! [`crate::models::user::upsert_from_oidc_and_maybe_promote`] and hands
//! back the canonical `User` record so axum-login can persist the identity
//! into the session store.
//!
//! # Threat model
//!
//! Credentials flowing through this module are OIDC claims (subject, display
//! name, optional email) that have already been verified by the
//! `openidconnect` library against the provider's JWKS. This module does not
//! re-validate the token; its invariant is that callers only pass
//! [`crate::auth::backend::OidcCredentials`] values derived from a successfully-verified ID token.

use axum_login::{AuthnBackend, UserId};
use sqlx::PgPool;

use crate::models::user::{self, User};

/// Credentials produced after validating an OIDC callback.
///
/// Values are extracted from a verified ID token. Callers must not construct
/// this type from unverified provider responses; the `openidconnect` library's
/// verification step is the trust boundary.
#[derive(Clone)]
pub struct OidcCredentials {
    /// OIDC `sub` claim — stable, provider-scoped user identifier.
    ///
    /// Used as the upsert key in `users.oidc_subject`. Must not be empty.
    pub subject: String,

    /// Human-readable display name from the OIDC `name` claim.
    pub display_name: String,

    /// Email address from the OIDC `email` claim, if the provider includes it.
    pub email: Option<String>,
}

/// Authentication backend that upserts users from verified OIDC claims.
///
/// Implements [`AuthnBackend`] so axum-login can call into Reverie's user
/// model. The backend holds a Postgres pool with schema-owner credentials
/// because the upsert path intentionally runs outside user-context RLS
/// (the user row may not exist yet on first login).
///
/// # Threat model
///
/// The pool carried by this struct bypasses RLS. It must only be used for
/// the OIDC upsert path (`authenticate`) and session-based user reload
/// (`get_user`). Route handlers must never receive or borrow this pool
/// directly; they resolve identity through the `CurrentUser` extractor which
/// operates under `reverie_app` credentials. The `pool` field is `pub(crate)`
/// so that this restriction is enforced at the crate boundary by the type
/// system in addition to convention.
#[derive(Clone)]
pub struct AuthBackend {
    /// Postgres pool with schema-owner credentials.
    ///
    /// Bypasses RLS — used for the OIDC user-upsert and session reload paths
    /// that intentionally run outside user context. Must not be used for
    /// application data queries. Visibility is intentionally `pub(crate)`
    /// rather than `pub`: external callers (and any future consumer module)
    /// must not be able to extract this pool from an `AuthBackend` and use it
    /// for general queries.
    pub(crate) pool: PgPool,
}

impl AuthnBackend for AuthBackend {
    type User = User;
    type Credentials = OidcCredentials;
    type Error = sqlx::Error;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = user::upsert_from_oidc_and_maybe_promote(
            &self.pool,
            &creds.subject,
            &creds.display_name,
            creds.email.as_deref(),
        )
        .await?;
        Ok(Some(user))
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        user::find_by_id(&self.pool, *user_id).await
    }
}
