//! OIDC provider discovery and `OidcClient` construction for Reverie.
//!
//! This module owns the startup-time OIDC handshake: it performs provider
//! discovery against the configured issuer URL, parses the provider metadata
//! (JWKS URI, authorization endpoint, token endpoint), and assembles an
//! [`crate::auth::oidc::OidcClient`] with the redirect URI embedded. The resulting client is
//! stored in [`crate::state::AppState`] and reused for every login flow.
//!
//! ID-token verification and nonce binding happen in the OIDC callback route
//! handler, not here. This module's responsibility ends at client construction.
//!
//! # Threat model — issuer trust boundary
//!
//! The issuer URL is operator-supplied configuration. [`crate::auth::oidc::init_oidc_client`]
//! fetches the discovery document over HTTPS; TLS validation is performed by
//! the underlying `reqwest` client (system roots, no certificate override).
//! An operator pointing `OIDC_ISSUER_URL` at a malicious or compromised
//! provider can induce Reverie to trust attacker-controlled JWKS, enabling
//! ID-token forgery. This is an operator-level threat, not a user-level one;
//! the mitigation is operator key management and issuer selection.

use anyhow::{Context, Result};
use openidconnect::core::{CoreClient, CoreProviderMetadata};
use openidconnect::{
    ClientId, ClientSecret, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, RedirectUrl,
};

use crate::config::Config;

/// Fully-configured OIDC `CoreClient` with `redirect_uri` set.
///
/// The type alias spells out the endpoint state-machine parameters so that
/// callers can use [`OidcClient`] without importing the full generic form.
/// The `EndpointSet` final parameter confirms that `redirect_uri` has been
/// bound, which is required before calling
/// `authorize_url` to start the login flow.
pub type OidcClient = openidconnect::Client<
    openidconnect::EmptyAdditionalClaims,
    openidconnect::core::CoreAuthDisplay,
    openidconnect::core::CoreGenderClaim,
    openidconnect::core::CoreJweContentEncryptionAlgorithm,
    openidconnect::core::CoreJsonWebKey,
    openidconnect::core::CoreAuthPrompt,
    openidconnect::StandardErrorResponse<openidconnect::core::CoreErrorResponseType>,
    openidconnect::StandardTokenResponse<
        openidconnect::IdTokenFields<
            openidconnect::EmptyAdditionalClaims,
            openidconnect::EmptyExtraTokenFields,
            openidconnect::core::CoreGenderClaim,
            openidconnect::core::CoreJweContentEncryptionAlgorithm,
            openidconnect::core::CoreJwsSigningAlgorithm,
        >,
        openidconnect::core::CoreTokenType,
    >,
    openidconnect::StandardTokenIntrospectionResponse<
        openidconnect::EmptyExtraTokenFields,
        openidconnect::core::CoreTokenType,
    >,
    openidconnect::core::CoreRevocableToken,
    openidconnect::StandardErrorResponse<openidconnect::RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

/// Build an HTTP client for OIDC discovery and token exchange.
fn http_client() -> Result<openidconnect::reqwest::Client> {
    openidconnect::reqwest::ClientBuilder::new()
        .build()
        .context("failed to build OIDC HTTP client")
}

/// Discover the OIDC provider and return a client with `redirect_uri` bound.
///
/// Performs an async HTTP GET to `{OIDC_ISSUER_URL}/.well-known/openid-configuration`,
/// parses the provider metadata, and constructs an [`OidcClient`] ready for
/// authorization URL generation. Called once at startup; the resulting client
/// is stored in [`crate::state::AppState`].
///
/// Issuer URL and redirect URI are validated for syntactic correctness before
/// the network call; a malformed URL is an operator configuration error caught
/// at startup rather than at request time.
///
/// # Threat model
///
/// TLS certificate validation is delegated to the `reqwest` default client
/// (system certificate roots). The discovery document is parsed by
/// `openidconnect::CoreProviderMetadata`; malformed documents produce an
/// error rather than a partially-constructed client.
///
/// # Errors
///
/// Returns an error if `OIDC_ISSUER_URL` or `OIDC_REDIRECT_URI` is not a
/// valid URL, if the HTTP client cannot be constructed, or if the provider
/// discovery request fails or returns an unparseable response.
pub async fn init_oidc_client(config: &Config) -> Result<OidcClient> {
    let issuer =
        IssuerUrl::new(config.oidc_issuer_url.clone()).context("invalid OIDC_ISSUER_URL")?;

    let http = http_client()?;
    let provider_metadata = CoreProviderMetadata::discover_async(issuer, &http)
        .await
        .map_err(|e| anyhow::anyhow!("OIDC discovery failed: {e}"))?;

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(config.oidc_client_id.clone()),
        Some(ClientSecret::new(config.oidc_client_secret.clone())),
    )
    .set_redirect_uri(
        RedirectUrl::new(config.oidc_redirect_uri.clone()).context("invalid OIDC_REDIRECT_URI")?,
    );

    Ok(client)
}

/// Build an HTTP client for use in the OIDC token-exchange step.
///
/// The token exchange (authorization code → tokens) requires an HTTP client
/// separate from the one used at discovery time; the `openidconnect` API
/// consumes it by value. This function constructs a fresh client with the
/// same `reqwest` defaults (system TLS roots, no certificate override) as the
/// discovery client.
///
/// Callers in the OIDC callback route use this to perform the confidential
/// client credential exchange; the `client_secret` is transmitted over TLS
/// to the provider's token endpoint.
///
/// # Errors
///
/// Returns an error if `reqwest` cannot initialise its TLS backend.
pub fn exchange_http_client() -> Result<openidconnect::reqwest::Client> {
    http_client()
}
